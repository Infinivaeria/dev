require "json"
require "digest"
require "stringio"
require "tempfile"

module Rustby
  module Bytecode
    MAGIC = "RUSTBY-RBC\x00\x01".b.freeze
    MAX_HEADER = 16_384

    def self.runtime
      {"engine" => RUBY_ENGINE, "version" => RUBY_VERSION, "patchlevel" => RUBY_PATCHLEVEL,
       "platform" => RUBY_PLATFORM, "revision" => RUBY_REVISION.to_s}
    end

    def self.compile(source, output)
      raise "MRI Ruby with InstructionSequence support is required" unless defined?(RubyVM::InstructionSequence)
      payload = RubyVM::InstructionSequence.compile_file(source).to_binary
      header = JSON.generate({"format" => 1, "runtime" => runtime, "size" => payload.bytesize,
                              "sha256" => Digest::SHA256.hexdigest(payload)}).b
      raise "Output already exists: #{output}" if File.exist?(output) || File.symlink?(output)
      Tempfile.create([".rustby-", ".rbc"], File.dirname(output)) do |file|
        file.binmode
        file.write(MAGIC + [header.bytesize].pack("N") + header + payload)
        file.flush
        file.fsync
        File.link(file.path, output)
      end
    end

    def self.read(bytes)
      stream = StringIO.new(bytes.b)
      raise "Not Rustby bytecode" unless stream.read(MAGIC.bytesize) == MAGIC
      size = stream.read(4)
      raise "Truncated bytecode header" unless size&.bytesize == 4
      length = size.unpack1("N")
      raise "Invalid bytecode header length" unless length.between?(1, MAX_HEADER)
      header_bytes = stream.read(length)
      raise "Truncated bytecode metadata" unless header_bytes&.bytesize == length
      header = JSON.parse(header_bytes)
      raise "Unsupported bytecode format" unless header.is_a?(Hash) && header["format"] == 1
      raise "Ruby runtime mismatch; recompile with this Ruby" unless header["runtime"] == runtime
      payload = stream.read
      unless header["size"] == payload.bytesize && header["sha256"] == Digest::SHA256.hexdigest(payload)
        raise "Bytecode checksum or size mismatch"
      end
      payload
    end

    def self.execute(bytes, arguments)
      ARGV.replace(arguments)
      RubyVM::InstructionSequence.load_from_binary(read(bytes)).eval
      0
    rescue SystemExit => error
      error.status
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    input, output = ARGV
    raise "Usage: bytecode.rb INPUT.rb|INPUT.rbc OUTPUT.rbc" unless input && output
    if File.extname(input) == ".rb"
      Rustby::Bytecode.compile(input, output)
    else
      bytes = File.binread(input)
      Rustby::Bytecode.read(bytes)
      File.open(output, File::WRONLY | File::CREAT | File::EXCL, 0o644) { |file| file.write(bytes) }
    end
  rescue StandardError, SyntaxError => error
    warn "rustby: #{error.message}"
    exit 1
  end
end
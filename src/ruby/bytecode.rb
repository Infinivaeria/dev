require "json"
require "digest"
require "tempfile"

module Selenite
  module Bytecode
    MAGIC = "SELENITE-RBC\x00\x01".b.freeze
    MAX_HEADER = 16_384

    class Error < StandardError; end

    def self.runtime
      {
        "engine" => RUBY_ENGINE,
        "version" => RUBY_VERSION,
        "patchlevel" => RUBY_PATCHLEVEL,
        "platform" => RUBY_PLATFORM,
        "revision" => RUBY_REVISION.to_s
      }
    end

    def self.compile(source, output = nil, force = false)
      source = File.expand_path(source)
      output = File.expand_path(output || source.sub(/\.rb\z/, ".rbc"))
      raise Error, "compilation input must be a .rb file" unless File.extname(source) == ".rb"
      raise Error, "compilation output must be a .rbc file" unless File.extname(output) == ".rbc"
      raise Error, "output already exists: #{output} (use --force to replace it)" if File.exist?(output) && !force
      raise Error, "this runtime does not support MRI bytecode" unless defined?(RubyVM::InstructionSequence)

      sequence = RubyVM::InstructionSequence.compile(File.read(source, encoding: "UTF-8"), File.basename(output), File.basename(output), 1)
      payload = sequence.to_binary
      header = JSON.generate({ "format" => 1, "runtime" => runtime, "size" => payload.bytesize,
                               "sha256" => Digest::SHA256.hexdigest(payload) }).b
      Tempfile.create([".selenite-rbc-", ".tmp"], File.dirname(output)) do |temporary|
        temporary.binmode
        temporary.write(MAGIC)
        temporary.write([header.bytesize].pack("N"))
        temporary.write(header)
        temporary.write(payload)
        temporary.flush
        temporary.fsync
        if force
          File.rename(temporary.path, output)
        else
          begin
            File.link(temporary.path, output)
          rescue Errno::EEXIST
            raise Error, "output already exists: #{output} (use --force to replace it)"
          end
        end
      end
      output
    rescue SyntaxError => error
      raise Error, "Ruby compilation failed: #{error.message}"
    end

    def self.read(path)
      File.open(path, "rb") do |file|
        raise Error, "not Selenite .rbc bytecode: #{path}" unless file.read(MAGIC.bytesize) == MAGIC
        length_bytes = file.read(4)
        raise Error, "truncated .rbc header" unless length_bytes&.bytesize == 4
        length = length_bytes.unpack1("N")
        raise Error, "invalid .rbc header length" unless length.between?(1, MAX_HEADER)
        header_bytes = file.read(length)
        raise Error, "truncated .rbc header" unless header_bytes&.bytesize == length
        header = JSON.parse(header_bytes)
        raise Error, "unsupported .rbc format" unless header.is_a?(Hash) && header["format"] == 1
        unless header["runtime"] == runtime
          raise Error, ".rbc runtime mismatch; recompile using the Ruby runtime shipped with this executable"
        end
        payload = file.read
        unless header["size"] == payload.bytesize && header["sha256"] == Digest::SHA256.hexdigest(payload)
          raise Error, ".rbc checksum or size mismatch; recompile the damaged file"
        end
        payload
      end
    rescue JSON::ParserError, EncodingError
      raise Error, "invalid .rbc metadata"
    end

    def self.load(path)
      path = File.realpath(path)
      raise Error, "bytecode input must be a .rbc file" unless File.extname(path) == ".rbc"
      sequence = RubyVM::InstructionSequence.load_from_binary(read(path))
      Dir.chdir(File.dirname(path)) { sequence.eval }
    end

    def self.require(path)
      path = File.realpath(path)
      @loaded ||= {}
      return false if @loaded.key?(path)
      @loaded[path] = true
      begin
        load(path)
      rescue Exception
        @loaded.delete(path)
        raise
      end
      true
    end

    def self.evaluate(path)
      path = File.realpath(path)
      case File.extname(path)
      when ".rbc" then load(path)
      when ".rb" then TOPLEVEL_BINDING.eval(File.read(path, encoding: "UTF-8"), path, 1)
      else raise Error, "Ruby-only runtime: #{path} must have a .rb or .rbc extension"
      end
    rescue SyntaxError => error
      raise Error, "Ruby evaluation failed: #{error.message}"
    end
  end
end

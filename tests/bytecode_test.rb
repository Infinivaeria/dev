require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "open3"
require "rbconfig"
require_relative "../src/ruby/bytecode"

class BytecodeTest < Minitest::Test
  def setup
    @directory = Dir.mktmpdir("selenite-bytecode-")
    @source = File.join(@directory, "game.rb")
    @output = File.join(@directory, "game.rbc")
  end

  def teardown
    FileUtils.remove_entry(@directory)
  end

  def compile(source)
    File.write(@source, source)
    Selenite::Bytecode.compile(@source)
  end

  def rewrite_header
    bytes = File.binread(@output)
    start = Selenite::Bytecode::MAGIC.bytesize
    length = bytes.byteslice(start, 4).unpack1("N")
    header = JSON.parse(bytes.byteslice(start + 4, length))
    yield header
    replacement = JSON.generate(header).b
    File.binwrite(@output, Selenite::Bytecode::MAGIC + [replacement.bytesize].pack("N") +
                           replacement + bytes.byteslice(start + 4 + length..))
  end

  def test_compile_does_not_execute_and_relocated_game_needs_no_source
    marker = File.join(@directory, "executed")
    compile <<~RUBY
      # SOURCE_COMMENT_MUST_NOT_SHIP
      File.write("executed", "yes")
      require_relative "helper"
      result = [File.read(File.join(__dir__, "assets", "message")), bytecode_helper, __FILE__]
      result
    RUBY
    refute File.exist?(marker)
    refute_includes File.binread(@output), "SOURCE_COMMENT_MUST_NOT_SHIP"
    refute_includes File.binread(@output), @directory
    relocated = File.join(@directory, "relocated")
    FileUtils.mkdir_p(File.join(relocated, "assets"))
    File.write(File.join(relocated, "assets", "message"), "relocated")
    File.write(File.join(relocated, "helper.rb"), "def bytecode_helper; 42; end")
    target = File.join(relocated, "game.rbc")
    FileUtils.mv(@output, target)
    File.unlink(@source)
    original_directory = Dir.pwd
    assert_equal ["relocated", 42, "game.rbc"], Selenite::Bytecode.evaluate(target)
    assert_equal original_directory, Dir.pwd
    assert File.exist?(File.join(relocated, "executed"))
    refute File.exist?(marker)
  end

  def test_no_clobber_and_force_replacement
    compile "1 + 1"
    original = File.binread(@output)
    File.write(@source, "2 + 2")
    assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.compile(@source) }
    assert_equal original, File.binread(@output)
    Selenite::Bytecode.compile(@source, @output, true)
    assert_equal 4, Selenite::Bytecode.load(@output)
  end

  def test_embedded_bytecode_runs_without_payload_file_and_keeps_arguments
    compile "[ARGV.dup, File.read('message'), __FILE__]"
    bytes = File.binread(@output)
    File.unlink(@source)
    File.unlink(@output)
    File.write(File.join(@directory, "message"), "asset")
    original_arguments = ARGV.dup
    original_directory = Dir.pwd
    assert_equal [["one argument", "--literal"], "asset", "game.rbc"],
                 Selenite::Bytecode.evaluate_binary(bytes, @directory, ["one argument", "--literal"])
    assert_equal original_directory, Dir.pwd
    assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.evaluate_binary(bytes[0...-1], @directory) }
  ensure
    ARGV.replace(original_arguments) if original_arguments
  end

  def test_embedded_bytecode_preserves_system_exit
    compile "exit 7"
    assert_equal 7, Selenite::Bytecode.execute_binary(File.binread(@output), @directory)
  end

  def test_ruby_bootstrap_has_help_and_refuses_existing_output_without_execution
    bootstrap = File.expand_path("../bootstrap-executable.rb", __dir__)
    output, error, status = Open3.capture3(RbConfig.ruby, bootstrap, "--help")
    assert status.success?, error
    assert_includes output, "--game"
    File.write(@source, "File.write('must-not-run', 'bad')")
    existing = File.join(@directory, "existing")
    FileUtils.mkdir_p(existing)
    File.write(File.join(existing, "personal.txt"), "keep")
    _output, error, status = Open3.capture3(RbConfig.ruby, bootstrap, @source, "--output", existing)
    refute status.success?
    assert_includes error, "already exists"
    assert_equal "keep", File.read(File.join(existing, "personal.txt"))
    refute File.exist?(File.join(existing, "must-not-run"))
  end

  def test_invalid_source_does_not_replace_good_output
    compile "42"
    original = File.binread(@output)
    File.write(@source, "def broken(")
    assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.compile(@source, @output, true) }
    assert_equal original, File.binread(@output)
    assert_empty Dir.glob(File.join(@directory, ".selenite-rbc-*"))
  end

  def test_invalid_source_does_not_create_output
    File.write(@source, "def broken(")
    assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.compile(@source) }
    refute File.exist?(@output)
  end

  def test_invalid_output_and_source_extensions
    File.write(@source, "42")
    assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.compile(@source, @source, true) }
    assert_equal "42", File.read(@source)
    assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.compile(@source.sub(/\.rb$/, ".py")) }
  end

  def test_source_and_bytecode_preserve_unicode_with_ascii_locale
    previous_encoding = Encoding.default_external
    Encoding.default_external = Encoding::US_ASCII
    File.binwrite(@source, '"Selenite — Ruby"')
    assert_equal "Selenite — Ruby", Selenite::Bytecode.evaluate(@source)
    Selenite::Bytecode.compile(@source)
    assert_equal "Selenite — Ruby", Selenite::Bytecode.load(@output)
  ensure
    Encoding.default_external = previous_encoding
  end

  def test_corruption_and_truncation_are_rejected_before_native_loading
    compile "42"
    original = File.binread(@output)
    ["YARB", original.byteslice(0, 5), original.byteslice(0, Selenite::Bytecode::MAGIC.bytesize + 2),
     original.byteslice(0, original.bytesize - 1), original + "extra"].each do |bytes|
      File.binwrite(@output, bytes)
      assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.load(@output) }
    end
    File.binwrite(@output, original)
    rewrite_header { |header| header["sha256"] = "wrong" }
    assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.load(@output) }
  end

  def test_runtime_and_format_mismatches
    compile "42"
    original = File.binread(@output)
    rewrite_header { |header| header["runtime"]["version"] = "0.0.0" }
    error = assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.load(@output) }
    assert_includes error.message, "runtime mismatch"
    File.binwrite(@output, original)
    rewrite_header { |header| header["format"] = 999 }
    assert_raises(Selenite::Bytecode::Error) { Selenite::Bytecode.load(@output) }
  end

  def test_require_runs_once_and_retries_failures
    compile "$selenite_bytecode_count = ($selenite_bytecode_count || 0) + 1"
    assert Selenite::Bytecode.require(@output)
    refute Selenite::Bytecode.require(@output)
    assert_equal 1, $selenite_bytecode_count
    failing = File.join(@directory, "failure.rbc")
    File.write(@source, "raise 'expected failure'")
    Selenite::Bytecode.compile(@source, failing)
    original_directory = Dir.pwd
    2.times { assert_raises(RuntimeError) { Selenite::Bytecode.require(failing) } }
    assert_equal original_directory, Dir.pwd
  end

  def test_compiled_helper_keeps_resolved_assets_after_nested_load
    helpers = File.join(@directory, "helpers")
    FileUtils.mkdir_p(helpers)
    helper_source = File.join(helpers, "helper.rb")
    File.write(helper_source, <<~RUBY)
      module SeleniteBytecodeHelperFixture
        DIRECTORY = File.expand_path(__dir__)
        def self.message
          File.read(File.join(DIRECTORY, "message"))
        end
      end
    RUBY
    File.write(File.join(helpers, "message"), "compiled helper asset")
    Selenite::Bytecode.compile(helper_source)
    File.unlink(helper_source)
    compile <<~RUBY
      Selenite::Bytecode.require(File.join(__dir__, "helpers", "helper.rbc"))
      SeleniteBytecodeHelperFixture.message
    RUBY
    File.unlink(@source)
    assert_equal "compiled helper asset", Selenite::Bytecode.load(@output)
  end
end

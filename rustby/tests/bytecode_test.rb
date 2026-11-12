require "minitest/autorun"
require "tmpdir"
require_relative "../lib/bytecode"

class BytecodeTest < Minitest::Test
  def setup
    @directory = Dir.mktmpdir("rustby-test-")
    @source = File.join(@directory, "hello.rb")
    @output = File.join(@directory, "hello.rbc")
    File.write(@source, "40 + 2")
    Rustby::Bytecode.compile(@source, @output)
    @bytes = File.binread(@output)
  end

  def teardown
    FileUtils.remove_entry(@directory)
  end

  def test_round_trip
    assert_equal 42, RubyVM::InstructionSequence.load_from_binary(Rustby::Bytecode.read(@bytes)).eval
  end

  def test_refuses_overwrite
    assert_raises(RuntimeError) { Rustby::Bytecode.compile(@source, @output) }
  end

  def test_damage_and_truncation
    damaged = @bytes.dup
    damaged.setbyte(damaged.bytesize - 1, damaged.getbyte(-1) ^ 1)
    assert_raises(RuntimeError) { Rustby::Bytecode.read(damaged) }
    [0, 12, 16, 20].each do |length|
      assert_raises(RuntimeError) { Rustby::Bytecode.read(@bytes.byteslice(0, length)) }
    end
  end

  def test_runtime_mismatch
    changed = @bytes.sub(RUBY_VERSION, "0.0.0")
    error = assert_raises(RuntimeError) { Rustby::Bytecode.read(changed) }
    assert_match(/runtime mismatch/, error.message)
  end

  def test_arguments_and_exit_code
    File.write(@source, "raise 'missing argument' unless ARGV == ['hello']; exit 7")
    File.unlink(@output)
    Rustby::Bytecode.compile(@source, @output)
    original = ARGV.dup
    assert_equal 7, Rustby::Bytecode.execute(File.binread(@output), ["hello"])
  ensure
    ARGV.replace(original) if original
  end
end
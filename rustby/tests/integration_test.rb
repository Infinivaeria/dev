require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "open3"
require "json"

class IntegrationTest < Minitest::Test
  SDK = File.expand_path("..", __dir__)

  def setup
    @directory = Dir.mktmpdir("rustby integration ")
  end

  def teardown
    FileUtils.remove_entry(@directory)
  end

  def cli(*arguments)
    Open3.capture3({"RUSTBY_SDK" => SDK}, RbConfig.ruby, File.join(SDK, "bootstrap.rb"), *arguments)
  end

  def build(entry, name, *arguments)
    output = File.join(@directory, name)
    stdout, stderr, status = cli("build", entry, "--name", name, "--output", output, "--offline", *arguments)
    assert status.success?, "Build failed: #{stdout}\n#{stderr}"
    output
  end

  def execute(package, *arguments, direct: false)
    name = JSON.parse(File.read(File.join(package, "package.json"))).fetch("name")
    command = direct ? File.join(package, name) : File.join(package, "run.sh")
    Open3.capture3({"RUBYLIB" => nil, "LD_LIBRARY_PATH" => nil, "RUBYOPT" => nil,
                   "GEM_HOME" => File.join(@directory, "no-gems"), "GEM_PATH" => ""},
                  command, *arguments, chdir: "/tmp")
  end

  def test_source_bootstrap_relocation_and_rebuild
    package = build(File.join(SDK, "examples/hello.rb"), "hello")
    moved = package + " moved"
    File.rename(package, moved)
    [false, true].each do |direct|
      stdout, stderr, status = execute(moved, "first", "with spaces", "--flag", direct: direct)
      assert status.success?, stderr
      assert_equal ["first", "with spaces", "--flag"], JSON.parse(stdout).fetch("arguments")
    end
    stdout, stderr, status = execute(moved, "--rustby-check")
    assert status.success?, stderr
    assert_equal "hello", JSON.parse(stdout).fetch("name")
    assert File.file?(File.join(moved, "bootstrap.rb"))
    assert File.executable?(File.join(moved, "ruby/bin/ruby"))
    source = File.join(moved, "rust-project/payload.rb")
    File.write(source, 'puts "rebuilt bytecode"')
    stdout, stderr, status = Open3.capture3({"RUBY" => RbConfig.ruby}, "bash", File.join(moved, "rust-project/build.sh"), "--offline")
    assert status.success?, "Rebuild failed: #{stdout}\n#{stderr}"
    executable = File.join(moved, "rust-project/target/release/hello")
    stdout, stderr, status = Open3.capture3([executable, executable])
    assert status.success?, stderr
    assert_equal "rebuilt bytecode\n", stdout
  end

  def test_bytecode_input_and_ruby_exit_status
    source = File.join(@directory, "exit.rb")
    bytecode = File.join(@directory, "exit.rbc")
    File.write(source, "exit Integer(ARGV.fetch(0))")
    stdout, stderr, status = cli("compile", source, "--output", bytecode)
    assert status.success?, "#{stdout}\n#{stderr}"
    File.unlink(source)
    package = build(bytecode, "exit-code")
    _stdout, stderr, status = execute(package, "7")
    assert_equal 7, status.exitstatus, stderr
    _stdout, stderr, status = execute(package)
    assert_equal 1, status.exitstatus
    assert_match(/index 0 outside of array bounds|IndexError/, stderr)
  end

  def test_hybrid_rust_and_ruby
    package = build(File.join(SDK, "examples/hybrid.rs"), "hybrid", "--ruby", File.join(SDK, "examples/hello.rb"))
    stdout, stderr, status = execute(package, "hybrid-argument")
    assert status.success?, stderr
    lines = stdout.lines
    assert_equal "Hello from Rust\n", lines.first
    assert_equal ["hybrid-argument"], JSON.parse(lines.last).fetch("arguments")
  end

  def test_native_rust
    package = build(File.join(SDK, "examples/native.rs"), "native")
    stdout, stderr, status = execute(package, direct: true)
    assert status.success?, stderr
    assert_equal "Hello from a compiled Rust script\n", stdout
  end

  def test_invalid_inputs_leave_no_output
    invalid = File.join(@directory, "invalid.rb")
    output = File.join(@directory, "invalid-output")
    File.write(invalid, "def broken(")
    _stdout, _stderr, status = cli("build", invalid, "--output", output, "--offline")
    refute status.success?
    refute File.exist?(output)
    existing = File.join(@directory, "existing")
    Dir.mkdir(existing)
    sentinel = File.join(existing, "keep")
    File.write(sentinel, "untouched")
    _stdout, stderr, status = cli("build", invalid, "--output", existing)
    refute status.success?
    assert_match(/already exists/, stderr)
    assert_equal "untouched", File.read(sentinel)
  end
end
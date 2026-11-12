require "json"
require "rbconfig"

begin
  root = File.realpath(__dir__)
  package = JSON.parse(File.read(File.join(root, "package.json")))
  actual = {"engine" => RUBY_ENGINE, "version" => RUBY_VERSION, "patchlevel" => RUBY_PATCHLEVEL,
            "platform" => RUBY_PLATFORM, "revision" => RUBY_REVISION.to_s}
  raise "Ruby runtime mismatch; launch using run.sh" unless package.fetch("ruby") == actual
  name = package.fetch("name")
  raise "Invalid executable name" unless name.match?(/\A[a-z][a-z0-9-]{0,63}\z/)
  executable = File.join(root, name)
  raise "Executable is missing: #{executable}" unless File.executable?(executable)
  ENV["LD_LIBRARY_PATH"] = [File.join(root, "lib"), ENV["LD_LIBRARY_PATH"]].compact.join(File::PATH_SEPARATOR)
  ENV["RUBYLIB"] = [File.join(root, "lib/ruby"), File.join(root, "lib/ruby-arch"), ENV["RUBYLIB"]].compact.join(File::PATH_SEPARATOR)
  ENV["RUBYOPT"] = nil
  if ARGV == ["--rustby-check"]
    puts JSON.pretty_generate(package.merge("executable" => executable, "interpreter" => RbConfig.ruby))
  else
    exec(executable, *ARGV)
  end
rescue StandardError => error
  warn "rustby bootstrap: #{error.message}"
  exit 1
end
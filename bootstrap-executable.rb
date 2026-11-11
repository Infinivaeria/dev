require_relative "tools/build_executable"

begin
  SeleniteExecutable.build(ARGV)
rescue StandardError => error
  warn "Executable bootstrap failed: #{error.message}"
  exit 1
end
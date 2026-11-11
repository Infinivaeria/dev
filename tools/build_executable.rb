require "optparse"
require "fileutils"
require "json"
require "open3"
require "rbconfig"
require "shellwords"
require "tmpdir"
require "pathname"

module SeleniteExecutable
  def self.toml(value)
    case value
    when Hash then "{ " + value.map { |key, item| "#{JSON.generate(key.to_s)} = #{toml(item)}" }.join(", ") + " }"
    when Array then "[" + value.map { |item| toml(item) }.join(", ") + "]"
    else JSON.generate(value)
    end
  end

  def self.copy_data(source, destination, entry, excluded)
    total = 0
    pending = [[source, destination]]
    until pending.empty?
      directory, output = pending.pop
      FileUtils.mkdir_p(output)
      Dir.children(directory).sort.each do |name|
        path = File.join(directory, name)
        next if excluded.any? { |skip| path == skip || path.start_with?(skip + File::SEPARATOR) }
        next if File.symlink?(path) || path == entry || path == entry.sub(/\.rbc\z/, ".rb")
        if File.directory?(path)
          next if %w[.git target node_modules .bundle sdk dist dist-game].include?(name)
          pending << [path, File.join(output, name)]
        elsif File.file?(path)
          total += File.size(path)
          raise "Supporting files exceed 512 MiB; use a smaller project/data folder" if total > 512 * 1024 * 1024
          FileUtils.cp(path, File.join(output, name))
        end
      end
    end
  end

  def self.build(arguments)
    options = {mode: nil, sdk: File.expand_path("..", __dir__), offline: false}
    parser = OptionParser.new do |flags|
      flags.banner = "Usage: ruby build_executable.rb SCRIPT.rb|SCRIPT.rbc|PROJECT --output DIR [--game|--app] [--name NAME] [--offline]"
      flags.on("--output DIR") { |value| options[:output] = value }
      flags.on("--name NAME") { |value| options[:name] = value }
      flags.on("--game") { options[:mode] = "game" }
      flags.on("--app") { options[:mode] = "app" }
      flags.on("--sdk DIR") { |value| options[:sdk] = value }
      flags.on("--offline") { options[:offline] = true }
      flags.on("-h", "--help") { puts flags; return }
    end
    parser.parse!(arguments)
    raise parser.to_s unless arguments.size == 1 && options[:output]
    raise "Use MRI Ruby 3.1+ built with a shared library" unless RUBY_ENGINE == "ruby" && Gem::Version.new(RUBY_VERSION) >= Gem::Version.new("3.1")
    raise "This executable builder currently supports Linux hosts" unless RUBY_PLATFORM.include?("linux")
    input = File.realpath(arguments.first)
    raise "Build paths cannot contain control characters" if [input, options[:output], options[:sdk]].any? { |path| path.match?(/[\x00-\x1f]/) }
    if File.directory?(input)
      project = JSON.parse(File.read(File.join(input, "selenite-project.json")))
      raise "Unsupported project schema" unless project["schema"] == 1
      relative = project.fetch("entry")
      raise "Project entry must be a relative file inside the project" if Pathname.new(relative).absolute? || relative.split(/[\\\/]/).include?("..")
      entry = File.realpath(File.join(input, relative))
      raise "Project entry escapes its folder" unless entry.start_with?(input + File::SEPARATOR)
      options[:mode] ||= project.fetch("kind")
    else
      entry = input
    end
    raise "Choose Ruby source or Selenite bytecode" unless File.file?(entry) && %w[.rb .rbc].include?(File.extname(entry))
    raise "Entry exceeds 4 MiB" if File.size(entry) > 4 * 1024 * 1024
    options[:mode] ||= "app"
    raise "Mode must be app or game" unless %w[app game].include?(options[:mode])
    name = options[:name] || File.basename(entry, File.extname(entry)).downcase.gsub(/[^a-z0-9_-]/, "-")
    raise "Name must start with a lowercase letter and contain up to 64 letters, numbers, '-' or '_'" unless name.match?(/\A[a-z][a-z0-9_-]{0,63}\z/)
    sdk = File.realpath(options[:sdk])
    raise "Missing Selenite build SDK: #{sdk}" unless File.file?(File.join(sdk, "src/game_main.rs")) && File.file?(File.join(sdk, "build.rs"))
    require File.join(sdk, "src/ruby/bytecode")
    output = File.expand_path(options[:output])
    raise "Output already exists; choose a new folder: #{output}" if File.exist?(output) || File.symlink?(output)
    FileUtils.mkdir_p(File.dirname(output))
    Dir.mktmpdir(".selenite-executable-", File.dirname(output)) do |staging|
      project_dir = File.join(staging, "rust-project")
      runtime = File.join(project_dir, "runtime")
      bundle = File.join(staging, "bundle")
      FileUtils.mkdir_p([runtime, bundle])
      payload = File.join(project_dir, File.basename(entry, File.extname(entry)) + ".rbc")
      if File.extname(entry) == ".rb"
        Selenite::Bytecode.compile(entry, payload)
      else
        Selenite::Bytecode.read(entry)
        FileUtils.cp(entry, payload)
      end
      %w[src assets docs examples].each { |path| FileUtils.cp_r(File.join(sdk, path), File.join(runtime, path)) if File.directory?(File.join(sdk, path)) }
      FileUtils.cp(File.join(sdk, "build.rs"), runtime)
      FileUtils.cp(File.join(sdk, "Cargo.lock"), project_dir) if File.file?(File.join(sdk, "Cargo.lock"))
      FileUtils.cp(File.join(sdk, "rust-toolchain.toml"), project_dir) if File.file?(File.join(sdk, "rust-toolchain.toml"))
      manifest_text, status = Open3.capture2("python3", "-c", "import json,sys,tomllib; print(json.dumps(tomllib.load(open(sys.argv[1], 'rb'))))", File.join(sdk, "Cargo.toml"))
      raise "Reading Cargo.toml requires Python 3.11+" unless status.success?
      manifest = JSON.parse(manifest_text)
      cargo = "[package]\nname = #{toml(name)}\nversion = #{toml(manifest.fetch('package').fetch('version'))}\nedition = \"2021\"\nbuild = \"runtime/build.rs\"\n\n[[bin]]\nname = #{toml(name)}\npath = \"runtime/src/game_main.rs\"\n\n[features]\ndefault = [\"scripting\"]\nscripting = [\"dep:magnus\"]\n\n[dependencies]\n"
      manifest.fetch("dependencies").each { |key, value| cargo << "#{toml(key)} = #{toml(value)}\n" unless key == "arboard" }
      manifest.fetch("target", {}).each { |target, values| cargo << "\n[target.#{toml(target)}.dependencies]\n"; values.fetch("dependencies", {}).each { |key, value| cargo << "#{toml(key)} = #{toml(value)}\n" } }
      cargo << "\n[workspace]\n"
      File.write(File.join(project_dir, "Cargo.toml"), cargo)
      data_root = File.directory?(input) ? input : File.dirname(entry)
      data_directory = Pathname.new(File.dirname(entry)).relative_path_from(Pathname.new(data_root)).to_s
      configuration = {"mode" => options[:mode], "payload" => File.basename(payload), "name" => name, "data_directory" => data_directory, "ruby" => Selenite::Bytecode.runtime}
      File.write(File.join(project_dir, "build-config.json"), JSON.pretty_generate(configuration))
      File.write(File.join(project_dir, "build.sh"), <<~BASH)
        #!/usr/bin/env bash
        set -euo pipefail
        cd -- "$(dirname -- "$0")"
        export RUBY="${RUBY:-ruby}"
        export SELENITE_EMBEDDED_PAYLOAD="$PWD/"#{Shellwords.escape(File.basename(payload))}
        export SELENITE_EMBEDDED_MODE=#{options[:mode]}
        export SELENITE_EMBEDDED_DATA=#{Shellwords.escape(data_directory)}
        "$RUBY" -r ./runtime/src/ruby/bytecode -e 'Selenite::Bytecode.read(ARGV.fetch(0))' "$SELENITE_EMBEDDED_PAYLOAD"
        exec cargo build --release "$@"
      BASH
      File.chmod(0o755, File.join(project_dir, "build.sh"))
      ruby = RbConfig.ruby
      include_paths = Shellwords.split(RbConfig::CONFIG["CPPFLAGS"].to_s).grep(/\A-I./).map { |flag| flag.delete_prefix("-I") }
      environment = {"RUBY" => ruby, "SELENITE_EMBEDDED_PAYLOAD" => payload, "SELENITE_EMBEDDED_MODE" => options[:mode], "SELENITE_EMBEDDED_DATA" => data_directory}
      environment["CPATH"] = (include_paths + ENV.fetch("CPATH", "").split(File::PATH_SEPARATOR)).join(File::PATH_SEPARATOR) unless include_paths.empty?
      command = ["cargo", "build", "--release"]
      command << "--offline" if options[:offline]
      raise "Rust build failed; check Cargo, Ruby shared libraries and native build dependencies" unless system(environment, *command, chdir: project_dir)
      built = File.join(project_dir, "target/release", name)
      metadata, metadata_status = Open3.capture2("cargo", "metadata", "--no-deps", "--format-version", "1", chdir: project_dir)
      built = File.join(JSON.parse(metadata).fetch("target_directory"), "release", name) if metadata_status.success?
      FileUtils.cp(built, File.join(bundle, name))
      copy_data(data_root, File.join(bundle, "data"), entry, [staging, output])
      lib = File.join(bundle, "lib")
      FileUtils.mkdir_p(File.join(lib, "ruby"))
      ruby_libs = Dir.glob(File.join(RbConfig::CONFIG.fetch("libdir"), "libruby*.so*"))
      raise "Ruby must be built with a shared libruby" if ruby_libs.empty?
      ruby_libs.each { |path| FileUtils.cp(path, lib) }
      FileUtils.cp_r(RbConfig::CONFIG.fetch("rubylibdir"), File.join(lib, "ruby"))
      FileUtils.cp_r(RbConfig::CONFIG.fetch("rubyarchdir"), File.join(lib, "ruby-arch"))
      array_lib = File.join(sdk, "partitioned_array/lib")
      FileUtils.mkdir_p(File.join(bundle, "partitioned_array"))
      FileUtils.cp_r(array_lib, File.join(bundle, "partitioned_array/lib")) if File.directory?(array_lib)
      FileUtils.cp(File.join(sdk, "LICENSE"), bundle) if File.file?(File.join(sdk, "LICENSE"))
      File.write(File.join(staging, "build-info.json"), JSON.pretty_generate(configuration.merge("format" => 1, "entry" => File.basename(entry), "selenite" => manifest.fetch("package").fetch("version"))))
      File.write(File.join(staging, "README.md"), "# #{name}\n\nRun bundle/#{name} directly. Keep bundle/lib, bundle/data and bundle/partitioned_array beside it. The entry is embedded Ruby bytecode executed by Rust/Magnus, not Ruby-to-Rust translation or encryption. Arguments become Ruby ARGV. Rebuild with rust-project/build.sh using the Ruby runtime in build-info.json.\n")
      FileUtils.rm_rf(File.join(project_dir, "target")) if File.directory?(File.join(project_dir, "target"))
      File.rename(staging, output)
      Dir.mkdir(staging)
    end
    puts "Built Rust/Magnus executable: #{File.join(output, 'bundle', name)}"
  end
end

if $PROGRAM_NAME == __FILE__ && $PROGRAM_NAME != "-e"
  begin
    require "pathname"
    SeleniteExecutable.build(ARGV)
  rescue StandardError => error
    warn "Executable build failed: #{error.message}"
    exit 1
  end
end
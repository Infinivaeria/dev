require "optparse"
require "fileutils"
require "json"
require "rbconfig"
require "tmpdir"
require "shellwords"
require "open3"

module Rustby
  module Builder
    SDK = File.realpath(ENV.fetch("RUSTBY_SDK", __dir__))

    def self.build(entry, output, options)
      raise "Packaging currently supports Linux hosts only" unless RUBY_PLATFORM.include?("linux")
      raise "Use MRI Ruby built with a shared library" unless RUBY_ENGINE == "ruby" && RbConfig::CONFIG["ENABLE_SHARED"] == "yes"
      entry = File.realpath(entry)
      extension = File.extname(entry)
      raise "Entry must be .rb, .rbc or .rs" unless %w[.rb .rbc .rs].include?(extension)
      raise "--ruby can only inject into a Rust .rs entry" if options[:ruby] && extension != ".rs"
      payload = extension == ".rs" ? options[:ruby] : entry
      payload = File.realpath(payload) if payload
      raise "Injected Ruby must be .rb or .rbc" if payload && !%w[.rb .rbc].include?(File.extname(payload))
      name = options[:name] || File.basename(entry, extension).downcase.tr("_", "-")
      raise "Invalid name; use a lowercase letter followed by up to 63 lowercase letters, digits or hyphens" unless name.match?(/\A[a-z][a-z0-9-]{0,63}\z/)
      output = File.expand_path(output)
      raise "Output already exists: #{output}" if File.exist?(output) || File.symlink?(output)
      raise "Build paths cannot contain control characters" if [entry, payload, output].compact.any? { |path| path.match?(/[\x00-\x1f]/) }
      FileUtils.mkdir_p(File.dirname(output))
      Dir.mktmpdir(".rustby-", File.dirname(output)) do |staging|
        project = File.join(staging, "rust-project")
        runtime = File.join(project, "runtime")
        FileUtils.mkdir_p(runtime)
        %w[Cargo.toml build.rs src lib bootstrap.rb README.md LICENSE licenses].each { |path| FileUtils.cp_r(File.join(SDK, path), runtime) }
        File.write(File.join(project, "Cargo.toml"), <<~TOML)
          [package]
          name = #{JSON.generate(name)}
          version = "0.1.0"
          edition = "2021"

          [dependencies]
          rustby = { path = "runtime" }

          [workspace]
          exclude = ["runtime"]
        TOML
        FileUtils.cp(File.join(SDK, "Cargo.lock"), project) if File.file?(File.join(SDK, "Cargo.lock"))
        FileUtils.mkdir_p(File.join(project, "src"))
        FileUtils.cp(entry, File.join(project, "src/entry.rs")) if extension == ".rs"
        payload_name = payload && "payload#{File.extname(payload)}"
        FileUtils.cp(payload, File.join(project, payload_name)) if payload
        FileUtils.cp(File.join(SDK, "lib/bytecode.rb"), File.join(project, "bytecode.rb"))
        File.write(File.join(project, "build.rs"), build_script(payload_name))
        main = payload ? <<~RUST : ""
          mod embedded {
              pub fn run() -> Result<i32, String> {
                  rustby::configure_runtime()?;
                  unsafe {
                      rustby::run(include_bytes!(concat!(env!("OUT_DIR"), "/payload.rbc")),
                          std::env::args().skip(1))
                  }
              }
          }
        RUST
        main += if extension == ".rs"
          "include!(\"entry.rs\");\n"
        else
          <<~RUST
            fn main() {
                let code = match embedded::run() {
                    Ok(code) => code,
                    Err(error) => { eprintln!("rustby: {error}"); 1 }
                };
                std::process::exit(code);
            }
          RUST
        end
        File.write(File.join(project, "src/main.rs"), main)
        File.write(File.join(project, "build.sh"), <<~BASH)
          #!/usr/bin/env bash
          set -euo pipefail
          cd -- "$(dirname -- "$0")"
          export RUBY="${RUBY:-ruby}"
          exec cargo build --release "$@"
        BASH
        File.chmod(0o755, File.join(project, "build.sh"))
        command = ["cargo", "build", "--release", "--target-dir", File.join(project, "target")]
        command << "--offline" if options[:offline]
        includes = Shellwords.split(RbConfig::CONFIG["CPPFLAGS"].to_s).grep(/\A-I./).map { |flag| flag.delete_prefix("-I") }
        environment = {"RUBY" => RbConfig.ruby}
        environment["CPATH"] = (includes + ENV.fetch("CPATH", "").split(File::PATH_SEPARATOR)).join(File::PATH_SEPARATOR) unless includes.empty?
        raise "Cargo build failed" unless system(environment, *command, chdir: project)
        FileUtils.cp(File.join(project, "target/release", name), File.join(staging, name))
        bundle_runtime(staging)
        information = {"format" => 1, "name" => name, "ruby" => Rustby::Bytecode.runtime,
                       "entry_kind" => extension.delete_prefix("."), "embedded_ruby" => !payload.nil?}
        File.write(File.join(staging, "package.json"), JSON.pretty_generate(information))
        FileUtils.cp(File.join(SDK, "lib/package_bootstrap.rb"), File.join(staging, "bootstrap.rb"))
        File.write(File.join(staging, "run.sh"), <<~BASH)
          #!/usr/bin/env bash
          set -euo pipefail
          root="$(cd -- "$(dirname -- "$0")" && pwd)"
          export LD_LIBRARY_PATH="$root/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
          export RUBYLIB="$root/lib/ruby:$root/lib/ruby-arch${RUBYLIB:+:$RUBYLIB}"
          unset RUBYOPT
          exec "$root/ruby/bin/ruby" "$root/bootstrap.rb" "$@"
        BASH
        File.chmod(0o755, File.join(staging, "run.sh"))
        FileUtils.cp(File.join(SDK, "README.md"), File.join(staging, "RUSTBY.md"))
        FileUtils.cp(File.join(SDK, "LICENSE"), staging)
        FileUtils.cp_r(File.join(SDK, "licenses"), staging)
        compiler, compiler_status = Open3.capture2("rustc", "-vV")
        host = compiler.lines.find { |line| line.start_with?("host: ") }&.delete_prefix("host: ")&.strip
        raise "Could not determine Rust host target" unless compiler_status.success? && host && !host.empty?
        metadata, status = Open3.capture2("cargo", "metadata", "--format-version", "1", "--offline", "--filter-platform", host, chdir: project)
        raise "Could not collect dependency license notices" unless status.success?
        JSON.parse(metadata).fetch("packages").each do |package|
          next if package.fetch("source").nil?
          directory = File.dirname(package.fetch("manifest_path"))
          notices = Dir.glob(File.join(directory, "{LICENSE*,COPYING*,NOTICE*}"), File::FNM_EXTGLOB).select { |path| File.file?(path) }
          next if notices.empty?
          destination = File.join(staging, "licenses", "#{package.fetch('name')}-#{package.fetch('version')}")
          FileUtils.mkdir_p(destination)
          notices.each { |path| FileUtils.cp(path, destination) }
        end
        FileUtils.rm_rf(File.join(project, "target"))
        File.rename(staging, output)
        Dir.mkdir(staging)
      end
      puts "Built #{File.join(output, name)}"
      output
    end

    def self.build_script(payload)
      compile = if payload
        <<~RUST
          let input = #{JSON.generate(payload)};
          println!("cargo:rerun-if-changed={input}");
          let target = std::path::PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("payload.rbc");
          if target.exists() { std::fs::remove_file(&target).unwrap(); }
          let status = Command::new(&ruby).args(["bytecode.rb", input]).arg(&target).status().expect("Could not compile Ruby bytecode");
          assert!(status.success(), "Ruby bytecode compilation failed");
        RUST
      end
      <<~RUST
        use std::{env, process::Command};
        fn main() {
            println!("cargo:rerun-if-env-changed=RUBY");
            println!("cargo:rerun-if-changed=bytecode.rb");
            let ruby = env::var_os("RUBY").unwrap_or_else(|| "ruby".into());
            let output = Command::new(&ruby).args(["-rrbconfig", "-e", "print RbConfig::CONFIG.fetch('libdir')"]).output().expect("Ruby is required");
            assert!(output.status.success(), "Could not query Ruby");
            let directory = String::from_utf8(output.stdout).expect("Ruby path must be UTF-8");
            assert!(!directory.contains(['\\n', '\\r', ',']), "Unsupported Ruby path");
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/lib");
            println!("cargo:rustc-link-arg=-Wl,-rpath,{directory}");
            #{compile}
        }
      RUST
    end

    def self.bundle_runtime(output)
      FileUtils.mkdir_p([File.join(output, "ruby/bin"), File.join(output, "lib/ruby"), File.join(output, "lib/ruby-arch")])
      FileUtils.cp(File.realpath(RbConfig.ruby), File.join(output, "ruby/bin/ruby"))
      libraries = Dir.glob(File.join(RbConfig::CONFIG.fetch("libdir"), "libruby*.so*"))
      raise "Could not locate shared libruby" if libraries.empty?
      libraries.each { |path| FileUtils.cp(path, File.join(output, "lib")) }
      FileUtils.cp_r(File.join(RbConfig::CONFIG.fetch("rubylibdir"), "."), File.join(output, "lib/ruby"))
      FileUtils.cp_r(File.join(RbConfig::CONFIG.fetch("rubyarchdir"), "."), File.join(output, "lib/ruby-arch"))
    end

    def self.main(arguments)
      require File.join(SDK, "lib/bytecode")
      command = arguments.shift
      options = {}
      parser = OptionParser.new do |flags|
        flags.banner = "Usage: rustby build ENTRY.rb|ENTRY.rbc|ENTRY.rs --output DIR [--ruby SCRIPT.rb|SCRIPT.rbc]\n       rustby compile SCRIPT.rb --output FILE.rbc\n       rustby run ENTRY [options] -- [arguments]\n       rustby doctor"
        flags.on("--output PATH") { |value| options[:output] = value }
        flags.on("--name NAME") { |value| options[:name] = value }
        flags.on("--ruby PATH") { |value| options[:ruby] = value }
        flags.on("--offline") { options[:offline] = true }
        flags.on("-h", "--help") { puts flags; return 0 }
      end
      if command.nil? || %w[help --help -h].include?(command)
        puts parser
        return 0
      end
      if command == "doctor"
        puts JSON.pretty_generate(Rustby::Bytecode.runtime.merge("ruby" => RbConfig.ruby, "shared" => RbConfig::CONFIG["ENABLE_SHARED"], "sdk" => SDK))
        return system("cargo", "--version") ? 0 : 1
      end
      separator = arguments.index("--")
      runtime_arguments = separator ? arguments.slice!(separator..)[1..] : []
      parser.parse!(arguments)
      raise parser.to_s unless arguments.length == 1
      case command
      when "compile"
        raise "compile needs --output FILE.rbc and a .rb input" unless options[:output]&.end_with?(".rbc") && arguments.first.end_with?(".rb")
        Rustby::Bytecode.compile(arguments.first, options[:output])
        puts "Compiled #{options[:output]}"
        0
      when "build"
        raise "build needs --output DIR" unless options[:output]
        build(arguments.first, options[:output], options)
        0
      when "run"
        raise "run does not accept --output" if options[:output]
        Dir.mktmpdir("rustby-run-") do |temporary|
          package = build(arguments.first, File.join(temporary, "app"), options)
          system(File.join(package, "run.sh"), *runtime_arguments)
          $?.exitstatus || 1
        end
      else
        raise "Unknown command: #{command}"
      end
    end
  end
end

begin
  exit Rustby::Builder.main(ARGV)
rescue StandardError, SyntaxError => error
  warn "rustby: #{error.message}"
  exit 1
end
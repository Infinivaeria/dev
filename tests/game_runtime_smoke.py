import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import tomllib


def run(command, *, environment, directory, success=True):
    result = subprocess.run(command, env=environment, cwd=directory,
                            text=True, capture_output=True, timeout=25)
    if success and result.returncode != 0:
        raise AssertionError(result.stdout + result.stderr)
    if not success and result.returncode == 0:
        raise AssertionError(f"Unexpected success: {command}")
    return result


ASSET_SOURCE = '''root = Selenite.grid
if ENV.fetch("ASSET_PHASE") == "create"
    root.new_grid(-7, 3)
    child = root.subgrid(-7, 3)
    %w[first second].each do |folder|
        child.import(2, 4, File.join(ENV.fetch("ASSET_SOURCE"), folder, "shared.txt"))
    end
    child.inventory.add("tokens", 7)
    child.inventory.set_meta("tokens", "source", "integration")
end
child = root.subgrid(-7, 3)
raise "nested inventory missing" unless child.inventory["tokens"] == 7
raise "metadata missing" unless child.inventory.meta("tokens")["source"] == "integration"
raise "wrong stack count" unless child.stack_size(2, 4) == 2
raise "nested assets leaked" unless root.assets.size == 1 && child.assets.size == 2
paths = child.assets.map(&:path)
raise "name collision" unless paths.uniq.size == 2
raise "wrong asset data" unless paths.map { |path| File.read(path) }.sort == ["first asset", "second asset"]
raise "wrong folder" unless paths.all? { |path| path.start_with?(child.assets_dir + File::SEPARATOR) }
raise "missing manager API" unless Selenite.respond_to?(:asset_inventory)
def factorial(number); number <= 1 ? 1 : number * factorial(number - 1); end
raise "recursion failed" unless factorial(10) == 3628800
counter = 0
tape = Hash.new(0)
while counter < 100
    tape[counter] = counter * counter
    counter += 1
end
raise "loop failed" unless tape[99] == 9801
Selenite.save
puts "ASSET_INVENTORY_OK"
'''


def asset_inventory(organizer, workspace, environment):
        assets = workspace / "original assets"
        (assets / "first").mkdir(parents=True)
        (assets / "second").mkdir()
        (assets / "first/shared.txt").write_text("first asset")
        (assets / "second/shared.txt").write_text("second asset")
        source = workspace / "asset-integration.rb"
        source.write_text(ASSET_SOURCE)
        save = workspace / "asset-integration.json"
        for phase in ("create", "reload"):
                asset_environment = {**environment, "ASSET_PHASE": phase, "ASSET_SOURCE": str(assets)}
                result = run([str(organizer), "--script", str(source), "--grid", str(save)],
                                         environment=asset_environment, directory=workspace)
                assert "ASSET_INVENTORY_OK" in result.stdout
                if phase == "create":
                        shutil.rmtree(assets)
        assert save.is_file()
        print("PASS: packaged per-grid multi-asset import, same-name copies, isolated nested inventories, metadata, reload without originals, Ruby loops and recursion")


def native_inventory(repository, organizer, workspace, environment, artifacts):
    from PIL import Image, ImageStat

    driver = workspace / "x11-inventory-driver"
    subprocess.run(["cc", "-Wall", "-Wextra", "-Werror", str(repository / "tests/x11_inventory_driver.c"),
                    "-lX11", "-o", str(driver)], check=True)
    home = workspace / "native-home"
    profile = home / "profiles/default"
    profile.mkdir(parents=True)
    (profile / "init.rb").write_text("Selenite.asset_inventory\n")
    sources = workspace / "native original assets"
    sources.mkdir()
    for name in ("first.txt", "second.txt", "third.txt"):
        (sources / name).write_text(name)
    process = subprocess.Popen([str(organizer), "--profile", "default"], cwd=workspace,
                               env={**environment, "SELENITE_HOME": str(home)},
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    save = profile / "selenite.json"
    try:
        deadline = time.monotonic() + 15
        while True:
            if process.poll() is not None:
                raise AssertionError("Organizer exited early: " + "".join(process.communicate()))
            probe = subprocess.run([str(driver), str(process.pid), "capture", str(workspace / "panel.ppm")],
                                   capture_output=True, timeout=5)
            if probe.returncode == 0:
                with Image.open(workspace / "panel.ppm") as image:
                    if sum(ImageStat.Stat(image).stddev) > 20:
                        break
            assert time.monotonic() < deadline, "Asset Inventory window did not render"
            time.sleep(0.05)
        for phase, names in (("blank", ("first.txt", "second.txt")), ("row", ("third.txt",))):
            uris = workspace / "drop-uris.txt"
            uris.write_text("".join((sources / name).as_uri() + "\r\n" for name in names))
            run([str(driver), str(process.pid), "drop", str(uris), phase],
                environment=environment, directory=workspace)
            deadline = time.monotonic() + 10
            while True:
                if save.is_file():
                    try:
                        state = json.loads(save.read_text())
                        stacks = [cell["items"] for cell in state["root"]["cells"]]
                        if len(stacks) == 1 and len(stacks[0]) == (2 if phase == "blank" else 3):
                            break
                    except (json.JSONDecodeError, KeyError):
                        pass
                assert time.monotonic() < deadline, f"Native {phase} drop was not saved"
                time.sleep(0.05)
        run([str(driver), str(process.pid), "capture", str(workspace / "panel.ppm")],
            environment=environment, directory=workspace)
        with Image.open(workspace / "panel.ppm") as image:
            assert image.size == (960, 720)
            assert sum(ImageStat.Stat(image).stddev) > 20
            image.save(artifacts / "asset-inventory.png")
        run([str(driver), str(process.pid), "close"], environment=environment, directory=workspace)
        output, errors = process.communicate(timeout=10)
        assert process.returncode == 0, output + errors
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate()
    shutil.rmtree(sources)
    check = workspace / "native-reload.rb"
    check.write_text('assets = Selenite.assets; raise "wrong count" unless assets.size == 3; '
                     'raise "wrong data" unless assets.map { |item| File.read(item.path) }.sort == '
                     '["first.txt", "second.txt", "third.txt"]; puts "NATIVE_RELOAD_OK"')
    result = run([str(organizer), "--script", str(check), "--grid", str(save)],
                 environment=environment, directory=workspace)
    assert "NATIVE_RELOAD_OK" in result.stdout
    print("PASS: native X11 multi-file drop into blank Asset Inventory, drop into an existing stack, rendered panel, reload without originals")


def native_mic_freq(repository, runtime, workspace, environment, artifacts):
    from PIL import Image, ImageStat

    driver = workspace / "x11-mic-freq-driver"
    subprocess.run(["cc", "-Wall", "-Wextra", "-Werror", str(repository / "tests/x11_key_driver.c"),
                    "-lX11", "-o", str(driver)], check=True)
    config = workspace / "mic-freq-config"
    (config / "mic_freq").mkdir(parents=True)
    old_assets = workspace / "old installation artwork"
    old_assets.mkdir()
    (config / "mic_freq/settings.json").write_text(json.dumps({"assets": str(old_assets)}))
    mic_environment = {**environment, "XDG_CONFIG_HOME": str(config)}
    process = subprocess.Popen([str(runtime / "mic_freq/mic_freq"), "--capture"],
                               env=mic_environment, cwd=workspace, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True)
    screenshot = workspace / "mic-freq.ppm"
    try:
        deadline = time.monotonic() + 20
        while True:
            if process.poll() is not None:
                raise AssertionError("mic_freq exited before rendering: " + "".join(process.communicate()))
            capture = subprocess.run([str(driver), str(process.pid), "capture:" + str(screenshot)],
                                     capture_output=True, timeout=12)
            if capture.returncode == 0:
                with Image.open(screenshot) as image:
                    if min(image.size) >= 100 and sum(ImageStat.Stat(image.convert("RGB")).stddev) > 20:
                        image.save(artifacts / "mic-freq-portable.png")
                        break
            assert time.monotonic() < deadline, "Relocated mic_freq did not render its avatar"
            time.sleep(0.05)
        run([str(driver), str(process.pid), "close"], environment=mic_environment, directory=workspace)
        output, errors = process.communicate(timeout=10)
        assert process.returncode == 0, output + errors
        assert f"Avatar images: {runtime / 'mic_freq/assets'}" in output, output
        saved = json.loads((config / "mic_freq/settings.json").read_text())
        assert saved["assets"] == str(runtime / "mic_freq/assets")
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate()
    print("PASS: relocated mic_freq direct executable, adjacent artwork priority, rendered avatar, isolated preferences and clean shutdown")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--graphics", action="store_true")
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--artifacts", type=Path)
    options = parser.parse_args()
    repository = Path(__file__).resolve().parent.parent
    with (repository / "Cargo.toml").open("rb") as manifest:
        version = tomllib.load(manifest)["package"]["version"]
    archive = options.archive or repository / f"selenite-game-{version}-portable.tar.gz"
    assert archive.is_file(), "Build the requested portable package first"

    with tempfile.TemporaryDirectory(prefix="selenite-game-smoke-") as temporary:
        workspace = Path(temporary)
        artifacts = options.artifacts or workspace
        artifacts.mkdir(parents=True, exist_ok=True)
        runtime = workspace / "relocated runtime"
        runtime.mkdir()
        subprocess.run(["tar", "-xzf", str(archive), "-C", str(runtime)], check=True)
        for filename in ("README.md", "CHANGELOG.md", "LICENSE", "docs/manual.md",
                         "docs/packaging.md", "partitioned_array/lib/managed_partitioned_array.rb",
                         "licenses/LICENSE-DejaVu.txt", "package-info.json"):
            assert (runtime / filename).is_file(), f"Missing package file: {filename}"
        document_manifest = json.loads((runtime / "documentation-manifest.json").read_text())
        assert document_manifest["schema"] == 1
        excluded_paths = {repository / path for path in document_manifest["excluded_source_directories"]}
        ignored_names = {".git", "target", "node_modules", ".venv", "__pycache__", "tmp", "test-artifacts"}
        documentation = []
        for directory, folders, filenames in os.walk(repository):
            parent = Path(directory)
            kept = []
            for folder in folders:
                child = parent / folder
                first = child.relative_to(repository).parts[0]
                generated = first == "dist" or first.startswith(("dist-", "dist_"))
                if folder not in ignored_names and child not in excluded_paths and not generated:
                    kept.append(folder)
            folders[:] = kept
            documentation.extend(parent / filename for filename in filenames
                                 if Path(filename).suffix.lower() == ".md" and (parent / filename).is_file())
        expected_documents = {source.relative_to(repository).as_posix() for source in documentation}
        packaged_documents = {entry["path"] for entry in document_manifest["documents"]}
        assert len(packaged_documents) == len(document_manifest["documents"]), "Duplicate documentation entries"
        assert packaged_documents == expected_documents, f"Documentation set mismatch: {packaged_documents ^ expected_documents}"
        for entry in document_manifest["documents"]:
            bundled = runtime / entry["path"]
            assert bundled.is_file(), f"Missing package document: {entry['path']}"
            assert hashlib.sha256(bundled.read_bytes()).hexdigest() == entry["sha256"], f"Document checksum mismatch: {entry['path']}"
        documentation.extend((repository / "partitioned_array").glob("*.txt"))
        for source in documentation:
            relative = source.relative_to(repository)
            bundled = runtime / relative
            assert bundled.is_file(), f"Missing package documentation: {relative}"
            assert bundled.read_bytes() == source.read_bytes(), f"Outdated package documentation: {relative}"
        print(f"PASS: all {len(expected_documents)} Selenite/product Markdown documents, original paths and checksums")
        package_info = json.loads((runtime / "package-info.json").read_text())
        assert package_info["schema"] == 1
        assert package_info["name"] in ("selenite", "selenite-game")
        expected_binaries = ["selenite", "selenite-game", "mic_freq/mic_freq"] if package_info["name"] == "selenite" else ["selenite-game"]
        assert package_info["binaries"] == expected_binaries
        assert (runtime / "run.sh").is_file() == ("selenite" in expected_binaries)
        assert package_info["version"] == version
        assert package_info["release_channel"] == ("release_candidate" if "-rc." in version else "stable")
        assert package_info["scripting"] is True
        assert package_info["capabilities"]["projects"] is True
        assert package_info["capabilities"]["native_raylib_functions"] == 600
        assert package_info["capabilities"]["ide"] == ("selenite" in expected_binaries)
        assert (runtime / "run-ide.sh").is_file() == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["mic_freq"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["audio_composer"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["chiptunes"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["wav_audio_import_export"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["ogg_audio_import"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["midi_import"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["music_theory"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["pitch_analysis"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["harmonic_analysis"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["rhythm_analysis"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["live_audio_analysis"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["microphone_tools"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["pitch_tuner"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["frequency_spectrum"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["input_diagnostics"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["guided_mic_calibration"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["microphone_scripting"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["mic_freq_turing_complete"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["graphics_studio"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["image_layers"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["image_filters"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["animated_gif"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["video_tools"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["portable_ffmpeg"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["pngtuber_graphics"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["direct_suite_launch"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["computer_algebra"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["symbolic_math"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["numerical_linear_algebra"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["cas_notebook"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["direct_executables"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["shell_shims"] == ("selenite" in expected_binaries)
        assert package_info["capabilities"]["rustby_sdk"] is True
        assert package_info["capabilities"]["complete_documentation"] is True
        assert (runtime / "run-mic-freq.sh").is_file() == ("selenite" in expected_binaries)
        assert all((runtime / binary).is_file() for binary in package_info["binaries"])
        if "selenite" in expected_binaries:
            component = package_info["components"]["mic_freq"]
            with (repository / "mic_freq/Cargo.toml").open("rb") as manifest:
                assert component["version"] == tomllib.load(manifest)["package"]["version"]
            assert component["binary"] == "mic_freq/mic_freq"
            assert component["launcher"] == "mic_freq/mic_freq"
            assert component["shell_shim"] == "run-mic-freq.sh"
            assert component["assets"] == "mic_freq/assets"
            assert component["manuals"] == ["mic_freq/README.md", "mic_freq/MANUAL.md"]
            assert component["examples"] == "mic_freq/examples"
            examples = runtime / component["examples"]
            assert (examples / "README.md").is_file()
            scripts = list(examples.glob("*.rhai"))
            assert len(scripts) >= 30
            assert all((runtime / manual).is_file() for manual in component["manuals"])
            assert (runtime / component["assets"]).is_dir()
            for source in (repository / "mic_freq/assets").rglob("*"):
                if source.is_file():
                    bundled = runtime / "mic_freq/assets" / source.relative_to(repository / "mic_freq/assets")
                    assert bundled.is_file() and bundled.read_bytes() == source.read_bytes(), f"Missing or outdated companion asset: {source}"
        environment = os.environ.copy()
        for variable in ("RUBYLIB", "RUBYOPT", "RUBY_YJIT_ENABLE", "GEM_HOME", "GEM_PATH",
                         "LD_LIBRARY_PATH", "SELENITE_PARTITIONED_ARRAY_LIB"):
            environment.pop(variable, None)
        environment["SELENITE_HOME"] = str(workspace / "organizer-home")
        if "selenite" in expected_binaries:
            result = run([str(runtime / "mic_freq/mic_freq"), "--help"], environment=environment, directory=workspace)
            assert result.stdout.startswith("mic_freq [avatar-folder]")
            assert "--device" in result.stdout and "--capture" in result.stdout
            print("PASS: extracted mic_freq direct executable, artwork, manuals, component metadata and help")
            run([str(runtime / "run-ide.sh"), "--version"], environment=environment, directory=workspace)
            if (runtime / "run-mic-freq.sh").is_file():
                run([str(runtime / "run-mic-freq.sh"), "--help"], environment=environment, directory=workspace)
            poses = workspace / "generated pngtuber poses"
            run([str(runtime / "selenite"), "--graphics-pngtuber",
                 str(runtime / "mic_freq/assets/idle.png"), str(poses)],
                environment=environment, directory=workspace)
            assert all((poses / name).is_file() for name in (
                "idle.png", "idle_blink.png", "talking.png", "talking_blink.png",
                "loud.png", "loud_blink.png",
            ))
            run([str(runtime / "mic_freq/mic_freq"), "--self-test"],
                environment=environment, directory=workspace)
            rustby_manifest = json.loads((runtime / "rustby/rustby-manifest.json").read_text())
            assert rustby_manifest["schema"] == 1 and rustby_manifest["files"]
            for entry in rustby_manifest["files"]:
                source = runtime / "rustby" / entry["path"]
                assert source.is_file()
                assert hashlib.sha256(source.read_bytes()).hexdigest() == entry["sha256"]
            assert (runtime / "rustby/Cargo.toml").is_file()
            assert (runtime / "rustby/src").is_dir()
            result = run([str(runtime / "selenite"), "--cas-run", "diff x^3+2*x; x"],
                         environment=environment, directory=workspace)
            assert "3*x^2" in result.stdout
            ffmpeg = runtime / "ffmpeg/bin/ffmpeg"
            ffprobe = runtime / "ffmpeg/bin/ffprobe"
            assert ffmpeg.is_file() and ffprobe.is_file()
            bundled_environment = {**environment, "LD_LIBRARY_PATH": str(runtime / "lib")}
            run([str(ffmpeg), "-version"], environment=bundled_environment, directory=workspace)
            run([str(ffprobe), "-version"], environment=bundled_environment, directory=workspace)
        run([str(runtime / "run-game.sh"), "--version"], environment=environment, directory=workspace)
        for script in runtime.glob("run*.sh"):
            script.unlink()
        launcher = str(runtime / "selenite-game")
        if shutil.which("ldd"):
            linked = run(["ldd", launcher], environment=environment, directory=workspace)
            ruby = next(line for line in linked.stdout.splitlines() if "libruby" in line)
            assert str(runtime / "lib") in ruby, f"Ruby was not loaded from the relocated package: {ruby}"
        result = run([launcher, "--version"], environment=environment, directory=workspace)
        assert result.stdout.strip() == f"selenite-game {version}"
        for arguments in (["game.py"], ["game.js"], ["--script", "game.rb"],
                          ["--profile", "default"], ["missing.rb"],
                          ["game.rb", "--grid", "game.rb"]):
            run([launcher, *arguments], environment=environment, directory=workspace, success=False)

        jit_opt_out = workspace / "jit-opt-out.rb"
        jit_opt_out.write_text('raise "YJIT opt-out ignored" unless ENV["RUBY_YJIT_ENABLE"] == "0"; '
                               'raise "YJIT enabled" if defined?(RubyVM::YJIT) && RubyVM::YJIT.enabled?')
        run([launcher, str(jit_opt_out)], environment={**environment, "RUBY_YJIT_ENABLE": "0"},
            directory=workspace)

        fixture = workspace / "integration.rb"
        fixture.write_text('''require "json"
require "digest"
require "zlib"
require "socket"
raise "YJIT launcher default missing" unless ENV["RUBY_YJIT_ENABLE"] == "1"
if defined?(RubyVM::YJIT)
  raise "YJIT disabled" unless RubyVM::YJIT.enabled?
end
if RUBY_VERSION.split(".").first.to_i >= 4
  require "prism"
  raise "Prism parse failed" unless Prism.parse("1 + 2").success?
end
raise "wrong version" unless Selenite::VERSION == "VERSION"
raise "not game mode" unless Selenite::Game.game_mode?
api = Selenite.raylib_native.enable!
raise "native API incomplete" unless api::API["functions"].size == 600
raise "native records broken" unless api.ColorFromHSV(120, 1, 1).to_a == [0, 255, 0, 255]
raise "digest missing" unless Digest::SHA256.hexdigest("ruby").size == 64
raise "zlib missing" unless Zlib::Inflate.inflate(Zlib::Deflate.deflate("ruby")) == "ruby"
game = Selenite::Game.new(audio: false)
store = game.storage(path: File.join(__dir__, "game-data", "fixture"), partition_size: 2)
records = store.table("records")
unless records[0]
  8.times { |index| records.insert(value: index) }
end
records.sleep!
raise "wrong record" unless records[7]["value"] == 7
raise "not revenant" unless records.status(records.partition_for(7)) == :revenant
tree = Selenite::BSP.new(bounds: [0, 0, 200, 200], depth: 2)
raise "BSP missing" unless tree.leaves.size == 4
game.save
store.close
puts "RUBY_STORAGE_OK"
'''.replace('"VERSION"', json.dumps(version)))
        for _iteration in range(2):
            result = run([launcher, str(fixture)], environment=environment, directory=workspace)
            assert "RUBY_STORAGE_OK" in result.stdout
        assert (workspace / "save.json").is_file()
        assert (workspace / "game-data/fixture/manifest.json").is_file()
        assert not (workspace / "organizer-home").exists()
        result = run([launcher, "--compile", str(fixture)], environment=environment, directory=workspace)
        compiled = fixture.with_suffix(".rbc")
        assert compiled.is_file()
        original_bytecode = compiled.read_bytes()
        run([launcher, "--compile", str(fixture)], environment=environment, directory=workspace, success=False)
        assert compiled.read_bytes() == original_bytecode
        relocated = workspace / "compiled-project"
        relocated.mkdir()
        compiled.rename(relocated / "game.rbc")
        fixture.unlink()
        result = run([launcher, str(relocated / "game.rbc"), "--grid", "compiled-save.json"],
                     environment=environment, directory=workspace)
        assert "RUBY_STORAGE_OK" in result.stdout
        assert (workspace / "compiled-save.json").is_file()
        assert not (relocated / "compiled-save.json").exists()
        assert (relocated / "game-data/fixture/manifest.json").is_file()
        damaged = relocated / "damaged.rbc"
        damaged.write_bytes(original_bytecode[:-1])
        result = run([launcher, str(damaged)], environment=environment, directory=workspace, success=False)
        assert "checksum" in result.stderr
        fixture = relocated / "game.rbc"
        invalid_save = workspace / "bad.json"
        invalid_save.write_text("invalid json")
        run([launcher, str(fixture), "--grid", str(invalid_save)],
            environment=environment, directory=workspace, success=False)
        assert invalid_save.read_text() == "invalid json"
        print("PASS: relocated runtime, Ruby-only CLI, bytecode compilation without source, corruption rejection, native extensions, storage, revenants, BSP")

        organizer = runtime / "selenite"
        if organizer.is_file():
            result = run([str(organizer), "--version"], environment=environment, directory=workspace)
            assert result.stdout.strip() == f"selenite {version}"
            assert (runtime / "examples/projects/starter/selenite-project.json").is_file()
            application = workspace / "portable application project"
            run([str(organizer), "--new-project", str(application), "--app"], environment=environment, directory=workspace)
            original = (application / "app.rb").read_bytes()
            run([str(organizer), "--new-project", str(application)], environment=environment, directory=workspace, success=False)
            assert (application / "app.rb").read_bytes() == original
            for count in (1, 2):
                result = run([str(organizer), "--project", str(application)], environment=environment, directory=workspace)
                assert f"application run: {count}" in result.stdout
            print("PASS: relocated IDE launcher, project scaffolding, configured application runs and native raylib records")
            result = run([str(organizer), "--help"], environment=environment, directory=workspace)
            assert result.stdout.startswith(f"Selenite {version}")
            assert package_info["binaries"] == ["selenite", "selenite-game", "mic_freq/mic_freq"]
            terminal_environment = {**environment, "SHELL": "/bin/sh", "SELENITE_TERMINAL_TEST": "inherited"}
            result = run([str(organizer), "--terminal", "-c",
                          'printf "%s|%s" "$SELENITE_TERMINAL_TEST" "$1"; exit 17',
                          "shell", "argument with spaces"],
                         environment=terminal_environment, directory=workspace, success=False)
            assert result.returncode == 17 and result.stdout == "inherited|argument with spaces"
            result = run([str(organizer), "--terminal", "-c", 'printf "%s" "$SELENITE_EXE"; command -v selenite'],
                         environment=terminal_environment, directory=workspace)
            assert result.stdout.startswith(str(runtime)) and result.stdout.count("selenite") >= 2
            command_file = workspace / "command with spaces;literal.sh"
            command_file.write_text('printf "%s\\n" "$@"; exit 29\n')
            result = run([str(organizer), "--terminal-run", str(command_file), "first argument", "$(literal)"],
                         environment=terminal_environment, directory=workspace, success=False)
            assert result.returncode == 29 and result.stdout == "first argument\n$(literal)\n"
            ruby = shutil.which("ruby", path=environment.get("PATH"))
            ruby_works = ruby and subprocess.run(
                [ruby, "--version"], env=environment, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL, check=False, timeout=10,
            ).returncode == 0
            if ruby_works:
                ruby_file = workspace / "command with spaces.rb"
                ruby_file.write_text('puts ARGV; exit 31\n')
                result = run([str(organizer), "--terminal-run", str(ruby_file), "ruby argument"],
                             environment=terminal_environment, directory=workspace, success=False)
                assert result.returncode == 31 and result.stdout == "ruby argument\n"
            result = run([str(organizer), "--terminal-run", str(workspace / "missing.sh")],
                         environment=terminal_environment, directory=workspace, success=False)
            assert "could not open" in result.stderr
            print("PASS: relocated organizer terminal, shell and command-file arguments, exit codes and missing files")
            binary_file = workspace / "binary.txt"
            binary_file.write_bytes(b"\xff\xfe\x00")
            result = run([str(organizer), "--edit", str(binary_file)], environment=environment,
                         directory=workspace, success=False)
            assert result.returncode == 1 and "not UTF-8" in result.stderr
            print("PASS: relocated organizer text editor refuses non-text files")
            asset_inventory(organizer, workspace, environment)
            organizer_source = workspace / "organizer.rb"
            organizer_source.write_text('raise "game mode" if Selenite::Game.game_mode?; '
                                        'Selenite.grid.set(4, 5, "asset.png"); Selenite.save; puts "ORGANIZER_RBC_OK"')
            run([str(organizer), "--compile", str(organizer_source)], environment=environment, directory=workspace)
            assert not (workspace / "organizer-home").exists()
            organizer_source.unlink()
            result = run([str(organizer), "--script", str(organizer_source.with_suffix(".rbc")), "--grid", "organizer.json"],
                         environment=environment, directory=workspace)
            assert "ORGANIZER_RBC_OK" in result.stdout
            assert (workspace / "organizer.json").is_file()
            result = run([str(organizer), "--game", str(fixture), "--grid", "organizer-game.json"],
                         environment=environment, directory=workspace)
            assert "RUBY_STORAGE_OK" in result.stdout
            print("PASS: organizer compilation, compiled script and compiled game")

        if options.graphics:
            from PIL import Image, ImageStat

            if organizer.is_file():
                native_inventory(repository, organizer, workspace, environment, artifacts)
                native_mic_freq(repository, runtime, workspace, environment, artifacts)
                for size in ("420x300", "1920x1080"):
                    shot = (artifacts / f"ide-{size}.png").resolve()
                    run([str(organizer), "--ide"], environment={**environment,
                        "SELENITE_EDITOR_FRAMES": "6", "SELENITE_EDITOR_SIZE": size,
                        "SELENITE_EDITOR_SCREENSHOT": str(shot)}, directory=workspace)
                    with Image.open(shot) as image:
                        assert image.size == tuple(map(int, size.split("x")))
                        assert sum(ImageStat.Stat(image.convert("RGB")).stddev) > 20
                print("PASS: relocated project IDE renders compact and desktop layouts")
                note = workspace / "editor-note.md"
                note.write_text("# Title\n\n```ruby\nclass Foo\nend\n```\n\n- [x] item\n")
                shot = (artifacts / "editor-markdown.png").resolve()
                run([str(organizer), "--edit", str(note)],
                    environment={**environment, "SELENITE_EDITOR_FRAMES": "6",
                                 "SELENITE_EDITOR_SIZE": "1280x800",
                                 "SELENITE_EDITOR_SCREENSHOT": str(shot)},
                    directory=workspace)
                with Image.open(shot) as image:
                    assert image.size == (1280, 800)
                    assert sum(ImageStat.Stat(image.convert("RGB")).stddev) > 20
                print("PASS: relocated organizer text editor renders a Markdown preview")
            if not (runtime / "game.rb").is_file():
                shutil.copy(runtime / "examples/games/starter.rb", runtime / "game.rb")
            environment["SELENITE_GAME_FRAMES"] = "8"
            environment["SELENITE_GAME_SCREENSHOT"] = str(workspace / "starter.png")
            for _iteration in range(2):
                run([launcher], environment=environment, directory=workspace)
            manifest = json.loads((runtime / "game-data/starter/manifest.json").read_text())
            state_file = manifest["tables"]["session"]["files"][0]
            state = json.loads((runtime / "game-data/starter" / state_file).read_text())[0]
            assert state["launches"] == 2
            with Image.open(workspace / "starter.png") as image:
                assert min(image.size) >= 100
                assert sum(ImageStat.Stat(image).stddev) > 20
                image.save(artifacts / "starter-source.png")
            run([launcher, "--compile", str(runtime / "game.rb")], environment=environment, directory=workspace)
            (runtime / "game.rb").unlink()
            (workspace / "starter.png").unlink()
            run([launcher], environment=environment, directory=workspace)
            manifest = json.loads((runtime / "game-data/starter/manifest.json").read_text())
            state_file = manifest["tables"]["session"]["files"][0]
            state = json.loads((runtime / "game-data/starter" / state_file).read_text())[0]
            assert state["launches"] == 3
            with Image.open(workspace / "starter.png") as image:
                assert sum(ImageStat.Stat(image).stddev) > 20
                image.save(artifacts / "starter-bytecode.png")
            shutil.copy(runtime / "examples/games/revenant_world.rb", workspace / "world.rb")
            run([launcher, str(workspace / "world.rb"), "--grid", str(workspace / "world.json")],
                environment=environment, directory=workspace)
            manifest = json.loads((workspace / "game-data/revenant-world/manifest.json").read_text())
            assert len([name for name in manifest["tables"] if name.startswith("world_leaf_")]) == 16
            print("PASS: graphical source and bytecode starter, default compiled entry, game-local save, progress on relaunch, streamed world")


if __name__ == "__main__":
    main()

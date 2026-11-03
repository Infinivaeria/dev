import argparse
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
        documentation = [repository / "README.md", repository / "CHANGELOG.md",
                         repository / "mic_freq/README.md", repository / "mic_freq/MANUAL.md"]
        documentation.extend((repository / "docs").rglob("*.md"))
        documentation.extend((repository / "partitioned_array").glob("*.md"))
        documentation.extend((repository / "partitioned_array").glob("*.txt"))
        for source in documentation:
            relative = source.relative_to(repository)
            bundled = runtime / relative
            assert bundled.is_file(), f"Missing package documentation: {relative}"
            assert bundled.read_bytes() == source.read_bytes(), f"Outdated package documentation: {relative}"
        package_info = json.loads((runtime / "package-info.json").read_text())
        assert package_info["schema"] == 1
        assert package_info["name"] in ("selenite", "selenite-game")
        expected_binaries = ["selenite", "selenite-game"] if package_info["name"] == "selenite" else ["selenite-game"]
        assert package_info["binaries"] == expected_binaries
        assert (runtime / "run.sh").is_file() == ("selenite" in expected_binaries)
        assert package_info["version"] == version
        assert package_info["scripting"] is True
        assert all((runtime / binary).is_file() for binary in package_info["binaries"])
        environment = os.environ.copy()
        for variable in ("RUBYLIB", "RUBYOPT", "RUBY_YJIT_ENABLE", "GEM_HOME", "GEM_PATH",
                         "LD_LIBRARY_PATH", "SELENITE_PARTITIONED_ARRAY_LIB"):
            environment.pop(variable, None)
        environment["SELENITE_HOME"] = str(workspace / "organizer-home")
        launcher = str(runtime / "run-game.sh")
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

        organizer = runtime / "run.sh"
        if organizer.is_file():
            result = run([str(organizer), "--help"], environment=environment, directory=workspace)
            assert result.stdout.startswith(f"Selenite {version}")
            assert package_info["binaries"] == ["selenite", "selenite-game"]
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
            if shutil.which("ruby", path=environment.get("PATH")):
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

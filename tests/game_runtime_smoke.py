import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


def run(command, *, environment, directory, success=True):
    result = subprocess.run(command, env=environment, cwd=directory,
                            text=True, capture_output=True, timeout=25)
    if success and result.returncode != 0:
        raise AssertionError(result.stdout + result.stderr)
    if not success and result.returncode == 0:
        raise AssertionError(f"Unexpected success: {command}")
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--graphics", action="store_true")
    options = parser.parse_args()
    repository = Path(__file__).resolve().parent.parent
    version = re.search(r'^version = "([^"]+)"',
                        (repository / "Cargo.toml").read_text(), re.MULTILINE).group(1)
    archive = repository / f"selenite-game-{version}-portable.tar.gz"
    assert archive.is_file(), "Run ./bootstrap-game.sh first"

    with tempfile.TemporaryDirectory(prefix="selenite-game-smoke-") as temporary:
        workspace = Path(temporary)
        runtime = workspace / "relocated-runtime"
        runtime.mkdir()
        subprocess.run(["tar", "-xzf", str(archive), "-C", str(runtime)], check=True)
        environment = os.environ.copy()
        for variable in ("RUBYLIB", "RUBYOPT", "GEM_HOME", "GEM_PATH",
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

        fixture = workspace / "integration.rb"
        fixture.write_text('''require "json"
require "digest"
require "zlib"
require "socket"
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

        organizer = repository / "dist/run.sh"
        if organizer.is_file():
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
            environment["SELENITE_GAME_FRAMES"] = "8"
            environment["SELENITE_GAME_SCREENSHOT"] = str(workspace / "starter.png")
            for _iteration in range(2):
                run([launcher], environment=environment, directory=workspace)
            manifest = json.loads((runtime / "game-data/starter/manifest.json").read_text())
            state_file = manifest["tables"]["session"]["files"][0]
            state = json.loads((runtime / "game-data/starter" / state_file).read_text())[0]
            assert state["launches"] == 2
            assert (workspace / "starter.png").stat().st_size > 0
            run([launcher, "--compile", str(runtime / "game.rb")], environment=environment, directory=workspace)
            (runtime / "game.rb").unlink()
            run([launcher], environment=environment, directory=workspace)
            manifest = json.loads((runtime / "game-data/starter/manifest.json").read_text())
            state_file = manifest["tables"]["session"]["files"][0]
            state = json.loads((runtime / "game-data/starter" / state_file).read_text())[0]
            assert state["launches"] == 3
            shutil.copy(runtime / "examples/games/revenant_world.rb", workspace / "world.rb")
            run([launcher, str(workspace / "world.rb"), "--grid", str(workspace / "world.json")],
                environment=environment, directory=workspace)
            manifest = json.loads((workspace / "game-data/revenant-world/manifest.json").read_text())
            assert len([name for name in manifest["tables"] if name.startswith("world_leaf_")]) == 16
            print("PASS: graphical source and bytecode starter, default compiled entry, game-local save, progress on relaunch, streamed world")


if __name__ == "__main__":
    main()

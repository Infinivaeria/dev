import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--selenite", type=Path, required=True)
    parser.add_argument("--graphics", action="store_true")
    options = parser.parse_args()
    repository = Path(__file__).resolve().parent.parent
    binary = options.selenite.resolve()
    environment = {**os.environ, "CARGO_TARGET_DIR": str(repository / "target")}
    with tempfile.TemporaryDirectory(prefix="selenite-executable-smoke-") as temporary:
        workspace = Path(temporary)
        source = workspace / "source"
        shutil.copytree(repository / "examples/executables", source)

        def build(entry, name, game=False):
            output = workspace / name
            result = subprocess.run([str(binary), "--build-executable", str(entry), "--output", str(output), "--name", name, "--game" if game else "--app", "--offline"], env=environment, text=True, capture_output=True, timeout=300)
            assert result.returncode == 0, result.stdout + result.stderr
            assert not (output / "bundle/save.json").exists(), "Build executed the program"
            return output

        built = build(source / "hello.rb", "native-app")
        bundled = workspace / "relocated app"
        shutil.copytree(built / "bundle", bundled)
        assert not (bundled / "data/hello.rb").exists()
        assert not list(bundled.glob("*.rbc"))
        assert not list(bundled.glob("*.sh"))
        clean = environment.copy()
        for variable in ("LD_LIBRARY_PATH", "RUBYLIB", "RUBYOPT", "SELENITE_PARTITIONED_ARRAY_LIB", "SELENITE_GRID"):
            clean.pop(variable, None)
        def execute(arguments=(), expected=0):
            result = subprocess.run([str(bundled / "native-app"), *arguments], env=clean, cwd=workspace, capture_output=True, text=True, timeout=30)
            assert result.returncode == expected, result.stdout + result.stderr
            return json.loads(result.stdout)
        first = execute(["one argument", "--literal"])
        assert first["arguments"] == ["one argument", "--literal"] and first["runs"] == 1
        assert execute(["--exit-seven"], 7)["runs"] == 2
        original = hashlib.sha256((built / "bundle/native-app").read_bytes()).hexdigest()
        refused = subprocess.run([str(binary), "--build-executable", str(source / "hello.rb"), "--output", str(built)], env=environment, capture_output=True, text=True)
        assert refused.returncode != 0 and "already exists" in refused.stderr
        assert hashlib.sha256((built / "bundle/native-app").read_bytes()).hexdigest() == original
        payload = built / "rust-project/hello.rbc"
        precompiled = source / "entry.rbc"
        precompiled.write_bytes(payload.read_bytes())
        payload.write_bytes(b"damaged")
        rebuild = subprocess.run(["bash", str(built / "rust-project/build.sh"), "--offline"], env=environment, capture_output=True, text=True, timeout=30)
        assert rebuild.returncode != 0 and "not Selenite" in rebuild.stderr, "Rebuild failed to reject corrupt bytecode before compilation"
        assert execute()["runs"] == 3, "Executable depended on external build-time bytecode"
        compiled = build(precompiled, "precompiled-app")
        result = subprocess.run([str(compiled / "bundle/precompiled-app"), "bytecode"], env=clean, cwd=workspace, capture_output=True, text=True, timeout=30)
        assert result.returncode == 0 and json.loads(result.stdout)["arguments"] == ["bytecode"], result.stderr
        project = workspace / "nested project"
        (project / "lib").mkdir(parents=True)
        (project / "selenite-project.json").write_text(json.dumps({"schema": 1, "entry": "lib/main.rb", "kind": "app"}))
        (project / "lib/main.rb").write_text("require_relative 'helper'; puts nested_message\n")
        (project / "lib/helper.rb").write_text("def nested_message; 'NESTED_OK'; end\n")
        nested = build(project, "nested-app")
        result = subprocess.run([str(nested / "bundle/nested-app")], env=clean, cwd=workspace, capture_output=True, text=True, timeout=30)
        assert result.returncode == 0 and result.stdout.strip() == "NESTED_OK", result.stderr
        print("PASS: Rust/Magnus source-free embedded bytecode, ARGV, exit status, helpers/assets/native extensions, state, no-clobber, precompiled input and nested project entries")
        if options.graphics:
            game = build(source / "game.rb", "native-game", True)
            result = subprocess.run([str(game / "bundle/native-game")], env=clean, cwd=workspace, capture_output=True, text=True, timeout=30)
            assert result.returncode == 0, result.stdout + result.stderr
            assert (game / "bundle/data/graphics-ok.txt").read_text() == "rendered"
            print("PASS: graphical Rust/Magnus executable using embedded Ruby bytecode")


if __name__ == "__main__":
    main()
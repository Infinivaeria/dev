import argparse
import json
import os
from pathlib import Path
import re
import shutil
import socket
import subprocess
import tempfile
import time


HOST_SOURCE = '''raise "wrong mode" unless Selenite::Game.game_mode? == (ENV.fetch("NET_GAME_MODE") == "true")
session = Selenite::Network.host(port: 0, id: "host", room: "portable-network", key: ENV.fetch("SELENITE_NET_KEY"))
store = Selenite::Storage::Store.new(ENV.fetch("NET_STORE"), partition_size: 2)
begin
  table = store.table("records")
  8.times { |index| table.insert(value: index) }
  table.sleep!
  raise "revenants unavailable" unless table[6]["value"] == 6 && table.status(table.partition_for(6)) == :revenant
  asset = File.expand_path(File.join(__dir__, "host-tile.png"))
  shared = Selenite::Network::GridRoom.new(session, grid: Selenite.grid, assets: { tile: asset })
  session.set("score", 41)
  completed = []
  session.on(:error) { |_peer, reason| raise reason }
  session.on("hello") do |peer, data|
    raise "payload corrupt" unless data == { "message" => "hello", "unicode" => "Selenite — network" }
    session.set("score", session.state("score") + 1)
    session.send("echo", data, to: peer)
  end
  session.on("done") do |peer, phase|
    raise "bad phase" unless [1, 2].include?(phase) && !completed.include?(phase)
    raise "wrong local asset" unless Selenite.grid[phase, 3].path == asset
    completed << phase
    table.update(0) { |record| record["completed"] = completed.size }
    store.flush
    raise "save failed" unless Selenite.save
    session.send("done-ack", phase, to: peer)
  end
  File.write(ENV.fetch("NET_READY"), JSON.generate(session.address))
  deadline = Selenite::Network.clock + 15
  until completed.size == 2 && session.peers.empty?
    session.poll
    raise "host timed out" if Selenite::Network.clock > deadline
    sleep 0.002
  end
  raise "wrong record count" unless table[0]["completed"] == 2
  puts "NETWORK_HOST_OK"
ensure
  session.close
  store.close
end
'''

CLIENT_SOURCE = '''phase = Integer(ENV.fetch("NET_PHASE"))
session = Selenite::Network.join("127.0.0.1", port: Integer(ENV.fetch("NET_PORT")),
                                 id: "client#{phase}", room: "portable-network", key: ENV.fetch("SELENITE_NET_KEY"))
begin
  asset = File.expand_path(File.join(__dir__, "client#{phase}-tile.png"))
  shared = Selenite::Network::GridRoom.new(session, grid: Selenite.grid, assets: { tile: asset })
  echo = nil
  acknowledged = false
  session.on(:error) { |_peer, reason| raise reason }
  session.on("echo") { |_peer, data| echo = data }
  session.on("done-ack") { |_peer, data| acknowledged = data == phase }
  deadline = Selenite::Network.clock + 10
  until session.synced?
    session.poll
    raise "client handshake timed out" if Selenite::Network.clock > deadline
    sleep 0.002
  end
  raise "late-join score missing" unless session.state("score") == 40 + phase
  raise "late-join layout missing" if phase == 2 && Selenite.grid[1, 3].path != asset
  session.send("hello", { message: "hello", unicode: "Selenite — network" })
  shared.place(phase, 3, "tile")
  until echo && Selenite.grid[phase, 3] && session.state("score") == 41 + phase
    session.poll
    raise "replication timed out" if Selenite::Network.clock > deadline
    sleep 0.002
  end
  raise "wrong asset mapping" unless Selenite.grid[phase, 3].path == asset
  raise "echo corrupt" unless echo["unicode"] == "Selenite — network"
  raise "save failed" unless Selenite.save
  session.send("done", phase)
  until acknowledged
    session.poll
    raise "acknowledgement timed out" if Selenite::Network.clock > deadline
    sleep 0.002
  end
  puts "NETWORK_CLIENT_OK"
ensure
  session.close
end
'''


def run(command, environment, directory, timeout=25):
    result = subprocess.run(command, env=environment, cwd=directory, text=True,
                            capture_output=True, timeout=timeout)
    if result.returncode:
        raise AssertionError(result.stdout + result.stderr)
    return result


def fixture(launcher, workspace, environment, *, organizer=False, compiled=False):
    project = workspace / f"{'organizer' if organizer else 'game'}-{'bytecode' if compiled else 'source'}"
    project.mkdir()
    host = project / "host.rb"
    client = project / "client.rb"
    host.write_text(HOST_SOURCE)
    client.write_text(CLIENT_SOURCE)
    environment = environment.copy()
    environment["NET_GAME_MODE"] = "false" if organizer else "true"
    environment["NET_READY"] = str(project / "ready.json")
    environment["NET_STORE"] = str(project / "store")
    if compiled:
        for source in (host, client):
            run([launcher, "--compile", str(source)], environment, workspace)
            source.unlink()
        host = host.with_suffix(".rbc")
        client = client.with_suffix(".rbc")
        relocated = project.with_name(project.name + "-relocated")
        project.rename(relocated)
        project = relocated
        host = project / host.name
        client = project / client.name
        environment["NET_READY"] = str(project / "ready.json")
        environment["NET_STORE"] = str(project / "store")
    for name in ("host-tile.png", "client1-tile.png", "client2-tile.png"):
        (project / name).write_bytes(b"local asset fixture")
    mode = ["--script"] if organizer else []
    process = subprocess.Popen([launcher, *mode, str(host), "--grid", str(project / "host.json")],
                               env=environment, cwd=workspace, text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        deadline = time.monotonic() + 10
        while not (project / "ready.json").exists():
            if process.poll() is not None:
                output, error = process.communicate()
                raise AssertionError(output + error)
            if time.monotonic() > deadline:
                raise AssertionError("network host did not become ready")
            time.sleep(0.01)
        endpoint = json.loads((project / "ready.json").read_text())
        for phase in (1, 2):
            client_environment = environment.copy()
            client_environment["NET_PHASE"] = str(phase)
            client_environment["NET_PORT"] = str(endpoint["port"])
            result = run([launcher, *mode, str(client), "--grid", str(project / f"client{phase}.json")],
                         client_environment, workspace)
            assert "NETWORK_CLIENT_OK" in result.stdout
        output, error = process.communicate(timeout=10)
        assert process.returncode == 0, output + error
        assert "NETWORK_HOST_OK" in output
        assert all((project / name).is_file() for name in ("host.json", "client1.json", "client2.json"))
        assert (project / "store/manifest.json").is_file()
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate()


def graphics(launcher, runtime, workspace, environment):
    project = workspace / "graphical-game"
    project.mkdir()
    source = project / "game.rb"
    shutil.copy(runtime / "examples/games/multiplayer.rb", source)
    run([launcher, "--compile", str(source)], environment, workspace)
    source.unlink()
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    host_environment = environment.copy()
    host_environment.update(SELENITE_NET_ROLE="host", SELENITE_NET_PORT=str(port),
                            SELENITE_GAME_FRAMES="150", SELENITE_GAME_SCREENSHOT=str(project / "host.png"))
    client_environment = environment.copy()
    client_environment.update(SELENITE_NET_ROLE="join", SELENITE_NET_PORT=str(port),
                              SELENITE_GAME_FRAMES="75", SELENITE_GAME_SCREENSHOT=str(project / "client.png"))
    host = subprocess.Popen([launcher, str(project / "game.rbc"), "--grid", str(project / "host.json")],
                            env=host_environment, cwd=workspace, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        deadline = time.monotonic() + 8
        while True:
            if host.poll() is not None:
                output, error = host.communicate()
                raise AssertionError(output + error)
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=0.05):
                    break
            except OSError:
                if time.monotonic() > deadline:
                    raise AssertionError("graphical host did not listen")
                time.sleep(0.02)
        client = run([launcher, str(project / "game.rbc"), "--grid", str(project / "client.json")],
                     client_environment, workspace)
        assert "Multiplayer connected:" in client.stdout
        assert "Multiplayer state ready:" in client.stdout
        output, error = host.communicate(timeout=15)
        assert host.returncode == 0, output + error
        assert "Multiplayer connected:" in output
        assert "Multiplayer disconnected:" in output
        assert (project / "host.png").stat().st_size > 0
        assert (project / "client.png").stat().st_size > 0
        assert (project / "game-data/multiplayer-host/manifest.json").is_file()
    finally:
        if host.poll() is None:
            host.kill()
            host.communicate()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--graphics", action="store_true")
    options = parser.parse_args()
    repository = Path(__file__).resolve().parent.parent
    version = re.search(r'^version = "([^"]+)"', (repository / "Cargo.toml").read_text(), re.MULTILINE).group(1)
    archives = [repository / f"{name}-{version}-portable.tar.gz" for name in ("selenite-game", "selenite")]
    assert all(archive.is_file() for archive in archives), "Build both portable packages first"
    with tempfile.TemporaryDirectory(prefix="selenite-network-") as temporary:
        workspace = Path(temporary)
        runtimes = []
        for index, archive in enumerate(archives):
            runtime = workspace / f"runtime-{index}"
            runtime.mkdir()
            subprocess.run(["tar", "-xzf", str(archive), "-C", str(runtime)], check=True)
            runtimes.append(runtime)
        environment = os.environ.copy()
        for variable in ("RUBYLIB", "RUBYOPT", "GEM_HOME", "GEM_PATH", "LD_LIBRARY_PATH", "SELENITE_PARTITIONED_ARRAY_LIB"):
            environment.pop(variable, None)
        for variable in list(environment):
            if variable.startswith("SELENITE_NET_") or variable == "SELENITE_MULTIPLAYER_STORE":
                environment.pop(variable, None)
        environment["SELENITE_NET_KEY"] = "portable-multiplayer-test-secret"
        environment["SELENITE_HOME"] = str(workspace / "organizer-home")
        for organizer, runtime in enumerate(runtimes):
            launcher = str(runtime / ("run.sh" if organizer else "run-game.sh"))
            for compiled in (False, True):
                fixture(launcher, workspace, environment, organizer=bool(organizer), compiled=compiled)
            print(f"PASS: {'organizer' if organizer else 'game runtime'} separate-process authenticated peers, late-join state, local shared grids, storage, relocated bytecode without source")
        if options.graphics:
            graphics(str(runtimes[0] / "run-game.sh"), runtimes[0], workspace, environment)
            print("PASS: two graphical multiplayer processes using source-free bytecode, screenshots, host storage and cleanup")


if __name__ == "__main__":
    main()

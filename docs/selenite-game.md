# selenite-game: bootstrap and run Ruby games

This guide now lives at [packaging.md](packaging.md).

The rest of this file focuses on the Ruby game runtime itself.

## Bootstrap from source

On Linux, install Rust/Cargo (Rust 1.88 or newer), Ruby with development headers
and shared libruby (Ruby 3.1 or newer), a C/C++ compiler, CMake, libclang,
pkg-config, and raylib's desktop build dependencies. Debian/Ubuntu native
dependencies are typically installed with:

```sh
sudo apt-get install build-essential cmake clang libclang-dev pkg-config ruby ruby-dev \
  libx11-dev libxrandr-dev libxi-dev libxinerama-dev libxcursor-dev libgl1-mesa-dev
```

Use equivalent packages on your distribution; Cargo comes from your Rust
installation. The first build needs access to the crate registry and the pinned
Magnus Git repository. Packaging fetches the array source if it is missing.
Bootstrap checks basic tools and lets the compiler report missing native
headers. It does not install system packages or Rust automatically.

From the repository root:

```sh
./bootstrap-game.sh
./dist-game/run-game.sh
```

`./bootstrap-game.sh` is a convenience wrapper that runs `./package.sh --game`.
Packaging replaces `dist/` or `dist-game/`.

Keep personal projects outside build outputs: the portable starter writes its
progress to `dist-game/game-data/starter`, which a rebuild removes. Copy the
extracted package to a working folder if you want to keep that progress.

`./package.sh` builds the full organizer and game runtime. See the
[packaging guide](packaging.md) for the organizer distributable.

For development without packaging:

```sh
cargo build --features scripting --bin selenite-game
./target/debug/selenite-game examples/games/starter.rb
```

Cargo skips the game executable when `scripting` is disabled. There is no
non-Ruby variant of `selenite-game`. The organizer still supports normal builds
without scripting.

## Launch and save paths

```sh
./dist-game/run-game.sh /absolute/path/to/game.rb
./dist-game/run-game.sh examples/games/revenant_world.rb --grid /tmp/world-grid.json
./dist-game/run-game.sh --version
./dist-game/run-game.sh --help
```

| Command | Behavior |
| --- | --- |
| `selenite-game GAME.rb` | Run a Ruby game; use `save.json` beside its script |
| `selenite-game GAME.rb --grid SAVE.json` | Use an explicit grid save |
| `selenite-game --game GAME.rb --grid SAVE.json` | Compatibility form with the same behavior |
| `selenite-game --help` | Print supported arguments without starting graphics |
| `selenite-game --version` | Print the release version |
| `selenite-game game.rbc [--grid save.json]` | Run a compiled Ruby game |
| `selenite-game --compile game.rb [--output game.rbc] [--force]` | Compile without executing the game; existing outputs require `--force` |

Relative arguments resolve against your current directory. The raw executable
without arguments prefers `./game.rbc`, then `./game.rb`. The portable launcher
instead looks for those entries in its own package folder. Use the launcher for a
portable package: it configures Ruby and array-library search paths.

Unknown options, extra arguments, non-Ruby entries, and missing games exit with
errors. The executable has no organizer, image viewer, profile-selection, or
`--script` mode. Missing grid saves start empty; malformed existing saves
report an error instead of being replaced.

Unlike `selenite --game`, the dedicated runtime does not default to the last
organizer profile, load organizer plugins, or run `init.rb`. Grid and inventory
APIs work normally, but organizer UI requests have no interface here; status
messages are printed. Organizer workflows belong in the full application.

Two scripts beside each other share the default `save.json`. Use separate
project folders or explicit `--grid` arguments to isolate them. Partitioned
storage is separate from the grid JSON: `game.storage` defaults to the current
grid's folder, while an explicit storage path is independent of `--grid`.

## Start a project

```sh
mkdir -p my-game
cp examples/games/starter.rb my-game/game.rb
./dist-game/run-game.sh my-game/game.rb
```

WASD/arrows move, Space adds a persistent point, and Escape saves and exits.
The starter saves launch count and score using PartitionedArray-backed tables.
The built-in API is the same as the organizer's game mode:

```ruby
Selenite::Game.run(title: "My Ruby game", audio: false) do |game|
  store = game.storage(path: File.join(__dir__, "game-data", "campaign"))
  state = store.table("session")
  state.insert(score: 0) unless state[0]

  game.update do |_elapsed|
    if game.pressed?(Raylib::KEY_SPACE)
      state.update(0) { |record| record["score"] += 1 }
      game.save
    end
  end
  game.draw_ui { game.text("Score: #{state[0]['score']}", 20, 20) }
end
```

Game-owned stores flush on `game.save` and normal autosave exit. Save logical
JSON data and reconstruct runtime objects on launch; live functions, textures,
and Ruby execution stacks cannot be stored as records.

Continue with the [game tutorial](game-tutorial.md), [storage guide](game-storage.md),
and [Revenant World](../examples/games/revenant_world.rb) example. They explain
tables, stable IDs, growth, BSP rooms, streamed data labeled as revenants,
checkpoints, and storage limitations.

For multiplayer, see [networking and P2P](networking.md) and the playable
[Cooperative Gems example](../examples/games/multiplayer.rb). The runtime
includes the same reliable TCP peer/session API, LAN discovery, and shared-grid
abstractions as the organizer, including when running `.rbc` entries.

## Distribution and export

To distribute the **game-only runtime**, extract `selenite-game-*-portable.tar.gz`
into a writable folder and run `./run-game.sh`.

To distribute the full organizer (`selenite`) as dimensional grids, follow the
[packaging guide](packaging.md).

The tarballs include Ruby and PartitionedArray, so a matching system Ruby
installation is unnecessary. They still require a compatible Linux CPU/ABI,
system libraries, graphics driver, and rendering context. Ruby native
extensions may have system library dependencies. These are not cross-platform
executables or OS installers. Additional gems and helper scripts your game
loads are not gathered automatically.

The full organizer's **Export standalone game** action prefers `selenite-game`
when available. Export copies sibling `assets` and `game-data`, the saved grid,
and the runtime. Close the game first so storage files are consistent. For Ruby
libraries included with exports, run the organizer from the full portable package.
Source-build exports still require compatible system libraries.

Without the organizer, copy a closed project folder beside an extracted runtime
package. Include its `game.rb`, relative assets, game data, and Ruby helpers;
launch that script using the package's `run-game.sh`. For distribution without
the original source, compile with that runtime and include `game.rbc` instead.
The [bytecode instructions](game.md#compile-and-distribute-ruby-bytecode) cover
helper loading, compatibility, exports, and the limits of source hiding.
The bootstrap tarball
ships the starter, not your personal project. See [export details](game.md#standalone-export).

## Validation and troubleshooting

```sh
cargo test --features scripting
ruby tests/game_storage_test.rb
ruby tests/bytecode_test.rb
python3 tests/game_runtime_smoke.py
python3 tests/game_runtime_smoke.py --graphics
SELENITE_GAME_FRAMES=90 ./dist-game/run-game.sh examples/games/starter.rb
```

Frame limits and screenshots require a graphics context. Use a temporary game
copy for testing to avoid changing real progress.
The Python smoke test requires a bootstrapped tarball, extracts it to a temporary
directory, and verifies its relocated Ruby runtime and storage. `--graphics`
also opens the starter and streamed world and checks relaunch persistence.

- **Ruby-only error:** choose a `.rb` or `.rbc` entry; no foreign-language runners exist.
- **Bytecode mismatch:** recompile with the exact Ruby runtime in the target package.
- **Bytecode checksum error:** replace the damaged file from a good build or recompile it.
- **Missing libruby/extensions:** keep the packaged `lib` tree and use the launcher.
  The Ruby used to package must match the Ruby linked by Cargo.
- **Missing array library:** keep `partitioned_array/lib` with the package.
  Source builds can set `SELENITE_PARTITIONED_ARRAY_LIB` to its `lib` directory.
- **Save error:** choose a writable project or explicit `--grid` path.
- **Storage already open:** close the other instance or use another store.
- **Failed display:** check the platform's graphics context and drivers.

# Packaging Selenite (organizer + game runtime)

If you want Selenite as **dimensional grids**, distribute **`selenite`** (the
organizer). `selenite-game` is optional and is only for running Ruby games
without the organizer UI.

Selenite has two executables:

- `selenite`: the full **dimensional grids** organizer (2D + 3D nested grids,
  profiles, inventories, plugins, exports, and the integrated `--game` mode).
- `selenite-game`: a Ruby-only runtime for running games without opening the
  organizer UI.

Both are built from the same Rust codebase. **All Rust crates are compiled into
the executables**; there is no separate “Rust library folder” to ship.
Distributions instead bundle the executables plus the non-Rust runtime pieces
they need (embedded Ruby support, standard library, native extensions, and the
PartitionedArray Ruby library).

In particular, Selenite does not ship separate `.rlib`/`.so` files for its Rust
dependencies; the Raylib bindings are linked into the executables.

## Build the distributable organizer (recommended)

Build the full portable package from the repository root:

```sh
./bootstrap.sh
./dist/run.sh

# or equivalently:
# ./package.sh
# ./dist/run.sh
```

`./bootstrap.sh` is a convenience wrapper around `./package.sh` that checks for
basic prerequisites and produces the full distributable bundle.

This produces a distributable folder + tarball:

```text
dist/
  selenite                  full organizer (dimensional grids)
  selenite-game             bundled game runtime
  run.sh                    portable launcher for selenite
  run-game.sh               portable launcher for selenite-game
  package-info.json         release version, binaries, Ruby build metadata
  LICENSE
  lib/                      Ruby shared library, stdlib, native extensions
  partitioned_array/lib/
  partitioned_array/*.md, *.txt bundled library documentation
  mic_freq/README.md, MANUAL.md companion instructions only
  docs/
  examples/
  licenses/
selenite-7.0.0-rc.1-portable.tar.gz
```

`./package.sh` builds both executables with embedded Ruby support (`scripting`)
so the organizer can run plugins/games and export standalone runtimes.

## Build the game-only runtime (optional)

To build only the dedicated Ruby game runtime package:

```sh
./bootstrap-game.sh
./dist-game/run-game.sh
```

This produces:

```text
dist-game/
  selenite-game
  run-game.sh
  game.rb                   playable starter
  package-info.json
  LICENSE
  lib/                      Ruby shared library, stdlib, native extensions
  partitioned_array/lib/
  partitioned_array/*.md, *.txt
  mic_freq/README.md, MANUAL.md
  docs/
  examples/games/
  licenses/
selenite-game-7.0.0-rc.1-portable.tar.gz
```

The game-only bundle intentionally does **not** include the organizer binary
(`selenite`). Use the full package above to distribute dimensional grids.

## What “portable” means

The portable bundles include everything Selenite ships itself:

- The `selenite` / `selenite-game` executables (compiled Rust code).
- Embedded Ruby support (`lib/` Ruby stdlib + native extensions).
- The PartitionedArray Ruby library (`partitioned_array/lib`).
- Documentation, examples, and licenses.

Both packages include the Selenite manuals under `docs/`, the top-level README
and changelog, PartitionedArray Markdown and text instructions and changelogs,
and the mic_freq README and manual.
They do not include the mic_freq executable, assets, or build sources.

They still require a compatible Linux CPU/ABI, system graphics stack, and
system libraries (X11/OpenGL and related dependencies). They are not
cross-platform installers and they do not bundle the OS or graphics drivers.

## Bootstrap from source

On Linux, install Rust/Cargo via rustup with the **nightly** toolchain, Ruby with development headers
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
./package.sh
./dist/run.sh
```

Or, to build only the game runtime:

```sh
./package.sh --game
./dist-game/run-game.sh
```

`./bootstrap-game.sh` is a convenience wrapper that runs `./package.sh --game`.
Packaging replaces `dist/` or `dist-game/`.

Packaging builds only the Selenite package with `--locked`; it does not build
other workspace applications. Cargo metadata supplies the release version and
target directory, including a custom `CARGO_TARGET_DIR`. Each archive includes
`package-info.json` with its schema version, package name, release version,
binary list, scripting support, Ruby version, and Ruby platform.

## Verify the release candidate

Add `--verify` to build and integration-test either archive. Verification needs
Python 3.11 or newer; Python is not required by recipients of the bundle.

```sh
./package.sh --verify
./package.sh --game --verify
python3 tests/network_runtime_smoke.py
```

The package checks extract the selected archive into a temporary path containing
a space and use only its launchers and files, not an existing `dist/` directory.
They validate release metadata, current manuals and other bundled documentation,
licenses, bundled Ruby native extensions,
YJIT defaults and opt-out, saves, storage, revenants, BSP, source-free bytecode,
and corruption rejection. Full-package checks also cover organizer scripts and
games, same-name asset imports into nested grids, stack enumeration, per-grid
inventory and metadata isolation, reload after deleting original files, and
Ruby loops and recursion. Networking checks exercise both archives in separate
processes, including authenticated replication, late joins, persistent storage,
logical asset mappings, and relocated bytecode.

Optional graphical checks need an accessible X11 display (including Xwayland),
working OpenGL, Pillow, and, for native organizer tests, `cc` and X11 development
headers. They open temporary windows and close them automatically:

```sh
python3 tests/game_runtime_smoke.py --archive selenite-7.0.0-rc.1-portable.tar.gz \
  --graphics --artifacts test-artifacts/integration-7.0.0-rc.1
python3 tests/game_runtime_smoke.py --graphics
python3 tests/network_runtime_smoke.py --graphics
```

The full-package graphical test sends real X11 file-drop events into blank
Asset Inventory space and an existing stack, checks copied assets after reload,
and captures the panel. Game checks run bounded frames, validate nonblank source
and bytecode screenshots, and check progress across relaunches. `--artifacts`
preserves the inventory and game screenshots at the requested location.

Packaging looks for libruby in the active Ruby's configured `libdir` and
`archlibdir`, then checks the executable's resolved shared-library dependencies.
This supports both rbenv and distribution Ruby installations. If no shared
libruby is found, use a Ruby installation built with `--enable-shared`.

To package with the rbenv Ruby 4.0.9 build containing YJIT and Prism:

```sh
RBENV_VERSION=4.0.9 ./package.sh
# Or: RBENV_VERSION=4.0.9 ./package.sh --game
```

Packaging uses the selected Ruby executable and shared library consistently,
even if `LD_LIBRARY_PATH` previously pointed at another Ruby installation.
Additional include directories from Ruby's configured `CPPFLAGS` are passed
to the native build tools, including CMake. Keep those development headers
available while building; they are not required by the portable runtime.
Prism is included in Ruby 4.0. Portable launchers enable YJIT by default when
the bundled Ruby supports it; set `RUBY_YJIT_ENABLE=0` to opt out. The recipient
does not need rbenv installed.

For development without packaging:

```sh
cargo build --features scripting --bin selenite
./target/debug/selenite

cargo build --features scripting --bin selenite-game
./target/debug/selenite-game examples/games/starter.rb
```

Cargo skips the game executable when `scripting` is disabled. There is no
non-Ruby variant of `selenite-game`. The organizer still supports normal builds
without scripting.

## Distribution and export

To distribute the **organizer** (dimensional grids), extract
`selenite-*-portable.tar.gz` into a writable folder and run `./run.sh`.

To distribute the **game-only runtime**, extract `selenite-game-*-portable.tar.gz`
and run `./run-game.sh`.

The tarballs include embedded Ruby and PartitionedArray, so a matching system
Ruby installation is unnecessary for Selenite scripting. Terminal commands
that invoke `ruby` still require a Ruby executable on `PATH`. They still require a compatible Linux CPU/ABI,
system libraries, graphics driver, and rendering context. Ruby native
extensions may have system library dependencies. These are not cross-platform
executables or OS installers.

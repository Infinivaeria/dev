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
  lib/                      Ruby shared library, stdlib, native extensions
  partitioned_array/lib/
  docs/
  examples/
  licenses/
selenite-5.0.0-portable.tar.gz
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
  lib/                      Ruby shared library, stdlib, native extensions
  partitioned_array/lib/
  docs/
  examples/games/
  licenses/
selenite-game-5.0.0-portable.tar.gz
```

The game-only bundle intentionally does **not** include the organizer binary
(`selenite`). Use the full package above to distribute dimensional grids.

## What “portable” means

The portable bundles include everything Selenite ships itself:

- The `selenite` / `selenite-game` executables (compiled Rust code).
- Embedded Ruby support (`lib/` Ruby stdlib + native extensions).
- The PartitionedArray Ruby library (`partitioned_array/lib`).
- Documentation, examples, and licenses.

They still require a compatible Linux CPU/ABI, system graphics stack, and
system libraries (X11/OpenGL and related dependencies). They are not
cross-platform installers and they do not bundle the OS or graphics drivers.

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

Packaging looks for libruby in the active Ruby's configured `libdir` and
`archlibdir`, then checks the executable's resolved shared-library dependencies.
This supports both rbenv and distribution Ruby installations. If no shared
libruby is found, use a Ruby installation built with `--enable-shared`.

To package with the rbenv Ruby 4.0.7 build containing YJIT and Prism:

```sh
RBENV_VERSION=4.0.7-yjit-prism ./package.sh
# Or: RBENV_VERSION=4.0.7-yjit-prism ./package.sh --game
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

The tarballs include Ruby and PartitionedArray, so a matching system Ruby
installation is unnecessary. They still require a compatible Linux CPU/ABI,
system libraries, graphics driver, and rendering context. Ruby native
extensions may have system library dependencies. These are not cross-platform
executables or OS installers.

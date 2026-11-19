# Packaging the Selenite Suite

If you want Selenite as **dimensional grids**, distribute **`selenite`** (the
organizer). `selenite-game` is optional and is only for running Ruby games
without the organizer UI.

The full extraction package has three executables:

- `selenite`: the full **dimensional grids** organizer (2D + 3D nested grids,
  profiles, inventories, plugins, exports, the integrated IDE/project suite and
  the integrated `--game` mode).
- `selenite-game`: a Ruby-only runtime for running games without opening the
  organizer UI.
- `mic_freq/mic_freq`: the microphone-driven PNGTuber Studio companion, with
  adjacent avatar artwork and its own saved preferences.

All are built from the same Rust workspace. **All Rust crates are compiled into
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
  run-ide.sh                IDE launcher with a bundled starter project
  run-mic-freq.sh           PNGTuber Studio launcher
  package-info.json         release version, binaries, Ruby build metadata
  LICENSE
  lib/                      Ruby shared library, stdlib, native extensions
  partitioned_array/lib/
  partitioned_array/*.md, *.txt bundled library documentation
  rustby/                  source SDK, scripts, examples, tests, licenses
  rustby/rustby-manifest.json checked source-file SHA-256 manifest
  mic_freq/
    mic_freq               compiled PNGTuber Studio companion
    assets/                copied artwork; built-in demo if no pose images exist
    README.md, MANUAL.md    companion instructions
  docs/
  examples/
  licenses/
selenite-18.0.0-rc.1-portable.tar.gz
```

`./package.sh` builds Selenite with embedded Ruby support (`scripting`) and the
portable Audio/Graphics suites. Installed FFmpeg tools are bundled automatically. If they are not on PATH,
`package.sh` first reuses a previous `dist/ffmpeg/bin` or its local cache, then
downloads/extracts the distro `ffmpeg` package without root privileges on an
apt-based Linux host. `SELENITE_FFMPEG_BIN` and `SELENITE_FFPROBE_BIN` remain
supported for explicit tool paths or non-apt build hosts.
Set `SELENITE_NATIVE_VIDEO=1` when the matching FFmpeg development libraries
are available to compile the in-process `video-rs` backend.
`mic_freq` workspace package, so the extraction bundle includes the organizer,
IDE, game runtime and voice-avatar studio together.

To keep earlier bundles, use `./package.sh --output dist-v11 --verify` (or add
`--game --output dist-game-v11` for the runtime-only package). Existing output
folders without recognized Selenite package metadata are not removed. The
package metadata includes project, IDE, terminal, native API and `mic_freq`
capabilities, plus the companion's version, binary, launcher and asset paths.
Run `./dist-v11/run-ide.sh` for the starter, or pass your own project directory.
Run `./dist-v11/run-mic-freq.sh` for the studio. Its launcher accepts the same
folder, `--device`, `--capture` and `--list-devices` arguments as the executable.

Extract the full archive into any directory and start the sibling launchers;
there is no Cargo build step or source checkout required after extraction:

```sh
mkdir selenite-suite
tar -xzf selenite-18.0.0-rc.1-portable.tar.gz -C selenite-suite
./selenite-suite/run.sh
./selenite-suite/run-ide.sh
./selenite-suite/run-mic-freq.sh --capture
```

## Distribute Into Your Home

`distribute.sh` copies the full archive into `$HOME` and extracts it into
`$HOME/selenite-VERSION` by default, such as
`/home/aylon-ayrlon/selenite-18.0.0-rc.1`. It requires Python 3.11 with safe tar
extraction support (Python 3.12+ recommended), but not Cargo to use an existing
archive. Build first as your normal user:

```sh
./package.sh --output dist-v11 --verify
./distribute.sh --dry-run
./distribute.sh
```

The default destination uses your writable home directory and normally does not
require elevated permissions. You can choose another installation directory:

```sh
./distribute.sh --destination "$HOME/selenite-18.0.0-rc.1"
```

**Updates replace package files in place.** Existing files at package paths are
replaced from the archive, while the existing `mic_freq/assets` folder is left
untouched. Files not present in the new archive, including old starter-project
edits and other save files, are also left in place. Back up anything you want
to keep and close running apps before updating.
The home directory itself, the source checkout and unrelated destinations are
protected; existing targets must contain recognized Selenite package metadata.
The script verifies a matching release
checksum when available, requires the full organizer/IDE/game/mic_freq bundle,
rejects archive traversal and unsafe links, and extracts through a temporary
directory. The archive and a `.sha256` checksum are kept beside the installation;
`distribution-info.json` records its version, archive digest and component metadata.
The new suite is validated in a temporary directory before its files are
overlaid onto the existing distribution folder. The folder itself is not
replaced; the new archive's files overwrite matching package paths, except for
`mic_freq/assets` when it already exists.
When invoked through sudo, extracted files are owned by the originating user so
the bundled starter project remains editable. It never builds Cargo artifacts
as root. `--build` rebuilds before a normal-user distribution, `--archive FILE`
selects an existing full bundle, and `--dry-run` makes no filesystem changes.

`package.sh` refreshes `selenite-VERSION-SHA256SUMS` whenever it builds an archive.

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
selenite-game-18.0.0-rc.1-portable.tar.gz
```

The game-only bundle intentionally does **not** include the organizer, IDE or
`mic_freq` executable/artwork. Use the full package above for the complete suite.

## What “portable” means

Run the binaries directly; shell launchers are optional compatibility aliases:

```sh
/path/to/selenite/selenite
/path/to/selenite/selenite --ide
/path/to/selenite/selenite --terminal-window
/path/to/selenite/selenite-game
/path/to/selenite/mic_freq/mic_freq
```

Executable-relative linker paths find bundled libraries before startup. The
executables configure Ruby's standard library/native extensions, YJIT defaults,
and PartitionedArray themselves. Explicit arguments and YJIT opt-outs are kept.
`--ide` with no path selects a project in the working directory or the bundled
starter. The game runtime prefers local game.rbc/game.rb, then adjacent bundled
entries. Explicit relative file arguments remain relative to your working directory.
Keep the package directories intact when moving it: resources are discovered
beside the actual executable, not by requiring a particular installation path.
Desktop shortcuts and current VS Code Run tasks target the binaries directly.

The portable bundles include everything Selenite ships itself:

- The `selenite` / `selenite-game` executables (compiled Rust code), plus
  `mic_freq/mic_freq` and avatar assets in the full suite.
- Embedded Ruby support (`lib/` Ruby stdlib + native extensions).
- The PartitionedArray Ruby library (`partitioned_array/lib`).
- The complete Rustby source SDK (`rustby/`), excluding generated `target/` and
  `out/` trees, with its own `rustby-manifest.json`.
- Documentation, examples, and licenses.

Both packages include the Selenite manuals under `docs/`, the top-level README
and changelog, PartitionedArray Markdown and text instructions and changelogs,
and the mic_freq README and manual.
Every source `.md` file in this Selenite workspace is collected automatically,
including additional root documents and nested mic_freq, module, asset and example
guides. Original relative paths are preserved. Build outputs, previous `dist*`
copies, temporary/test artifacts, dependency caches and Git internals are excluded
so the archive does not recursively include generated copies. Each bundle has
`documentation-manifest.json` listing all included source documents and SHA-256
hashes; `--verify` checks that list against the source files and extracted contents.
Rustby source is likewise covered by `rustby/rustby-manifest.json`, allowing
distribution to verify every bundled Rustby file without trusting the archive.
Only the full suite contains the `mic_freq` executable and assets. Neither
package needs the Rust build sources after extraction. Adjacent companion assets
take precedence over a remembered folder from another installation; an empty
artwork directory still works with the compiled-in animated demo character.

They still require a compatible Linux CPU/ABI, system graphics stack, and
system libraries (X11/OpenGL and related dependencies; ALSA `libasound.so.2` for
`mic_freq`). Microphone access uses the host's audio devices and permissions.
They are not
cross-platform installers and they do not bundle the OS or graphics drivers.

## Bootstrap from source

On Linux, install Rust/Cargo via rustup with the **nightly** toolchain, Ruby with development headers
and shared libruby (Ruby 3.1 or newer), a C/C++ compiler, CMake, libclang,
pkg-config, and raylib's desktop build dependencies. Debian/Ubuntu native
dependencies are typically installed with:

```sh
sudo apt-get install build-essential cmake clang libclang-dev pkg-config ruby ruby-dev \
  libx11-dev libxrandr-dev libxi-dev libxinerama-dev libxcursor-dev libgl1-mesa-dev libasound2-dev
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

Full packaging builds `selenite` and `mic_freq` with `--locked`; `--game` builds
only the dedicated Selenite game binary. Cargo metadata supplies component versions and the
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
python3 tests/game_runtime_smoke.py --archive selenite-18.0.0-rc.1-portable.tar.gz \
  --graphics --artifacts test-artifacts/integration-18.0.0-rc.1
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

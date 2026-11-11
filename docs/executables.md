# Ruby and Selenite Executable Builds

Selenite can build a dedicated Rust executable for a Ruby application or
Selenite game. Magnus embeds MRI, and the entry point is compiled to verified
Selenite `.rbc` bytecode and included in the executable with Rust `include_bytes!`.
This preserves Ruby behavior; it is not a Ruby-to-Rust language translator,
does not remove the Ruby VM, and does not encrypt the program.

## Build

```sh
selenite --build-executable app.rb --output build/my-app --name my-app --app
selenite --build-executable game.rb --output build/my-game --name my-game --game
selenite --build-executable project-folder --output build/project
selenite --build-executable game.rbc --output build/from-bytecode --game
```

An output folder must not already exist. Compilation does not execute the Ruby
entry. Project manifests select the mode unless --app/--game overrides it.
Ordinary script inputs default to app mode. Names must start with a lowercase
letter and contain only lowercase letters, numbers, hyphens or underscores.

The equivalent Ruby and Bash bootstrappers are:

```sh
ruby bootstrap-executable.rb examples/executables/hello.rb --output build/hello --app
bash bootstrap-executable.sh examples/executables/game.rb --output build/game --game
RUBY=/path/to/ruby bash bootstrap-executable.sh app.rb --output build/other-ruby --app
```

This builder currently supports Linux. Builds need Cargo/Rust, MRI Ruby 3.1+ with shared libraries and development
headers, Python 3.11+, and Selenite's native build dependencies. Dependencies
download through Cargo unless --offline is specified. The full suite includes
`sdk/` with runtime sources and both bootstrappers; the game-only bundle does
not include the SDK. SELENITE_BUILD_SDK can select a different compatible SDK.

To select another development Ruby for the Selenite command, set RUBY to its
executable path. The builder compiles bytecode and links Magnus against that
same Ruby, then bundles its standard library, extensions and libruby. Existing
bytecode must match that Ruby's engine, version, patch level and platform.

## Output

```text
build/my-app/
  bundle/
    my-app                  native Rust/Magnus executable
    data/                   supporting scripts and assets, without the entry source
    lib/                    matching Ruby runtime and extensions
    partitioned_array/lib/
  rust-project/
    Cargo.toml
    Cargo.lock
    build-config.json
    build.sh
    entry.rbc               rebuild input, already embedded in the executable
    runtime/                editable Selenite runtime sources
  build-info.json
  README.md
```

Run `bundle/my-app` directly from any directory. Arguments become Ruby ARGV,
including arguments beginning with --. Script exit codes are preserved. The
entry runs in its data directory; nested project entries retain their relative
helper/asset location. The default Selenite save is beside the executable;
SELENITE_GRID can select another writable save path.

Move the whole bundle, not just the executable. It does not need rust-project,
the original entry source, an external .rbc file, or a launch script at runtime.
Rebuild using rust-project/build.sh with the matching Ruby. The rebuild script
rejects corrupt or incompatible bytecode before Cargo is invoked.

Supporting files are copied; ordinary Ruby helpers remain source files unless
you compile them and explicitly load them with Selenite::Bytecode.require.
Symlinks, build/dependency folders and the output are excluded. Supporting data
is limited to 512 MiB and entry files to 4 MiB. Required gems outside the bundled
runtime, external commands and system graphics/audio libraries must be supplied
separately. Builds target the current platform, not arbitrary cross-platform
single-file executables. Only build and run trusted code/SDKs.

## Verification

```sh
ruby tests/bytecode_test.rb
python3 tests/executable_build_smoke.py --selenite dist/selenite --graphics
```

The smoke test builds source, precompiled-bytecode, nested-project and game
inputs; checks source-free relocation, helper/assets/native extensions, ARGV,
exit status, saves, and refusal to overwrite existing output. Graphics needs a
working X11/OpenGL environment. `--offline` tests require cached Cargo dependencies.
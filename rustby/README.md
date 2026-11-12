# Rustby

Independent Rust crate and Ruby bootstrap system built from Selenite's Magnus
embedding and checked-bytecode approach. Compile a Rust `.rs` entry with Cargo,
or compile a Ruby `.rb` entry to MRI bytecode and embed it in a Rust executable.
Existing Rustby `.rbc` files are also accepted. Ruby is not translated to Rust
machine code: Magnus executes its bytecode with a matching MRI VM.

## Prerequisites

Linux, Cargo/Rust 1.88+, a C compiler, libclang for rb-sys/bindgen, and MRI Ruby
with development headers and a shared libruby. Set `RUBY=/absolute/path/to/ruby`
to select a runtime. Internet access is needed on the first Cargo build;
`--offline` works after dependencies are cached. No Selenite graphics libraries
are needed. This is a separate Cargo workspace and does not change Selenite.

## Commands

From this directory:

```bash
bash build.sh --release
bash rustby.sh doctor
bash rustby.sh compile examples/hello.rb --output /tmp/hello.rbc
bash rustby.sh build examples/hello.rb --output out/hello
bash out/hello/run.sh first "second argument"
bash rustby.sh build /tmp/hello.rbc --output out/bytecode
bash rustby.sh build examples/native.rs --output out/native
bash rustby.sh build examples/hybrid.rs --ruby examples/hello.rb --output out/hybrid
bash out/hybrid/run.sh hello
bash rustby.sh run examples/hello.rb -- --anything "with spaces"
bash test.sh
```

The compiled `target/release/rustby` CLI accepts the same commands as
`rustby.sh`. The builder CLI requires its SDK directory to remain available;
the executables it produces do not. `--name` sets an executable name. Output
paths must not already exist. Rust entries are normal Rust programs with a
`fn main()`, not arbitrary top-level Rust statements. Generated projects can
add dependencies in their Cargo manifest and rebuild using their `build.sh`.

## Ruby Injection

With `--ruby`, a Rust entry can call `embedded::run() -> Result<i32, String>`.
It executes the injected bytecode, forwards executable arguments as Ruby `ARGV`,
and returns the script's exit status. Ruby exceptions become error strings.
Call it only once and only from the main thread. The lower-level unsafe
`rustby::run(bytes, arguments)` API has the same main-thread and VM ownership
requirements. `rustby::configure_runtime()` sets up adjacent bundled load paths
before VM initialization and must be called before starting worker threads.

Each build compiles `.rb` inputs in its Cargo build script. Rebuilding after an
entry changes recompiles its bytecode. Bytecode inputs are checked before being
embedded. There is no source-eval fallback for the entry payload.

## Delivered Package

Output includes the executable, `bootstrap.rb`, `run.sh`, `package.json`, a Ruby
interpreter under `ruby/bin`, Ruby shared libraries and standard library under
`lib`, and a rebuildable `rust-project` with a local copy of Rustby. Run
`bash run.sh --rustby-check` to inspect the bootstrap/runtime configuration.
The launcher uses the bundled interpreter, not whichever Ruby is on PATH.
The Ruby bootstrap validates its runtime, configures library paths, clears
`RUBYOPT`, and replaces itself with the executable, preserving arguments and
exit status. The executable configures adjacent standard-library paths and
uses an origin-relative RPATH so it can also run directly.

Distribute the whole output directory, not just its executable. Packages target
compatible Linux systems with the same architecture and compatible glibc and
native dependencies (such as libssl, libcrypto and libz); this is not static
linking or a universal cross-platform binary. Extra gems, native extensions,
assets and `require_relative` support files are not automatically collected.
Include those separately and use explicit runtime paths. Ruby source locations
inside bytecode refer to the generated build project. Gems are not vendored.

Bytecode is MRI-version/platform/revision specific. Rustby records those values
and verifies payload size and SHA-256 before loading. Raw MRI `.rbc` files and
Selenite's distinct `.rbc` format are not accepted: recompile their Ruby source.
Checksums detect damage, not malicious input. Build and execute only trusted
Rust, Ruby and bytecode; this is neither sandboxing nor source encryption.
Ruby loader/bootstrap support is delivered as source; only the application
entry is embedded as bytecode. Rustby does not currently register a Ruby API
for calling arbitrary Rust functions.
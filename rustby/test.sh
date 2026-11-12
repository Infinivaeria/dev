#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "$0")" && pwd)"
export RUBY="${RUBY:-ruby}"
"$RUBY" "$root/tests/bytecode_test.rb"
cargo test --manifest-path "$root/Cargo.toml" --offline
"$RUBY" "$root/tests/integration_test.rb"
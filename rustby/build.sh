#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "$0")" && pwd)"
export RUBY="${RUBY:-ruby}"
exec cargo build --manifest-path "$root/Cargo.toml" "$@"
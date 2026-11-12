#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "$0")" && pwd)"
export RUSTBY_SDK="$root"
exec "${RUBY:-ruby}" "$root/bootstrap.rb" "$@"
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec "${RUBY:-ruby}" "${ROOT}/bootstrap-executable.rb" "$@"
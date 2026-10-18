#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

rustup update

gem update --system && gem update

if command -v sudo >/dev/null 2>&1; then
  sudo apt update
  sudo apt upgrade
else
  echo "Skipping apt updates (no sudo available)."
fi

cargo update --workspace
cargo build --release --workspace

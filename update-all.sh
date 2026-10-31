#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

rustup update nightly

gem update --system && gem update

if command -v sudo >/dev/null 2>&1; then
  sudo apt update
  sudo apt upgrade
else
  echo "Skipping apt updates (no sudo available)."
fi

rustup run nightly cargo update --workspace
rustup run nightly cargo build --release --workspace

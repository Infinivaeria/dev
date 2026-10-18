#!/usr/bin/env bash
set -euo pipefail

RUNTIME_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ "$(basename "${BASH_SOURCE[0]}")" == "run-game.sh" ]]; then
    RUNTIME_BINARY="selenite-game"
    if [[ "$#" == 0 && -f "${RUNTIME_DIR}/game.rbc" ]]; then
        set -- "${RUNTIME_DIR}/game.rbc"
    elif [[ "$#" == 0 && -f "${RUNTIME_DIR}/game.rb" ]]; then
        set -- "${RUNTIME_DIR}/game.rb"
    fi
else
    RUNTIME_BINARY="selenite"
fi
export LD_LIBRARY_PATH="${RUNTIME_DIR}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
export RUBY_YJIT_ENABLE="${RUBY_YJIT_ENABLE:-1}"
if [[ -d "${RUNTIME_DIR}/lib/ruby" ]]; then
    RUBY_VERSION_PATH="$(find "${RUNTIME_DIR}/lib/ruby" -maxdepth 1 -mindepth 1 -type d | sort | head -n 1)"
    export RUBYLIB="${RUBY_VERSION_PATH}:${RUNTIME_DIR}/lib/ruby-arch${RUBYLIB:+:${RUBYLIB}}"
fi
export SELENITE_PARTITIONED_ARRAY_LIB="${RUNTIME_DIR}/partitioned_array/lib"
exec "${RUNTIME_DIR}/${RUNTIME_BINARY}" "$@"

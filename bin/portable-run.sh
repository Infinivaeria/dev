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
elif [[ "$(basename "${BASH_SOURCE[0]}")" == "run-mic-freq.sh" ]]; then
    RUNTIME_BINARY="mic_freq/mic_freq"
elif [[ "$(basename "${BASH_SOURCE[0]}")" == "run-ide.sh" ]]; then
    RUNTIME_BINARY="selenite"
    if [[ "$#" == 0 ]]; then
        set -- --ide "${RUNTIME_DIR}/examples/projects/starter"
    elif [[ "$1" != -* ]]; then
        set -- --ide "$@"
    fi
else
    RUNTIME_BINARY="selenite"
fi
exec "${RUNTIME_DIR}/${RUNTIME_BINARY}" "$@"

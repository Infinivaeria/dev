#!/usr/bin/env bash
set -euo pipefail

SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ "$#" -gt 0 ]]; then
    printf 'Usage: ./bootstrap-game.sh\nBuilds and packages the Ruby-only selenite-game runtime.\n' >&2
    exit 2
fi

ensure_cargo() {
    if command -v cargo >/dev/null 2>&1; then
        return 0
    fi

    # rustup installs Cargo to ~/.cargo/bin by default, which may not be on PATH
    # in non-interactive shells.
    if [[ -f "${HOME}/.cargo/env" ]]; then
        # shellcheck disable=SC1090
        source "${HOME}/.cargo/env"
    elif [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
        export PATH="${HOME}/.cargo/bin:${PATH}"
    fi

    if command -v cargo >/dev/null 2>&1; then
        return 0
    fi

    if command -v rustup >/dev/null 2>&1; then
        local cargo_bin
        cargo_bin="$(rustup which cargo 2>/dev/null || true)"
        if [[ -n "${cargo_bin}" && -x "${cargo_bin}" ]]; then
            export PATH="$(dirname "${cargo_bin}"):${PATH}"
        fi
    fi

    if command -v cargo >/dev/null 2>&1; then
        return 0
    fi

    if [[ -d "${HOME}/.rustup/toolchains" ]]; then
        local cargo_bin
        cargo_bin="$(find "${HOME}/.rustup/toolchains" -type f -path '*/bin/cargo' -perm -111 2>/dev/null | sort | head -n 1)"
        if [[ -n "${cargo_bin}" && -x "${cargo_bin}" ]]; then
            export PATH="$(dirname "${cargo_bin}"):${PATH}"
        fi
    fi

    command -v cargo >/dev/null 2>&1
}

ensure_cargo
for dependency in cargo ruby cmake; do
    if ! command -v "${dependency}" >/dev/null 2>&1; then
        if [[ "${dependency}" == "cargo" ]]; then
            printf 'Missing build prerequisite: cargo. If you installed Rust via rustup, ensure it is on PATH (try: source ~/.cargo/env). See docs/selenite-game.md.\n' >&2
            exit 1
        fi
        printf 'Missing build prerequisite: %s. See docs/selenite-game.md.\n' "${dependency}" >&2
        exit 1
    fi
done
bash "${SOURCE_DIR}/package.sh" --game
printf '\nRun a game with: ./dist-game/run-game.sh path/to/game.rb\n'

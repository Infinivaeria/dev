#!/usr/bin/env bash
set -euo pipefail

PREFIX="${PREFIX:-/usr/local}"
SESSION_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/selenite-appliance"
if [[ "${EUID}" -eq 0 ]]; then
    SESSION_DIR="${PREFIX}/share/wayland-sessions"
fi

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -d "${HERE}/session" ]]; then
    SCRIPT_DIR="${HERE}"
else
    SCRIPT_DIR="${HERE}/.."
fi
install -d "${SESSION_DIR}"
install -m 0644 "${SCRIPT_DIR}/session/selenite-appliance.desktop" \
    "${SESSION_DIR}/selenite-appliance.desktop"
sed -i "s|^Exec=.*$|Exec=${PREFIX}/bin/selenite-appliance|" \
    "${SESSION_DIR}/selenite-appliance.desktop"

if [[ "${EUID}" -ne 0 ]]; then
    printf 'Installed the session record under %s.\n' "${SESSION_DIR}"
    printf 'A display-manager-wide installation requires root and PREFIX/bin/selenite-appliance.\n'
else
        if [[ -f "${SCRIPT_DIR}/bin/selenite-appliance" ]]; then
            launcher="${SCRIPT_DIR}/bin/selenite-appliance"
        else
            launcher="${SCRIPT_DIR}/selenite-appliance"
        fi
        install -m 0755 "${launcher}" \
        "${PREFIX}/bin/selenite-appliance"
    printf 'Installed the Selenite Appliance Wayland session.\n'
fi

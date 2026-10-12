#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_NAME="selenite"
DIST_DIR="${SCRIPT_DIR}/dist"
BUILD_TARGETS=(--bins)
if [[ "$#" == 1 && "$1" == "--game" ]]; then
    APP_NAME="selenite-game"
    DIST_DIR="${SCRIPT_DIR}/dist-game"
    BUILD_TARGETS=(--bin selenite-game)
elif [[ "$#" != 0 ]]; then
    printf 'Usage: ./package.sh [--game]\n' >&2
    exit 2
fi

echo "=== Building Selenite with embedded Ruby scripting ==="
cd "${SCRIPT_DIR}"
cargo build --release --features scripting "${BUILD_TARGETS[@]}"

echo "=== Preparing distribution folder at ${DIST_DIR} ==="
rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}/lib"

echo "=== Copying binary ==="
cp "target/release/${APP_NAME}" "${DIST_DIR}/"
if [[ "${APP_NAME}" == "selenite" ]]; then
    cp target/release/selenite-game "${DIST_DIR}/"
fi

echo "=== Bootstrapping Ruby runtime libraries ==="
RUBY_LIBDIR="$(ruby -rrbconfig -e 'puts RbConfig::CONFIG["archlibdir"] || RbConfig::CONFIG["libdir"]')"
RUBY_STDLIB="$(ruby -rrbconfig -e 'puts RbConfig::CONFIG["rubylibdir"]')"
RUBY_ARCHLIB="$(ruby -rrbconfig -e 'puts RbConfig::CONFIG["rubyarchdir"]')"
RUBY_SHARED="$(ruby -rrbconfig -e 'puts RbConfig::CONFIG["LIBRUBY_SO"]')"

echo "Copying libruby from ${RUBY_LIBDIR}..."
if [[ ! -f "${RUBY_LIBDIR}/${RUBY_SHARED}" ]]; then
    RUBY_LIBDIR="$(dirname "$(ldd "target/release/${APP_NAME}" | awk '/libruby.*=>/ {print $3; exit}')")"
fi
cp -L "${RUBY_LIBDIR}"/libruby*.so* "${DIST_DIR}/lib/"

mkdir -p "${DIST_DIR}/lib/ruby" "${DIST_DIR}/lib/ruby-arch"
cp -r "${RUBY_STDLIB}" "${DIST_DIR}/lib/ruby/"
cp -r "${RUBY_ARCHLIB}/." "${DIST_DIR}/lib/ruby-arch/"

echo "=== Bundling partitioned_array library ==="
PA_DIR="${SCRIPT_DIR}/partitioned_array"
if [ ! -f "${PA_DIR}/lib/managed_partitioned_array.rb" ]; then
    git -c credential.helper= clone https://github.com/Infinivaeria/partitioned_array.git "${PA_DIR}"
fi
mkdir -p "${DIST_DIR}/partitioned_array"
cp -r "${PA_DIR}/lib" "${PA_DIR}/LICENSE" "${DIST_DIR}/partitioned_array/"

echo "=== Copying third-party licenses ==="
mkdir -p "${DIST_DIR}/licenses"
cp "${SCRIPT_DIR}/assets/fonts/LICENSE-DejaVu.txt" "${DIST_DIR}/licenses/"

echo "=== Copying documentation and example plugins ==="
cp -r "${SCRIPT_DIR}/docs" "${SCRIPT_DIR}/examples" "${DIST_DIR}/"
cp "${SCRIPT_DIR}/README.md" "${SCRIPT_DIR}/CHANGELOG.md" "${DIST_DIR}/"
if [[ "${APP_NAME}" == "selenite-game" ]]; then
    cp "${SCRIPT_DIR}/examples/games/starter.rb" "${DIST_DIR}/game.rb"
fi

echo "=== Creating portable run launcher ==="
cp "${SCRIPT_DIR}/bin/portable-run.sh" "${DIST_DIR}/run-game.sh"
chmod +x "${DIST_DIR}/run-game.sh"
if [[ "${APP_NAME}" == "selenite" ]]; then
    cp "${SCRIPT_DIR}/bin/portable-run.sh" "${DIST_DIR}/run.sh"
    chmod +x "${DIST_DIR}/run.sh"
fi

echo "=== Creating portable tarball ==="
cd "${SCRIPT_DIR}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
TARBALL="${APP_NAME}-${VERSION}-portable.tar.gz"
tar -czf "${TARBALL}" -C "${DIST_DIR}" .

echo "=== Distribution bundle complete! ==="
echo "Files ready in ${DIST_DIR}/ and packaged in ${TARBALL}"

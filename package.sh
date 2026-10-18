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
RUBY_LIBDIR="$(ruby -rrbconfig -e 'puts RbConfig::CONFIG["libdir"]')"
export LD_LIBRARY_PATH="${RUBY_LIBDIR}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
RUBY="$(ruby -rrbconfig -e 'puts RbConfig.ruby')"
export RUBY
RUBY_INCLUDE_PATH="$("${RUBY}" -rrbconfig -rshellwords -e 'puts Shellwords.split(RbConfig::CONFIG["CPPFLAGS"]).grep(/\A-I./).map { |flag| flag.delete_prefix("-I") }.join(":")')"
if [[ -n "${RUBY_INCLUDE_PATH}" ]]; then
    export CPATH="${RUBY_INCLUDE_PATH}${CPATH:+:${CPATH}}"
    export CMAKE_INCLUDE_PATH="${RUBY_INCLUDE_PATH}${CMAKE_INCLUDE_PATH:+:${CMAKE_INCLUDE_PATH}}"
fi
echo "Using Ruby: $("${RUBY}" --version)"
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
RUBY_ARCHLIBDIR="$("${RUBY}" -rrbconfig -e 'puts RbConfig::CONFIG["archlibdir"]')"
RUBY_STDLIB="$("${RUBY}" -rrbconfig -e 'puts RbConfig::CONFIG["rubylibdir"]')"
RUBY_ARCHLIB="$("${RUBY}" -rrbconfig -e 'puts RbConfig::CONFIG["rubyarchdir"]')"
RUBY_SHARED="$("${RUBY}" -rrbconfig -e 'puts RbConfig::CONFIG["LIBRUBY_SO"]')"

if [[ ! -f "${RUBY_LIBDIR}/${RUBY_SHARED}" ]]; then
    RUBY_LIBDIR="${RUBY_ARCHLIBDIR}"
fi
if [[ ! -f "${RUBY_LIBDIR}/${RUBY_SHARED}" ]]; then
    RUBY_LINKED_LIBRARY="$(ldd "target/release/${APP_NAME}" | awk '/libruby.*=>/ && $3 ~ /^\// {print $3; exit}')"
    if [[ -z "${RUBY_LINKED_LIBRARY}" || ! -f "${RUBY_LINKED_LIBRARY}" ]]; then
        printf 'Unable to locate Ruby shared library %s in libdir, archlibdir, or linked dependencies. Use a Ruby installation built with --enable-shared.\n' "${RUBY_SHARED}" >&2
        exit 1
    fi
    RUBY_LIBDIR="$(dirname "${RUBY_LINKED_LIBRARY}")"
fi
echo "Copying libruby from ${RUBY_LIBDIR}..."
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

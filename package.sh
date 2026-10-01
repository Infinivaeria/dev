#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIST_DIR="${SCRIPT_DIR}/dist"
APP_NAME="selenite"

echo "=== Building Selenite with embedded Ruby scripting ==="
cd "${SCRIPT_DIR}"
cargo build --release --features scripting

echo "=== Preparing distribution folder at ${DIST_DIR} ==="
rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}/lib"

echo "=== Copying binary ==="
cp "target/release/${APP_NAME}" "${DIST_DIR}/"

echo "=== Bootstrapping Ruby runtime libraries ==="
RUBY_LIBDIR="$(ruby -rrbconfig -e 'puts RbConfig::CONFIG["libdir"]')"

echo "Copying libruby from ${RUBY_LIBDIR}..."
cp -d "${RUBY_LIBDIR}"/libruby.so* "${DIST_DIR}/lib/"

if [ -d "${RUBY_LIBDIR}/ruby" ]; then
    echo "Copying Ruby standard library..."
    cp -r "${RUBY_LIBDIR}/ruby" "${DIST_DIR}/lib/"
fi

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

echo "=== Creating portable run launcher ==="
cat << 'EOF' > "${DIST_DIR}/run.sh"
#!/usr/bin/env bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Ensure bundled lib directory takes precedence for dynamic linker
export LD_LIBRARY_PATH="${SCRIPT_DIR}/lib${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"

# Configure Ruby library search paths to bundled lib/ruby if present
if [ -d "${SCRIPT_DIR}/lib/ruby" ]; then
    RUBY_VER_DIR="$(find "${SCRIPT_DIR}/lib/ruby" -maxdepth 1 -mindepth 1 -type d ! -name "gems" ! -name "site_ruby" ! -name "vendor_ruby" | head -n 1)"
    if [ -n "${RUBY_VER_DIR}" ]; then
        ARCH_DIR="$(find "${RUBY_VER_DIR}" -maxdepth 1 -mindepth 1 -type d | head -n 1)"
        export RUBYLIB="${RUBY_VER_DIR}${ARCH_DIR:+:${ARCH_DIR}}"
    fi
fi

exec "${SCRIPT_DIR}/selenite" "$@"
EOF
chmod +x "${DIST_DIR}/run.sh"

echo "=== Creating portable tarball ==="
cd "${SCRIPT_DIR}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
TARBALL="${APP_NAME}-${VERSION}-portable.tar.gz"
tar -czf "${TARBALL}" -C dist .

echo "=== Distribution bundle complete! ==="
echo "Files ready in ${DIST_DIR}/ and packaged in ${TARBALL}"

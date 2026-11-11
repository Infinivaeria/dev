#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_NAME="selenite"
DIST_DIR="${SCRIPT_DIR}/dist"
BUILD_TARGETS=(--bins)
VERIFY=false
OUTPUT_DIR=""
while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --game)
            APP_NAME="selenite-game"
            DIST_DIR="${SCRIPT_DIR}/dist-game"
            BUILD_TARGETS=(--bin selenite-game)
            ;;
        --verify) VERIFY=true ;;
        --output)
            if [[ "$#" -lt 2 || -z "$2" || "$2" == -* ]]; then
                printf '%s\n' '--output needs a distribution directory' >&2
                exit 2
            fi
            OUTPUT_DIR="$2"
            shift
            ;;
        *) printf 'Usage: ./package.sh [--game] [--verify] [--output DIR]\n' >&2; exit 2 ;;
    esac
    shift
done
if [[ "${VERIFY}" == true ]]; then
    command -v python3 >/dev/null || { printf 'Verification requires python3.\n' >&2; exit 1; }
    python3 -c 'import sys; assert sys.version_info >= (3, 11)' >/dev/null 2>&1 || {
        printf 'Verification requires Python 3.11 or newer.\n' >&2; exit 1;
    }
fi

echo "=== Building Selenite with embedded Ruby scripting ==="
cd "${SCRIPT_DIR}"
RUBY_LIBDIR="$(ruby -rrbconfig -e 'puts RbConfig::CONFIG["libdir"]')"
export LD_LIBRARY_PATH="${RUBY_LIBDIR}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
RUBY="$(ruby -rrbconfig -e 'puts RbConfig.ruby')"
export RUBY
if [[ -n "${OUTPUT_DIR}" ]]; then
    DIST_DIR="$("${RUBY}" -e 'puts File.expand_path(ARGV.fetch(0))' "${OUTPUT_DIR}")"
fi
"${RUBY}" -rjson -e 'destination, source = ARGV.map { |path| File.expand_path(path) }; abort "Refusing an unsafe distribution path" if destination == "/" || destination == source || source.start_with?(destination + "/") || File.symlink?(destination); if File.exist?(destination); info = File.join(destination, "package-info.json"); abort "Refusing to replace a directory without Selenite package metadata" unless File.file?(info) && %w[selenite selenite-game].include?(JSON.parse(File.read(info))["name"]); end' "${DIST_DIR}" "${SCRIPT_DIR}"
RUBY_INCLUDE_PATH="$("${RUBY}" -rrbconfig -rshellwords -e 'puts Shellwords.split(RbConfig::CONFIG["CPPFLAGS"]).grep(/\A-I./).map { |flag| flag.delete_prefix("-I") }.join(":")')"
if [[ -n "${RUBY_INCLUDE_PATH}" ]]; then
    export CPATH="${RUBY_INCLUDE_PATH}${CPATH:+:${CPATH}}"
    export CMAKE_INCLUDE_PATH="${RUBY_INCLUDE_PATH}${CMAKE_INCLUDE_PATH:+:${CMAKE_INCLUDE_PATH}}"
fi
echo "Using Ruby: $("${RUBY}" --version)"
PACKAGE_INFO="$(cargo metadata --locked --no-deps --format-version 1 | "${RUBY}" -rjson -e 'metadata = JSON.parse(STDIN.read); packages = metadata.fetch("packages"); package = packages.find { |entry| entry.fetch("name") == "selenite" }; companion = packages.find { |entry| entry.fetch("name") == "mic_freq" }; abort "Selenite workspace packages not found" unless package && companion; puts package.fetch("version"), metadata.fetch("target_directory"), companion.fetch("version")')"
mapfile -t PACKAGE_FIELDS <<< "${PACKAGE_INFO}"
VERSION="${PACKAGE_FIELDS[0]}"
RELEASE_DIR="${PACKAGE_FIELDS[1]}/release"
MIC_FREQ_VERSION="${PACKAGE_FIELDS[2]}"
BUILD_PACKAGES=(-p selenite)
if [[ "${APP_NAME}" == "selenite" ]]; then
    BUILD_PACKAGES+=(-p mic_freq)
fi
cargo build --locked --release "${BUILD_PACKAGES[@]}" --features selenite/scripting "${BUILD_TARGETS[@]}"

echo "=== Preparing distribution folder at ${DIST_DIR} ==="
rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}/lib"

echo "=== Copying binary ==="
cp "${RELEASE_DIR}/${APP_NAME}" "${DIST_DIR}/"
if [[ "${APP_NAME}" == "selenite" ]]; then
    cp "${RELEASE_DIR}/selenite-game" "${DIST_DIR}/"
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
    RUBY_LINKED_LIBRARY="$(ldd "${RELEASE_DIR}/${APP_NAME}" | awk '/libruby.*=>/ && $3 ~ /^\// {print $3; exit}')"
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
if [[ ! -f "${PA_DIR}/lib/managed_partitioned_array.rb" ]]; then
    git -c credential.helper= clone https://github.com/Infinivaeria/partitioned_array.git "${PA_DIR}"
fi
mkdir -p "${DIST_DIR}/partitioned_array"
cp -r "${PA_DIR}/lib" "${PA_DIR}/LICENSE" "${DIST_DIR}/partitioned_array/"
for documentation in "${PA_DIR}"/*.md "${PA_DIR}"/*.txt; do
    if [[ -f "${documentation}" ]]; then
        cp "${documentation}" "${DIST_DIR}/partitioned_array/"
    fi
done

echo "=== Copying third-party licenses ==="
mkdir -p "${DIST_DIR}/licenses"
cp "${SCRIPT_DIR}/assets/fonts/LICENSE-DejaVu.txt" "${DIST_DIR}/licenses/"

echo "=== Copying documentation and example plugins ==="
cp -r "${SCRIPT_DIR}/docs" "${SCRIPT_DIR}/examples" "${DIST_DIR}/"
cp "${SCRIPT_DIR}/LICENSE" "${DIST_DIR}/"
mkdir -p "${DIST_DIR}/mic_freq"
if [[ "${APP_NAME}" == "selenite" ]]; then
    cp "${RELEASE_DIR}/mic_freq" "${DIST_DIR}/mic_freq/mic_freq"
    cp -r "${SCRIPT_DIR}/mic_freq/assets" "${DIST_DIR}/mic_freq/assets"
    cp "${SCRIPT_DIR}/mic_freq/README.md" "${SCRIPT_DIR}/mic_freq/MANUAL.md" "${DIST_DIR}/mic_freq/"
    mkdir -p "${DIST_DIR}/sdk/tools" "${DIST_DIR}/sdk/partitioned_array"
    cp "${SCRIPT_DIR}/Cargo.toml" "${SCRIPT_DIR}/Cargo.lock" "${SCRIPT_DIR}/build.rs" "${SCRIPT_DIR}/rust-toolchain.toml" "${SCRIPT_DIR}/LICENSE" "${DIST_DIR}/sdk/"
    cp -r "${SCRIPT_DIR}/src" "${SCRIPT_DIR}/assets" "${SCRIPT_DIR}/docs" "${SCRIPT_DIR}/examples" "${DIST_DIR}/sdk/"
    cp -r "${SCRIPT_DIR}/partitioned_array/lib" "${DIST_DIR}/sdk/partitioned_array/"
    cp "${SCRIPT_DIR}/tools/build_executable.rb" "${DIST_DIR}/sdk/tools/"
    cp "${SCRIPT_DIR}/bootstrap-executable.sh" "${SCRIPT_DIR}/bootstrap-executable.rb" "${DIST_DIR}/sdk/"
fi
if [[ "${APP_NAME}" == "selenite-game" ]]; then
    cp "${SCRIPT_DIR}/examples/games/starter.rb" "${DIST_DIR}/game.rb"
fi
"${RUBY}" "${SCRIPT_DIR}/tools/package_documentation.rb" "${SCRIPT_DIR}" "${DIST_DIR}" "${PACKAGE_FIELDS[1]}"

echo "=== Creating portable run launcher ==="
cp "${SCRIPT_DIR}/bin/portable-run.sh" "${DIST_DIR}/run-game.sh"
chmod +x "${DIST_DIR}/run-game.sh"
if [[ "${APP_NAME}" == "selenite" ]]; then
    cp "${SCRIPT_DIR}/bin/portable-run.sh" "${DIST_DIR}/run.sh"
    cp "${SCRIPT_DIR}/bin/portable-run.sh" "${DIST_DIR}/run-ide.sh"
    cp "${SCRIPT_DIR}/bin/portable-run.sh" "${DIST_DIR}/run-mic-freq.sh"
    chmod +x "${DIST_DIR}/run.sh"
    chmod +x "${DIST_DIR}/run-ide.sh"
    chmod +x "${DIST_DIR}/run-mic-freq.sh"
    "${DIST_DIR}/selenite" --new-project "${DIST_DIR}/examples/projects/starter"
fi

"${RUBY}" -rjson -e 'name, version, api_file, mic_version = ARGV; full = name == "selenite"; binaries = full ? ["selenite", "selenite-game", "mic_freq/mic_freq"] : ["selenite-game"]; api = JSON.parse(File.read(api_file)); components = full ? { mic_freq: { version: mic_version, binary: "mic_freq/mic_freq", launcher: "run-mic-freq.sh", assets: "mic_freq/assets" } } : {}; puts JSON.pretty_generate(schema: 1, name: name, version: version, binaries: binaries, scripting: true, ruby_version: RUBY_VERSION, ruby_platform: RUBY_PLATFORM, components: components, capabilities: { ide: full, projects: true, project_checks: true, suite_menu: full, scale_menu: full, quick_open: full, terminal_tabs: full, asset_diagnostics: full, audio_presets: full, embedded_terminal: full, mic_freq: full, native_raylib_functions: api.fetch("functions").size })' "${APP_NAME}" "${VERSION}" "${SCRIPT_DIR}/src/ruby/raylib_api.json" "${MIC_FREQ_VERSION}" > "${DIST_DIR}/package-info.json"

echo "=== Creating portable tarball ==="
cd "${SCRIPT_DIR}"
TARBALL="${APP_NAME}-${VERSION}-portable.tar.gz"
tar -czf "${TARBALL}" -C "${DIST_DIR}" .
CHECKSUM_ARCHIVES=()
for name in selenite selenite-game; do
    archive="${name}-${VERSION}-portable.tar.gz"
    if [[ -f "${archive}" ]]; then CHECKSUM_ARCHIVES+=("${archive}"); fi
done
sha256sum "${CHECKSUM_ARCHIVES[@]}" > "selenite-${VERSION}-SHA256SUMS.tmp"
mv "selenite-${VERSION}-SHA256SUMS.tmp" "selenite-${VERSION}-SHA256SUMS"

if [[ "${VERIFY}" == true ]]; then
    echo "=== Verifying relocated portable archive ==="
    python3 "${SCRIPT_DIR}/tests/game_runtime_smoke.py" --archive "${SCRIPT_DIR}/${TARBALL}"
fi

echo "=== Distribution bundle complete! ==="
echo "Files ready in ${DIST_DIR}/ and packaged in ${TARBALL}"
echo "Release checksums: selenite-${VERSION}-SHA256SUMS"

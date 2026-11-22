#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ARCHIVE=""
DESTINATION=""
BUILD=false
DRY_RUN=false
while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --archive|--destination)
            if [[ "$#" -lt 2 || -z "$2" || "$2" == -* ]]; then
                printf '%s needs a path\n' "$1" >&2
                exit 2
            fi
            if [[ "$1" == --archive ]]; then ARCHIVE="$2"; else DESTINATION="$2"; fi
            shift
            ;;
        --build) BUILD=true ;;
        --dry-run) DRY_RUN=true ;;
        --help|-h)
            printf '%s\n' 'Usage: ./distribute.sh [--build] [--archive FILE] [--destination DIR] [--dry-run]' \
                'Copies and extracts the full Selenite suite (including mic_freq).' \
                'Default destination: $HOME/selenite.' \
                'Package files are replaced in place; existing mic_freq/assets and unmatched files are left untouched.' \
                '--build rebuilds the full package first; --archive uses an existing full archive.' \
                '--dry-run validates the archive and reports paths without building or writing files.' \
                'Python 3.11+ is required. Packaging and distribution normally run without elevated permissions.'
            exit 0
            ;;
        *) printf 'Unknown argument: %s\n' "$1" >&2; exit 2 ;;
    esac
    shift
done
command -v python3 >/dev/null || { printf 'Distribution requires Python 3.11+.\n' >&2; exit 1; }
PACKAGE_VERSIONS="$(python3 - "${SCRIPT_DIR}/Cargo.toml" "${SCRIPT_DIR}/mic_freq/Cargo.toml" <<'PY'
import sys
if sys.version_info < (3, 11):
    raise SystemExit("Distribution requires Python 3.11+.")
import tomllib
for manifest in sys.argv[1:]:
    with open(manifest, "rb") as source:
        print(tomllib.load(source)["package"]["version"])
PY
)"
mapfile -t PACKAGE_VERSIONS <<< "${PACKAGE_VERSIONS}"
VERSION="${PACKAGE_VERSIONS[0]}"
MIC_FREQ_VERSION="${PACKAGE_VERSIONS[1]}"
CHECKSUMS=""
if [[ -n "${ARCHIVE}" && "${BUILD}" == true ]]; then
    printf '%s\n' '--archive and --build cannot be combined.' >&2
    exit 2
fi
if [[ "${BUILD}" == true && "${DRY_RUN}" == true ]]; then
    printf '%s\n' '--build and --dry-run cannot be combined.' >&2
    exit 2
fi
if [[ -z "${ARCHIVE}" ]]; then
    ARCHIVE="${SCRIPT_DIR}/selenite-${VERSION}-portable.tar.gz"
    CHECKSUMS="${SCRIPT_DIR}/selenite-${VERSION}-SHA256SUMS"
    ARCHIVE_CURRENT=false
    if [[ -f "${ARCHIVE}" ]] && python3 - "${ARCHIVE}" "${VERSION}" "${MIC_FREQ_VERSION}" <<'PY'
import json
import sys
import tarfile
from pathlib import PurePosixPath

try:
    with tarfile.open(sys.argv[1], "r:gz") as bundle:
        member = next((item for item in bundle.getmembers()
                       if PurePosixPath(item.name).as_posix() == "package-info.json"), None)
        if member is None:
            raise SystemExit(1)
        metadata = json.load(bundle.extractfile(member))
    component = metadata.get("components", {}).get("mic_freq", {})
    if metadata.get("name") != "selenite" or metadata.get("version") != sys.argv[2] or component.get("version") != sys.argv[3]:
        raise SystemExit(1)
except (OSError, ValueError, KeyError, TypeError, tarfile.TarError):
    raise SystemExit(1)
PY
    then
        ARCHIVE_CURRENT=true
    fi
    if [[ "${BUILD}" == true || "${ARCHIVE_CURRENT}" != true ]]; then
        if [[ "${DRY_RUN}" == true ]]; then
            printf 'Archive is missing or out of date: %s\nBuild it with package.sh first.\n' "${ARCHIVE}" >&2
            exit 1
        fi
        if [[ "${EUID}" == 0 ]]; then
            printf '%s\n' 'Build with package.sh as your normal user before distributing with elevated permissions.' >&2
            exit 1
        fi
        "${SCRIPT_DIR}/package.sh" --output "${SCRIPT_DIR}/dist-v${VERSION%%.*}" --verify
        CHECKSUMS=""
    fi
fi
if [[ -z "${DESTINATION}" ]]; then
    DESTINATION="${HOME:?HOME is not set}/selenite"
fi

python3 - "${ARCHIVE}" "${DESTINATION}" "${VERSION}" "${MIC_FREQ_VERSION}" "${CHECKSUMS}" "${DRY_RUN}" "${SCRIPT_DIR}" <<'PY'
import datetime
import hashlib
import json
import os
import posixpath
import shutil
import stat
import sys
import tarfile
import tempfile
from pathlib import Path, PurePosixPath

if not hasattr(tarfile, "data_filter"):
    raise SystemExit("Distribution requires Python with safe tar extraction (Python 3.12+ recommended).")


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def safe_name(name):
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts:
        raise ValueError(f"Unsafe archive path: {name}")
    return path.as_posix()


def validate_target(path, directory=False):
    if path.is_symlink():
        raise ValueError(f"Refusing to write through an existing symlink: {path}")
    if path.exists() and (path.is_dir() != directory or not directory and not path.is_file()):
        raise ValueError(f"File/directory conflict at: {path}")


def validate_installation(destination):
    validate_target(destination, directory=True)
    if not destination.exists():
        return
    marker = destination / "package-info.json"
    if marker.is_symlink() or not marker.is_file() or marker.stat().st_size > 65536:
        raise ValueError(f"Refusing to replace a directory without Selenite package metadata: {destination}")
    metadata = json.loads(marker.read_text())
    if not isinstance(metadata, dict) or metadata.get("schema") != 1 or metadata.get("name") != "selenite":
        raise ValueError(f"Refusing to replace an unrelated directory: {destination}")


def overlay_directory(source, destination, preserve_assets=False, owner=None, relative=PurePosixPath()):
    for entry in source.iterdir():
        name = (relative / entry.name).as_posix()
        target = destination / entry.name
        if preserve_assets and name == "mic_freq/assets":
            continue
        if entry.is_dir() and not entry.is_symlink():
            if target.is_symlink() or (target.exists() and not target.is_dir()):
                if target.is_dir() and not target.is_symlink():
                    shutil.rmtree(target)
                else:
                    target.unlink()
            was_missing = not target.exists()
            target.mkdir(exist_ok=True)
            if was_missing and owner is not None:
                os.chown(target, *owner)
            overlay_directory(entry, target, preserve_assets, owner, relative / entry.name)
            shutil.copystat(entry, target, follow_symlinks=False)
            continue
        if target.is_dir() and not target.is_symlink():
            shutil.rmtree(target)
        elif target.exists() or target.is_symlink():
            target.unlink()
        os.replace(entry, target)


def main():
    archive = Path(sys.argv[1]).expanduser().resolve(strict=True)
    destination = Path(os.path.abspath(os.path.expanduser(sys.argv[2])))
    version, mic_freq_version, checksum_file, dry_run = sys.argv[3], sys.argv[4], sys.argv[5], sys.argv[6] == "true"
    resolved_destination = destination.resolve()
    source_root = Path(sys.argv[7]).resolve()
    if resolved_destination == Path(destination.anchor) or resolved_destination == Path.home().resolve() or source_root.is_relative_to(resolved_destination) or archive.is_relative_to(resolved_destination):
        raise ValueError(f"Refusing an unsafe installation destination: {destination}")
    validate_installation(destination)
    if "\n" in archive.name or "\r" in archive.name:
        raise ValueError("Archive filename cannot contain a newline")
    checksum = digest(archive)
    required_binaries = {"selenite", "selenite-game", "mic_freq/mic_freq"}
    required_tools = {"ffmpeg/bin/ffmpeg", "ffmpeg/bin/ffprobe"}
    launchers = ("run.sh", "run-ide.sh", "run-game.sh", "run-mic-freq.sh")

    if checksum_file and Path(checksum_file).is_file():
        expected = None
        for line in Path(checksum_file).read_text().splitlines():
            fields = line.split(maxsplit=1)
            if len(fields) == 2 and fields[1].lstrip("*") == archive.name:
                expected = fields[0]
        if expected is None or expected != checksum:
            raise ValueError("Archive checksum mismatch. Rebuild the full package or refresh its SHA256SUMS file.")
    parent = destination.parent
    copied_archive = parent / archive.name
    validate_target(copied_archive)
    validate_target(parent / (copied_archive.name + ".sha256"))
    print(f"Archive: {archive}", flush=True)
    print(f"Copy to: {copied_archive}", flush=True)
    print(f"Extract to: {destination}", flush=True)
    print(f"SHA256: {checksum}", flush=True)
    with tarfile.open(archive, "r:gz") as bundle:
        members = bundle.getmembers()
        names = set()
        metadata = None
        for member in members:
            name = safe_name(member.name)
            if name in names:
                raise ValueError(f"Duplicate archive path: {name}")
            names.add(name)
            if not (member.isfile() or member.isdir() or member.issym() or member.islnk()):
                raise ValueError(f"Unsupported archive member: {name}")
            if member.issym() or member.islnk():
                link = member.linkname if member.islnk() else posixpath.join(posixpath.dirname(name), member.linkname)
                safe_name(posixpath.normpath(link))
            if name == "package-info.json":
                if not member.isfile() or member.size > 65536:
                    raise ValueError("Invalid package-info.json")
                metadata = json.load(bundle.extractfile(member))
        mic_component = metadata.get("components", {}).get("mic_freq", {}) if isinstance(metadata, dict) else {}
        if not isinstance(metadata, dict) or metadata.get("schema") != 1 or metadata.get("name") != "selenite" or metadata.get("version") != version or mic_component.get("version") != mic_freq_version:
            raise ValueError(f"Expected a full Selenite {version} package with mic_freq {mic_freq_version}")
        if (mic_component.get("binary") != "mic_freq/mic_freq"
                or mic_component.get("launcher") != "mic_freq/mic_freq"
                or mic_component.get("manuals") != ["mic_freq/README.md", "mic_freq/MANUAL.md"]):
            raise ValueError("mic_freq component must launch directly and include its README and manual")
        capabilities = metadata.get("capabilities", {})
        required_capabilities = {
            "mic_freq", "audio_composer", "pitch_analysis", "harmonic_analysis",
            "rhythm_analysis", "graphics_studio", "image_layers", "image_filters",
            "animated_gif", "video_tools", "portable_ffmpeg", "pngtuber_graphics", "direct_suite_launch",
            "computer_algebra", "symbolic_math", "numerical_linear_algebra", "cas_notebook",
            "direct_executables", "shell_shims",
            "rustby_sdk", "complete_documentation",
            "linux_host", "linux_appliance",
        }
        if (not required_binaries.issubset(set(metadata.get("binaries", [])))
                or any(capabilities.get(name) is not True for name in required_capabilities)):
            raise ValueError("Incomplete suite: Selenite binaries, Audio Studio, Graphics Studio and Linux Host are required")
        if not any(name.startswith("lib/libavcodec.so") for name in names) or not any(
                name.startswith("lib/libavutil.so") for name in names):
            raise ValueError("Incomplete suite: portable ffmpeg codec libraries are missing")
        for name in required_binaries | required_tools | {"partitioned_array/lib/managed_partitioned_array.rb", "mic_freq/README.md", "mic_freq/MANUAL.md"}:
            if name not in names:
                raise ValueError(f"Incomplete suite: missing {name}")
        for name in ("lib/ruby", "lib/ruby-arch", "mic_freq/assets", "docs", "examples"):
            if name not in names:
                raise ValueError(f"Incomplete suite: missing {name}")
        rustby_manifest_name = "rustby/rustby-manifest.json"
        if rustby_manifest_name not in names:
            raise ValueError("Incomplete suite: missing rustby/rustby-manifest.json")
        rustby_manifest_member = next(
            member for member in members if safe_name(member.name) == rustby_manifest_name
        )
        rustby_manifest = json.load(bundle.extractfile(rustby_manifest_member))
        if rustby_manifest.get("schema") != 1 or not isinstance(rustby_manifest.get("files"), list):
            raise ValueError("Invalid Rustby manifest")
        for entry in rustby_manifest["files"]:
            relative = entry.get("path") if isinstance(entry, dict) else None
            expected = entry.get("sha256") if isinstance(entry, dict) else None
            if not isinstance(relative, str) or not isinstance(expected, str):
                raise ValueError("Invalid Rustby manifest entry")
            name = safe_name(posixpath.join("rustby", relative))
            if name not in names:
                raise ValueError(f"Incomplete Rustby payload: missing {name}")
            member = next(member for member in members if safe_name(member.name) == name)
            if not member.isfile() or hashlib.sha256(bundle.extractfile(member).read()).hexdigest() != expected:
                raise ValueError(f"Rustby payload checksum mismatch: {name}")
        if dry_run:
            print("Full suite validated; package files will be replaced in place, leaving existing mic_freq/assets untouched. Dry run made no changes.")
            return
        if not parent.is_dir() or not os.access(parent, os.W_OK):
            raise PermissionError(f"Destination parent is not writable: {parent}. Use a writable --destination or run the distribution step with appropriate permissions.")
        with tempfile.TemporaryDirectory(prefix=".selenite-distribute-", dir=parent) as staging:
            staging = Path(staging)
            staged_archive = staging / archive.name
            shutil.copyfile(archive, staged_archive)
            if digest(staged_archive) != checksum:
                raise ValueError("Archive changed while copying")
            extracted = staging / "suite"
            extracted.mkdir()
            with tarfile.open(staged_archive, "r:gz") as copied:
                copied.extractall(extracted, filter="data")
            mic_freq_directory = destination / "mic_freq"
            existing_assets = mic_freq_directory / "assets"
            if mic_freq_directory.is_symlink():
                raise ValueError(f"Refusing to preserve assets through a symlink: {mic_freq_directory}")
            if existing_assets.is_symlink() or (existing_assets.exists() and not existing_assets.is_dir()):
                raise ValueError(f"Refusing to preserve invalid mic_freq assets path: {existing_assets}")
            for name in required_binaries:
                path = extracted / name
                if not path.is_file() or not path.stat().st_mode & stat.S_IXUSR:
                    raise ValueError(f"Missing or non-executable suite component: {name}")
            for name in launchers:
                path = extracted / name
                if path.exists() and (not path.is_file() or not path.stat().st_mode & stat.S_IXUSR):
                    raise ValueError(f"Invalid optional shell shim: {name}")
            extracted.chmod(0o755)
            record = {"schema": 1, "version": version, "archive": copied_archive.name, "sha256": checksum,
                      "installed_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                      "components": metadata.get("components", {})}
            (extracted / "distribution-info.json").write_text(json.dumps(record, indent=2) + "\n")
            if os.geteuid() == 0 and os.environ.get("SUDO_UID") and os.environ.get("SUDO_GID"):
                uid, gid = int(os.environ["SUDO_UID"]), int(os.environ["SUDO_GID"])
                for folder, directories, files in os.walk(extracted, followlinks=False):
                    os.chown(folder, uid, gid, follow_symlinks=False)
                    for name in directories + files:
                        os.chown(Path(folder) / name, uid, gid, follow_symlinks=False)
            staged_archive.chmod(0o644)
            staged_checksum = staging / (copied_archive.name + ".sha256")
            staged_checksum.write_text(f"{checksum}  {copied_archive.name}\n")
            staged_checksum.chmod(0o644)
            validate_installation(destination)
            destination.mkdir(exist_ok=True)
            owner = None
            if os.geteuid() == 0 and os.environ.get("SUDO_UID") and os.environ.get("SUDO_GID"):
                owner = (int(os.environ["SUDO_UID"]), int(os.environ["SUDO_GID"]))
            overlay_directory(extracted, destination, preserve_assets=existing_assets.is_dir(), owner=owner)
            shutil.copyfile(staged_archive, copied_archive)
            copied_archive.chmod(0o644)
            if digest(copied_archive) != checksum:
                raise ValueError("Distributed archive checksum mismatch")
            staged_checksum.replace(parent / staged_checksum.name)
    print(f"Updated Selenite {version}: {destination} (package files replaced in place; existing mic_freq assets left untouched)")
    print(f"  {destination / 'mic_freq/mic_freq'}")
    for launcher in launchers:
        if (destination / launcher).is_file():
            print(f"  {destination / launcher} (compatibility shim)")


try:
    main()
except (OSError, ValueError, KeyError, TypeError, tarfile.TarError) as error:
    raise SystemExit(f"Distribution failed: {error}")
PY
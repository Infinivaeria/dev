import argparse
import configparser
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def read_entry(path):
    parser = configparser.ConfigParser(interpolation=None, delimiters=("=",))
    parser.optionxform = str
    if path.is_symlink():
        raise ValueError(f"Refusing a shortcut symlink: {path}")
    if path.exists():
        parser.read(path, encoding="utf-8")
        if parser.get("Desktop Entry", "X-Selenite-Managed", fallback="false") != "true":
            raise ValueError(f"Refusing to overwrite an unmanaged shortcut: {path}")
    return parser


def exec_argument(value):
    value = str(value)
    if any(ord(character) < 32 for character in value):
        raise ValueError("Shortcut paths cannot contain control characters")
    for character in ("\\", '"', "`", "$"):
        value = value.replace(character, "\\" + character)
    return ('"' + value.replace("%", "%%") + '"').replace("\\", "\\\\")


def entry_value(value):
    return str(value).replace("\\", "\\\\").replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")


def write_entry(path, parser):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent, delete=False) as output:
            temporary = Path(output.name)
            parser.write(output, space_around_delimiters=False)
        temporary.chmod(0o755)
        temporary.replace(path)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def desktop_directory(home):
    if os.environ.get("XDG_DESKTOP_DIR"):
        return Path(os.environ["XDG_DESKTOP_DIR"]).expanduser()
    try:
        result = subprocess.run(["xdg-user-dir", "DESKTOP"], text=True, capture_output=True, timeout=3, check=True)
        path = Path(result.stdout.strip())
        if path.is_absolute():
            return path
    except (OSError, subprocess.SubprocessError):
        pass
    return home / "Desktop"


def shortcut_tools(root, tool, target=None, grid=None):
    def launcher(binary, script, arguments=(), legacy_arguments=None):
        if (root / binary).is_file():
            return root / binary, list(arguments)
        return root / script, list(arguments if legacy_arguments is None else legacy_arguments)

    tools = {
        "organizer": ("selenite", "Selenite", *launcher("selenite", "run.sh")),
        "ide": ("selenite-ide", "Selenite IDE", *launcher("selenite", "run-ide.sh", ["--ide", str(target)] if target else ["--ide"], [str(target)] if target else [])),
        "terminal": ("selenite-consolegpu", "Selenite Terminal", *launcher("selenite", "run.sh", ["--terminal-window"])),
        "audio": ("selenite-audio", "Selenite Audio Studio", *launcher("mic_freq/mic_freq", "run-mic-freq.sh")),
        "composer": ("selenite-composer", "Selenite Music Composer", *launcher("selenite", "run.sh", ["--composer"])),
        "graphics": ("selenite-graphics", "Selenite Graphics Studio", *launcher("selenite", "run.sh", ["--graphics-studio"])),
        "cas": ("selenite-cas", "Selenite CAS Notebook", *launcher("selenite", "run.sh", ["--cas"])),
    }
    if tool == "all":
        result = [tools[key] for key in ("organizer", "ide", "terminal", "audio", "composer", "graphics", "cas")]
        result.append(("selenite-assets", "Selenite Asset Manager", *launcher("selenite", "run.sh", ["--inventory"])))
        starter = root / "examples/games/starter.rb"
        if ((root / "selenite-game").is_file() or (root / "run-game.sh").is_file()) and starter.is_file():
            result.append(("selenite-game", "Selenite Game", *launcher("selenite-game", "run-game.sh", [str(starter)])))
        if grid is not None:
            result.extend(shortcut_tools(root, "assets", grid, grid))
        return result
    if tool in tools:
        return [tools[tool]]
    if target is None:
        if tool == "game":
            target = root / "examples/games/starter.rb"
        else:
            raise ValueError(f"Choose a target for the {tool} shortcut")
    target = target.expanduser().resolve(strict=True)
    if any(ord(character) < 32 for character in str(target)):
        raise ValueError("Shortcut targets cannot contain control characters")
    name = target.name
    identifier = "selenite-" + tool + "-" + hashlib.sha256(os.fsencode(target)).hexdigest()[:12]
    executable, _ = launcher("selenite", "run.sh")
    if tool in ("project", "project-ide"):
        if not (target / "selenite-project.json").is_file():
            raise ValueError("Choose a Selenite project folder")
        arguments = ["--project" if tool == "project" else "--ide", str(target)]
        name += " - Run" if tool == "project" else " - IDE"
    elif tool == "game":
        if not target.is_file() or target.suffix not in (".rb", ".rbc"):
            raise ValueError("Choose a Ruby game source or bytecode file")
        executable, _ = launcher("selenite-game", "run-game.sh")
        arguments = [str(target)]
    elif tool in ("grid", "assets"):
        if not target.is_file() or target.suffix.lower() != ".json":
            raise ValueError("Choose an existing saved grid JSON file")
        arguments = ["--grid" if tool == "grid" else "--inventory", str(target)]
        name += " - Grid" if tool == "grid" else " - Assets"
    else:
        if target.is_file() and target.suffix.lower() in (".rb", ".md", ".txt", ".toml", ".json", ".sh", ".py", ".rs", ".js"):
            arguments = ["--edit", str(target)]
        else:
            opener = shutil.which("xdg-open")
            if opener is None:
                raise ValueError("Opening this file/folder requires xdg-open")
            executable = Path(opener)
            arguments = [str(target)]
    return [(identifier, name, executable, arguments)]


def main():
    parser = argparse.ArgumentParser(description="Install Selenite Linux application/desktop shortcuts or change their icon.")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--desktop", action="store_true", help="Also create desktop shortcuts")
    parser.add_argument("--desktop-only", action="store_true", help="Create only desktop shortcuts")
    parser.add_argument("--tool", choices=("all", "organizer", "ide", "terminal", "audio", "composer", "graphics", "cas", "game", "assets", "grid", "file", "project", "project-ide"), default="all")
    parser.add_argument("--target", type=Path)
    parser.add_argument("--grid", type=Path)
    parser.add_argument("--json", action="store_true", help="Return shortcut paths as JSON")
    parser.add_argument("--allow-empty", action="store_true", help="Allow icon updates before shortcuts are installed")
    parser.add_argument("--icon", default="applications-graphics", help="PNG/SVG/XPM image, or applications-graphics to reset")
    parser.add_argument("--update-icon", action="store_true", help="Update existing managed shortcuts without changing launch commands")
    options = parser.parse_args()
    home = Path.home()
    data = Path(os.environ.get("XDG_DATA_HOME", home / ".local/share")).expanduser()
    desktop = desktop_directory(home)
    applications = data / "applications"
    icon = options.icon
    icon_source = None
    if icon != "applications-graphics":
        icon_source = Path(icon).expanduser().resolve(strict=True)
        if not icon_source.is_file() or icon_source.suffix.lower() not in (".png", ".svg", ".xpm"):
            raise ValueError("Choose a PNG, SVG or XPM icon")
        if icon_source.stat().st_size > 8 * 1024 * 1024:
            raise ValueError("Icon exceeds 8 MiB")
        icon = str(data / "icons" / ("selenite-custom" + icon_source.suffix.lower()))
    if options.update_icon:
        paths = list(applications.glob("selenite*.desktop")) + list(desktop.glob("selenite*.desktop"))
        entries = []
        for path in paths:
            entry = read_entry(path)
            if entry.has_section("Desktop Entry"):
                entry["Desktop Entry"]["Icon"] = entry_value(icon)
                entries.append((path, entry))
        if not entries and not options.allow_empty:
            raise ValueError("No managed Selenite shortcuts found; install shortcuts first")
    else:
        root = options.root.expanduser().resolve(strict=True)
        tools = shortcut_tools(root, options.tool, options.target, options.grid)
        entries = []
        for identifier, name, executable, arguments in tools:
            if not executable.is_file() or not os.access(executable, os.X_OK):
                raise ValueError(f"Build the full portable suite first; missing executable: {executable}")
            command = [executable, *arguments]
            if "%" in str(executable):
                environment = shutil.which("env")
                if environment is None:
                    raise ValueError("An installation path containing '%' requires the env utility")
                command = [environment, "--", *command]
            directories = [desktop] if options.desktop_only else [applications] + ([desktop] if options.desktop else [])
            for directory in directories:
                path = directory / (identifier + ".desktop")
                entry = read_entry(path)
                entry["Desktop Entry"] = {
                    "Type": "Application", "Name": entry_value(name),
                    "Exec": " ".join(exec_argument(value) for value in command),
                    "Path": entry_value(root), "Icon": entry.get("Desktop Entry", "Icon", fallback=entry_value(icon)) if icon_source is None else entry_value(icon), "Terminal": "false",
                    "Categories": "Development;Utility;", "StartupNotify": "false",
                    "X-Selenite-Managed": "true",
                }
                if identifier == "selenite-consolegpu":
                    entry["Desktop Entry"]["Categories"] = "Development;Utility;System;TerminalEmulator;"
                    entry["Desktop Entry"]["X-TerminalArgExec"] = "--terminal"
                    entry["Desktop Entry"]["X-ExecArg"] = "--terminal"
                    entry["Desktop Entry"]["X-TerminalArgDir"] = "--working-directory"
                    entry["Desktop Entry"]["Actions"] = "SetAsDefaultTerminal;RestoreDefaultTerminal;"
                    for action, action_name, argument in (
                        ("SetAsDefaultTerminal", "Use Selenite as Default Terminal", "--set-gnome-terminal"),
                        ("RestoreDefaultTerminal", "Restore Previous Terminal", "--restore-gnome-terminal"),
                    ):
                        entry[f"Desktop Action {action}"] = {
                            "Name": entry_value(action_name),
                            "Exec": " ".join(exec_argument(value) for value in (executable, argument)),
                        }
                entries.append((path, entry))
    if icon_source is not None:
        destination = Path(icon)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if icon_source != destination.resolve():
            shutil.copyfile(icon_source, destination)
    for path, entry in entries:
        write_entry(path, entry)
        if not options.json:
            print(path)
    if options.json:
        print(json.dumps({"shortcuts": [str(path) for path, _ in entries]}))
    else:
        print("Shortcuts updated. Your desktop may require Allow Launching for desktop files.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, configparser.Error) as error:
        raise SystemExit(f"Desktop integration failed: {error}")
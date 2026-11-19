use std::{path::{Path, PathBuf}, process::Command};

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Created {
    shortcuts: Vec<PathBuf>,
}

#[derive(Deserialize, Serialize)]
struct GnomeTerminalSettings {
    exec: String,
    exec_arg: String,
    #[serde(default)]
    xdg_terminals_path: PathBuf,
    #[serde(default)]
    xdg_terminals_original: Option<String>,
    #[serde(default)]
    xdg_terminals_installed: String,
}

const GNOME_TERMINAL_SCHEMA: &str = "org.gnome.desktop.default-applications.terminal";

fn gsettings_get(key: &str) -> Result<String, String> {
    let output = Command::new("gsettings")
        .args(["get", GNOME_TERMINAL_SCHEMA, key])
        .output()
        .map_err(|error| format!("GNOME terminal settings require gsettings: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn gsettings_set(key: &str, value: &str) -> Result<(), String> {
    let output = Command::new("gsettings")
        .args(["set", GNOME_TERMINAL_SCHEMA, key, value])
        .output()
        .map_err(|error| format!("GNOME terminal settings require gsettings: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(())
}

fn gvariant_string(value: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}

fn terminal_settings_path() -> PathBuf {
    crate::profiles::default_base_dir().join("gnome-terminal-previous.json")
}

fn xdg_terminals_path() -> Result<PathBuf, String> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or("Could not locate the GNOME configuration directory")?;
    Ok(config_home.join("gnome-xdg-terminals.list"))
}

fn read_xdg_terminals(path: &Path) -> Result<Option<String>, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(format!("Refusing to replace a symlink: {}", path.display()))
        }
        Ok(metadata) if !metadata.is_file() => {
            Err(format!("Not a regular file: {}", path.display()))
        }
        Ok(_) => std::fs::read_to_string(path)
            .map(Some)
            .map_err(|error| error.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn write_xdg_terminals(path: &Path, contents: &str) -> Result<(), String> {
    if path.is_symlink() {
        return Err(format!("Refusing to replace a symlink: {}", path.display()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("list.tmp");
    std::fs::write(&temporary, contents).map_err(|error| error.to_string())?;
    std::fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn preferred_xdg_terminals(contents: Option<&str>) -> String {
    let retained = contents
        .unwrap_or_default()
        .lines()
        .filter(|line| line.trim() != "selenite-consolegpu.desktop")
        .collect::<Vec<_>>();
    if retained.is_empty() {
        "selenite-consolegpu.desktop\n".to_owned()
    } else {
        format!("selenite-consolegpu.desktop\n{}\n", retained.join("\n"))
    }
}

fn without_selenite_terminal(contents: &str) -> String {
    let had_trailing_newline = contents.ends_with('\n');
    let mut cleaned = contents
        .lines()
        .filter(|line| line.trim() != "selenite-consolegpu.desktop")
        .collect::<Vec<_>>()
        .join("\n");
    if had_trailing_newline && !cleaned.is_empty() {
        cleaned.push('\n');
    }
    cleaned
}

fn restore_xdg_terminals(settings: &GnomeTerminalSettings) -> Result<(), String> {
    if settings.xdg_terminals_path.as_os_str().is_empty()
        || settings.xdg_terminals_installed.is_empty()
    {
        return Ok(());
    }
    let Some(current) = read_xdg_terminals(&settings.xdg_terminals_path)? else {
        return Ok(());
    };
    if current == settings.xdg_terminals_installed {
        if let Some(original) = &settings.xdg_terminals_original {
            return write_xdg_terminals(&settings.xdg_terminals_path, original);
        }
        std::fs::remove_file(&settings.xdg_terminals_path)
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    let cleaned = without_selenite_terminal(&current);
    if cleaned.is_empty() {
        std::fs::remove_file(&settings.xdg_terminals_path)
            .map_err(|error| error.to_string())?;
        Ok(())
    } else {
        write_xdg_terminals(&settings.xdg_terminals_path, &cleaned)
    }
}

fn save_terminal_settings(path: &Path, settings: &GnomeTerminalSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("tmp");
    let contents = serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?;
    std::fs::write(&temporary, contents).map_err(|error| error.to_string())?;
    std::fs::rename(temporary, path).map_err(|error| error.to_string())
}

pub fn use_as_gnome_terminal() -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("GNOME terminal integration is available on Linux only".into());
    }
    let state_path = terminal_settings_path();
    let mut previous = if state_path.is_file() {
        let contents = std::fs::read(&state_path).map_err(|error| error.to_string())?;
        serde_json::from_slice(&contents).map_err(|error| error.to_string())?
    } else {
        let xdg_path = xdg_terminals_path()?;
        let previous = GnomeTerminalSettings {
            exec: gsettings_get("exec")?,
            exec_arg: gsettings_get("exec-arg")?,
            xdg_terminals_original: read_xdg_terminals(&xdg_path)?,
            xdg_terminals_path: xdg_path,
            xdg_terminals_installed: String::new(),
        };
        previous
    };
    if previous.xdg_terminals_path.as_os_str().is_empty() {
        previous.xdg_terminals_path = xdg_terminals_path()?;
        previous.xdg_terminals_original = read_xdg_terminals(&previous.xdg_terminals_path)?;
    }
    previous.xdg_terminals_installed = preferred_xdg_terminals(
        read_xdg_terminals(&previous.xdg_terminals_path)?.as_deref(),
    );
    save_terminal_settings(&state_path, &previous)?;
    write_xdg_terminals(
        &previous.xdg_terminals_path,
        &previous.xdg_terminals_installed,
    )?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    if let Err(error) = gsettings_set("exec", &gvariant_string(&executable.to_string_lossy())?) {
        let _ = restore_xdg_terminals(&previous);
        return Err(error);
    }
    if let Err(error) = gsettings_set("exec-arg", &gvariant_string("--terminal-window")?) {
        let _ = gsettings_set("exec", &previous.exec);
        let _ = restore_xdg_terminals(&previous);
        return Err(error);
    }
    Ok(())
}

pub fn restore_gnome_terminal() -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("GNOME terminal integration is available on Linux only".into());
    }
    let state_path = terminal_settings_path();
    let contents = std::fs::read(&state_path)
        .map_err(|error| format!("No saved GNOME terminal setting to restore: {error}"))?;
    let previous: GnomeTerminalSettings =
        serde_json::from_slice(&contents).map_err(|error| error.to_string())?;
    let current_exec = gsettings_get("exec")?;
    let current_arg = gsettings_get("exec-arg")?;
    gsettings_set("exec", &previous.exec)?;
    if let Err(error) = gsettings_set("exec-arg", &previous.exec_arg) {
        let _ = gsettings_set("exec", &current_exec);
        let _ = gsettings_set("exec-arg", &current_arg);
        return Err(error);
    }
    if let Err(error) = restore_xdg_terminals(&previous) {
        let _ = gsettings_set("exec", &current_exec);
        let _ = gsettings_set("exec-arg", &current_arg);
        return Err(error);
    }
    std::fs::remove_file(state_path).map_err(|error| error.to_string())
}

pub fn create(tool: &str, target: Option<&Path>, grid: Option<&Path>) -> Result<String, String> {
    create_shortcuts(tool, target, grid, true)
}

pub fn create_application_shortcuts() -> Result<String, String> {
    create_shortcuts("all", None, None, false)
}

fn create_shortcuts(tool: &str, target: Option<&Path>, grid: Option<&Path>, desktop_only: bool) -> Result<String, String> {
    if !cfg!(target_os = "linux") { return Err("Desktop shortcut creation currently supports Linux".into()); }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let root = std::env::var_os("SELENITE_SHORTCUT_ROOT").map(PathBuf::from)
        .unwrap_or_else(|| executable.parent().unwrap_or(Path::new(".")).to_owned());
    let mut command = Command::new("python3");
    command.args(["-c", concat!("__file__ = 'desktop_shortcuts.py'\n", include_str!("../tools/desktop_shortcuts.py"))])
        .arg("--root").arg(root);
    if desktop_only { command.arg("--desktop-only"); }
    command.args(["--json", "--tool", tool]);
    if let Some(target) = target { command.arg("--target").arg(target); }
    if let Some(grid) = grid { command.arg("--grid").arg(grid); }
    let settings = crate::settings::Settings::load(&crate::settings::Settings::path_in(&crate::profiles::default_base_dir()));
    if let Some(icon) = settings.application_icon.filter(|path| path.is_file()) { command.arg("--icon").arg(icon); }
    let output = command.output().map_err(|error| format!("Desktop shortcuts require Python 3: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let created: Created = serde_json::from_slice(&output.stdout).map_err(|error| format!("Invalid shortcut result: {error}"))?;
    Ok(if desktop_only {
        if created.shortcuts.len() == 1 { "Desktop shortcut created".into() } else { format!("Created {} desktop shortcuts", created.shortcuts.len()) }
    } else {
        format!("Added {} apps to the GNOME application menu", created.shortcuts.len())
    })
}

pub fn for_path(path: &Path) -> Result<String, String> {
    create(if path.is_dir() && path.join(crate::project::MANIFEST).is_file() { "project" } else { "file" }, Some(path), None)
}

pub fn icon_bytes(path: &Path) -> Result<Vec<u8>, String> {
    if std::fs::metadata(path).map_err(|error| error.to_string())?.len() > 8 * 1024 * 1024 {
        return Err("Choose an image smaller than 8 MiB".into());
    }
    let mut reader = image::ImageReader::open(path).map_err(|error| error.to_string())?
        .with_guessed_format().map_err(|error| error.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|error| format!("Cannot read image: {error}. Use PNG, JPEG, BMP, ICO or WebP."))?.thumbnail(256, 256).to_rgba8();
    let mut square = image::RgbaImage::new(256, 256);
    image::imageops::overlay(&mut square, &decoded, i64::from((256 - decoded.width()) / 2), i64::from((256 - decoded.height()) / 2));
    let mut output = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(square).write_to(&mut output, image::ImageFormat::Png).map_err(|error| error.to_string())?;
    Ok(output.into_inner())
}

pub fn change_icon(bytes: Option<&[u8]>) -> Result<Option<PathBuf>, String> {
    let settings_path = crate::settings::Settings::path_in(&crate::profiles::default_base_dir());
    let mut settings = crate::settings::Settings::load(&settings_path);
    let stored = settings_path.with_file_name("application-icon.png");
    let previous = if bytes.is_some() && stored.is_file() { Some(std::fs::read(&stored).map_err(|error| error.to_string())?) } else { None };
    if let Some(bytes) = bytes {
        std::fs::create_dir_all(stored.parent().unwrap()).map_err(|error| error.to_string())?;
        let temporary = stored.with_extension("png.tmp");
        std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
        std::fs::rename(temporary, &stored).map_err(|error| error.to_string())?;
    }
    let result = (|| {
    let mut command = Command::new("python3");
    command.args(["-c", concat!("__file__ = 'desktop_shortcuts.py'\n", include_str!("../tools/desktop_shortcuts.py")), "--update-icon", "--allow-empty", "--json", "--icon"]);
    command.arg(if bytes.is_some() { stored.as_os_str() } else { std::ffi::OsStr::new("applications-graphics") });
    let output = command.output().map_err(|error| format!("Shortcut icon updates require Python 3: {error}"))?;
    if !output.status.success() { return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()); }
    settings.application_icon = bytes.map(|_| stored.clone());
    settings.save(&settings_path)?;
    Ok(settings.application_icon)
    })();
    if result.is_err() && bytes.is_some() {
        if let Some(previous) = previous { let _ = std::fs::write(&stored, previous); }
        else { let _ = std::fs::remove_file(&stored); }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_images_are_validated_and_normalized_without_changing_sources() {
        let directory = std::env::temp_dir().join(format!("selenite-icon-images-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("wide.png");
        image::RgbaImage::from_pixel(120, 60, image::Rgba([20, 180, 90, 255])).save(&source).unwrap();
        let original = std::fs::read(&source).unwrap();
        let normalized = image::load_from_memory(&icon_bytes(&source).unwrap()).unwrap().to_rgba8();
        assert_eq!(normalized.dimensions(), (256, 256));
        assert_eq!(normalized.get_pixel(0, 0).0[3], 0);
        assert_eq!(normalized.get_pixel(128, 128).0, [20, 180, 90, 255]);
        assert_eq!(std::fs::read(&source).unwrap(), original);
        let invalid = directory.join("invalid.png");
        std::fs::write(&invalid, "not an image").unwrap();
        assert!(icon_bytes(&invalid).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
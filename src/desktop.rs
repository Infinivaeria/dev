use std::{path::{Path, PathBuf}, process::Command};

use serde::Deserialize;

#[derive(Deserialize)]
struct Created {
    shortcuts: Vec<PathBuf>,
}

pub fn create(tool: &str, target: Option<&Path>, grid: Option<&Path>) -> Result<String, String> {
    if !cfg!(target_os = "linux") { return Err("Desktop shortcut creation currently supports Linux".into()); }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let root = std::env::var_os("SELENITE_SHORTCUT_ROOT").map(PathBuf::from)
        .unwrap_or_else(|| executable.parent().unwrap_or(Path::new(".")).to_owned());
    let mut command = Command::new("python3");
    command.args(["-c", concat!("__file__ = 'desktop_shortcuts.py'\n", include_str!("../tools/desktop_shortcuts.py"))])
        .arg("--root").arg(root).args(["--desktop-only", "--json", "--tool", tool]);
    if let Some(target) = target { command.arg("--target").arg(target); }
    if let Some(grid) = grid { command.arg("--grid").arg(grid); }
    let settings = crate::settings::Settings::load(&crate::settings::Settings::path_in(&crate::profiles::default_base_dir()));
    if let Some(icon) = settings.application_icon.filter(|path| path.is_file()) { command.arg("--icon").arg(icon); }
    let output = command.output().map_err(|error| format!("Desktop shortcuts require Python 3: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    let created: Created = serde_json::from_slice(&output.stdout).map_err(|error| format!("Invalid shortcut result: {error}"))?;
    Ok(if created.shortcuts.len() == 1 { "Desktop shortcut created".into() } else { format!("Created {} desktop shortcuts", created.shortcuts.len()) })
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
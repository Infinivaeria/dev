//! User preferences shared by every profile, stored as
//! `<SELENITE_HOME>/settings.json`. Unknown or missing fields fall back to
//! defaults so older/newer files always load.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::music::Repeat;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Coordinates, file names and type badges on grid cells (2D and 3D).
    pub labels: bool,
    pub minimap: bool,
    /// Music player volume, 0.0..=1.0.
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: Repeat,
    /// Plugin file names (e.g. `clock.rb`) that should not be loaded.
    pub disabled_plugins: Vec<String>,
    /// Copy files dropped or pasted onto a grid into that grid's `assets/`
    /// folder (instead of only linking to the original).
    pub copy_files: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            labels: true,
            minimap: true,
            volume: 0.8,
            shuffle: false,
            repeat: Repeat::All,
            disabled_plugins: Vec::new(),
            copy_files: true,
        }
    }
}

impl Settings {
    pub fn path_in(base: &Path) -> PathBuf {
        base.join("settings.json")
    }

    /// Loads settings, falling back to defaults when the file is missing
    /// or unreadable (a broken settings file must never block startup).
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .map(Self::sanitized)
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|error| error.to_string())?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, text)
            .and_then(|()| fs::rename(&temporary, path))
            .map_err(|error| format!("could not write {}: {error}", path.display()))
    }

    pub fn plugin_enabled(&self, file_name: &str) -> bool {
        !self.disabled_plugins.iter().any(|name| name == file_name)
    }

    pub fn set_plugin_enabled(&mut self, file_name: &str, enabled: bool) {
        self.disabled_plugins.retain(|name| name != file_name);
        if !enabled {
            self.disabled_plugins.push(file_name.to_owned());
            self.disabled_plugins.sort();
        }
    }

    fn sanitized(mut self) -> Self {
        self.volume = if self.volume.is_finite() {
            self.volume.clamp(0.0, 1.0)
        } else {
            Self::default().volume
        };
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_tolerates_partial_files() {
        let dir = std::env::temp_dir().join(format!("selenite_settings_{}", std::process::id()));
        let path = Settings::path_in(&dir);
        assert_eq!(Settings::load(&path), Settings::default());

        let mut settings = Settings {
            labels: false,
            volume: 0.25,
            shuffle: true,
            repeat: Repeat::One,
            ..Settings::default()
        };
        settings.set_plugin_enabled("clock.rb", false);
        settings.set_plugin_enabled("a.rb", false);
        settings.set_plugin_enabled("a.rb", true);
        assert_eq!(settings.disabled_plugins, vec!["clock.rb".to_owned()]);
        assert!(!settings.plugin_enabled("clock.rb"));
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);

        fs::write(&path, r#"{"labels": false, "volume": 7.5}"#).unwrap();
        let loaded = Settings::load(&path);
        assert!(!loaded.labels);
        assert!(loaded.minimap);
        assert!(loaded.copy_files, "older settings files default to copying");
        assert_eq!(loaded.volume, 1.0);

        fs::write(&path, "not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        let _ = fs::remove_dir_all(&dir);
    }
}

//! Local account profiles. Each profile is a directory holding its own root
//! grid (`selenite.json`), downloaded/pasted assets, partitioned_array
//! exports and an optional `init.rb` that runs when the profile is opened.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::persistence::SavedGrid;

pub const DEFAULT_PROFILE: &str = "default";
const PROFILES_DIR: &str = "profiles";
const STATE_FILE: &str = "profiles.json";
const MAX_NAME_LEN: usize = 40;

#[derive(Default, Serialize, Deserialize)]
struct StoreState {
    last_used: Option<String>,
}

/// One profile's on-disk locations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub dir: PathBuf,
}

impl Profile {
    pub fn grid_path(&self) -> PathBuf {
        self.dir.join("selenite.json")
    }

    pub fn init_script(&self) -> PathBuf {
        self.dir.join("init.rb")
    }
}

/// Collection of profiles rooted at a base data directory.
#[derive(Clone, Debug)]
pub struct ProfileStore {
    base: PathBuf,
}

/// `$SELENITE_HOME`, else the platform data directory + `selenite`.
pub fn default_base_dir() -> PathBuf {
    if let Some(home) = env::var_os("SELENITE_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(home);
    }
    #[cfg(target_os = "windows")]
    if let Some(appdata) = env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("Selenite");
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join("Library/Application Support/Selenite");
    }
    if let Some(data) = env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(data).join("selenite");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/selenite");
    }
    PathBuf::from(".selenite")
}

/// Validates and normalises a user-typed profile name.
pub fn validate_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("profile name cannot be empty".to_owned());
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(format!(
            "profile name is longer than {MAX_NAME_LEN} characters"
        ));
    }
    if name.starts_with('.') {
        return Err("profile name cannot start with '.'".to_owned());
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !(c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.')))
    {
        return Err(format!("profile name cannot contain '{bad}'"));
    }
    Ok(name.to_owned())
}

impl ProfileStore {
    pub fn open_default() -> Self {
        Self::at(default_base_dir())
    }

    pub fn at(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    fn profiles_root(&self) -> PathBuf {
        self.base.join(PROFILES_DIR)
    }

    pub fn profile(&self, name: &str) -> Profile {
        Profile {
            name: name.to_owned(),
            dir: self.profiles_root().join(name),
        }
    }

    pub fn exists(&self, name: &str) -> bool {
        self.profile(name).dir.is_dir()
    }

    /// Sorted profile names (case-insensitive order).
    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.profiles_root())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| validate_name(name).is_ok())
            .collect();
        names.sort_by_key(|name| name.to_lowercase());
        names
    }

    pub fn create(&self, name: &str) -> Result<Profile, String> {
        let name = validate_name(name)?;
        if self.exists(&name) {
            return Err(format!("profile '{name}' already exists"));
        }
        self.ensure(&name)
    }

    /// Returns the profile, creating its directory if needed.
    pub fn ensure(&self, name: &str) -> Result<Profile, String> {
        let name = validate_name(name)?;
        let profile = self.profile(&name);
        fs::create_dir_all(&profile.dir)
            .map_err(|error| format!("could not create {}: {error}", profile.dir.display()))?;
        Ok(profile)
    }

    /// Renames a profile directory and rewrites saved file paths that
    /// pointed inside it (pasted/downloaded assets).
    pub fn rename(&self, old: &str, new: &str) -> Result<Profile, String> {
        let new = validate_name(new)?;
        if !self.exists(old) {
            return Err(format!("profile '{old}' does not exist"));
        }
        if old == new {
            return Ok(self.profile(old));
        }
        if self.exists(&new) {
            return Err(format!("profile '{new}' already exists"));
        }
        let from = self.profile(old);
        let to = self.profile(&new);
        fs::rename(&from.dir, &to.dir).map_err(|error| {
            format!(
                "could not rename {} to {}: {error}",
                from.dir.display(),
                to.dir.display()
            )
        })?;
        let grid_path = to.grid_path();
        if let Some(mut grid) = SavedGrid::load(&grid_path).map_err(|error| error.to_string())? {
            if grid.rewrite_path_prefix(&from.dir, &to.dir) > 0 {
                grid.save(&grid_path).map_err(|error| error.to_string())?;
            }
        }
        if self.last_used().as_deref() == Some(old) {
            self.set_last_used(&new)?;
        }
        Ok(to)
    }

    /// Deletes a profile directory and everything inside it.
    pub fn delete(&self, name: &str) -> Result<(), String> {
        let name = validate_name(name)?;
        let profile = self.profile(&name);
        if !profile.dir.is_dir() {
            return Err(format!("profile '{name}' does not exist"));
        }
        fs::remove_dir_all(&profile.dir)
            .map_err(|error| format!("could not delete {}: {error}", profile.dir.display()))?;
        if self.last_used().as_deref() == Some(name.as_str()) {
            let _ = fs::remove_file(self.base.join(STATE_FILE));
        }
        Ok(())
    }

    pub fn last_used(&self) -> Option<String> {
        let text = fs::read_to_string(self.base.join(STATE_FILE)).ok()?;
        serde_json::from_str::<StoreState>(&text).ok()?.last_used
    }

    pub fn set_last_used(&self, name: &str) -> Result<(), String> {
        fs::create_dir_all(&self.base)
            .map_err(|error| format!("could not create {}: {error}", self.base.display()))?;
        let state = StoreState {
            last_used: Some(name.to_owned()),
        };
        let text = serde_json::to_string_pretty(&state).map_err(|error| error.to_string())?;
        fs::write(self.base.join(STATE_FILE), text).map_err(|error| error.to_string())
    }

    /// The profile to open at startup: an explicit name, else the last
    /// used one, else `default`. Always exists on return.
    pub fn startup_profile(&self, requested: Option<&str>) -> Result<Profile, String> {
        let name = match requested {
            Some(name) => validate_name(name)?,
            None => self
                .last_used()
                .filter(|name| self.exists(name))
                .or_else(|| self.list().into_iter().next())
                .unwrap_or_else(|| DEFAULT_PROFILE.to_owned()),
        };
        let profile = self.ensure(&name)?;
        self.set_last_used(&profile.name)?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn temp_store(tag: &str) -> ProfileStore {
        let base = std::env::temp_dir().join(format!(
            "selenite-profiles-{tag}-{}-{}",
            std::process::id(),
            crate::timestamp()
        ));
        let _ = fs::remove_dir_all(&base);
        ProfileStore::at(base)
    }

    #[test]
    fn validates_names() {
        assert_eq!(validate_name("  Work 2 ").unwrap(), "Work 2");
        assert!(validate_name("").is_err());
        assert!(validate_name("../evil").is_err());
        assert!(validate_name(".hidden").is_err());
        assert!(validate_name("a/b").is_err());
        assert!(validate_name(&"x".repeat(41)).is_err());
    }

    #[test]
    fn create_list_rename_delete_and_remember_last_used() {
        let store = temp_store("crud");
        let first = store.startup_profile(None).unwrap();
        assert_eq!(first.name, DEFAULT_PROFILE);
        assert_eq!(store.last_used().as_deref(), Some(DEFAULT_PROFILE));

        let work = store.create("Work").unwrap();
        assert!(store.create("Work").is_err());
        assert_eq!(store.list(), vec!["default".to_owned(), "Work".to_owned()]);

        let mut grid = SavedGrid::new();
        let asset = work.dir.join(".selenite-assets/pic.png");
        grid.insert_file(0, 0, asset).unwrap();
        grid.insert_file(1, 0, PathBuf::from("/outside/song.mp3"))
            .unwrap();
        grid.save(&work.grid_path()).unwrap();
        store.set_last_used("Work").unwrap();

        let renamed = store.rename("Work", "Job").unwrap();
        assert!(!store.exists("Work"));
        assert_eq!(store.last_used().as_deref(), Some("Job"));
        let loaded = SavedGrid::load(&renamed.grid_path()).unwrap().unwrap();
        assert_eq!(
            loaded.file_at(0, 0),
            Some(renamed.dir.join(".selenite-assets/pic.png").as_path())
        );
        assert_eq!(
            loaded.file_at(1, 0),
            Some(PathBuf::from("/outside/song.mp3").as_path())
        );
        assert_eq!(store.startup_profile(None).unwrap().name, "Job");

        store.delete("Job").unwrap();
        assert_eq!(store.list(), vec!["default".to_owned()]);
        assert!(store.last_used().is_none());
        assert_eq!(store.startup_profile(None).unwrap().name, DEFAULT_PROFILE);
        let _ = fs::remove_dir_all(store.base());
    }
}

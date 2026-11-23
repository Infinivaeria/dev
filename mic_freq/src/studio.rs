use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::voice::VoiceBand;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FrequencyProfile {
    pub enabled: bool,
    pub bounce: f32,
    pub scale: f32,
    pub mirror: bool,
    pub blink: bool,
}

impl Default for FrequencyProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            bounce: 1.0,
            scale: 1.0,
            mirror: false,
            blink: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub gate_db: f32,
    pub loud_delta_db: f32,
    pub low_voice_gain_db: f32,
    pub mid_voice_gain_db: f32,
    pub high_voice_gain_db: f32,
    pub bounce: f32,
    pub scale: f32,
    pub mirror: bool,
    pub blink: bool,
    pub background: usize,
    pub device: Option<String>,
    pub assets: Option<PathBuf>,
    pub low_profile: FrequencyProfile,
    pub mid_profile: FrequencyProfile,
    pub high_profile: FrequencyProfile,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gate_db: -45.0,
            loud_delta_db: 24.0,
            low_voice_gain_db: 0.0,
            mid_voice_gain_db: 0.0,
            high_voice_gain_db: 0.0,
            bounce: 1.0,
            scale: 1.0,
            mirror: false,
            blink: true,
            background: 0,
            device: None,
            assets: None,
            low_profile: FrequencyProfile::default(),
            mid_profile: FrequencyProfile::default(),
            high_profile: FrequencyProfile::default(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let mut settings: Self = match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error.into()),
        };
        settings.gate_db = finite_clamp(settings.gate_db, -90.0, -26.0, -45.0);
        settings.loud_delta_db = finite_clamp(settings.loud_delta_db, 8.0, 36.0, 24.0);
        settings.low_voice_gain_db = finite_clamp(settings.low_voice_gain_db, -18.0, 18.0, 0.0);
        settings.mid_voice_gain_db = finite_clamp(settings.mid_voice_gain_db, -18.0, 18.0, 0.0);
        settings.high_voice_gain_db = finite_clamp(settings.high_voice_gain_db, -18.0, 18.0, 0.0);
        settings.bounce = finite_clamp(settings.bounce, 0.0, 2.0, 1.0);
        settings.scale = finite_clamp(settings.scale, 0.2, 1.0, 1.0);
        settings.low_profile.clamp();
        settings.mid_profile.clamp();
        settings.high_profile.clamp();
        settings.background %= 3;
        Ok(settings)
    }

    pub fn frequency_profile(&self, band: VoiceBand) -> Option<&FrequencyProfile> {
        match band {
            VoiceBand::Low => Some(&self.low_profile),
            VoiceBand::Mid => Some(&self.mid_profile),
            VoiceBand::High => Some(&self.high_profile),
            VoiceBand::Unknown => None,
        }
    }

    pub fn frequency_profile_mut(&mut self, band: VoiceBand) -> Option<&mut FrequencyProfile> {
        match band {
            VoiceBand::Low => Some(&mut self.low_profile),
            VoiceBand::Mid => Some(&mut self.mid_profile),
            VoiceBand::High => Some(&mut self.high_profile),
            VoiceBand::Unknown => None,
        }
    }

    pub fn frequency_gain(&self, band: VoiceBand) -> Option<f32> {
        match band {
            VoiceBand::Low => Some(self.low_voice_gain_db),
            VoiceBand::Mid => Some(self.mid_voice_gain_db),
            VoiceBand::High => Some(self.high_voice_gain_db),
            VoiceBand::Unknown => None,
        }
    }

    pub fn set_frequency_gain(&mut self, band: VoiceBand, gain_db: f32) -> bool {
        let target = match band {
            VoiceBand::Low => &mut self.low_voice_gain_db,
            VoiceBand::Mid => &mut self.mid_voice_gain_db,
            VoiceBand::High => &mut self.high_voice_gain_db,
            VoiceBand::Unknown => return false,
        };
        *target = finite_clamp(gain_db, -18.0, 18.0, 0.0);
        true
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temporary, path)?;
        Ok(())
    }

    fn preset_path(settings: &Path, name: &str) -> Result<PathBuf, Box<dyn Error>> {
        if name.trim().is_empty()
            || name.len() > 64
            || !name.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_')
            })
        {
            return Err(
                "Preset names must contain 1-64 letters, numbers, spaces, '-' or '_'".into(),
            );
        }
        Ok(settings
            .parent()
            .unwrap_or(Path::new("."))
            .join("presets")
            .join(format!("{name}.json")))
    }

    pub fn presets(settings: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        let directory = settings.parent().unwrap_or(Path::new(".")).join("presets");
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "json")
            {
                if let Some(name) = entry.path().file_stem().and_then(|name| name.to_str()) {
                    if Self::preset_path(settings, name).is_ok() {
                        names.push(name.to_owned());
                    }
                }
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn save_preset(&self, settings: &Path, name: &str) -> Result<(), Box<dyn Error>> {
        let mut preset = self.clone();
        preset.device = None;
        preset.assets = None;
        preset.save(&Self::preset_path(settings, name)?)
    }

    pub fn load_preset(&mut self, settings: &Path, name: &str) -> Result<(), Box<dyn Error>> {
        let path = Self::preset_path(settings, name)?;
        if !path.is_file() {
            return Err(format!("Preset does not exist: {name}").into());
        }
        let mut preset = Self::load(&path)?;
        preset.device = self.device.clone();
        preset.assets = self.assets.clone();
        *self = preset;
        Ok(())
    }
}

impl FrequencyProfile {
    fn clamp(&mut self) {
        self.bounce = finite_clamp(self.bounce, 0.0, 2.0, 1.0);
        self.scale = finite_clamp(self.scale, 0.2, 1.0, 1.0);
    }
}

fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

pub fn settings_path() -> PathBuf {
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(std::env::temp_dir);
    root.join("mic_freq/settings.json")
}

pub struct BlinkState {
    elapsed: f32,
    interval: f32,
    duration: f32,
    seed: u32,
}

impl Default for BlinkState {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            interval: 2.8,
            duration: 0.22,
            seed: 0x12345678,
        }
    }
}

impl BlinkState {
    pub fn update(&mut self, dt: f32, enabled: bool) -> bool {
        if !enabled {
            self.elapsed = 0.0;
            return false;
        }
        self.elapsed += dt.clamp(0.0, 0.1);
        if self.elapsed >= self.interval + self.duration {
            self.elapsed = 0.0;
            self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
            self.interval = 2.0 + (self.seed as f64 / u32::MAX as f64) as f32 * 3.0;
        }
        self.elapsed >= self.interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_presets_restore_voice_controls_without_switching_devices_or_assets() {
        let directory =
            std::env::temp_dir().join(format!("mic-freq-presets-{}", std::process::id()));
        let config = directory.join("settings.json");
        let mut settings = Settings {
            gate_db: -60.0,
            low_voice_gain_db: 7.0,
            high_voice_gain_db: -5.0,
            mirror: true,
            device: Some("Current microphone".into()),
            assets: Some("current avatar".into()),
            ..Settings::default()
        };
        settings.save_preset(&config, "Quiet room").unwrap();
        assert_eq!(Settings::presets(&config).unwrap(), vec!["Quiet room"]);
        settings.gate_db = -30.0;
        settings.low_voice_gain_db = -18.0;
        settings.high_voice_gain_db = 18.0;
        settings.mirror = false;
        settings.load_preset(&config, "Quiet room").unwrap();
        assert_eq!(settings.gate_db, -60.0);
        assert_eq!(settings.low_voice_gain_db, 7.0);
        assert_eq!(settings.high_voice_gain_db, -5.0);
        assert!(settings.mirror);
        assert_eq!(settings.device.as_deref(), Some("Current microphone"));
        assert_eq!(settings.assets, Some("current avatar".into()));
        assert!(settings.save_preset(&config, "../escape").is_err());
        assert!(settings.load_preset(&config, "Missing").is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn settings_round_trip_and_clamp_out_of_range_values() {
        let dir = std::env::temp_dir().join(format!("mic-freq-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        let settings = Settings {
            gate_db: 20.0,
            low_voice_gain_db: 100.0,
            mid_voice_gain_db: 9.0,
            high_voice_gain_db: -100.0,
            bounce: -4.0,
            scale: 8.0,
            device: Some("Test input".into()),
            mid_profile: FrequencyProfile {
                enabled: true,
                bounce: -1.0,
                scale: 9.0,
                mirror: true,
                blink: false,
            },
            ..Default::default()
        };
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.gate_db, -26.0);
        assert_eq!(loaded.low_voice_gain_db, 18.0);
        assert_eq!(loaded.mid_voice_gain_db, 9.0);
        assert_eq!(loaded.high_voice_gain_db, -18.0);
        assert_eq!(loaded.bounce, 0.0);
        assert_eq!(loaded.scale, 1.0);
        assert_eq!(loaded.mid_profile.bounce, 0.0);
        assert_eq!(loaded.mid_profile.scale, 1.0);
        assert!(loaded.mid_profile.mirror);
        assert!(!loaded.mid_profile.blink);
        assert_eq!(loaded.device, settings.device);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_settings_use_defaults_but_malformed_settings_report_errors() {
        let path =
            std::env::temp_dir().join(format!("mic-freq-invalid-{}.json", std::process::id()));
        assert_eq!(Settings::load(&path).unwrap().gate_db, -45.0);
        std::fs::write(&path, b"not JSON").unwrap();
        assert!(Settings::load(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn older_settings_files_receive_default_frequency_profiles() {
        let path = std::env::temp_dir().join(format!(
            "mic-freq-legacy-settings-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, br#"{"gate_db": -52, "mirror": true}"#).unwrap();
        let settings = Settings::load(&path).unwrap();
        assert_eq!(settings.gate_db, -52.0);
        assert!(settings.mirror);
        assert_eq!(settings.low_profile, FrequencyProfile::default());
        assert_eq!(settings.mid_profile, FrequencyProfile::default());
        assert_eq!(settings.high_profile, FrequencyProfile::default());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn blinking_is_brief_periodic_and_optional() {
        let mut blink = BlinkState::default();
        let mut closed = 0;
        for _ in 0..600 {
            closed += usize::from(blink.update(1.0 / 60.0, true));
        }
        assert!((8..40).contains(&closed));
        for _ in 0..600 {
            assert!(!blink.update(1.0 / 60.0, false));
        }
    }

    #[test]
    fn closed_eye_frame_is_visible_long_enough_to_notice() {
        let mut blink = BlinkState::default();
        let mut longest = 0;
        let mut current = 0;
        for _ in 0..600 {
            if blink.update(1.0 / 60.0, true) {
                current += 1;
                longest = longest.max(current);
            } else {
                current = 0;
            }
        }
        assert!(longest >= 10, "longest blink was only {longest} frames");
    }
}

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub gate_db: f32,
    pub loud_delta_db: f32,
    pub bounce: f32,
    pub scale: f32,
    pub mirror: bool,
    pub blink: bool,
    pub background: usize,
    pub device: Option<String>,
    pub assets: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gate_db: -45.0,
            loud_delta_db: 24.0,
            bounce: 1.0,
            scale: 1.0,
            mirror: false,
            blink: true,
            background: 0,
            device: None,
            assets: None,
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
        settings.bounce = finite_clamp(settings.bounce, 0.0, 2.0, 1.0);
        settings.scale = finite_clamp(settings.scale, 0.2, 1.0, 1.0);
        settings.background %= 3;
        Ok(settings)
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
    seed: u32,
}

impl Default for BlinkState {
    fn default() -> Self {
        Self {
            elapsed: 0.0,
            interval: 3.8,
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
        if self.elapsed >= self.interval + 0.14 {
            self.elapsed = 0.0;
            self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
            self.interval = 2.5 + (self.seed as f64 / u32::MAX as f64) as f32 * 3.0;
        }
        self.elapsed >= self.interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_clamp_out_of_range_values() {
        let dir = std::env::temp_dir().join(format!("mic-freq-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        let settings = Settings {
            gate_db: 20.0,
            bounce: -4.0,
            scale: 8.0,
            device: Some("Test input".into()),
            ..Default::default()
        };
        settings.save(&path).unwrap();
        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.gate_db, -26.0);
        assert_eq!(loaded.bounce, 0.0);
        assert_eq!(loaded.scale, 1.0);
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
}

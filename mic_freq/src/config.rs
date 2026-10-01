//! Per-image customization settings and their on-disk layout file.

use crate::analysis::log_spaced_centers;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::path::Path;

pub const LAYOUT_FILE: &str = "mic_freq.json";
pub const GENERATED_PREFIX: &str = "generated:";
pub const PLACEHOLDERS: [&str; 5] = ["bass", "low-mid", "mid", "high-mid", "treble"];
pub const DEFAULT_LO_HZ: f32 = 100.0;
pub const DEFAULT_HI_HZ: f32 = 1600.0;
pub const MIN_HZ: f32 = 50.0;
pub const MAX_HZ: f32 = 4000.0;
pub const IMAGE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "bmp", "gif", "qoi", "tga"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ImageSettings {
    /// Display name.
    pub name: String,
    /// File name inside the asset folder, or `generated:<kind>` for a built-in placeholder.
    pub source: String,
    pub visible: bool,
    /// Frequency (Hz) this image responds to most strongly.
    pub center_hz: f32,
    /// Band width (Gaussian std-dev) in octaves.
    pub width_octaves: f32,
    /// Loudness multiplier; > 1 makes the image react to quieter sounds.
    pub gain: f32,
    /// Overall size multiplier at full level.
    pub scale: f32,
    /// Size at level 0 as a fraction of full size.
    pub min_size: f32,
    /// Opacity at level 0 (0..1).
    pub min_opacity: f32,
    /// How far (px) the image rises at full level.
    pub lift_px: f32,
    /// Maximum wobble rotation (degrees) at full level.
    pub wobble_deg: f32,
    /// RGB tint multiplied into the image.
    pub tint: [u8; 3],
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            source: String::new(),
            visible: true,
            center_hz: 400.0,
            width_octaves: 0.6,
            gain: 1.0,
            scale: 1.0,
            min_size: 0.45,
            min_opacity: 0.25,
            lift_px: 60.0,
            wobble_deg: 4.0,
            tint: [255, 255, 255],
        }
    }
}

impl ImageSettings {
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self { name: name.into(), source: source.into(), ..Default::default() }
    }

    pub fn is_generated(&self) -> bool {
        self.source.starts_with(GENERATED_PREFIX)
    }

    /// Restore all tunable properties to defaults, keeping name, source and center frequency.
    pub fn reset_style(&mut self) {
        *self = Self { name: std::mem::take(&mut self.name), source: std::mem::take(&mut self.source), center_hz: self.center_hz, ..Default::default() };
    }

    /// Clamp every value to its valid range (protects against hand-edited JSON).
    pub fn sanitize(&mut self) {
        let fix = |v: f32, lo: f32, hi: f32, def: f32| if v.is_finite() { v.clamp(lo, hi) } else { def };
        let d = Self::default();
        self.center_hz = fix(self.center_hz, MIN_HZ, MAX_HZ, d.center_hz);
        self.width_octaves = fix(self.width_octaves, 0.05, 3.0, d.width_octaves);
        self.gain = fix(self.gain, 0.0, 8.0, d.gain);
        self.scale = fix(self.scale, 0.05, 4.0, d.scale);
        self.min_size = fix(self.min_size, 0.0, 1.0, d.min_size);
        self.min_opacity = fix(self.min_opacity, 0.0, 1.0, d.min_opacity);
        self.lift_px = fix(self.lift_px, 0.0, 400.0, d.lift_px);
        self.wobble_deg = fix(self.wobble_deg, 0.0, 180.0, d.wobble_deg);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Layout {
    /// Images in display order (left = first).
    pub images: Vec<ImageSettings>,
    /// Asset-folder files the user removed; they are not re-added automatically.
    pub ignored: Vec<String>,
}

impl Layout {
    pub fn load(path: &Path) -> Result<Option<Self>, Box<dyn Error>> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut layout: Layout = serde_json::from_str(&text)?;
                layout.images.iter_mut().for_each(ImageSettings::sanitize);
                Ok(Some(layout))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    /// Merge the saved layout with the image files currently in the asset folder:
    /// saved entries whose file vanished are dropped, new files are appended (with
    /// frequencies continuing the default log spread), and if nothing remains the
    /// built-in placeholders are used.
    pub fn reconcile(mut self, files: &[String]) -> Self {
        let present: HashSet<&str> = files.iter().map(String::as_str).collect();
        self.images.retain(|i| i.is_generated() || present.contains(i.source.as_str()));
        let known: HashSet<String> = self.images.iter().map(|i| i.source.clone()).chain(self.ignored.iter().cloned()).collect();
        let new: Vec<&String> = files.iter().filter(|f| !known.contains(*f)).collect();

        if !new.is_empty() {
            let total = self.images.len() + new.len();
            let centers = log_spaced_centers(total, DEFAULT_LO_HZ, DEFAULT_HI_HZ);
            let start = self.images.len();
            for (i, f) in new.into_iter().enumerate() {
                let mut s = ImageSettings::new(file_stem(f), f.clone());
                s.center_hz = centers[start + i];
                self.images.push(s);
            }
        }

        if self.images.is_empty() {
            self.images = PLACEHOLDERS.iter().map(|p| ImageSettings::new(*p, format!("{GENERATED_PREFIX}{p}"))).collect();
            spread_frequencies(&mut self.images, DEFAULT_LO_HZ, DEFAULT_HI_HZ);
        }
        self
    }
}

/// Re-distribute the center frequencies of all images evenly on a log scale.
pub fn spread_frequencies(images: &mut [ImageSettings], lo: f32, hi: f32) {
    let centers = log_spaced_centers(images.len(), lo, hi);
    for (img, c) in images.iter_mut().zip(centers) {
        img.center_hz = c;
    }
}

pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Sorted file names of the supported images directly inside `dir`.
pub fn scan_image_files(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file() && is_image_file(p))
                .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

pub fn file_stem(file: &str) -> String {
    Path::new(file).file_stem().map_or_else(|| file.to_string(), |s| s.to_string_lossy().into_owned())
}

/// A file name in `dir` based on `wanted` that does not collide with an existing file.
pub fn unique_file_name(dir: &Path, wanted: &str) -> String {
    if !dir.join(wanted).exists() {
        return wanted.to_string();
    }
    let p = Path::new(wanted);
    let stem = file_stem(wanted);
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (2..).map(|n| format!("{stem}-{n}{ext}")).find(|c| !dir.join(c).exists()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(l: &Layout) -> Vec<&str> {
        l.images.iter().map(|i| i.name.as_str()).collect()
    }

    #[test]
    fn empty_folder_uses_placeholders() {
        let l = Layout::default().reconcile(&[]);
        assert_eq!(names(&l), PLACEHOLDERS);
        assert!(l.images.iter().all(ImageSettings::is_generated));
        assert!((l.images[2].center_hz - 400.0).abs() < 0.01);
    }

    #[test]
    fn new_files_are_appended_and_spread() {
        let files = vec!["a.png".to_string(), "b.jpg".to_string(), "c.png".to_string()];
        let l = Layout::default().reconcile(&files);
        assert_eq!(names(&l), ["a", "b", "c"]);
        assert!((l.images[0].center_hz - 100.0).abs() < 0.01);
        assert!((l.images[2].center_hz - 1600.0).abs() < 0.01);
    }

    #[test]
    fn saved_settings_order_and_ignored_are_respected() {
        let mut b = ImageSettings::new("Bee", "b.png");
        b.center_hz = 777.0;
        b.tint = [1, 2, 3];
        let saved = Layout {
            images: vec![b.clone(), ImageSettings::new("gone", "gone.png"), ImageSettings::new("bass", "generated:bass")],
            ignored: vec!["x.png".into()],
        };
        let files = vec!["a.png".to_string(), "b.png".to_string(), "x.png".to_string()];
        let l = saved.reconcile(&files);
        assert_eq!(names(&l), ["Bee", "bass", "a"]);
        assert_eq!(l.images[0], b);
    }

    #[test]
    fn json_roundtrip_with_missing_fields_and_bad_values() {
        let dir = std::env::temp_dir().join(format!("mic_freq_test_{}", std::process::id()));
        let path = dir.join(LAYOUT_FILE);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, r#"{"images":[{"name":"n","source":"n.png","gain":99,"center_hz":5}]}"#).unwrap();
        let l = Layout::load(&path).unwrap().unwrap();
        assert_eq!(l.images[0].gain, 8.0);
        assert_eq!(l.images[0].center_hz, MIN_HZ);
        assert_eq!(l.images[0].scale, 1.0);

        l.save(&path).unwrap();
        assert_eq!(Layout::load(&path).unwrap().unwrap(), l);
        assert!(Layout::load(&dir.join("missing.json")).unwrap().is_none());

        std::fs::write(dir.join("n.png"), b"").unwrap();
        assert_eq!(unique_file_name(&dir, "n.png"), "n-2.png");
        assert_eq!(unique_file_name(&dir, "z.png"), "z.png");
        assert_eq!(scan_image_files(&dir), ["n.png"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reset_style_keeps_identity() {
        let mut s = ImageSettings::new("n", "n.png");
        s.center_hz = 900.0;
        s.gain = 3.0;
        s.visible = false;
        s.reset_style();
        assert_eq!((s.name.as_str(), s.source.as_str(), s.center_hz, s.gain, s.visible), ("n", "n.png", 900.0, 1.0, true));
    }
}

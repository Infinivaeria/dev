//! Pure signal analysis: FFT-based dominant-frequency detection and the
//! mapping from (frequency, loudness) to per-asset levels.

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

pub struct Analysis {
    /// Root-mean-square amplitude of the analysed window (0..1 for f32 audio).
    pub rms: f32,
    /// Dominant frequency in Hz, `None` when the input is below the gate or no peak was found.
    pub dominant_hz: Option<f32>,
    /// Magnitude spectrum, `fft_size / 2` bins.
    pub magnitudes: Vec<f32>,
    /// Width of one spectrum bin in Hz.
    pub bin_hz: f32,
}

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buffer: Vec<Complex<f32>>,
    sample_rate: f32,
    /// Search range for the dominant peak.
    pub min_hz: f32,
    pub max_hz: f32,
    /// RMS level (dBFS) below which the input is treated as silence.
    pub gate_db: f32,
}

impl Analyzer {
    pub fn new(fft_size: usize, sample_rate: f32) -> Self {
        assert!(fft_size >= 64 && fft_size.is_power_of_two());
        let fft = FftPlanner::new().plan_fft_forward(fft_size);
        let n = fft_size as f32;
        let window = (0..fft_size)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (n - 1.0)).cos())
            .collect();
        Self {
            fft,
            window,
            buffer: vec![Complex::default(); fft_size],
            sample_rate,
            min_hz: 50.0,
            max_hz: 4000.0,
            gate_db: -50.0,
        }
    }

    pub fn fft_size(&self) -> usize {
        self.window.len()
    }

    /// Analyse the most recent `fft_size` samples (zero-padded at the front if fewer are given).
    pub fn analyze(&mut self, samples: &[f32]) -> Analysis {
        let n = self.fft_size();
        let samples = &samples[samples.len().saturating_sub(n)..];
        let pad = n - samples.len();

        let mean = if samples.is_empty() { 0.0 } else { samples.iter().sum::<f32>() / samples.len() as f32 };
        let mut sum_sq = 0.0;
        for (i, slot) in self.buffer.iter_mut().enumerate() {
            let s = if i < pad { 0.0 } else { samples[i - pad] - mean };
            sum_sq += s * s;
            *slot = Complex::new(s * self.window[i], 0.0);
        }
        let rms = if samples.is_empty() { 0.0 } else { (sum_sq / samples.len() as f32).sqrt() };

        self.fft.process(&mut self.buffer);
        let half = n / 2;
        let scale = 2.0 / n as f32;
        let magnitudes: Vec<f32> = self.buffer[..half].iter().map(|c| c.norm() * scale).collect();
        let bin_hz = self.sample_rate / n as f32;

        let dominant_hz = if amplitude_to_db(rms) < self.gate_db {
            None
        } else {
            self.find_peak(&magnitudes, bin_hz)
        };

        Analysis { rms, dominant_hz, magnitudes, bin_hz }
    }

    fn find_peak(&self, mags: &[f32], bin_hz: f32) -> Option<f32> {
        let lo = ((self.min_hz / bin_hz).floor() as usize).max(1);
        let hi = ((self.max_hz / bin_hz).ceil() as usize).min(mags.len().saturating_sub(2));
        if lo >= hi {
            return None;
        }
        let (k, &peak) = mags[lo..=hi]
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, m)| (i + lo, m))?;
        if peak <= f32::EPSILON {
            return None;
        }
        // Parabolic interpolation on log magnitudes (Gaussian fit) for sub-bin accuracy.
        let (a, b, c) = (mags[k - 1].max(1e-12).ln(), peak.ln(), mags[k + 1].max(1e-12).ln());
        let denom = a - 2.0 * b + c;
        let offset = if denom.abs() > 1e-12 { (0.5 * (a - c) / denom).clamp(-0.5, 0.5) } else { 0.0 };
        Some((k as f32 + offset) * bin_hz)
    }
}

pub fn amplitude_to_db(amplitude: f32) -> f32 {
    20.0 * amplitude.max(1e-9).log10()
}

/// Loudness above the gate mapped to 0..1 over `range_db` decibels.
pub fn loudness(rms: f32, gate_db: f32, range_db: f32) -> f32 {
    ((amplitude_to_db(rms) - gate_db) / range_db).clamp(0.0, 1.0)
}

/// `count` band centre frequencies spaced evenly on a log (musical) scale from `lo` to `hi`.
pub fn log_spaced_centers(count: usize, lo: f32, hi: f32) -> Vec<f32> {
    match count {
        0 => Vec::new(),
        1 => vec![(lo * hi).sqrt()],
        _ => (0..count)
            .map(|i| lo * (hi / lo).powf(i as f32 / (count - 1) as f32))
            .collect(),
    }
}

/// How strongly `freq` belongs to the band centred at `center`: a Gaussian in
/// octaves, 1.0 at the centre and falling off with `width_octaves` std-dev.
pub fn band_affinity(freq: f32, center: f32, width_octaves: f32) -> f32 {
    let d = (freq / center).log2() / width_octaves;
    (-0.5 * d * d).exp()
}

/// Map a continuous 0..1 level to a discrete step in `0..=steps`.
pub fn discrete_level(level: f32, steps: u32) -> u32 {
    (level.clamp(0.0, 1.0) * steps as f32).round() as u32
}

/// Nearest musical note name (e.g. "A4") and the deviation in cents.
pub fn note_name(freq: f32) -> (String, f32) {
    const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    let nearest = midi.round();
    let cents = (midi - nearest) * 100.0;
    let n = nearest as i32;
    let name = NAMES[n.rem_euclid(12) as usize];
    (format!("{name}{}", n.div_euclid(12) - 1), cents)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, sr: f32, len: usize, amp: f32) -> Vec<f32> {
        (0..len)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin())
            .collect()
    }

    #[test]
    fn detects_sine_frequencies() {
        let sr = 48_000.0;
        let mut a = Analyzer::new(4096, sr);
        for f in [110.0, 261.63, 440.0, 1000.0, 3150.0] {
            let got = a.analyze(&sine(f, sr, 4096, 0.5)).dominant_hz.unwrap();
            assert!((got - f).abs() < 2.0, "expected {f}, got {got}");
        }
    }

    #[test]
    fn silence_is_gated() {
        let mut a = Analyzer::new(2048, 44_100.0);
        assert!(a.analyze(&vec![0.0; 2048]).dominant_hz.is_none());
        assert!(a.analyze(&sine(440.0, 44_100.0, 2048, 0.001)).dominant_hz.is_none());
        assert!(a.analyze(&[]).dominant_hz.is_none());
    }

    #[test]
    fn short_input_is_zero_padded() {
        let mut a = Analyzer::new(4096, 48_000.0);
        let got = a.analyze(&sine(500.0, 48_000.0, 3000, 0.5)).dominant_hz.unwrap();
        assert!((got - 500.0).abs() < 5.0, "got {got}");
    }

    #[test]
    fn level_mapping() {
        let c = log_spaced_centers(5, 100.0, 1600.0);
        assert_eq!(c.len(), 5);
        assert!((c[2] - 400.0).abs() < 0.01);
        assert!((band_affinity(400.0, 400.0, 0.5) - 1.0).abs() < 1e-6);
        assert!(band_affinity(800.0, 400.0, 0.5) < 0.2);
        assert_eq!(discrete_level(0.0, 5), 0);
        assert_eq!(discrete_level(0.55, 5), 3);
        assert_eq!(discrete_level(2.0, 5), 5);
        assert_eq!(loudness(1.0, -50.0, 40.0), 1.0);
        assert_eq!(loudness(0.0, -50.0, 40.0), 0.0);
    }

    #[test]
    fn note_names() {
        assert_eq!(note_name(440.0).0, "A4");
        assert_eq!(note_name(261.63).0, "C4");
        assert_eq!(note_name(55.0).0, "A1");
        assert!(note_name(445.0).1 > 10.0);
    }
}

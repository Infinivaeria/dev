//! Deterministic, allocation-bounded audio analysis for recordings and Composer renders.

use crate::composer::RenderedAudio;
use rustfft::{FftPlanner, num_complex::Complex32};
use std::fmt;

const MIN_PITCH_HZ: f32 = 50.0;
const MAX_PITCH_HZ: f32 = 2_000.0;
const MIN_RMS: f32 = 1.0e-4;
const MAX_ANALYSIS_SAMPLES: usize = 65_536;
const HARMONIC_COUNT: usize = 8;

/// A snapshot of time- and frequency-domain properties.
#[derive(Clone, Debug, PartialEq)]
pub struct Analysis {
    pub rms_db: f32,
    pub fundamental_hz: Option<f32>,
    pub midi_note: Option<u8>,
    pub cents: f32,
    pub confidence: f32,
    pub harmonic_amplitudes: [f32; HARMONIC_COUNT],
    pub harmonicity: f32,
    pub tonal_ratio: f32,
    pub percussive_ratio: f32,
    pub tonal_percussive_ratio: f32,
    pub spectral_centroid_hz: f32,
    pub brightness: f32,
    pub peak: f32,
    pub peak_amplitude: f32,
    pub zero_crossings: usize,
    pub zero_crossing_rate: f32,
}

impl Default for Analysis {
    fn default() -> Self {
        Self {
            rms_db: -120.0,
            fundamental_hz: None,
            midi_note: None,
            cents: 0.0,
            confidence: 0.0,
            harmonic_amplitudes: [0.0; HARMONIC_COUNT],
            harmonicity: 0.0,
            tonal_ratio: 0.0,
            percussive_ratio: 1.0,
            tonal_percussive_ratio: 0.0,
            spectral_centroid_hz: 0.0,
            brightness: 0.0,
            peak: 0.0,
            peak_amplitude: 0.0,
            zero_crossings: 0,
            zero_crossing_rate: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalysisError {
    InvalidSampleRate,
    InvalidChannels,
    InvalidSampleBuffer,
    NonFiniteSample,
    ZeroWindow,
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidSampleRate => "sample rate must be finite and greater than zero",
            Self::InvalidChannels => "audio must contain at least one channel",
            Self::InvalidSampleBuffer => "sample buffer does not contain complete frames",
            Self::NonFiniteSample => "sample buffer contains a non-finite value",
            Self::ZeroWindow => "frame window must be greater than zero",
        })
    }
}

impl std::error::Error for AnalysisError {}

/// Analyze mono samples. Invalid sample rates and samples produce an explicit
/// empty result rather than panicking; use [`try_analyze_samples`] to inspect errors.
pub fn analyze_samples(samples: &[f32], sample_rate: f32) -> Analysis {
    try_analyze_samples(samples, sample_rate).unwrap_or_default()
}

pub fn try_analyze_samples(samples: &[f32], sample_rate: f32) -> Result<Analysis, AnalysisError> {
    if !sample_rate.is_finite() || sample_rate <= 0.0 {
        return Err(AnalysisError::InvalidSampleRate);
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(AnalysisError::NonFiniteSample);
    }
    let start = samples.len().saturating_sub(MAX_ANALYSIS_SAMPLES);
    let samples = &samples[start..];
    let mut result = Analysis::default();
    if samples.is_empty() {
        return Ok(result);
    }

    let rms = (samples
        .iter()
        .map(|sample| (*sample as f64) * (*sample as f64))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt() as f32;
    result.rms_db = 20.0 * rms.max(1.0e-6).log10();
    result.peak = samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    result.peak_amplitude = result.peak;
    result.zero_crossings = samples
        .windows(2)
        .filter(|pair| (pair[0] < 0.0 && pair[1] >= 0.0) || (pair[0] >= 0.0 && pair[1] < 0.0))
        .count();
    result.zero_crossing_rate =
        result.zero_crossings as f32 / samples.len().saturating_sub(1).max(1) as f32;
    if rms < MIN_RMS {
        return Ok(result);
    }

    let (frequency, pitch_confidence) = yin_pitch(samples, sample_rate);
    if let Some(frequency) = frequency {
        result.fundamental_hz = Some(frequency);
        result.confidence = pitch_confidence;
        let midi_float = 69.0 + 12.0 * (frequency / 440.0).log2();
        let nearest = midi_float.round().clamp(0.0, 127.0) as u8;
        result.midi_note = Some(nearest);
        result.cents = (midi_float - nearest as f32) * 100.0;
    }

    let spectrum = magnitude_spectrum(samples);
    if spectrum.is_empty() {
        return Ok(result);
    }
    let fft_size = samples.len().next_power_of_two();
    let bin_hz = sample_rate / fft_size as f32;
    let total_power: f64 = spectrum
        .iter()
        .skip(1)
        .map(|magnitude| (*magnitude as f64) * (*magnitude as f64))
        .sum();
    let weighted = spectrum
        .iter()
        .enumerate()
        .skip(1)
        .map(|(bin, magnitude)| bin as f64 * *magnitude as f64)
        .sum::<f64>();
    let magnitude_sum = spectrum
        .iter()
        .skip(1)
        .map(|value| *value as f64)
        .sum::<f64>();
    result.spectral_centroid_hz = if magnitude_sum > 0.0 {
        (weighted / magnitude_sum) as f32 * bin_hz
    } else {
        0.0
    };
    let high_power: f64 = spectrum
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(bin, _)| *bin as f32 * bin_hz >= 2_000.0)
        .map(|(_, magnitude)| (*magnitude as f64) * (*magnitude as f64))
        .sum();
    result.brightness = if total_power > 0.0 {
        (high_power / total_power) as f32
    } else {
        0.0
    };

    if let Some(fundamental) = result.fundamental_hz {
        let mut powers = 0.0_f64;
        let mut raw = [0.0_f32; HARMONIC_COUNT];
        for (index, amplitude) in raw.iter_mut().enumerate() {
            let harmonic_hz = fundamental * (index + 1) as f32;
            if harmonic_hz >= sample_rate * 0.5 {
                break;
            }
            *amplitude = local_peak_magnitude(&spectrum, harmonic_hz / bin_hz, 2);
            powers += local_peak_power(&spectrum, harmonic_hz / bin_hz, 2);
        }
        let scale = raw[0];
        if scale > 0.0 {
            for amplitude in &mut raw {
                *amplitude /= scale;
            }
        }
        result.harmonic_amplitudes = raw;
        result.harmonicity = if total_power > 0.0 {
            (powers / total_power).min(1.0) as f32
        } else {
            0.0
        };
        result.tonal_ratio = result.harmonicity;
    } else {
        result.tonal_ratio = 0.0;
    }
    result.percussive_ratio = 1.0 - result.tonal_ratio;
    result.tonal_percussive_ratio = if result.percussive_ratio > f32::EPSILON {
        result.tonal_ratio / result.percussive_ratio
    } else {
        f32::INFINITY
    };
    Ok(result)
}

/// Detect voice pitch from a short microphone window at a bounded analysis rate.
pub fn try_voice_pitch(samples: &[f32], sample_rate: f32) -> Result<Option<f32>, AnalysisError> {
    if !sample_rate.is_finite() || sample_rate <= 0.0 {
        return Err(AnalysisError::InvalidSampleRate);
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(AnalysisError::NonFiniteSample);
    }
    let samples = &samples[samples.len().saturating_sub(MAX_ANALYSIS_SAMPLES)..];
    let factor = (sample_rate / 12_000.0).floor().max(1.0) as usize;
    let mut downsampled = Vec::with_capacity(samples.len() / factor);
    for chunk in samples.chunks_exact(factor) {
        downsampled.push(chunk.iter().sum::<f32>() / factor as f32);
    }
    let mean = downsampled.iter().sum::<f32>() / downsampled.len().max(1) as f32;
    let power = downsampled
        .iter()
        .map(|sample| (sample - mean).powi(2))
        .sum::<f32>()
        / downsampled.len().max(1) as f32;
    if power.sqrt() < MIN_RMS {
        return Ok(None);
    }
    Ok(yin_pitch(&downsampled, sample_rate / factor as f32).0)
}

pub const SPECTRUM_BANDS_HZ: [f32; 9] = [
    40.0, 90.0, 180.0, 355.0, 710.0, 1420.0, 2840.0, 5680.0, 11360.0,
];

/// Return relative band levels in dBFS for a live mono microphone window.
pub fn try_frequency_spectrum(
    samples: &[f32],
    sample_rate: f32,
) -> Result<[f32; 8], AnalysisError> {
    if !sample_rate.is_finite() || sample_rate <= 0.0 {
        return Err(AnalysisError::InvalidSampleRate);
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(AnalysisError::NonFiniteSample);
    }
    let samples = &samples[samples.len().saturating_sub(MAX_ANALYSIS_SAMPLES)..];
    let mut levels = [-120.0; 8];
    if samples.len() < 2 {
        return Ok(levels);
    }
    let spectrum = magnitude_spectrum(samples);
    let bin_hz = sample_rate / samples.len().next_power_of_two() as f32;
    for (index, level) in levels.iter_mut().enumerate() {
        let power: f32 = spectrum
            .iter()
            .enumerate()
            .filter(|(bin, _)| {
                let hz = *bin as f32 * bin_hz;
                hz >= SPECTRUM_BANDS_HZ[index] && hz < SPECTRUM_BANDS_HZ[index + 1]
            })
            .map(|(_, amplitude)| amplitude * amplitude)
            .sum();
        let amplitude = 2.0 * power.sqrt() / samples.len() as f32;
        *level = (20.0 * amplitude.max(1.0e-6).log10()).clamp(-120.0, 0.0);
    }
    Ok(levels)
}

fn yin_pitch(samples: &[f32], sample_rate: f32) -> (Option<f32>, f32) {
    if sample_rate <= MIN_PITCH_HZ || samples.len() < 64 {
        return (None, 0.0);
    }
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    let centered: Vec<f32> = samples.iter().map(|sample| sample - mean).collect();
    let min_lag = (sample_rate / MAX_PITCH_HZ).floor().max(2.0) as usize;
    let max_lag = ((sample_rate / MIN_PITCH_HZ).ceil() as usize)
        .min(centered.len() / 2)
        .min(MAX_ANALYSIS_SAMPLES / 2);
    if min_lag >= max_lag {
        return (None, 0.0);
    }
    let difference_len = max_lag + 1;
    let mut difference = vec![0.0_f32; difference_len];
    let comparison_len = centered.len() - max_lag;
    for lag in 1..=max_lag {
        let mut total = 0.0_f64;
        for index in 0..comparison_len {
            let delta = (centered[index] - centered[index + lag]) as f64;
            total += delta * delta;
        }
        difference[lag] = total as f32;
    }
    difference[0] = 1.0;
    let mut running = 0.0;
    for lag in 1..=max_lag {
        running += difference[lag];
        difference[lag] = if running > 0.0 {
            difference[lag] * lag as f32 / running
        } else {
            1.0
        };
    }
    let threshold = 0.15;
    let mut candidate = None;
    for lag in min_lag..max_lag {
        if difference[lag] < threshold {
            let mut falling = lag;
            while falling + 1 <= max_lag && difference[falling + 1] < difference[falling] {
                falling += 1;
            }
            candidate = Some(falling);
            break;
        }
    }
    let lag = candidate.or_else(|| {
        (min_lag..=max_lag)
            .min_by(|left, right| difference[*left].total_cmp(&difference[*right]))
            .filter(|best| difference[*best] < 0.35)
    });
    let Some(lag) = lag else {
        return (None, 0.0);
    };
    let left = difference[lag.saturating_sub(1)];
    let center = difference[lag];
    let right = difference[(lag + 1).min(max_lag)];
    let denominator = left - 2.0 * center + right;
    let offset = if denominator.abs() > f32::EPSILON {
        (0.5 * (left - right) / denominator).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    let frequency = sample_rate / (lag as f32 + offset);
    let confidence = (1.0 - center.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    (
        ((MIN_PITCH_HZ..=MAX_PITCH_HZ).contains(&frequency)).then_some(frequency),
        confidence,
    )
}

fn magnitude_spectrum(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let fft_size = samples.len().next_power_of_two();
    let mut buffer: Vec<Complex32> = (0..fft_size)
        .map(|index| {
            let window = if samples.len() <= 1 {
                1.0
            } else if index < samples.len() {
                0.5 - 0.5
                    * (std::f32::consts::TAU * index as f32 / (samples.len() - 1) as f32).cos()
            } else {
                0.0
            };
            Complex32::new(samples.get(index).copied().unwrap_or(0.0) * window, 0.0)
        })
        .collect();
    FftPlanner::<f32>::new()
        .plan_fft_forward(fft_size)
        .process(&mut buffer);
    buffer[..=fft_size / 2]
        .iter()
        .map(|value| value.norm())
        .collect()
}

fn local_peak_magnitude(spectrum: &[f32], target_bin: f32, radius: usize) -> f32 {
    let center = target_bin.round() as isize;
    (-(radius as isize)..=radius as isize)
        .filter_map(|offset| usize::try_from(center + offset).ok())
        .filter_map(|bin| spectrum.get(bin))
        .copied()
        .fold(0.0_f32, f32::max)
}

fn local_peak_power(spectrum: &[f32], target_bin: f32, radius: usize) -> f64 {
    let center = target_bin.round() as isize;
    (-(radius as isize)..=radius as isize)
        .filter_map(|offset| usize::try_from(center + offset).ok())
        .filter_map(|bin| spectrum.get(bin))
        .map(|magnitude| (*magnitude as f64) * (*magnitude as f64))
        .sum()
}

/// Average all interleaved channels into mono, rejecting malformed audio.
pub fn downmix(audio: &RenderedAudio) -> Result<Vec<f32>, AnalysisError> {
    if audio.channels == 0 {
        return Err(AnalysisError::InvalidChannels);
    }
    let channels = audio.channels as usize;
    if audio.samples.len() % channels != 0 {
        return Err(AnalysisError::InvalidSampleBuffer);
    }
    Ok(audio
        .samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect())
}

/// Analyze at most `max_frames` of a rendered track's final audio.
pub fn analyze_rendered_audio(
    audio: &RenderedAudio,
    max_frames: usize,
) -> Result<Analysis, AnalysisError> {
    if max_frames == 0 {
        return Err(AnalysisError::ZeroWindow);
    }
    let mono = downmix(audio)?;
    let start = mono.len().saturating_sub(max_frames);
    try_analyze_samples(&mono[start..], audio.sample_rate as f32)
}

/// Analyze Composer audio in consecutive, bounded mono windows.
pub fn analyze_rendered_audio_windows(
    audio: &RenderedAudio,
    max_frames: usize,
) -> Result<Vec<Analysis>, AnalysisError> {
    if max_frames == 0 {
        return Err(AnalysisError::ZeroWindow);
    }
    if audio.sample_rate == 0 {
        return Err(AnalysisError::InvalidSampleRate);
    }
    let mono = downmix(audio)?;
    mono.chunks(max_frames)
        .map(|window| try_analyze_samples(window, audio.sample_rate as f32))
        .collect()
}

pub fn analyze_composer_audio(
    audio: &RenderedAudio,
    max_frames: usize,
) -> Result<Analysis, AnalysisError> {
    analyze_rendered_audio(audio, max_frames)
}

/// A streaming rhythm estimate suitable for fixed- or variable-size mono chunks.
pub struct RhythmTracker {
    sample_rate: f32,
    elapsed_seconds: f64,
    previous_rms: f32,
    last_onset_seconds: Option<f64>,
    onset_times: Vec<f64>,
    latest: RhythmAnalysis,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RhythmAnalysis {
    pub onset_strength: f32,
    pub bpm: Option<f32>,
    pub beat_confidence: f32,
    pub interval_peak: Option<f32>,
    pub pulse_regularity: f32,
}

pub type RhythmUpdate = RhythmAnalysis;

impl RhythmTracker {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            elapsed_seconds: 0.0,
            previous_rms: 0.0,
            last_onset_seconds: None,
            onset_times: Vec::with_capacity(64),
            latest: RhythmAnalysis::default(),
        }
    }

    pub fn try_new(sample_rate: f32) -> Result<Self, AnalysisError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(AnalysisError::InvalidSampleRate);
        }
        Ok(Self::new(sample_rate))
    }

    /// Feed one sequential chunk and return the current estimate.
    pub fn update(&mut self, samples: &[f32]) -> RhythmAnalysis {
        if !self.sample_rate.is_finite() || self.sample_rate <= 0.0 {
            return self.latest;
        }
        let finite = samples.iter().all(|sample| sample.is_finite());
        let rms = if samples.is_empty() || !finite {
            0.0
        } else {
            (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32)
                .sqrt()
        };
        let onset = (rms - self.previous_rms * 0.55).max(0.0);
        let chunk_seconds = samples.len() as f64 / self.sample_rate as f64;
        let peak_index = samples
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.abs().total_cmp(&right.abs()))
            .map_or(0, |(index, _)| index);
        let onset_time = self.elapsed_seconds + peak_index as f64 / self.sample_rate as f64;
        let refractory = 0.12;
        if onset >= MIN_RMS
            && self
                .last_onset_seconds
                .map_or(true, |last| onset_time - last >= refractory)
        {
            self.last_onset_seconds = Some(onset_time);
            self.onset_times.push(onset_time);
            if self.onset_times.len() > 64 {
                self.onset_times.remove(0);
            }
        }
        self.previous_rms = rms;
        self.elapsed_seconds += chunk_seconds;
        let (bpm, regularity, interval_peak) = self.rhythm_estimate();
        self.latest = RhythmAnalysis {
            onset_strength: onset,
            bpm,
            beat_confidence: regularity
                * ((self.onset_times.len().saturating_sub(2)) as f32 / 6.0).min(1.0),
            interval_peak,
            pulse_regularity: regularity,
        };
        self.latest
    }

    pub fn push(&mut self, samples: &[f32]) -> RhythmAnalysis {
        self.update(samples)
    }

    pub fn try_update(&mut self, samples: &[f32]) -> Result<RhythmAnalysis, AnalysisError> {
        if !self.sample_rate.is_finite() || self.sample_rate <= 0.0 {
            return Err(AnalysisError::InvalidSampleRate);
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(AnalysisError::NonFiniteSample);
        }
        Ok(self.update(samples))
    }

    pub fn analyze_chunk(&mut self, samples: &[f32]) -> RhythmAnalysis {
        self.update(samples)
    }

    pub fn current(&self) -> RhythmAnalysis {
        self.latest
    }

    pub fn reset(&mut self) {
        self.elapsed_seconds = 0.0;
        self.previous_rms = 0.0;
        self.last_onset_seconds = None;
        self.onset_times.clear();
        self.latest = RhythmAnalysis::default();
    }

    fn rhythm_estimate(&self) -> (Option<f32>, f32, Option<f32>) {
        if self.onset_times.len() < 3 {
            return (None, 0.0, None);
        }
        let mut intervals: Vec<f64> = self
            .onset_times
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .filter(|interval| *interval > 0.0)
            .collect();
        intervals.sort_by(f64::total_cmp);
        let interval = intervals[intervals.len() / 2];
        let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let variance = intervals
            .iter()
            .map(|value| (*value - mean) * (*value - mean))
            .sum::<f64>()
            / intervals.len() as f64;
        let coefficient = variance.sqrt() / interval.max(f64::EPSILON);
        let regularity = (1.0 - coefficient as f32).clamp(0.0, 1.0);
        let mut bpm = 60.0 / interval as f32;
        if !bpm.is_finite() {
            return (None, regularity, Some(interval as f32));
        }
        while bpm < 60.0 {
            bpm *= 2.0;
        }
        while bpm > 200.0 {
            bpm *= 0.5;
        }
        ((60.0..=200.0).contains(&bpm))
            .then_some((bpm, regularity, Some(interval as f32)))
            .map(|(bpm, regularity, interval)| (Some(bpm), regularity, interval))
            .unwrap_or((None, regularity, Some(interval as f32)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frequency: f32, seconds: f32, sample_rate: f32) -> Vec<f32> {
        (0..(seconds * sample_rate) as usize)
            .map(|index| {
                0.5 * (std::f32::consts::TAU * frequency * index as f32 / sample_rate).sin()
            })
            .collect()
    }

    #[test]
    fn detects_fundamental_note_and_harmonic_content() {
        let sample_rate = 44_100.0;
        let samples = sine(440.0, 1.0, sample_rate);
        let analysis = analyze_samples(&samples, sample_rate);
        assert!((analysis.fundamental_hz.unwrap() - 440.0).abs() < 1.0);
        assert_eq!(analysis.midi_note, Some(69));
        assert!(analysis.cents.abs() < 5.0);
        assert!(analysis.confidence > 0.95);
        assert!(analysis.harmonicity > 0.8);
        assert!((analysis.harmonic_amplitudes[0] - 1.0).abs() < 0.01);

        let rich: Vec<f32> = sine(220.0, 1.0, sample_rate)
            .iter()
            .zip(sine(660.0, 1.0, sample_rate))
            .map(|(fundamental, third)| fundamental + third * 0.5)
            .collect();
        let rich_analysis = analyze_samples(&rich, sample_rate);
        assert!(rich_analysis.harmonic_amplitudes[2] > 0.35);
    }

    #[test]
    fn silence_and_bad_input_are_safe() {
        let silence = analyze_samples(&[], 44_100.0);
        assert_eq!(silence.fundamental_hz, None);
        assert_eq!(silence.peak, 0.0);
        assert_eq!(analyze_samples(&[0.0; 4096], 44_100.0).rms_db, -120.0);
        assert_eq!(
            try_analyze_samples(&[0.0], 0.0),
            Err(AnalysisError::InvalidSampleRate)
        );
    }

    #[test]
    fn voice_pitch_handles_high_sample_rates_and_voice_harmonics() {
        for sample_rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            for frequency in [80.0, 120.0, 220.0, 440.0] {
                let samples: Vec<f32> = (0..(sample_rate * 0.08) as usize)
                    .map(|index| {
                        let phase = std::f32::consts::TAU * frequency * index as f32 / sample_rate;
                        (0.25 * phase.sin()
                            + 0.15 * (2.0 * phase).sin()
                            + 0.08 * (3.0 * phase).sin())
                        .clamp(-1.0, 1.0)
                    })
                    .collect();
                let detected = try_voice_pitch(&samples, sample_rate).unwrap().unwrap();
                assert!(
                    (detected - frequency).abs() < 3.0,
                    "{sample_rate} Hz input: expected {frequency} Hz, got {detected} Hz"
                );
            }
        }
        assert_eq!(try_voice_pitch(&[0.0; 2048], 48_000.0), Ok(None));
        assert_eq!(
            try_voice_pitch(&[f32::NAN], 48_000.0),
            Err(AnalysisError::NonFiniteSample)
        );
    }

    #[test]
    fn frequency_spectrum_identifies_tone_band_and_silence() {
        let samples = sine(440.0, 0.1, 48_000.0);
        let spectrum = try_frequency_spectrum(&samples, 48_000.0).unwrap();
        let strongest = spectrum
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .unwrap()
            .0;
        assert_eq!(strongest, 3);
        assert!(spectrum[3] > -12.0);
        assert_eq!(
            try_frequency_spectrum(&[0.0; 1024], 48_000.0).unwrap(),
            [-120.0; 8]
        );
        assert_eq!(
            try_frequency_spectrum(&[f32::NAN], 48_000.0),
            Err(AnalysisError::NonFiniteSample)
        );
    }

    #[test]
    fn tracks_a_120_bpm_click_train_across_chunks() {
        let sample_rate = 8_000.0;
        let mut samples = vec![0.0; sample_rate as usize * 5];
        for beat in 0..10 {
            let start = beat * sample_rate as usize / 2;
            for offset in 0..32 {
                samples[start + offset] = 1.0 - offset as f32 / 32.0;
            }
        }
        let mut tracker = RhythmTracker::new(sample_rate);
        let mut estimate = RhythmAnalysis::default();
        for chunk in samples.chunks(256) {
            estimate = tracker.update(chunk);
        }
        assert!((estimate.bpm.unwrap() - 120.0).abs() < 2.0);
        assert!(estimate.pulse_regularity > 0.95);
        assert!(estimate.beat_confidence > 0.8);
        assert!((estimate.interval_peak.unwrap() - 0.5).abs() < 0.02);
    }

    #[test]
    fn rendered_audio_is_downmixed_without_changing_the_pitch() {
        let mono = sine(330.0, 0.5, 16_000.0);
        let interleaved: Vec<f32> = mono
            .iter()
            .flat_map(|sample| [*sample * 0.5, *sample])
            .collect();
        let audio = RenderedAudio {
            sample_rate: 16_000,
            channels: 2,
            samples: interleaved,
        };
        let downmixed = downmix(&audio).unwrap();
        assert!((downmixed[0] - mono[0] * 0.75).abs() < f32::EPSILON);
        let analysis = analyze_rendered_audio(&audio, 4_000).unwrap();
        assert!((analysis.fundamental_hz.unwrap() - 330.0).abs() < 1.0);
    }
}

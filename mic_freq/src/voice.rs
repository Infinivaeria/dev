use raylib::prelude::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AvatarState {
    Idle,
    Talking,
    Loud,
}

impl AvatarState {
    pub const ALL: [Self; 3] = [Self::Idle, Self::Talking, Self::Loud];

    pub fn index(self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Talking => 1,
            Self::Loud => 2,
        }
    }

    pub fn file_stem(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Talking => "talking",
            Self::Loud => "loud",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Talking => "Talking",
            Self::Loud => "Loud talking",
        }
    }

    pub fn placeholder_color(self) -> Color {
        match self {
            Self::Idle => Color::new(60, 95, 155, 255),
            Self::Talking => Color::new(70, 170, 125, 255),
            Self::Loud => Color::new(230, 105, 65, 255),
        }
    }
}

pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    (samples.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / samples.len() as f32).sqrt()
}

pub fn amplitude_to_db(amplitude: f32) -> f32 {
    20.0 * amplitude.max(1e-9).log10()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceBand {
    Low,
    Mid,
    High,
    Unknown,
}

impl VoiceBand {
    pub const ALL: [Self; 3] = [Self::Low, Self::Mid, Self::High];

    pub fn from_frequency(frequency_hz: Option<f32>) -> Self {
        match frequency_hz {
            Some(frequency) if frequency.is_finite() && frequency > 0.0 && frequency < 160.0 => {
                Self::Low
            }
            Some(frequency)
                if frequency.is_finite() && frequency >= 160.0 && frequency <= 300.0 =>
            {
                Self::Mid
            }
            Some(frequency) if frequency.is_finite() && frequency > 300.0 => Self::High,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Mid => "Mid",
            Self::High => "High",
            Self::Unknown => "Unpitched",
        }
    }

    pub fn cli_name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Mid => "mid",
            Self::High => "high",
            Self::Unknown => "unpitched",
        }
    }

    pub fn range_label(self) -> &'static str {
        match self {
            Self::Low => "below 160 Hz",
            Self::Mid => "160-300 Hz",
            Self::High => "above 300 Hz",
            Self::Unknown => "no stable pitch",
        }
    }

    pub fn from_cli_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "low" => Some(Self::Low),
            "mid" => Some(Self::Mid),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    pub fn index(self) -> Option<usize> {
        match self {
            Self::Low => Some(0),
            Self::Mid => Some(1),
            Self::High => Some(2),
            Self::Unknown => None,
        }
    }

    pub fn gain_db(self, low: f32, mid: f32, high: f32) -> f32 {
        match self {
            Self::Low => low,
            Self::Mid => mid,
            Self::High => high,
            Self::Unknown => 0.0,
        }
    }
}

#[derive(Default)]
pub struct VoiceBandTracker {
    band: Option<VoiceBand>,
    unpitched_elapsed: f32,
}

impl VoiceBandTracker {
    pub fn update(&mut self, frequency_hz: Option<f32>, dt: f32) -> VoiceBand {
        let detected = VoiceBand::from_frequency(frequency_hz);
        if detected != VoiceBand::Unknown {
            self.band = Some(detected);
            self.unpitched_elapsed = 0.0;
        } else {
            self.unpitched_elapsed += dt.max(0.0);
            if self.unpitched_elapsed >= 0.3 {
                self.band = None;
            }
        }
        self.band.unwrap_or(VoiceBand::Unknown)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn active_for_state(&mut self, state: AvatarState, muted: bool) -> VoiceBand {
        if muted || state == AvatarState::Idle {
            self.reset();
        }
        self.band.unwrap_or(VoiceBand::Unknown)
    }
}

pub fn frequency_adjusted_db(
    db: f32,
    frequency_hz: Option<f32>,
    low_gain_db: f32,
    mid_gain_db: f32,
    high_gain_db: f32,
) -> (f32, VoiceBand, f32) {
    let band = VoiceBand::from_frequency(frequency_hz);
    let gain = band.gain_db(low_gain_db, mid_gain_db, high_gain_db);
    (db + gain, band, gain)
}

#[derive(Default)]
pub struct AvatarMotion {
    phase: f32,
    amplitude: f32,
}

impl AvatarMotion {
    pub fn update(&mut self, state: AvatarState, dt: f32, strength: f32) -> f32 {
        let dt = dt.clamp(0.0, 0.1);
        let (height, frequency) = match state {
            AvatarState::Idle => (3.0, 0.65),
            AvatarState::Talking => (16.0, 3.0),
            AvatarState::Loud => (30.0, 4.5),
        };
        self.phase = (self.phase + dt * frequency * std::f32::consts::TAU) % std::f32::consts::TAU;
        let target = height * strength.clamp(0.0, 2.0);
        self.amplitude += (target - self.amplitude) * (1.0 - (-14.0 * dt).exp());
        -self.amplitude * (0.5 - 0.5 * self.phase.cos())
    }
}

pub struct VoiceState {
    pub gate_db: f32,
    pub loud_delta_db: f32,
    state: AvatarState,
    smoothed_db: Option<f32>,
    loud_elapsed: f32,
}

impl Default for VoiceState {
    fn default() -> Self {
        Self {
            gate_db: -45.0,
            loud_delta_db: 24.0,
            state: AvatarState::Idle,
            smoothed_db: None,
            loud_elapsed: 0.0,
        }
    }
}

impl VoiceState {
    pub fn current(&self) -> AvatarState {
        self.state
    }

    pub fn update(&mut self, db: f32, dt: f32) -> AvatarState {
        // Smooth in decibels to avoid switching frames on brief pauses and plosives.
        let previous = self.smoothed_db.unwrap_or(db);
        let rate = if db > previous { 20.0 } else { 8.0 };
        let level = previous + (db - previous) * (1.0 - (-rate * dt.max(0.0)).exp());
        self.smoothed_db = Some(level);
        let loud_threshold = self.gate_db + self.loud_delta_db;
        if self.state == AvatarState::Talking && db >= loud_threshold && level >= loud_threshold {
            self.loud_elapsed += dt.clamp(0.0, 0.1);
        } else {
            self.loud_elapsed = 0.0;
        }
        self.state = match self.state {
            AvatarState::Idle if level >= self.gate_db + 4.0 => AvatarState::Talking,
            AvatarState::Talking if self.loud_elapsed >= 0.3 => AvatarState::Loud,
            AvatarState::Talking if level < self.gate_db => AvatarState::Idle,
            AvatarState::Loud if level < self.gate_db + self.loud_delta_db - 4.0 => {
                AvatarState::Talking
            }
            state => state,
        };
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(voice: &mut VoiceState, db: f32) -> AvatarState {
        let mut state = AvatarState::Idle;
        for _ in 0..60 {
            state = voice.update(db, 1.0 / 60.0);
        }
        state
    }

    #[test]
    fn short_loud_peaks_do_not_skip_the_talking_pose() {
        let mut voice = VoiceState::default();
        assert_eq!(voice.update(-5.0, 1.0 / 60.0), AvatarState::Talking);
        for _ in 0..6 {
            assert_eq!(voice.update(-5.0, 1.0 / 60.0), AvatarState::Talking);
        }
        assert_eq!(voice.update(-35.0, 1.0 / 60.0), AvatarState::Talking);
        for _ in 0..12 {
            assert_eq!(voice.update(-5.0, 1.0 / 60.0), AvatarState::Talking);
        }
        assert_eq!(settle(&mut voice, -5.0), AvatarState::Loud);
    }

    #[test]
    fn loud_entry_requires_sustained_input_at_different_frame_rates() {
        for fps in [30, 60, 120] {
            let mut voice = VoiceState::default();
            let dt = 1.0 / fps as f32;
            assert_eq!(voice.update(-5.0, dt), AvatarState::Talking);
            for _ in 0..fps / 4 {
                assert_eq!(voice.update(-5.0, dt), AvatarState::Talking);
            }
            for _ in 0..fps / 4 {
                voice.update(-5.0, dt);
            }
            assert_eq!(voice.update(-5.0, dt), AvatarState::Loud);
        }
    }

    #[test]
    fn a_stalled_frame_cannot_instantly_trigger_loud() {
        let mut voice = VoiceState::default();
        voice.update(-5.0, 1.0 / 60.0);
        assert_eq!(voice.update(-5.0, 2.0), AvatarState::Talking);
        assert_eq!(voice.update(-5.0, -1.0), AvatarState::Talking);
    }

    #[test]
    fn motion_bounces_in_all_states_without_exceeding_headroom() {
        for state in AvatarState::ALL {
            let mut motion = AvatarMotion::default();
            let mut highest = 0.0_f32;
            for _ in 0..600 {
                let offset = motion.update(state, 1.0 / 60.0, 1.0);
                assert!((-30.0..=0.0).contains(&offset));
                highest = highest.min(offset);
            }
            let expected = [3.0, 16.0, 30.0][state.index()];
            assert!(highest < -expected * 0.9);
        }
    }

    #[test]
    fn motion_can_be_disabled_and_ignores_negative_time() {
        let mut motion = AvatarMotion::default();
        assert_eq!(motion.update(AvatarState::Loud, 1.0, 0.0), 0.0);
        assert_eq!(motion.update(AvatarState::Loud, -1.0, 1.0), 0.0);
    }

    #[test]
    fn rms_of_silence_and_constant_signal() {
        assert_eq!(rms(&[]), 0.0);
        assert_eq!(rms(&[0.5, -0.5]), 0.5);
        assert_eq!(rms(&[0.5, 0.5]), 0.0);
    }

    #[test]
    fn voice_switches_between_all_three_states() {
        let mut voice = VoiceState::default();
        assert_eq!(voice.update(-90.0, 1.0), AvatarState::Idle);
        assert_eq!(voice.update(-35.0, 1.0), AvatarState::Talking);
        assert_eq!(settle(&mut voice, -10.0), AvatarState::Loud);
        assert_eq!(voice.update(-26.0, 1.0), AvatarState::Talking);
        assert_eq!(voice.update(-50.0, 1.0), AvatarState::Idle);
    }

    #[test]
    fn hysteresis_prevents_threshold_flicker() {
        let mut voice = VoiceState::default();
        voice.update(-35.0, 1.0);
        assert_eq!(voice.update(-43.0, 1.0), AvatarState::Talking);
        settle(&mut voice, -10.0);
        assert_eq!(voice.update(-23.0, 1.0), AvatarState::Loud);
    }

    #[test]
    fn loud_sensitivity_can_be_adjusted_independently() {
        let mut voice = VoiceState {
            loud_delta_db: 12.0,
            ..Default::default()
        };
        assert_eq!(settle(&mut voice, -32.0), AvatarState::Loud);
        assert_eq!(voice.update(-36.0, 1.0), AvatarState::Loud);
        assert_eq!(voice.update(-39.0, 1.0), AvatarState::Talking);
    }

    #[test]
    fn expanded_gate_and_loud_thresholds_control_pose_transitions() {
        let mut quiet = VoiceState {
            gate_db: -120.0,
            loud_delta_db: 4.0,
            ..Default::default()
        };
        assert_eq!(settle(&mut quiet, -115.0), AvatarState::Loud);
        assert_eq!(settle(&mut quiet, -130.0), AvatarState::Idle);

        let mut loud = VoiceState {
            gate_db: -120.0,
            loud_delta_db: 120.0,
            ..Default::default()
        };
        assert_eq!(settle(&mut loud, -1.0), AvatarState::Talking);
        assert_eq!(settle(&mut loud, 1.0), AvatarState::Loud);

        let mut high_gate = VoiceState {
            gate_db: 0.0,
            ..Default::default()
        };
        assert_eq!(settle(&mut high_gate, 3.0), AvatarState::Idle);
        assert_eq!(settle(&mut high_gate, 5.0), AvatarState::Talking);
    }

    #[test]
    fn gate_adjustment_changes_sensitivity() {
        let mut voice = VoiceState {
            gate_db: -30.0,
            ..Default::default()
        };
        assert_eq!(voice.update(-35.0, 1.0), AvatarState::Idle);
        voice.gate_db = -45.0;
        assert_eq!(voice.update(-35.0, 1.0), AvatarState::Talking);
    }

    #[test]
    fn voice_frequency_bands_apply_independent_gain() {
        assert_eq!(VoiceBand::from_frequency(Some(125.0)), VoiceBand::Low);
        assert_eq!(VoiceBand::from_frequency(Some(220.0)), VoiceBand::Mid);
        assert_eq!(VoiceBand::from_frequency(Some(440.0)), VoiceBand::High);
        assert_eq!(VoiceBand::from_frequency(Some(0.0)), VoiceBand::Unknown);
        assert_eq!(
            VoiceBand::from_frequency(Some(f32::NAN)),
            VoiceBand::Unknown
        );
        assert_eq!(VoiceBand::from_frequency(None), VoiceBand::Unknown);
        assert_eq!(
            frequency_adjusted_db(-30.0, Some(125.0), 6.0, 0.0, -4.0),
            (-24.0, VoiceBand::Low, 6.0)
        );
        assert_eq!(
            frequency_adjusted_db(-30.0, Some(440.0), 6.0, 0.0, -4.0),
            (-34.0, VoiceBand::High, -4.0)
        );
        assert_eq!(
            frequency_adjusted_db(-30.0, None, 6.0, 0.0, -4.0),
            (-30.0, VoiceBand::Unknown, 0.0)
        );
    }

    #[test]
    fn pitch_dropouts_keep_the_same_gain_briefly_then_expire() {
        for (frequency, band, gain) in [
            (120.0, VoiceBand::Low, 6.0),
            (220.0, VoiceBand::Mid, -3.0),
            (440.0, VoiceBand::High, 12.0),
        ] {
            let mut tracker = VoiceBandTracker::default();
            assert_eq!(tracker.update(Some(frequency), 0.1), band);
            assert_eq!(tracker.update(None, 0.1), band);
            assert_eq!(tracker.update(None, 0.1).gain_db(6.0, -3.0, 12.0), gain);
            assert_eq!(tracker.update(None, 0.11), VoiceBand::Unknown);
            assert_eq!(tracker.update(Some(frequency), 0.1), band);
            tracker.reset();
            assert_eq!(tracker.update(None, 0.01), VoiceBand::Unknown);
        }
    }

    #[test]
    fn idle_and_mute_restore_global_artwork_and_animation() {
        let mut tracker = VoiceBandTracker::default();
        assert_eq!(tracker.update(Some(120.0), 0.1), VoiceBand::Low);
        assert_eq!(
            tracker.active_for_state(AvatarState::Talking, false),
            VoiceBand::Low
        );
        assert_eq!(
            tracker.active_for_state(AvatarState::Idle, false),
            VoiceBand::Unknown
        );
        assert_eq!(tracker.update(None, 0.01), VoiceBand::Unknown);
        tracker.update(Some(440.0), 0.1);
        assert_eq!(
            tracker.active_for_state(AvatarState::Loud, true),
            VoiceBand::Unknown
        );
        assert_eq!(tracker.update(None, 0.01), VoiceBand::Unknown);
    }

    #[test]
    fn loud_voice_preserves_band_gain_across_short_pitch_dropouts() {
        let mut tracker = VoiceBandTracker::default();
        let mut voice = VoiceState::default();
        for frame in 0..60 {
            let pitch = if frame % 12 < 6 { Some(120.0) } else { None };
            let band = tracker.update(pitch, 1.0 / 60.0);
            assert_eq!(band, VoiceBand::Low);
            voice.update(-25.0 + band.gain_db(8.0, 0.0, 0.0), 1.0 / 60.0);
        }
        assert_eq!(voice.update(-17.0, 1.0 / 60.0), AvatarState::Loud);
        assert_eq!(settle(&mut voice, -90.0), AvatarState::Idle);
        tracker.reset();
        assert_eq!(tracker.update(None, 1.0 / 60.0), VoiceBand::Unknown);
    }

    #[test]
    fn pitch_band_is_detected_for_quiet_and_loud_input() {
        let sample_rate = 48_000.0;
        for (frequency, expected) in [
            (120.0, VoiceBand::Low),
            (220.0, VoiceBand::Mid),
            (440.0, VoiceBand::High),
        ] {
            for amplitude in [0.02_f32, 0.9, 1.8] {
                let samples: Vec<f32> = (0..2048)
                    .map(|index| {
                        ((index as f32 * frequency * std::f32::consts::TAU / sample_rate).sin()
                            * amplitude)
                            .clamp(-1.0, 1.0)
                    })
                    .collect();
                let analysis = crate::analysis::try_analyze_samples(&samples, sample_rate).unwrap();
                assert_eq!(VoiceBand::from_frequency(analysis.fundamental_hz), expected);
            }
        }
    }
}

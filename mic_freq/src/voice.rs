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
    fn gate_adjustment_changes_sensitivity() {
        let mut voice = VoiceState {
            gate_db: -30.0,
            ..Default::default()
        };
        assert_eq!(voice.update(-35.0, 1.0), AvatarState::Idle);
        voice.gate_db = -45.0;
        assert_eq!(voice.update(-35.0, 1.0), AvatarState::Talking);
    }
}

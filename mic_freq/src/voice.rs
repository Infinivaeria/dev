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

pub struct VoiceState {
    pub gate_db: f32,
    state: AvatarState,
    smoothed_db: Option<f32>,
}

impl Default for VoiceState {
    fn default() -> Self {
        Self {
            gate_db: -45.0,
            state: AvatarState::Idle,
            smoothed_db: None,
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
        self.state = match self.state {
            AvatarState::Idle if level >= self.gate_db + 24.0 => AvatarState::Loud,
            AvatarState::Idle if level >= self.gate_db + 4.0 => AvatarState::Talking,
            AvatarState::Talking if level >= self.gate_db + 24.0 => AvatarState::Loud,
            AvatarState::Talking if level < self.gate_db => AvatarState::Idle,
            AvatarState::Loud if level < self.gate_db + 20.0 => AvatarState::Talking,
            state => state,
        };
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(voice.update(-10.0, 1.0), AvatarState::Loud);
        assert_eq!(voice.update(-26.0, 1.0), AvatarState::Talking);
        assert_eq!(voice.update(-50.0, 1.0), AvatarState::Idle);
    }

    #[test]
    fn hysteresis_prevents_threshold_flicker() {
        let mut voice = VoiceState::default();
        voice.update(-35.0, 1.0);
        assert_eq!(voice.update(-43.0, 1.0), AvatarState::Talking);
        voice.update(-10.0, 1.0);
        assert_eq!(voice.update(-23.0, 1.0), AvatarState::Loud);
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

use crate::analysis;
use crate::composer::{frequency_to_midi, midi_to_frequency, note_name};
use crate::studio::{GATE_MAX_DB, GATE_MIN_DB, LOUD_OFFSET_MAX_DB, LOUD_OFFSET_MIN_DB};
use crate::voice::{amplitude_to_db, rms};
use raylib::prelude::*;

const X: f32 = 18.0;
const Y: f32 = 18.0;
const WIDTH: f32 = 510.0;
const HEIGHT: f32 = 440.0;
const MEASUREMENT_SECONDS: f32 = 3.0;

#[derive(Clone, Copy, Debug)]
pub struct Recommendation {
    pub gate_db: f32,
    pub loud_delta_db: f32,
}

#[derive(Default)]
struct Measurement {
    elapsed: f32,
    levels: Vec<f32>,
    samples: usize,
    clipped: usize,
}

impl Measurement {
    fn update(
        &mut self,
        dt: f32,
        samples: &[f32],
        gain_db: f32,
    ) -> Result<(), analysis::AnalysisError> {
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(analysis::AnalysisError::NonFiniteSample);
        }
        self.elapsed += dt.max(0.0);
        if !samples.is_empty() {
            let level = amplitude_to_db(rms(samples)) + gain_db;
            if self.levels.len() < 600 {
                self.levels.push(level);
            }
            self.samples += samples.len();
            self.clipped += samples.iter().filter(|sample| sample.abs() >= 0.98).count();
        }
        Ok(())
    }

    fn level_db(&self, sample_rate: f32, percentile: f32) -> Option<f32> {
        if (self.samples as f32) < sample_rate * 0.75 || self.levels.is_empty() {
            return None;
        }
        let mut levels = self.levels.clone();
        levels.sort_by(f32::total_cmp);
        let position = ((levels.len() - 1) as f32 * percentile.clamp(0.0, 1.0)).round() as usize;
        Some(levels[position])
    }

    fn clipped(&self) -> bool {
        self.samples > 0 && self.clipped.saturating_mul(200) >= self.samples
    }
}

enum Calibration {
    Idle,
    Noise(Measurement),
    AwaitingVoice(f32),
    Voice {
        noise_db: f32,
        measurement: Measurement,
    },
    Ready {
        noise_db: f32,
        recommendation: Recommendation,
    },
}

impl Default for Calibration {
    fn default() -> Self {
        Self::Idle
    }
}

fn recommend(noise_db: f32, voice_db: f32) -> Result<Recommendation, &'static str> {
    let gate_db = (noise_db + 6.0).clamp(GATE_MIN_DB, GATE_MAX_DB);
    if voice_db < gate_db + 4.0 {
        return Err("Voice too quiet compared with noise; try again");
    }
    let loud_delta_db = (voice_db + 6.0 - gate_db).clamp(LOUD_OFFSET_MIN_DB, LOUD_OFFSET_MAX_DB);
    Ok(Recommendation {
        gate_db,
        loud_delta_db,
    })
}

#[derive(Clone, Copy, Debug)]
struct TunerPitch {
    midi: u8,
    frequency: f32,
    cents: f32,
}

fn tuner_pitch(frequency: Option<f32>) -> Option<TunerPitch> {
    let frequency = frequency?;
    let midi = frequency_to_midi(frequency).ok()?;
    let cents = 1200.0 * (frequency / midi_to_frequency(midi)).log2();
    Some(TunerPitch {
        midi,
        frequency,
        cents,
    })
}

#[derive(Default)]
pub struct MicTools {
    pub open: bool,
    calibration: Calibration,
    message: String,
    spectrum: [f32; 8],
    spectrum_delay: f32,
    no_samples_elapsed: f32,
}

impl MicTools {
    fn calibration_status(&self) -> String {
        match &self.calibration {
            Calibration::Idle => "Calibration: ready".into(),
            Calibration::Noise(measurement) => format!(
                "Measuring noise  {:.1} / {:.1}s",
                measurement.elapsed.min(MEASUREMENT_SECONDS),
                MEASUREMENT_SECONDS
            ),
            Calibration::AwaitingVoice(noise) => format!("Noise measured {noise:.0} dB; measure voice"),
            Calibration::Voice { measurement, .. } => format!(
                "Measuring voice  {:.1} / {:.1}s",
                measurement.elapsed.min(MEASUREMENT_SECONDS),
                MEASUREMENT_SECONDS
            ),
            Calibration::Ready {
                noise_db,
                recommendation,
            } => format!(
                "Ready: noise {noise_db:.0} dB, gate {:.0} dB, loud +{:.0} dB",
                recommendation.gate_db, recommendation.loud_delta_db
            ),
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        samples: &[f32],
        fresh_samples: &[f32],
        sample_rate: f32,
        gain_db: f32,
    ) -> Result<(), analysis::AnalysisError> {
        if !self.open {
            return Ok(());
        }

        if fresh_samples.is_empty() {
            self.no_samples_elapsed += dt.max(0.0);
        } else {
            self.no_samples_elapsed = 0.0;
        }
        self.spectrum_delay -= dt;
        if self.spectrum_delay <= 0.0 {
            self.spectrum = analysis::try_frequency_spectrum(samples, sample_rate)?;
            self.spectrum_delay = 0.1;
        }
        match &mut self.calibration {
            Calibration::Noise(measurement) => {
                measurement.update(dt, fresh_samples, 0.0)?;
                if measurement.elapsed >= MEASUREMENT_SECONDS {
                    if measurement.clipped() {
                        self.calibration = Calibration::Idle;
                        self.message = "Input clipping; lower system microphone gain".into();
                    } else if let Some(noise_db) = measurement.level_db(sample_rate, 0.90) {
                        self.calibration = Calibration::AwaitingVoice(noise_db);
                        self.message = "Now select Voice and speak normally".into();
                    } else {
                        self.calibration = Calibration::Idle;
                        self.message = "No microphone samples; check the input".into();
                    }
                }
            }
            Calibration::Voice {
                noise_db,
                measurement,
            } => {
                measurement.update(dt, fresh_samples, gain_db)?;
                if measurement.elapsed >= MEASUREMENT_SECONDS {
                    let noise_db = *noise_db;
                    if measurement.clipped() {
                        self.calibration = Calibration::AwaitingVoice(noise_db);
                        self.message = "Input clipping; lower system microphone gain".into();
                    } else if let Some(voice_db) = measurement.level_db(sample_rate, 0.60) {
                        match recommend(noise_db, voice_db) {
                            Ok(recommendation) => {
                                self.calibration = Calibration::Ready {
                                    noise_db,
                                    recommendation,
                                };
                                self.message = format!(
                                    "Noise {noise_db:.0} dB, voice {voice_db:.0} dB; apply?"
                                );
                            }
                            Err(message) => {
                                self.calibration = Calibration::AwaitingVoice(noise_db);
                                self.message = message.into();
                            }
                        }
                    } else {
                        self.calibration = Calibration::AwaitingVoice(noise_db);
                        self.message = "No microphone samples; check the input".into();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn click(&mut self, mouse: Vector2) -> Option<Recommendation> {
        if Rectangle::new(X + 16.0, Y + 314.0, 145.0, 34.0).check_collision_point_rec(mouse) {
            self.calibration = Calibration::Noise(Measurement::default());
            self.message = "Stay quiet for 3 seconds".into();
        } else if Rectangle::new(X + 170.0, Y + 314.0, 145.0, 34.0).check_collision_point_rec(mouse)
        {
            let noise = match self.calibration {
                Calibration::AwaitingVoice(noise)
                | Calibration::Ready {
                    noise_db: noise, ..
                } => Some(noise),
                _ => None,
            };
            if let Some(noise_db) = noise {
                self.calibration = Calibration::Voice {
                    noise_db,
                    measurement: Measurement::default(),
                };
                self.message = "Speak normally for 3 seconds".into();
            } else {
                self.message = "Measure room noise first".into();
            }
        } else if Rectangle::new(X + 324.0, Y + 314.0, 145.0, 34.0).check_collision_point_rec(mouse)
        {
            if let Calibration::Ready { recommendation, .. } = self.calibration {
                self.message = "Recommendation applied to Audio Studio".into();
                return Some(recommendation);
            }
            self.message = "Measure noise and voice before applying".into();
        }
        None
    }

    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        samples: &[f32],
        pitch: Option<f32>,
        device: Option<(&str, f32, bool)>,
    ) {
        let input_active = device.is_some_and(|(_, _, failed)| !failed)
            && self.no_samples_elapsed < 2.0
            && !samples.is_empty();
        d.draw_rectangle_rec(
            Rectangle::new(X, Y, WIDTH, HEIGHT),
            Color::new(18, 22, 28, 245),
        );
        d.draw_text(
            "MICROPHONE TOOLS",
            (X + 16.0) as i32,
            (Y + 10.0) as i32,
            20,
            Color::RAYWHITE,
        );
        let device_status = match device {
            None => "No input device".to_owned(),
            Some((name, _, true)) => format!("Disconnected: {name}"),
            Some((name, rate, false)) if self.no_samples_elapsed >= 2.0 => {
                format!("No samples: {name} ({:.0} Hz)", rate)
            }
            Some((name, rate, false)) => format!("{name}  |  {:.0} Hz", rate),
        };
        let device_status: String = device_status.chars().take(48).collect();
        d.draw_text(
            &device_status,
            (X + 16.0) as i32,
            (Y + 37.0) as i32,
            16,
            Color::LIGHTGRAY,
        );
        let level = amplitude_to_db(rms(samples));
        let clipped = input_active
            && samples.iter().filter(|sample| sample.abs() >= 0.98).count() * 200 >= samples.len();
        let level_text = if !input_active {
            "Input: no samples".to_owned()
        } else if clipped {
            format!("Input {level:.0} dBFS  |  CLIPPING: lower system input gain")
        } else {
            format!("Input {level:.0} dBFS")
        };
        d.draw_text(
            &level_text,
            (X + 16.0) as i32,
            (Y + 59.0) as i32,
            16,
            if clipped {
                Color::RED
            } else {
                Color::LIGHTGRAY
            },
        );

        d.draw_text(
            "PITCH TUNER  |  A4 = 440 Hz",
            (X + 16.0) as i32,
            (Y + 89.0) as i32,
            16,
            Color::RAYWHITE,
        );
        if let Some(tuning) = tuner_pitch(if input_active { pitch } else { None }) {
            let text = format!(
                "{}   {:.1} Hz   {:+.1} cents",
                note_name(tuning.midi),
                tuning.frequency,
                tuning.cents
            );
            let color = if tuning.cents.abs() <= 5.0 {
                Color::GREEN
            } else {
                Color::GOLD
            };
            d.draw_text(&text, (X + 16.0) as i32, (Y + 111.0) as i32, 18, color);
            let needle = X + 255.0 + tuning.cents.clamp(-50.0, 50.0) * 3.4;
            d.draw_line(
                needle as i32,
                (Y + 140.0) as i32,
                needle as i32,
                (Y + 154.0) as i32,
                color,
            );
        } else {
            d.draw_text(
                "No stable pitch",
                (X + 16.0) as i32,
                (Y + 111.0) as i32,
                18,
                Color::GRAY,
            );
        }
        d.draw_line(
            (X + 85.0) as i32,
            (Y + 147.0) as i32,
            (X + 425.0) as i32,
            (Y + 147.0) as i32,
            Color::GRAY,
        );
        d.draw_text(
            "-50",
            (X + 57.0) as i32,
            (Y + 140.0) as i32,
            14,
            Color::GRAY,
        );
        d.draw_text("0", (X + 251.0) as i32, (Y + 158.0) as i32, 14, Color::GRAY);
        d.draw_text(
            "+50",
            (X + 430.0) as i32,
            (Y + 140.0) as i32,
            14,
            Color::GRAY,
        );

        d.draw_text(
            "SPECTRUM",
            (X + 16.0) as i32,
            (Y + 180.0) as i32,
            16,
            Color::RAYWHITE,
        );
        for (index, label) in ["63", "125", "250", "500", "1k", "2k", "4k", "8k"]
            .iter()
            .enumerate()
        {
            let bx = X + 25.0 + index as f32 * 60.0;
            let height = if input_active {
                ((self.spectrum[index] + 90.0) / 90.0).clamp(0.0, 1.0) * 48.0
            } else {
                0.0
            };
            d.draw_rectangle(
                bx as i32,
                (Y + 257.0 - height) as i32,
                36,
                height as i32,
                Color::new(100, 220, 160, 255),
            );
            d.draw_text(label, bx as i32, (Y + 262.0) as i32, 14, Color::LIGHTGRAY);
        }
        d.draw_text(
            "CALIBRATION",
            (X + 16.0) as i32,
            (Y + 292.0) as i32,
            16,
            Color::RAYWHITE,
        );
        for (offset, label) in [(16.0, "Noise 3s"), (170.0, "Voice 3s"), (324.0, "Apply")] {
            let rect = Rectangle::new(X + offset, Y + 314.0, 145.0, 34.0);
            d.draw_rectangle_rec(rect, Color::new(44, 46, 51, 255));
            d.draw_text(
                label,
                (rect.x + 10.0) as i32,
                (rect.y + 8.0) as i32,
                18,
                Color::RAYWHITE,
            );
        }
        d.draw_text(
            &self.calibration_status(),
            (X + 16.0) as i32,
            (Y + 354.0) as i32,
            16,
            Color::LIGHTGRAY,
        );
        if let Calibration::Ready { recommendation, .. } = self.calibration {
            d.draw_text(
                &format!(
                    "Gate {:.0} dB  |  Loud offset {:.0} dB",
                    recommendation.gate_db, recommendation.loud_delta_db
                ),
                (X + 16.0) as i32,
                (Y + 373.0) as i32,
                16,
                Color::LIGHTGRAY,
            );
        }
        let message: String = self.message.chars().take(55).collect();
        d.draw_text(
            &message,
            (X + 16.0) as i32,
            (Y + 408.0) as i32,
            16,
            Color::GOLD,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amplitude: f32) -> Vec<f32> {
        (0..800)
            .map(|index| amplitude * (index as f32 * std::f32::consts::TAU * 200.0 / 8_000.0).sin())
            .collect()
    }

    fn run_measurement(tools: &mut MicTools, samples: &[f32]) {
        for _ in 0..31 {
            tools.update(0.1, samples, samples, 8_000.0, 0.0).unwrap();
        }
    }

    #[test]
    fn tuner_reports_note_and_sharp_or_flat_cents() {
        let a = tuner_pitch(Some(440.0)).unwrap();
        assert_eq!(note_name(a.midi), "A4");
        assert!(a.cents.abs() < 0.01);
        for cents in [-25.0_f32, 25.0] {
            let pitch = tuner_pitch(Some(440.0 * 2.0_f32.powf(cents / 1200.0))).unwrap();
            assert_eq!(pitch.midi, a.midi);
            assert!((pitch.cents - cents).abs() < 0.01);
        }
        assert!(tuner_pitch(None).is_none());
        assert!(tuner_pitch(Some(f32::NAN)).is_none());
    }

    #[test]
    fn calibration_requires_explicit_apply_after_noise_and_voice() {
        let mut tools = MicTools {
            open: true,
            ..Default::default()
        };
        assert!(tools.click(Vector2::new(X + 340.0, Y + 325.0)).is_none());
        tools.click(Vector2::new(X + 20.0, Y + 325.0));
        run_measurement(&mut tools, &tone(0.001));
        assert!(matches!(tools.calibration, Calibration::AwaitingVoice(_)));
        tools.click(Vector2::new(X + 180.0, Y + 325.0));
        run_measurement(&mut tools, &tone(0.1));
        assert!(matches!(tools.calibration, Calibration::Ready { .. }));
        let result = tools.click(Vector2::new(X + 340.0, Y + 325.0)).unwrap();
        assert!((result.gate_db - -57.0).abs() < 0.1);
        assert!((result.loud_delta_db - 40.0).abs() < 0.1);
    }

    #[test]
    fn calibration_rejects_missing_quiet_and_clipped_input() {
        let mut tools = MicTools {
            open: true,
            ..Default::default()
        };
        tools.click(Vector2::new(X + 20.0, Y + 325.0));
        run_measurement(&mut tools, &[]);
        assert!(tools.message.contains("No microphone samples"));
        tools.click(Vector2::new(X + 20.0, Y + 325.0));
        run_measurement(&mut tools, &tone(0.01));
        tools.click(Vector2::new(X + 180.0, Y + 325.0));
        run_measurement(&mut tools, &tone(0.01));
        assert!(tools.message.contains("too quiet"));
        tools.click(Vector2::new(X + 180.0, Y + 325.0));
        run_measurement(&mut tools, &tone(1.0));
        assert!(tools.message.contains("clipping"));
        assert!(tools.click(Vector2::new(X + 340.0, Y + 325.0)).is_none());
    }

    #[test]
    fn calibration_ignores_short_pauses_and_noise_peaks() {
        let mut measurement = Measurement::default();
        let quiet = tone(0.001);
        let spike = tone(0.03);
        for index in 0..31 {
            measurement
                .update(
                    0.1,
                    if index == 17 { &spike } else { &quiet },
                    0.0,
                )
                .unwrap();
        }
        let noise = measurement.level_db(8_000.0, 0.90).unwrap();
        assert!(noise < -59.0 && noise > -66.0, "noise {noise}");

        let mut voice = Measurement::default();
        let speech = tone(0.1);
        for index in 0..31 {
            voice
                .update(
                    0.1,
                    if index % 3 == 0 { &quiet } else { &speech },
                    0.0,
                )
                .unwrap();
        }
        let voice = voice.level_db(8_000.0, 0.60).unwrap();
        assert!(voice > -25.0 && voice < -19.0, "voice {voice}");
        assert!(recommend(noise, voice).is_ok());
    }
}

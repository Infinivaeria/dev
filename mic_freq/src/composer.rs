//! Small, deterministic music-composition and offline-rendering engine.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use lewton::inside_ogg::OggStreamReader;
use midly::{MetaMessage, Smf, Timing, TrackEventKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub const SAMPLE_RATE: u32 = 44_100;
pub const STEPS_PER_PATTERN: usize = 16;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub name: String,
    pub tempo: f32,
    pub tracks: Vec<Track>,
}

impl Project {
    pub fn new(name: impl Into<String>, tempo: f32) -> Self {
        let instruments = [
            Waveform::Sine,
            Waveform::Square,
            Waveform::Triangle,
            Waveform::Saw,
        ];
        Self {
            name: name.into(),
            tempo,
            tracks: instruments
                .into_iter()
                .enumerate()
                .map(|(index, waveform)| Track {
                    name: format!("Track {}", index + 1),
                    instrument: Instrument {
                        waveform,
                        ..Instrument::default()
                    },
                    ..Track::default()
                })
                .collect(),
        }
    }

    pub fn to_json(&self) -> Result<Vec<u8>, ComposerError> {
        self.validate()?;
        Ok(serde_json::to_vec_pretty(self).map_err(ComposerError::Json)?)
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, ComposerError> {
        let project: Self = serde_json::from_slice(bytes).map_err(ComposerError::Json)?;
        project.validate()?;
        Ok(project)
    }

    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), ComposerError> {
        std::fs::write(path.as_ref(), self.to_json()?).map_err(ComposerError::Io)
    }

    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, ComposerError> {
        let bytes = std::fs::read(path.as_ref()).map_err(ComposerError::Io)?;
        Self::from_json(&bytes)
    }

    pub fn validate(&self) -> Result<(), ComposerError> {
        if !self.tempo.is_finite() || self.tempo <= 0.0 {
            return Err(ComposerError::InvalidData(
                "tempo must be finite and greater than zero",
            ));
        }
        if self.tracks.len() < 4 {
            return Err(ComposerError::InvalidData(
                "project must contain at least four tracks",
            ));
        }
        for track in &self.tracks {
            if !track.volume.is_finite() || !(0.0..=2.0).contains(&track.volume) {
                return Err(ComposerError::InvalidData(
                    "track volume must be between 0 and 2",
                ));
            }
            if !track.pan.is_finite() || !(-1.0..=1.0).contains(&track.pan) {
                return Err(ComposerError::InvalidData(
                    "track pan must be between -1 and 1",
                ));
            }
            if track.patterns.is_empty() {
                return Err(ComposerError::InvalidData("track has no patterns"));
            }
            for pattern in &track.patterns {
                for step in &pattern.steps {
                    if let Step::Notes { notes } = step {
                        for note in notes {
                            validate_note(*note)?;
                        }
                    }
                }
            }
            track.instrument.validate()?;
        }
        Ok(())
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new("Untitled", 120.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub name: String,
    pub instrument: Instrument,
    #[serde(default = "one_empty_pattern")]
    pub patterns: Vec<Pattern>,
    #[serde(default = "one")]
    pub volume: f32,
    #[serde(default)]
    pub pan: f32,
    #[serde(default)]
    pub muted: bool,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            name: "Track".into(),
            instrument: Instrument::default(),
            patterns: one_empty_pattern(),
            volume: 1.0,
            pan: 0.0,
            muted: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Instrument {
    pub waveform: Waveform,
    pub envelope: Envelope,
    #[serde(default = "one")]
    pub gain: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_path: Option<PathBuf>,
}

impl Instrument {
    fn validate(&self) -> Result<(), ComposerError> {
        if !self.gain.is_finite() || !(0.0..=4.0).contains(&self.gain) {
            return Err(ComposerError::InvalidData(
                "instrument gain must be between 0 and 4",
            ));
        }
        self.envelope.validate()
    }
}

impl Default for Instrument {
    fn default() -> Self {
        Self {
            waveform: Waveform::Sine,
            envelope: Envelope::default(),
            gain: 1.0,
            sample_path: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
    Noise,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Envelope {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Envelope {
    fn validate(&self) -> Result<(), ComposerError> {
        if [self.attack, self.decay, self.sustain, self.release]
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
            || !(0.0..=1.0).contains(&self.sustain)
        {
            return Err(ComposerError::InvalidData("invalid instrument envelope"));
        }
        Ok(())
    }
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            attack: 0.005,
            decay: 0.05,
            sustain: 0.7,
            release: 0.1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pattern {
    pub name: String,
    pub steps: [Step; STEPS_PER_PATTERN],
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            name: "Pattern".into(),
            steps: std::array::from_fn(|_| Step::Rest),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    Rest,
    Notes { notes: Vec<Note> },
}

impl Step {
    pub fn note(midi: u8, velocity: f32) -> Self {
        Self::Notes {
            notes: vec![Note {
                midi,
                velocity,
                duration_steps: 1,
            }],
        }
    }

    pub fn chord(notes: impl IntoIterator<Item = u8>, velocity: f32) -> Self {
        Self::Notes {
            notes: notes
                .into_iter()
                .map(|midi| Note {
                    midi,
                    velocity,
                    duration_steps: 1,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub midi: u8,
    #[serde(default = "one")]
    pub velocity: f32,
    #[serde(default = "one_step")]
    pub duration_steps: usize,
}

fn one() -> f32 {
    1.0
}

fn one_step() -> usize {
    1
}

fn one_empty_pattern() -> Vec<Pattern> {
    vec![Pattern::default()]
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderedAudio {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl RenderedAudio {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }
}

/// Render all patterns sequentially as interleaved stereo f32 PCM.
pub fn render(project: &Project) -> Result<RenderedAudio, ComposerError> {
    project.validate()?;
    let pattern_count = project
        .tracks
        .iter()
        .map(|track| track.patterns.len())
        .max()
        .unwrap_or(0);
    let step_seconds = 60.0 / project.tempo / 4.0;
    let base_frames =
        (step_seconds * SAMPLE_RATE as f32 * pattern_count as f32 * 16.0).ceil() as usize;
    let max_release = project
        .tracks
        .iter()
        .map(|track| track.instrument.envelope.release)
        .fold(0.0_f32, f32::max);
    let release_frames = (max_release * SAMPLE_RATE as f32).ceil() as usize;
    let mut mix = vec![0.0_f32; (base_frames + release_frames) * 2];
    let step_frames = step_seconds * SAMPLE_RATE as f32;
    let mut random_state = 0x9e37_79b9_u32;

    for track in &project.tracks {
        if track.muted {
            continue;
        }
        let sample = match &track.instrument.sample_path {
            Some(path) => Some(import_audio(path)?),
            None => None,
        };
        for (pattern_index, pattern) in track.patterns.iter().enumerate() {
            for (step_index, step) in pattern.steps.iter().enumerate() {
                let Step::Notes { notes } = step else {
                    continue;
                };
                let start_frame =
                    ((pattern_index * 16 + step_index) as f32 * step_frames).round() as usize;
                for note in notes {
                    validate_note(*note)?;
                    let held = step_frames * note.duration_steps as f32;
                    let release = track.instrument.envelope.release * SAMPLE_RATE as f32;
                    let voice_frames = held.ceil() as usize + release.ceil() as usize;
                    for offset in 0..voice_frames {
                        let frame = start_frame + offset;
                        if frame >= base_frames + release_frames {
                            break;
                        }
                        let time = offset as f32 / SAMPLE_RATE as f32;
                        let held_seconds = held / SAMPLE_RATE as f32;
                        let amp = envelope_value(&track.instrument.envelope, time, held_seconds);
                        let source = if let Some(source) = &sample {
                            sample_value(source, note.midi, offset)
                        } else {
                            let frequency = midi_to_frequency(note.midi);
                            oscillator(
                                track.instrument.waveform,
                                frequency,
                                time,
                                &mut random_state,
                            )
                        };
                        let value =
                            source * amp * note.velocity * track.instrument.gain * track.volume;
                        let (left, right) = stereo_pan(value, track.pan);
                        mix[frame * 2] += left;
                        mix[frame * 2 + 1] += right;
                    }
                }
            }
        }
    }
    for sample in &mut mix {
        *sample = sample.clamp(-1.0, 1.0);
    }
    Ok(RenderedAudio {
        sample_rate: SAMPLE_RATE,
        channels: 2,
        samples: mix,
    })
}

fn oscillator(waveform: Waveform, frequency: f32, time: f32, state: &mut u32) -> f32 {
    let phase = (time * frequency).fract();
    match waveform {
        Waveform::Sine => (time * frequency * std::f32::consts::TAU).sin(),
        Waveform::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Saw => 2.0 * phase - 1.0,
        Waveform::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
        Waveform::Noise => {
            *state ^= *state << 13;
            *state ^= *state >> 17;
            *state ^= *state << 5;
            (*state as f32 / u32::MAX as f32) * 2.0 - 1.0
        }
    }
}

fn envelope_value(envelope: &Envelope, time: f32, held: f32) -> f32 {
    if time >= held {
        if envelope.release == 0.0 {
            return 0.0;
        }
        return envelope.sustain * (1.0 - (time - held) / envelope.release).clamp(0.0, 1.0);
    }
    if envelope.attack > 0.0 && time < envelope.attack {
        return time / envelope.attack;
    }
    if time < envelope.attack + envelope.decay {
        if envelope.decay == 0.0 {
            return envelope.sustain;
        }
        let part = (time - envelope.attack) / envelope.decay;
        return 1.0 + (envelope.sustain - 1.0) * part;
    }
    envelope.sustain
}

fn stereo_pan(value: f32, pan: f32) -> (f32, f32) {
    const SQRT_HALF: f32 = 0.707_106_77;
    let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
    (
        value * angle.cos() * SQRT_HALF.recip(),
        value * angle.sin() * SQRT_HALF.recip(),
    )
}

fn validate_note(note: Note) -> Result<(), ComposerError> {
    if note.midi > 127
        || !note.velocity.is_finite()
        || !(0.0..=1.0).contains(&note.velocity)
        || note.duration_steps == 0
    {
        return Err(ComposerError::InvalidData("invalid note event"));
    }
    Ok(())
}

fn sample_value(audio: &RenderedAudio, midi: u8, offset: usize) -> f32 {
    let channels = audio.channels.max(1) as usize;
    let rate = midi_to_frequency(midi) / midi_to_frequency(60);
    let position = offset as f32 * rate;
    let frame = position.floor() as usize;
    if frame >= audio.frames() {
        return 0.0;
    }
    let next = (frame + 1).min(audio.frames() - 1);
    let fraction = position.fract();
    let read = |frame: usize| {
        (0..channels)
            .map(|channel| audio.samples[frame * channels + channel])
            .sum::<f32>()
            / channels as f32
    };
    read(frame) * (1.0 - fraction) + read(next) * fraction
}

pub fn midi_to_frequency(midi: u8) -> f32 {
    440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
}

pub fn frequency_to_midi(frequency: f32) -> Result<u8, ComposerError> {
    if !frequency.is_finite() || frequency <= 0.0 {
        return Err(ComposerError::InvalidData("frequency must be positive"));
    }
    let midi = (69.0 + 12.0 * (frequency / 440.0).log2()).round();
    if !(0.0..=127.0).contains(&midi) {
        return Err(ComposerError::InvalidData(
            "frequency is outside MIDI range",
        ));
    }
    Ok(midi as u8)
}

pub fn note_name(midi: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NAMES[midi as usize % 12], midi as i16 / 12 - 1)
}

pub fn pitch_class(midi: u8) -> u8 {
    midi % 12
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    Chromatic,
    Major,
    NaturalMinor,
}

impl Scale {
    pub fn intervals(self) -> &'static [u8] {
        match self {
            Self::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            Self::Major => &[0, 2, 4, 5, 7, 9, 11],
            Self::NaturalMinor => &[0, 2, 3, 5, 7, 8, 10],
        }
    }
}

pub fn scale_notes(root: u8, scale: Scale, octaves: u8) -> Vec<u8> {
    (0..octaves.max(1))
        .flat_map(|octave| {
            scale
                .intervals()
                .iter()
                .filter_map(move |interval| checked_midi(root, octave * 12 + *interval))
        })
        .collect()
}

pub fn chromatic_notes(root: u8, octaves: u8) -> Vec<u8> {
    scale_notes(root, Scale::Chromatic, octaves)
}

fn checked_midi(root: u8, interval: u8) -> Option<u8> {
    root.checked_add(interval).filter(|pitch| *pitch <= 127)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
    Augmented,
    Dominant7,
    Major7,
    Minor7,
}

impl ChordQuality {
    pub fn intervals(self) -> &'static [u8] {
        match self {
            Self::Major => &[0, 4, 7],
            Self::Minor => &[0, 3, 7],
            Self::Diminished => &[0, 3, 6],
            Self::Augmented => &[0, 4, 8],
            Self::Dominant7 => &[0, 4, 7, 10],
            Self::Major7 => &[0, 4, 7, 11],
            Self::Minor7 => &[0, 3, 7, 10],
        }
    }
}

pub fn chord_tones(root: u8, quality: ChordQuality, inversion: usize) -> Vec<u8> {
    let mut notes: Vec<u8> = quality
        .intervals()
        .iter()
        .filter_map(|interval| checked_midi(root, *interval))
        .collect();
    for _ in 0..inversion % notes.len().max(1) {
        let pitch = notes.remove(0).saturating_add(12).min(127);
        notes.push(pitch);
    }
    notes
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Chord {
    pub root: u8,
    pub quality: ChordQuality,
}

pub fn diatonic_progression(
    root: u8,
    scale: Scale,
    degrees: impl IntoIterator<Item = usize>,
) -> Vec<Chord> {
    let intervals = scale.intervals();
    degrees
        .into_iter()
        .filter_map(|degree| {
            degree
                .checked_sub(1)
                .filter(|index| *index < intervals.len())
                .map(|index| {
                    let chord_root = root.saturating_add(intervals[index]).min(127);
                    let third =
                        (intervals[(index + 2) % intervals.len()] + 12 - intervals[index]) % 12;
                    let fifth =
                        (intervals[(index + 4) % intervals.len()] + 12 - intervals[index]) % 12;
                    let quality = match (third, fifth) {
                        (3, 6) => ChordQuality::Diminished,
                        (3, _) => ChordQuality::Minor,
                        (4, 8) => ChordQuality::Augmented,
                        _ => ChordQuality::Major,
                    };
                    Chord {
                        root: chord_root,
                        quality,
                    }
                })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArpeggioPattern {
    Up,
    Down,
    UpDown,
    DownUp,
    Random,
}

pub fn arpeggiate(notes: &[u8], pattern: ArpeggioPattern, length: usize) -> Vec<u8> {
    if notes.is_empty() || length == 0 {
        return Vec::new();
    }
    match pattern {
        ArpeggioPattern::Up => (0..length).map(|i| notes[i % notes.len()]).collect(),
        ArpeggioPattern::Down => (0..length)
            .map(|i| notes[notes.len() - 1 - i % notes.len()])
            .collect(),
        ArpeggioPattern::UpDown => ping_pong(notes, length, false),
        ArpeggioPattern::DownUp => ping_pong(notes, length, true),
        ArpeggioPattern::Random => {
            let mut state = 0x1234_abcd_u32;
            (0..length)
                .map(|_| {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    notes[state as usize % notes.len()]
                })
                .collect()
        }
    }
}

fn ping_pong(notes: &[u8], length: usize, reverse: bool) -> Vec<u8> {
    if notes.len() == 1 {
        return vec![notes[0]; length];
    }
    let path_len = notes.len() * 2 - 2;
    (0..length)
        .map(|index| {
            let position = if reverse {
                (path_len - index % path_len) % path_len
            } else {
                index % path_len
            };
            notes[if position < notes.len() {
                position
            } else {
                path_len - position
            }]
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyAnalysis {
    pub tonic: u8,
    pub scale: Scale,
    pub score: usize,
}

pub fn analyze_key(notes: &[u8]) -> Option<KeyAnalysis> {
    let pitch_classes: Vec<u8> = notes.iter().map(|note| note % 12).collect();
    let mut candidates = (0_u8..12)
        .flat_map(|tonic| [Scale::Major, Scale::NaturalMinor].map(|scale| (tonic, scale)))
        .map(|(tonic, scale)| {
            let allowed: Vec<u8> = scale
                .intervals()
                .iter()
                .map(|interval| (tonic + interval) % 12)
                .collect();
            let mut score = pitch_classes
                .iter()
                .filter(|pitch| allowed.contains(pitch))
                .count();
            if pitch_classes.first() == Some(&tonic) {
                score += 2;
            }
            KeyAnalysis {
                tonic,
                scale,
                score,
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.score));
    candidates
        .into_iter()
        .next()
        .filter(|candidate| candidate.score > 0)
}

pub fn analyze_chord(notes: &[u8]) -> Option<Chord> {
    let mut classes: Vec<u8> = notes.iter().map(|note| note % 12).collect();
    classes.sort_unstable();
    classes.dedup();
    (0_u8..12)
        .flat_map(|root| {
            [
                ChordQuality::Major,
                ChordQuality::Minor,
                ChordQuality::Diminished,
                ChordQuality::Augmented,
                ChordQuality::Dominant7,
                ChordQuality::Major7,
                ChordQuality::Minor7,
            ]
            .map(|quality| Chord { root, quality })
        })
        .find(|chord| {
            let mut expected: Vec<u8> = chord
                .quality
                .intervals()
                .iter()
                .map(|interval| (chord.root + interval) % 12)
                .collect();
            expected.sort_unstable();
            classes == expected
        })
}

pub fn import_wav(path: impl AsRef<Path>) -> Result<RenderedAudio, ComposerError> {
    let mut reader = hound::WavReader::open(path.as_ref()).map_err(ComposerError::Wav)?;
    let spec = reader.spec();
    if spec.channels == 0 {
        return Err(ComposerError::InvalidData("WAV has no channels"));
    }
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(ComposerError::Wav)?,
        hound::SampleFormat::Int => {
            let divisor = (1_i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / divisor))
                .collect::<Result<Vec<_>, _>>()
                .map_err(ComposerError::Wav)?
        }
    };
    Ok(resample_audio(
        RenderedAudio {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            samples,
        },
        SAMPLE_RATE,
    ))
}

pub fn export_wav(audio: &RenderedAudio, path: impl AsRef<Path>) -> Result<(), ComposerError> {
    if audio.channels == 0
        || audio.sample_rate == 0
        || audio.samples.len() % audio.channels as usize != 0
    {
        return Err(ComposerError::InvalidData("invalid audio buffer"));
    }
    let spec = hound::WavSpec {
        channels: audio.channels,
        sample_rate: audio.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path.as_ref(), spec).map_err(ComposerError::Wav)?;
    for sample in &audio.samples {
        writer.write_sample(*sample).map_err(ComposerError::Wav)?;
    }
    writer.finalize().map_err(ComposerError::Wav)
}

pub fn import_ogg_vorbis(path: impl AsRef<Path>) -> Result<RenderedAudio, ComposerError> {
    let file = File::open(path.as_ref()).map_err(ComposerError::Io)?;
    let mut reader = OggStreamReader::new(file).map_err(ComposerError::Ogg)?;
    let sample_rate = reader.ident_hdr.audio_sample_rate;
    let channels = reader.ident_hdr.audio_channels as u16;
    if sample_rate == 0 || channels == 0 {
        return Err(ComposerError::InvalidData("invalid OGG audio header"));
    }
    let mut samples = Vec::new();
    while let Some(packet) = reader.read_dec_packet_itl().map_err(ComposerError::Ogg)? {
        samples.extend(packet.into_iter().map(|sample| sample as f32 / 32768.0));
    }
    Ok(resample_audio(
        RenderedAudio {
            sample_rate,
            channels,
            samples,
        },
        SAMPLE_RATE,
    ))
}

pub fn import_audio(path: impl AsRef<Path>) -> Result<RenderedAudio, ComposerError> {
    match path
        .as_ref()
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav") | Some("wave") => import_wav(path),
        Some("ogg") | Some("oga") => import_ogg_vorbis(path),
        _ => Err(ComposerError::UnsupportedFormat(
            path.as_ref().display().to_string(),
        )),
    }
}

fn resample_audio(mut audio: RenderedAudio, target_rate: u32) -> RenderedAudio {
    if audio.sample_rate == target_rate {
        return audio;
    }
    let frames = audio.frames();
    let out_frames =
        (frames as f64 * target_rate as f64 / audio.sample_rate as f64).ceil() as usize;
    let channels = audio.channels as usize;
    let mut output = Vec::with_capacity(out_frames * channels);
    for frame in 0..out_frames {
        let position = frame as f64 * audio.sample_rate as f64 / target_rate as f64;
        let first = position.floor() as usize;
        let second = (first + 1).min(frames.saturating_sub(1));
        let fraction = position.fract() as f32;
        for channel in 0..channels {
            let a = audio
                .samples
                .get(first * channels + channel)
                .copied()
                .unwrap_or(0.0);
            let b = audio
                .samples
                .get(second * channels + channel)
                .copied()
                .unwrap_or(0.0);
            output.push(a * (1.0 - fraction) + b * fraction);
        }
    }
    audio.sample_rate = target_rate;
    audio.samples = output;
    audio
}

pub fn import_midi(path: impl AsRef<Path>) -> Result<Project, ComposerError> {
    let bytes = std::fs::read(path.as_ref()).map_err(ComposerError::Io)?;
    midi_from_bytes(&bytes)
}

pub fn midi_from_bytes(bytes: &[u8]) -> Result<Project, ComposerError> {
    let smf = Smf::parse(bytes).map_err(ComposerError::Midi)?;
    let ticks_per_quarter = match smf.header.timing {
        Timing::Metrical(ticks) => u16::from(ticks).max(1) as u32,
        Timing::Timecode(..) => 480,
    };
    let mut tempo = 120.0_f32;
    for track in &smf.tracks {
        for event in track {
            if let TrackEventKind::Meta(MetaMessage::Tempo(value)) = event.kind {
                let micros = u32::from(value) as f32;
                if micros > 0.0 {
                    tempo = 60_000_000.0 / micros;
                    break;
                }
            }
        }
    }

    let mut tracks = Vec::new();
    let mut max_end_step = 0_usize;
    for (track_index, midi_track) in smf.tracks.iter().enumerate() {
        let mut absolute_ticks = 0_u64;
        let mut active: HashMap<(u8, u8), (u64, u8)> = HashMap::new();
        let mut notes = Vec::new();
        let mut track_name = format!("MIDI Track {}", track_index + 1);
        for event in midi_track {
            absolute_ticks += event.delta.as_int() as u64;
            match event.kind {
                TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                    if let Ok(name) = std::str::from_utf8(name) {
                        track_name = name.to_owned();
                    }
                }
                TrackEventKind::Midi { channel, message } => match message {
                    midly::MidiMessage::NoteOn { key, vel } => {
                        if vel.as_int() > 0 {
                            active.insert(
                                (channel.as_int(), key.as_int()),
                                (absolute_ticks, vel.as_int()),
                            );
                        } else if let Some((start, velocity)) =
                            active.remove(&(channel.as_int(), key.as_int()))
                        {
                            notes.push(ImportedNote {
                                start_ticks: start,
                                end_ticks: absolute_ticks.max(start),
                                midi: key.as_int(),
                                velocity,
                            });
                        }
                    }
                    midly::MidiMessage::NoteOff { key, .. } => {
                        if let Some((start, velocity)) =
                            active.remove(&(channel.as_int(), key.as_int()))
                        {
                            notes.push(ImportedNote {
                                start_ticks: start,
                                end_ticks: absolute_ticks.max(start),
                                midi: key.as_int(),
                                velocity,
                            });
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        for ((_, key), (start, velocity)) in active {
            notes.push(ImportedNote {
                start_ticks: start,
                end_ticks: absolute_ticks.max(start),
                midi: key,
                velocity,
            });
        }
        if notes.is_empty() {
            continue;
        }
        let scale = |ticks: u64| (ticks * 4).div_ceil(ticks_per_quarter as u64) as usize;
        max_end_step = max_end_step.max(
            notes
                .iter()
                .map(|note| scale(note.end_ticks))
                .max()
                .unwrap_or(0),
        );
        let pattern_count = notes
            .iter()
            .map(|note| scale(note.end_ticks) / 16 + 1)
            .max()
            .unwrap_or(1);
        let mut patterns = (0..pattern_count)
            .map(|_| Pattern::default())
            .collect::<Vec<_>>();
        for note in notes {
            let step = scale(note.start_ticks);
            if step >= pattern_count * 16 {
                continue;
            }
            let duration = (scale(note.end_ticks).saturating_sub(step)).max(1);
            let imported = Note {
                midi: note.midi,
                velocity: note.velocity as f32 / 127.0,
                duration_steps: duration,
            };
            let pattern = &mut patterns[step / 16];
            let slot = &mut pattern.steps[step % 16];
            match slot {
                Step::Rest => {
                    *slot = Step::Notes {
                        notes: vec![imported],
                    }
                }
                Step::Notes { notes } => notes.push(imported),
            }
        }
        tracks.push(Track {
            name: track_name,
            patterns,
            ..Track::default()
        });
    }
    if tracks.is_empty() {
        return Err(ComposerError::InvalidData("MIDI contains no note events"));
    }
    // Keep a conventional minimum project shape for files without multiple tracks.
    while tracks.len() < 4 {
        let mut track = Track::default();
        track.name = format!("Imported {}", tracks.len() + 1);
        track.patterns = (0..max_end_step.div_ceil(16).max(1))
            .map(|_| Pattern::default())
            .collect();
        tracks.push(track);
    }
    Ok(Project {
        name: "Imported MIDI".into(),
        tempo,
        tracks,
    })
}

#[derive(Clone, Copy)]
struct ImportedNote {
    start_ticks: u64,
    end_ticks: u64,
    midi: u8,
    velocity: u8,
}

#[derive(Default)]
pub struct PlaybackController {
    stream: Option<Stream>,
    position: Option<Arc<AtomicUsize>>,
}

impl PlaybackController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn output_devices() -> Result<Vec<String>, ComposerError> {
        cpal::default_host()
            .output_devices()
            .map_err(ComposerError::Devices)?
            .map(|device| device.name().map_err(ComposerError::DeviceName))
            .collect()
    }

    /// Start playback on an explicitly named output device. No default-device
    /// fallback is performed.
    pub fn start(&mut self, audio: RenderedAudio, device_name: &str) -> Result<(), ComposerError> {
        if self.stream.is_some() {
            return Err(ComposerError::Playback("playback is already active"));
        }
        if audio.channels != 2 || audio.sample_rate != SAMPLE_RATE {
            return Err(ComposerError::InvalidData(
                "playback requires 44.1 kHz stereo audio",
            ));
        }
        let host = cpal::default_host();
        let device = host
            .output_devices()
            .map_err(ComposerError::Devices)?
            .find(|device| device.name().is_ok_and(|name| name == device_name))
            .ok_or_else(|| ComposerError::OutputDeviceNotFound(device_name.to_owned()))?;
        let supported = device
            .default_output_config()
            .map_err(ComposerError::DefaultStreamConfig)?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let data = Arc::new(resample_audio(audio, config.sample_rate.0).samples);
        let position = Arc::new(AtomicUsize::new(0));
        let build = |kind: PlaybackSample| -> Result<Stream, ComposerError> {
            match kind {
                PlaybackSample::F32 => {
                    build_playback::<f32>(&device, &config, data.clone(), position.clone())
                }
                PlaybackSample::I16 => {
                    build_playback::<i16>(&device, &config, data.clone(), position.clone())
                }
                PlaybackSample::I32 => {
                    build_playback::<i32>(&device, &config, data.clone(), position.clone())
                }
                PlaybackSample::U8 => {
                    build_playback::<u8>(&device, &config, data.clone(), position.clone())
                }
                PlaybackSample::U16 => {
                    build_playback::<u16>(&device, &config, data.clone(), position.clone())
                }
                PlaybackSample::F64 => {
                    build_playback::<f64>(&device, &config, data.clone(), position.clone())
                }
            }
            .map_err(ComposerError::BuildStream)
        };
        let stream = match format {
            SampleFormat::F32 => build(PlaybackSample::F32)?,
            SampleFormat::I16 => build(PlaybackSample::I16)?,
            SampleFormat::I32 => build(PlaybackSample::I32)?,
            SampleFormat::U8 => build(PlaybackSample::U8)?,
            SampleFormat::U16 => build(PlaybackSample::U16)?,
            SampleFormat::F64 => build(PlaybackSample::F64)?,
            other => return Err(ComposerError::PlaybackFormat(other)),
        };
        stream.play().map_err(ComposerError::PlayStream)?;
        self.position = Some(position);
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), ComposerError> {
        let stream = self
            .stream
            .take()
            .ok_or(ComposerError::Playback("playback is not active"))?;
        self.position = None;
        stream.pause().map_err(ComposerError::PauseStream)
    }

    pub fn is_playing(&self) -> bool {
        self.stream.is_some()
    }

    pub fn played_frames(&self) -> Option<usize> {
        self.position
            .as_ref()
            .map(|position| position.load(Ordering::Relaxed) / 2)
    }
}

#[derive(Clone, Copy)]
enum PlaybackSample {
    F32,
    I16,
    I32,
    U8,
    U16,
    F64,
}

fn build_playback<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &StreamConfig,
    data: Arc<Vec<f32>>,
    position: Arc<AtomicUsize>,
) -> Result<Stream, cpal::BuildStreamError> {
    let channels = config.channels.max(1) as usize;
    device.build_output_stream(
        config,
        move |output: &mut [T], _| {
            for frame in output.chunks_mut(channels) {
                let offset = position.load(Ordering::Relaxed);
                let left = data.get(offset).copied().unwrap_or(0.0);
                let right = data.get(offset + 1).copied().unwrap_or(0.0);
                if channels == 1 {
                    frame[0] = T::from_sample((left + right) * 0.5);
                } else {
                    frame[0] = T::from_sample(left);
                    frame[1] = T::from_sample(right);
                    for sample in &mut frame[2..] {
                        *sample = T::from_sample(0.0);
                    }
                }
                if offset + 1 < data.len() {
                    position.store(offset + 2, Ordering::Relaxed);
                }
            }
        },
        |error| eprintln!("composer playback error: {error}"),
        None,
    )
}

#[derive(Debug)]
pub enum ComposerError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Wav(hound::Error),
    Ogg(lewton::VorbisError),
    Midi(midly::Error),
    InvalidData(&'static str),
    UnsupportedFormat(String),
    Devices(cpal::DevicesError),
    DeviceName(cpal::DeviceNameError),
    DefaultStreamConfig(cpal::DefaultStreamConfigError),
    BuildStream(cpal::BuildStreamError),
    PlayStream(cpal::PlayStreamError),
    PauseStream(cpal::PauseStreamError),
    OutputDeviceNotFound(String),
    Playback(&'static str),
    PlaybackFormat(SampleFormat),
}

impl fmt::Display for ComposerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::Wav(error) => write!(formatter, "WAV error: {error}"),
            Self::Ogg(error) => write!(formatter, "OGG error: {error}"),
            Self::Midi(error) => write!(formatter, "MIDI error: {error}"),
            Self::InvalidData(message) | Self::Playback(message) => formatter.write_str(message),
            Self::UnsupportedFormat(path) => write!(formatter, "unsupported audio format: {path}"),
            Self::Devices(error) => write!(formatter, "cannot enumerate devices: {error}"),
            Self::DeviceName(error) => write!(formatter, "cannot read device name: {error}"),
            Self::DefaultStreamConfig(error) => {
                write!(formatter, "cannot read output configuration: {error}")
            }
            Self::BuildStream(error) => write!(formatter, "cannot build playback: {error}"),
            Self::PlayStream(error) => write!(formatter, "cannot start playback: {error}"),
            Self::PauseStream(error) => write!(formatter, "cannot stop playback: {error}"),
            Self::OutputDeviceNotFound(name) => {
                write!(formatter, "output device not found: {name}")
            }
            Self::PlaybackFormat(format) => {
                write!(formatter, "unsupported playback format: {format:?}")
            }
        }
    }
}

impl Error for ComposerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theory_helpers_produce_expected_music() {
        assert!((midi_to_frequency(69) - 440.0).abs() < f32::EPSILON);
        assert_eq!(frequency_to_midi(440.0).unwrap(), 69);
        assert_eq!(
            scale_notes(60, Scale::Major, 1),
            vec![60, 62, 64, 65, 67, 69, 71]
        );
        assert_eq!(chord_tones(60, ChordQuality::Major, 1), vec![64, 67, 72]);
        assert_eq!(
            arpeggiate(&[60, 64, 67], ArpeggioPattern::Up, 5),
            [60, 64, 67, 60, 64]
        );
        assert_eq!(
            analyze_chord(&[67, 60, 64]),
            Some(Chord {
                root: 0,
                quality: ChordQuality::Major
            })
        );
        assert_eq!(
            analyze_chord(&[55, 52, 59]),
            Some(Chord {
                root: 4,
                quality: ChordQuality::Minor
            })
        );
        let progression = diatonic_progression(60, Scale::Major, [1, 4, 5]);
        assert_eq!(
            progression,
            [
                Chord {
                    root: 60,
                    quality: ChordQuality::Major
                },
                Chord {
                    root: 65,
                    quality: ChordQuality::Major
                },
                Chord {
                    root: 67,
                    quality: ChordQuality::Major
                }
            ]
        );
        assert_eq!(
            analyze_key(&[60, 62, 64, 67, 69, 71]).unwrap().scale,
            Scale::Major
        );
    }

    #[test]
    fn project_json_roundtrip_preserves_shape() {
        let mut project = Project::default();
        project.tracks[0].patterns[0].steps[0] = Step::chord([60, 64, 67], 0.8);
        let encoded = project.to_json().unwrap();
        let decoded = Project::from_json(&encoded).unwrap();
        assert_eq!(decoded, project);
        assert_eq!(decoded.tracks.len(), 4);
        assert_eq!(decoded.tracks[0].patterns[0].steps.len(), 16);
    }

    #[test]
    fn render_is_deterministic_stereo_44100() {
        let mut project = Project::new("Test", 120.0);
        project.tracks[0].patterns[0].steps[0] = Step::note(60, 0.7);
        let first = render(&project).unwrap();
        let second = render(&project).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.channels, 2);
        assert_eq!(first.sample_rate, 44_100);
        assert!(first.samples.iter().any(|sample| *sample != 0.0));
    }

    #[test]
    fn wav_roundtrip_preserves_samples() {
        let path = std::path::Path::new("composer-test.wav");
        let audio = RenderedAudio {
            sample_rate: SAMPLE_RATE,
            channels: 2,
            samples: vec![0.0, 0.25, -0.5, 1.0],
        };
        export_wav(&audio, path).unwrap();
        let imported = import_wav(path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(imported.samples, audio.samples);
    }

    #[test]
    fn midi_import_reads_notes_and_tempo_with_fallback_tracks() {
        let smf = format_midi_test_bytes();
        let project = midi_from_bytes(&smf).unwrap();
        assert!((project.tempo - 120.0).abs() < 0.01);
        assert!(project.tracks.len() >= 4);
        assert!(project
            .tracks
            .iter()
            .flat_map(|track| &track.patterns)
            .flat_map(|pattern| &pattern.steps)
            .any(|step| matches!(step, Step::Notes { notes } if notes.iter().any(|note| note.midi == 60))));
    }

    fn format_midi_test_bytes() -> Vec<u8> {
        // Header, format 0, one track, 120 BPM, C4 note, end of track.
        vec![
            b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xe0, b'M', b'T', b'r', b'k', 0,
            0, 0, 19, 0, 0xff, 0x51, 3, 0x07, 0xa1, 0x20, 0, 0x90, 60, 100, 0x83, 0x60, 0x80, 60,
            0, 0, 0xff, 0x2f, 0,
        ]
    }
}

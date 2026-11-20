//! Interactive raylib front-end for the music composer engine.

use crate::analysis::{self, Analysis, RhythmAnalysis, RhythmTracker};
use crate::composer::{
    self, ArpeggioPattern, ChordQuality, Envelope, PlaybackController, Project, Scale, Step,
    Waveform,
};
use raylib::prelude::*;
use std::error::Error;
use std::path::PathBuf;

pub const LOGICAL_WIDTH: f32 = 960.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;
const TRACK_COUNT: usize = 4;
const PITCH_COUNT: usize = 12;
const LOW_PITCH: u8 = 60;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiGeometry {
    pub scale: f32,
    pub offset: Vector2,
}

impl UiGeometry {
    pub fn from_screen(width: f32, height: f32) -> Self {
        let scale = (width.max(1.0) / LOGICAL_WIDTH).min(height.max(1.0) / LOGICAL_HEIGHT);
        Self {
            scale,
            offset: Vector2::new(
                (width - LOGICAL_WIDTH * scale) * 0.5,
                (height - LOGICAL_HEIGHT * scale) * 0.5,
            ),
        }
    }

    pub fn to_logical(self, point: Vector2) -> Vector2 {
        (point - self.offset) * (1.0 / self.scale.max(f32::EPSILON))
    }
}

/// Calculate the centered, aspect-preserving transform for the composer canvas.
pub fn ui_geometry(width: f32, height: f32) -> UiGeometry {
    UiGeometry::from_screen(width, height)
}

#[derive(Default)]
struct UiState {
    project: Project,
    project_path: Option<PathBuf>,
    selected_track: usize,
    selected_step: usize,
    root: u8,
    scale: usize,
    quality: usize,
    inversion: usize,
    arpeggio: usize,
    devices: Vec<String>,
    device: usize,
    playback: PlaybackController,
    signal: Analysis,
    rhythm: RhythmAnalysis,
    status: String,
    is_error: bool,
    help_open: bool,
    close: bool,
}

impl UiState {
    fn new(project_path: Option<PathBuf>) -> Self {
        let (project, path, load_error) = match project_path {
            Some(path) => match Project::load_json(&path) {
                Ok(project) => (project, Some(path), None),
                Err(error) => (
                    Project::default(),
                    Some(path),
                    Some(format!("Could not load project: {error}")),
                ),
            },
            None => (Project::default(), None, None),
        };
        let is_load_error = load_error.is_some();
        let (devices, status, is_error) = match PlaybackController::output_devices() {
            Ok(devices) => {
                let status =
                    load_error.unwrap_or_else(|| "Ready. Drop JSON, MIDI, WAV or OGG.".into());
                (devices, status, is_load_error)
            }
            Err(error) => (Vec::new(), format!("Audio devices: {error}"), true),
        };
        Self {
            project,
            project_path: path,
            devices,
            status,
            is_error,
            ..Self::default()
        }
    }

    fn message(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.is_error = false;
    }

    fn error(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.is_error = true;
    }

    fn save_path(&self) -> Result<PathBuf, Box<dyn Error>> {
        if let Some(path) = &self.project_path {
            return Ok(path.clone());
        }
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .ok_or("No configuration directory is available")?;
        let mut stem: String = self
            .project
            .name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        if stem.is_empty() {
            stem = "untitled".into();
        }
        Ok(root.join("mic_freq/projects").join(format!("{stem}.json")))
    }

    fn save(&mut self) {
        let result = (|| -> Result<(), Box<dyn Error>> {
            let path = self.save_path()?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            self.project.save_json(&path)?;
            self.project_path = Some(path);
            Ok(())
        })();
        match result {
            Ok(()) => self.message(format!(
                "Saved {}",
                self.project_path.as_ref().unwrap().display()
            )),
            Err(error) => self.error(format!("Save failed: {error}")),
        }
    }

    fn export(&mut self) {
        let result = (|| -> Result<PathBuf, Box<dyn Error>> {
            let project_path = self.save_path()?;
            let output = project_path.with_extension("wav");
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let audio = composer::render(&self.project)?;
            composer::export_wav(&audio, &output)?;
            Ok(output)
        })();
        match result {
            Ok(path) => self.message(format!("Exported {}", path.display())),
            Err(error) => self.error(format!("Export failed: {error}")),
        }
    }

    fn stop(&mut self) {
        if self.playback.is_playing() {
            if let Err(error) = self.playback.stop() {
                self.error(format!("Stop failed: {error}"));
            }
        }
    }

    fn toggle_play(&mut self) {
        if self.playback.is_playing() {
            self.stop();
            self.message("Stopped");
            return;
        }
        if self.devices.is_empty() {
            self.error("No output devices are available");
            return;
        }
        let device = self.devices[self.device % self.devices.len()].clone();
        match composer::render(&self.project).and_then(|audio| {
            self.refresh_signal(&audio);
            self.playback.start(audio, &device)
        }) {
            Ok(()) => self.message(format!("Playing on {device}")),
            Err(error) => self.error(format!("Playback failed: {error}")),
        }
    }

    fn refresh_signal(&mut self, audio: &composer::RenderedAudio) {
        match analysis::analyze_rendered_audio(audio, 65_536) {
            Ok(analysis) => self.signal = analysis,
            Err(error) => {
                self.error(format!("Signal analysis failed: {error}"));
                return;
            }
        }
        let mut tracker = match RhythmTracker::try_new(audio.sample_rate as f32) {
            Ok(tracker) => tracker,
            Err(error) => {
                self.error(format!("Rhythm analysis failed: {error}"));
                return;
            }
        };
        if let Ok(mono) = analysis::downmix(audio) {
            for chunk in mono.chunks(2_048) {
                self.rhythm = tracker.update(chunk);
            }
        }
    }

    fn analyze_project(&mut self) {
        match composer::render(&self.project) {
            Ok(audio) => {
                self.refresh_signal(&audio);
                let pitch = self
                    .signal
                    .midi_note
                    .map(composer::note_name)
                    .unwrap_or_else(|| "no stable pitch".into());
                let bpm = self
                    .rhythm
                    .bpm
                    .map_or_else(|| "no stable pulse".into(), |bpm| format!("{bpm:.1} BPM"));
                self.message(format!("Analysis: {pitch}, {bpm}"));
            }
            Err(error) => self.error(format!("Analysis render failed: {error}")),
        }
    }

    fn cycle_device(&mut self) {
        if self.devices.is_empty() {
            self.error("No output devices are available");
            return;
        }
        self.stop();
        self.device = (self.device + 1) % self.devices.len();
        self.message(format!("Output: {}", self.devices[self.device]));
    }

    fn toggle_note(&mut self, pitch: u8) {
        let step =
            &mut self.project.tracks[self.selected_track].patterns[0].steps[self.selected_step];
        match step {
            Step::Rest => *step = Step::note(pitch, 0.85),
            Step::Notes { notes } => {
                if let Some(index) = notes.iter().position(|note| note.midi == pitch) {
                    notes.remove(index);
                    if notes.is_empty() {
                        *step = Step::Rest;
                    }
                } else {
                    notes.push(composer::Note {
                        midi: pitch,
                        velocity: 0.85,
                        duration_steps: 1,
                    });
                }
            }
        }
    }

    fn set_note_chord(&mut self, step_index: usize, notes: Vec<u8>) {
        self.project.tracks[self.selected_track].patterns[0].steps[step_index] =
            Step::chord(notes, 0.8);
    }

    fn add_chord(&mut self) {
        let notes = composer::chord_tones(60 + self.root, QUALITIES[self.quality], self.inversion);
        self.set_note_chord(self.selected_step, notes);
        self.message(format!(
            "Added {:?} chord at step {}",
            QUALITIES[self.quality],
            self.selected_step + 1
        ));
    }

    fn progression(&mut self) {
        let chords =
            composer::diatonic_progression(60 + self.root, SCALES[self.scale], [1, 4, 5, 1]);
        for (step, chord) in chords.into_iter().enumerate() {
            self.set_note_chord(
                step * 4,
                composer::chord_tones(chord.root, chord.quality, self.inversion),
            );
        }
        self.message("Added I-IV-V-I progression");
    }

    fn apply_arp(&mut self) {
        self.arpeggio = (self.arpeggio + 1) % ARPEGGIOS.len();
        let notes = current_step_notes(&self.project, self.selected_track, self.selected_step);
        if notes.is_empty() {
            self.error("Arp needs a note or chord at the selected step");
            return;
        }
        for (offset, midi) in composer::arpeggiate(&notes, ARPEGGIOS[self.arpeggio], 8)
            .into_iter()
            .enumerate()
        {
            let index = (self.selected_step + offset) % composer::STEPS_PER_PATTERN;
            self.project.tracks[self.selected_track].patterns[0].steps[index] =
                Step::note(midi, 0.8);
        }
        self.message(format!("Arp: {:?}", ARPEGGIOS[self.arpeggio]));
    }

    fn current_analysis(&self) -> String {
        let notes = project_notes(&self.project, self.selected_track);
        let chord = composer::analyze_chord(&notes)
            .map(|chord| format!("{} {:?}", NOTE_NAMES[chord.root as usize], chord.quality));
        let key = composer::analyze_key(&notes)
            .map(|key| format!("{} {:?}", NOTE_NAMES[key.tonic as usize], key.scale));
        format!(
            "Chord: {}  |  Key: {}",
            chord.unwrap_or_else(|| "—".into()),
            key.unwrap_or_else(|| "—".into())
        )
    }

    fn dropped(&mut self, files: &[PathBuf]) {
        if files.len() != 1 {
            self.error("Drop exactly one project, MIDI or audio file");
            return;
        }
        let path = &files[0];
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let result = match extension.as_str() {
            "json" => Project::load_json(path).map(|project| {
                self.project = project;
                self.project_path = Some(path.clone());
                self.message(format!("Loaded {}", path.display()));
            }),
            "mid" | "midi" => composer::import_midi(path).map(|project| {
                self.project = project;
                self.project_path = None;
                self.message(format!("Imported MIDI {}", path.display()));
            }),
            "wav" | "ogg" => composer::import_audio(path).map(|_| {
                self.project.tracks[self.selected_track]
                    .instrument
                    .sample_path = Some(path.clone());
                self.message(format!(
                    "Assigned sample to track {}",
                    self.selected_track + 1
                ));
            }),
            _ => {
                self.error("Unsupported drop. Use .json, .mid, .midi, .wav or .ogg");
                return;
            }
        };
        if let Err(error) = result {
            self.error(format!("Import failed: {error}"));
        }
    }

    fn input(&mut self, rl: &mut RaylibHandle) {
        let dismiss_key = rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) && self.help_open;
        if dismiss_key {
            self.help_open = false;
            return;
        }
        let geometry = ui_geometry(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let mouse = geometry.to_logical(rl.get_mouse_position());
        let help_clicked = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
            && (rect(858.0, 55.0, 82.0, 38.0).check_collision_point_rec(mouse)
                || (self.help_open
                    && rect(804.0, 520.0, 80.0, 32.0).check_collision_point_rec(mouse)));
        if help_clicked {
            self.help_open = !self.help_open;
            return;
        }
        if self.help_open {
            return;
        }
        if rl.is_file_dropped() {
            let files: Vec<_> = rl
                .load_dropped_files()
                .paths()
                .iter()
                .map(PathBuf::from)
                .collect();
            self.dropped(&files);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.close = true;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            self.toggle_play();
        }
        if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) && rl.is_key_pressed(KeyboardKey::KEY_S) {
            self.save();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            self.selected_track = self.selected_track.saturating_sub(1);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            self.selected_track = (self.selected_track + 1).min(TRACK_COUNT - 1);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_LEFT) {
            self.selected_step = self.selected_step.saturating_sub(1);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_RIGHT) {
            self.selected_step = (self.selected_step + 1).min(composer::STEPS_PER_PATTERN - 1);
        }

        let click = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let held = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let hit = |bounds: Rectangle| click && bounds.check_collision_point_rec(mouse);

        if hit(rect(12.0, 55.0, 72.0, 38.0)) {
            self.stop();
            self.project = Project::default();
            self.project_path = None;
            self.message("New project");
        }
        if hit(rect(90.0, 55.0, 72.0, 38.0)) {
            self.save();
        }
        if hit(rect(168.0, 55.0, 105.0, 38.0)) {
            self.export();
        }
        if hit(rect(290.0, 55.0, 92.0, 38.0)) {
            self.toggle_play();
        }
        if hit(rect(388.0, 55.0, 40.0, 38.0)) {
            self.project.tempo = (self.project.tempo - 1.0).max(20.0);
        }
        if hit(rect(430.0, 55.0, 40.0, 38.0)) {
            self.project.tempo = (self.project.tempo + 1.0).min(300.0);
        }
        if hit(rect(485.0, 55.0, 222.0, 38.0)) {
            self.cycle_device();
        }
        if hit(rect(713.0, 55.0, 137.0, 38.0)) {
            self.analyze_project();
        }
        for track in 0..TRACK_COUNT {
            if hit(track_rect(track)) {
                self.selected_track = track;
            }
            let mute = rect(119.0, 137.0 + track as f32 * 48.0, 40.0, 34.0);
            if hit(mute) {
                self.project.tracks[track].muted = !self.project.tracks[track].muted;
            }
        }
        for row in 0..PITCH_COUNT {
            for step in 0..composer::STEPS_PER_PATTERN {
                let bounds = grid_rect(step, row);
                if hit(bounds) {
                    self.selected_step = step;
                    self.toggle_note((LOW_PITCH as usize + PITCH_COUNT - 1 - row) as u8);
                }
            }
        }

        for (index, bounds) in waveform_buttons().iter().enumerate() {
            if hit(*bounds) {
                self.project.tracks[self.selected_track].instrument.waveform = WAVEFORMS[index];
            }
        }
        for (index, bounds) in envelope_buttons().iter().enumerate() {
            if hit(*bounds) {
                self.project.tracks[self.selected_track].instrument.envelope = ENVELOPES[index];
            }
        }
        let volume = rect(82.0, 535.0, 220.0, 22.0);
        let pan = rect(82.0, 575.0, 220.0, 22.0);
        if held && volume.check_collision_point_rec(mouse) {
            self.project.tracks[self.selected_track].volume =
                ((mouse.x - volume.x) / volume.width).clamp(0.0, 1.0) * 2.0;
        }
        if held && pan.check_collision_point_rec(mouse) {
            self.project.tracks[self.selected_track].pan =
                ((mouse.x - pan.x) / pan.width * 2.0 - 1.0).clamp(-1.0, 1.0);
        }

        if hit(rect(730.0, 150.0, 210.0, 38.0)) {
            self.root = (self.root + 1) % 12;
        }
        if hit(rect(730.0, 198.0, 210.0, 38.0)) {
            self.scale = (self.scale + 1) % SCALES.len();
        }
        if hit(rect(730.0, 246.0, 210.0, 38.0)) {
            self.quality = (self.quality + 1) % QUALITIES.len();
        }
        if hit(rect(730.0, 294.0, 150.0, 38.0)) {
            self.inversion = (self.inversion + 1) % 4;
        }
        if hit(rect(730.0, 342.0, 102.0, 40.0)) {
            self.add_chord();
        }
        if hit(rect(838.0, 342.0, 102.0, 40.0)) {
            self.progression();
        }
        if hit(rect(730.0, 390.0, 210.0, 40.0)) {
            self.apply_arp();
        }
    }

    fn draw(&self, d: &mut impl RaylibDraw) {
        d.clear_background(BG);
        text(d, "MIC FREQ COMPOSER", 14.0, 15.0, 25, INK);
        text(d, &self.project.name, 720.0, 18.0, 18, MUTED);

        button(d, rect(12.0, 55.0, 72.0, 38.0), "New", false);
        button(d, rect(90.0, 55.0, 72.0, 38.0), "Save", false);
        button(d, rect(168.0, 55.0, 105.0, 38.0), "Export WAV", false);
        button(
            d,
            rect(290.0, 55.0, 92.0, 38.0),
            if self.playback.is_playing() {
                "Stop"
            } else {
                "Play"
            },
            self.playback.is_playing(),
        );
        button(d, rect(388.0, 55.0, 40.0, 38.0), "-", false);
        text(
            d,
            &format!("{:.0}", self.project.tempo),
            431.0,
            101.0,
            15,
            MUTED,
        );
        button(d, rect(430.0, 55.0, 40.0, 38.0), "+", false);
        let device = self
            .devices
            .get(self.device)
            .cloned()
            .unwrap_or_else(|| "No output device".into());
        button(
            d,
            rect(485.0, 55.0, 222.0, 38.0),
            &format!("Device: {}", truncate(&device, 20)),
            false,
        );
        button(d, rect(713.0, 55.0, 137.0, 38.0), "Analyze Mix", false);
        button(d, rect(858.0, 55.0, 82.0, 38.0), "Help", self.help_open);

        panel(d, rect(12.0, 108.0, 150.0, 322.0), "TRACKS");
        for track in 0..TRACK_COUNT {
            let selected = track == self.selected_track;
            button(
                d,
                track_rect(track),
                &format!("{} {}", if selected { ">" } else { " " }, track + 1),
                selected,
            );
            text(
                d,
                &truncate(&self.project.tracks[track].name, 8),
                18.0,
                170.0 + track as f32 * 48.0,
                14,
                INK,
            );
            button(
                d,
                rect(119.0, 137.0 + track as f32 * 48.0, 40.0, 34.0),
                if self.project.tracks[track].muted {
                    "M"
                } else {
                    "On"
                },
                self.project.tracks[track].muted,
            );
        }

        panel(d, rect(170.0, 108.0, 542.0, 322.0), "PIANO ROLL  C4-B4");
        for step in 0..composer::STEPS_PER_PATTERN {
            text(
                d,
                &format!("{}", step + 1),
                195.0 + step as f32 * 32.0,
                130.0,
                12,
                if step == self.selected_step {
                    ACCENT
                } else {
                    MUTED
                },
            );
        }
        for row in 0..PITCH_COUNT {
            let pitch = (LOW_PITCH as usize + PITCH_COUNT - 1 - row) as u8;
            text(
                d,
                &composer::note_name(pitch),
                176.0,
                151.0 + row as f32 * 24.0,
                12,
                MUTED,
            );
            for step in 0..composer::STEPS_PER_PATTERN {
                let active = note_at(&self.project, self.selected_track, step, pitch);
                let bounds = grid_rect(step, row);
                d.draw_rectangle_rec(
                    bounds,
                    if active {
                        ACCENT
                    } else if step / 4 % 2 == 0 {
                        GRID_A
                    } else {
                        GRID_B
                    },
                );
                d.draw_rectangle_lines_ex(bounds, 1.0, OUTLINE);
                if step == self.selected_step && active {
                    d.draw_rectangle_lines_ex(bounds, 2.0, Color::WHITE);
                }
            }
        }

        panel(d, rect(12.0, 442.0, 700.0, 210.0), "SELECTED TRACK");
        text(d, "Waveform", 20.0, 474.0, 15, MUTED);
        for (index, bounds) in waveform_buttons().iter().enumerate() {
            button(
                d,
                *bounds,
                WAVE_LABELS[index],
                self.project.tracks[self.selected_track].instrument.waveform == WAVEFORMS[index],
            );
        }
        text(d, "Envelope", 20.0, 514.0, 15, MUTED);
        for (index, bounds) in envelope_buttons().iter().enumerate() {
            button(
                d,
                *bounds,
                ENVELOPE_LABELS[index],
                self.project.tracks[self.selected_track].instrument.envelope == ENVELOPES[index],
            );
        }
        text(d, "Volume", 20.0, 538.0, 15, MUTED);
        slider(
            d,
            rect(82.0, 535.0, 220.0, 22.0),
            self.project.tracks[self.selected_track].volume / 2.0,
        );
        text(
            d,
            &format!(
                "{:.0}%",
                self.project.tracks[self.selected_track].volume * 100.0
            ),
            310.0,
            538.0,
            14,
            INK,
        );
        text(d, "Pan", 20.0, 578.0, 15, MUTED);
        slider(
            d,
            rect(82.0, 575.0, 220.0, 22.0),
            (self.project.tracks[self.selected_track].pan + 1.0) * 0.5,
        );
        text(
            d,
            &format!("{:+.2}", self.project.tracks[self.selected_track].pan),
            310.0,
            578.0,
            14,
            INK,
        );
        let sample = self.project.tracks[self.selected_track]
            .instrument
            .sample_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Synth (drop sample to replace)");
        text(
            d,
            &format!("Source: {}", truncate(sample, 48)),
            370.0,
            542.0,
            15,
            MUTED,
        );
        text(d, &self.current_analysis(), 370.0, 578.0, 15, INK);
        text(
            d,
            "Keyboard: Space play  |  arrows select  |  Ctrl+S save  |  Esc close",
            20.0,
            625.0,
            14,
            MUTED,
        );
        let pitch = self
            .signal
            .midi_note
            .map(composer::note_name)
            .unwrap_or_else(|| "—".into());
        text(
            d,
            &format!(
                "Signal: {}  {:.1} Hz {:+.0}c  Harm {:.0}%  Perc {:.0}%  BPM {}",
                pitch,
                self.signal.fundamental_hz.unwrap_or(0.0),
                self.signal.cents,
                self.signal.harmonicity * 100.0,
                self.signal.percussive_ratio * 100.0,
                self.rhythm
                    .bpm
                    .map_or_else(|| "—".into(), |bpm| format!("{bpm:.0}"))
            ),
            370.0,
            610.0,
            13,
            INK,
        );

        panel(d, rect(720.0, 108.0, 228.0, 544.0), "THEORY");
        text(d, "Root (click to cycle)", 730.0, 132.0, 14, MUTED);
        button(
            d,
            rect(730.0, 150.0, 210.0, 38.0),
            &format!("Root: {}", NOTE_NAMES[self.root as usize]),
            false,
        );
        button(
            d,
            rect(730.0, 198.0, 210.0, 38.0),
            &format!("Scale: {:?}", SCALES[self.scale]),
            false,
        );
        button(
            d,
            rect(730.0, 246.0, 210.0, 38.0),
            &format!("Quality: {:?}", QUALITIES[self.quality]),
            false,
        );
        button(
            d,
            rect(730.0, 294.0, 150.0, 38.0),
            &format!("Inversion: {}", self.inversion),
            false,
        );
        button(d, rect(730.0, 342.0, 102.0, 40.0), "Add Chord", false);
        button(d, rect(838.0, 342.0, 102.0, 40.0), "I-IV-V-I", false);
        button(
            d,
            rect(730.0, 390.0, 210.0, 40.0),
            &format!("Arp: {:?}", ARPEGGIOS[self.arpeggio]),
            false,
        );
        text(d, "Drop files here", 730.0, 455.0, 18, ACCENT);
        text(
            d,
            "JSON project replaces this project.",
            730.0,
            482.0,
            14,
            MUTED,
        );
        text(d, "MIDI imports note patterns.", 730.0, 504.0, 14, MUTED);
        text(
            d,
            "WAV / OGG assigns selected track.",
            730.0,
            526.0,
            14,
            MUTED,
        );
        text(d, "Analysis uses every note on", 730.0, 566.0, 14, MUTED);
        text(d, "the selected track.", 730.0, 588.0, 14, MUTED);

        d.draw_rectangle_rec(
            rect(12.0, 664.0, 936.0, 44.0),
            if self.is_error { ERR_BG } else { PANEL },
        );
        text(
            d,
            &truncate(&self.status, 115),
            24.0,
            678.0,
            16,
            if self.is_error { WARNING } else { MUTED },
        );
        if self.help_open {
            draw_help(d);
        }
    }
}

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const SCALES: [Scale; 3] = [Scale::Major, Scale::NaturalMinor, Scale::Chromatic];
const QUALITIES: [ChordQuality; 7] = [
    ChordQuality::Major,
    ChordQuality::Minor,
    ChordQuality::Diminished,
    ChordQuality::Augmented,
    ChordQuality::Dominant7,
    ChordQuality::Major7,
    ChordQuality::Minor7,
];
const ARPEGGIOS: [ArpeggioPattern; 5] = [
    ArpeggioPattern::Up,
    ArpeggioPattern::Down,
    ArpeggioPattern::UpDown,
    ArpeggioPattern::DownUp,
    ArpeggioPattern::Random,
];
const WAVEFORMS: [Waveform; 5] = [
    Waveform::Sine,
    Waveform::Square,
    Waveform::Triangle,
    Waveform::Saw,
    Waveform::Noise,
];
const WAVE_LABELS: [&str; 5] = ["Sine", "Square", "Tri", "Saw", "Noise"];
const ENVELOPES: [Envelope; 3] = [
    Envelope {
        attack: 0.005,
        decay: 0.05,
        sustain: 0.7,
        release: 0.1,
    },
    Envelope {
        attack: 0.001,
        decay: 0.1,
        sustain: 0.2,
        release: 0.2,
    },
    Envelope {
        attack: 0.2,
        decay: 0.15,
        sustain: 0.8,
        release: 0.8,
    },
];
const ENVELOPE_LABELS: [&str; 3] = ["Default", "Pluck", "Pad"];

fn current_step_notes(project: &Project, track: usize, step: usize) -> Vec<u8> {
    match &project.tracks[track].patterns[0].steps[step] {
        Step::Rest => Vec::new(),
        Step::Notes { notes } => notes.iter().map(|note| note.midi).collect(),
    }
}

fn project_notes(project: &Project, track: usize) -> Vec<u8> {
    project.tracks[track]
        .patterns
        .iter()
        .flat_map(|pattern| &pattern.steps)
        .flat_map(|step| match step {
            Step::Rest => Vec::new(),
            Step::Notes { notes } => notes.iter().map(|note| note.midi).collect(),
        })
        .collect()
}

fn note_at(project: &Project, track: usize, step: usize, pitch: u8) -> bool {
    matches!(
        &project.tracks[track].patterns[0].steps[step],
        Step::Notes { notes } if notes.iter().any(|note| note.midi == pitch)
    )
}

fn track_rect(track: usize) -> Rectangle {
    rect(17.0, 135.0 + track as f32 * 48.0, 140.0, 38.0)
}

fn grid_rect(step: usize, row: usize) -> Rectangle {
    rect(
        198.0 + step as f32 * 32.0,
        147.0 + row as f32 * 24.0,
        31.0,
        24.0,
    )
}

fn waveform_buttons() -> [Rectangle; 5] {
    std::array::from_fn(|index| rect(82.0 + index as f32 * 65.0, 462.0, 60.0, 34.0))
}

fn envelope_buttons() -> [Rectangle; 3] {
    std::array::from_fn(|index| rect(82.0 + index as f32 * 82.0, 502.0, 77.0, 34.0))
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
    Rectangle::new(x, y, width, height)
}

const BG: Color = Color::new(15, 19, 27, 255);
const PANEL: Color = Color::new(29, 36, 49, 255);
const GRID_A: Color = Color::new(37, 45, 59, 255);
const GRID_B: Color = Color::new(42, 51, 67, 255);
const OUTLINE: Color = Color::new(64, 74, 91, 255);
const INK: Color = Color::new(231, 237, 245, 255);
const MUTED: Color = Color::new(151, 163, 180, 255);
const ACCENT: Color = Color::new(73, 202, 178, 255);
const WARNING: Color = Color::new(255, 190, 105, 255);
const ERR_BG: Color = Color::new(76, 35, 38, 255);

fn text(d: &mut impl RaylibDraw, label: &str, x: f32, y: f32, size: i32, color: Color) {
    d.draw_text(label, x as i32, y as i32, size, color);
}

fn panel(d: &mut impl RaylibDraw, bounds: Rectangle, title: &str) {
    d.draw_rectangle_rec(bounds, PANEL);
    text(d, title, bounds.x + 8.0, bounds.y + 5.0, 15, MUTED);
}

fn button(d: &mut impl RaylibDraw, bounds: Rectangle, label: &str, active: bool) {
    d.draw_rectangle_rec(
        bounds,
        if active {
            Color::new(37, 90, 83, 255)
        } else {
            Color::new(49, 61, 80, 255)
        },
    );
    d.draw_rectangle_lines_ex(bounds, 1.0, if active { ACCENT } else { OUTLINE });
    text(
        d,
        label,
        bounds.x + 6.0,
        bounds.y + (bounds.height - 15.0) * 0.5,
        15,
        if active { ACCENT } else { INK },
    );
}

fn slider(d: &mut impl RaylibDraw, bounds: Rectangle, value: f32) {
    d.draw_rectangle_rec(bounds, Color::new(20, 25, 34, 255));
    d.draw_rectangle(
        bounds.x as i32 + 2,
        bounds.y as i32 + 2,
        ((bounds.width - 4.0) * value.clamp(0.0, 1.0)) as i32,
        bounds.height as i32 - 4,
        ACCENT,
    );
    d.draw_rectangle_lines_ex(bounds, 1.0, OUTLINE);
}

fn truncate(value: &str, chars: usize) -> String {
    if value.chars().count() <= chars {
        value.into()
    } else {
        value
            .chars()
            .take(chars.saturating_sub(1))
            .chain(['…'])
            .collect()
    }
}

fn draw_help(d: &mut impl RaylibDraw) {
    d.draw_rectangle_rec(
        rect(0.0, 0.0, LOGICAL_WIDTH, LOGICAL_HEIGHT),
        Color::new(0, 0, 0, 190),
    );
    let bounds = rect(60.0, 120.0, 840.0, 450.0);
    d.draw_rectangle_rec(bounds, PANEL);
    d.draw_rectangle_lines_ex(bounds, 1.0, OUTLINE);
    text(d, "Composer Help", 80.0, 136.0, 20, INK);
    for (index, line) in [
        "Click a piano-roll cell to add or remove a note. Click a track to select it.",
        "Use Space or Play to audition the arrangement. Select an output device in the top row.",
        "New starts a project; Save stores JSON; Export WAV renders the full arrangement.",
        "Click waveform and envelope controls to shape the selected track.",
        "Root, Scale, Quality and Inversion configure Add Chord and the I-IV-V-I progression.",
        "Arp fills steps from the selected chord. Analyze Mix reports pitch, rhythm and chord/key.",
        "Drop JSON to open a project, MIDI to import patterns, or WAV/OGG to sample the selected track.",
        "Keyboard: arrows select track/step, Ctrl+S saves, Esc closes help (or the window).",
    ]
    .iter()
    .enumerate()
    {
        text(d, line, 80.0, 178.0 + index as f32 * 38.0, 15, INK);
    }
    button(d, rect(804.0, 520.0, 80.0, 32.0), "Close", false);
}

/// Open a complete composer window. `project_path` is loaded when supplied and
/// becomes the destination of subsequent saves and WAV exports.
pub fn run(project_path: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let (mut rl, thread) = raylib::init()
        .size(LOGICAL_WIDTH as i32, LOGICAL_HEIGHT as i32)
        .title("mic_freq - Composer")
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_window_min_size(LOGICAL_WIDTH as i32, LOGICAL_HEIGHT as i32);
    rl.set_exit_key(None);
    let mut state = UiState::new(project_path);

    while !rl.window_should_close() && !state.close {
        state.input(&mut rl);
        let geometry = ui_geometry(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        {
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::BLACK);
            let camera = Camera2D {
                offset: geometry.offset,
                target: Vector2::zero(),
                rotation: 0.0,
                zoom: geometry.scale,
            };
            let mut d = d.begin_mode2D(camera);
            state.draw(&mut d);
        }
    }
    state.stop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_geometry_is_exact_at_reference_size() {
        let geometry = ui_geometry(960.0, 720.0);
        assert_eq!(geometry.scale, 1.0);
        assert_eq!(geometry.offset, Vector2::zero());
        assert_eq!(
            geometry.to_logical(Vector2::new(123.0, 456.0)),
            Vector2::new(123.0, 456.0)
        );
    }

    #[test]
    fn geometry_letterboxes_and_preserves_aspect_ratio() {
        let geometry = ui_geometry(1920.0, 720.0);
        assert_eq!(geometry.scale, 1.0);
        assert_eq!(geometry.offset, Vector2::new(480.0, 0.0));
        assert_eq!(
            geometry.to_logical(Vector2::new(480.0, 0.0)),
            Vector2::zero()
        );
    }

    #[test]
    fn geometry_scales_down_without_inverting_or_panicking() {
        let geometry = ui_geometry(480.0, 360.0);
        assert_eq!(geometry.scale, 0.5);
        assert_eq!(
            geometry.to_logical(Vector2::new(240.0, 180.0)),
            Vector2::new(480.0, 360.0)
        );
    }

    #[test]
    fn grid_contains_sixteen_by_twelve_non_overlapping_cells() {
        let first = grid_rect(0, 0);
        let last = grid_rect(15, 11);
        assert_eq!(first.width, 31.0);
        assert_eq!(first.height, 24.0);
        assert!(first.width >= 24.0 && first.height >= 24.0);
        assert!((last.x + last.width) <= 712.0);
        assert!((last.y + last.height) <= 435.0);
    }
}

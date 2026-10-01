//! Built-in music player: playlist logic (shuffle / repeat / queue), a
//! raylib audio-stream backend with a live level visualizer, and the
//! player bar + playlist panel UI.
//!
//! The raw raylib ffi is wrapped in an owning [`Player`] because the safe
//! raylib-rs `Music<'aud>` borrows the audio device, which does not fit an
//! app that lazily opens audio and keeps the player for its whole lifetime.
//! Call [`Player::shutdown`] before the window closes.

use std::{
    ffi::{c_uint, c_void, CString},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, AtomicUsize, Ordering},
};

use raylib::{ffi, prelude::*};
use serde::{Deserialize, Serialize};

use crate::ui;

/// Extensions the built-in player decodes; other audio opens in the
/// system player.
pub const PLAYABLE: &[&str] = &["wav", "ogg", "mp3", "flac", "qoa", "xm", "mod"];

pub fn is_playable(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| PLAYABLE.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    Off,
    #[default]
    All,
    One,
}

impl Repeat {
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::All => "all",
            Self::One => "one",
        }
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn parse(text: &str) -> Option<Self> {
        match text
            .trim()
            .trim_start_matches(':')
            .to_ascii_lowercase()
            .as_str()
        {
            "off" | "none" | "false" => Some(Self::Off),
            "all" | "true" => Some(Self::All),
            "one" | "track" | "single" => Some(Self::One),
            _ => None,
        }
    }
}

/// Track list plus play order. Pure logic (no audio) so it is unit-tested.
#[derive(Clone, Debug)]
pub struct Playlist {
    tracks: Vec<PathBuf>,
    /// Track indices in play order (identity, or shuffled).
    order: Vec<usize>,
    /// Index into `order` of the current track.
    position: Option<usize>,
    shuffle: bool,
    pub repeat: Repeat,
    seed: u64,
}

impl Default for Playlist {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|time| time.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self::with_seed(seed)
    }
}

impl Playlist {
    pub fn with_seed(seed: u64) -> Self {
        Self {
            tracks: Vec::new(),
            order: Vec::new(),
            position: None,
            shuffle: false,
            repeat: Repeat::All,
            seed: seed | 1,
        }
    }

    pub fn tracks(&self) -> &[PathBuf] {
        &self.tracks
    }

    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// Index (into [`tracks`](Self::tracks)) of the current track.
    pub fn current(&self) -> Option<usize> {
        self.position.map(|position| self.order[position])
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current().map(|index| self.tracks[index].as_path())
    }

    /// Replaces the list and makes `start` current. With shuffle on the
    /// start track plays first and the rest follow in random order.
    pub fn set(&mut self, tracks: Vec<PathBuf>, start: usize) {
        self.tracks = tracks;
        if self.tracks.is_empty() {
            self.order.clear();
            self.position = None;
            return;
        }
        let start = start.min(self.tracks.len() - 1);
        self.rebuild_order(start);
    }

    /// Appends a track; returns its index. Becomes current if the list was
    /// empty.
    pub fn enqueue(&mut self, path: PathBuf) -> usize {
        self.tracks.push(path);
        let index = self.tracks.len() - 1;
        self.order.push(index);
        if self.position.is_none() {
            self.position = Some(self.order.len() - 1);
        }
        index
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.order.clear();
        self.position = None;
    }

    pub fn set_shuffle(&mut self, shuffle: bool) {
        if self.shuffle == shuffle {
            return;
        }
        self.shuffle = shuffle;
        if let Some(current) = self.current() {
            self.rebuild_order(current);
        } else {
            self.order = (0..self.tracks.len()).collect();
        }
    }

    /// Makes track `index` current.
    pub fn jump(&mut self, index: usize) -> Option<usize> {
        let position = self.order.iter().position(|&track| track == index)?;
        self.position = Some(position);
        Some(index)
    }

    /// Moves to the next track. `manual` is a user "next" (repeat-one does
    /// not hold it on the same track). Returns the new current track, or
    /// `None` when playback should stop.
    pub fn advance(&mut self, manual: bool) -> Option<usize> {
        let position = self.position?;
        if !manual && self.repeat == Repeat::One {
            return self.current();
        }
        if position + 1 < self.order.len() {
            self.position = Some(position + 1);
        } else if self.repeat != Repeat::Off {
            if self.shuffle && self.order.len() > 2 {
                // Fresh order each lap, never repeating the last track first.
                let last = self.order[position];
                self.shuffle_order();
                if self.order[0] == last {
                    let end = self.order.len() - 1;
                    self.order.swap(0, end);
                }
            }
            self.position = Some(0);
        } else {
            return None;
        }
        self.current()
    }

    /// Moves to the previous track (wrapping when repeat is on).
    pub fn back(&mut self) -> Option<usize> {
        let position = self.position?;
        if position > 0 {
            self.position = Some(position - 1);
        } else if self.repeat != Repeat::Off {
            self.position = Some(self.order.len() - 1);
        }
        self.current()
    }

    /// Upcoming track indices in play order (after the current one).
    pub fn upcoming(&self) -> Vec<usize> {
        match self.position {
            Some(position) => self.order[position + 1..].to_vec(),
            None => self.order.clone(),
        }
    }

    fn rebuild_order(&mut self, current: usize) {
        self.order = (0..self.tracks.len()).collect();
        if self.shuffle {
            self.shuffle_order();
            let at = self
                .order
                .iter()
                .position(|&track| track == current)
                .unwrap_or(0);
            self.order.swap(0, at);
            self.position = Some(0);
        } else {
            self.position = Some(current);
        }
    }

    fn shuffle_order(&mut self) {
        for index in (1..self.order.len()).rev() {
            let pick = (self.next_random() % (index as u64 + 1)) as usize;
            self.order.swap(index, pick);
        }
    }

    fn next_random(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.seed;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.seed = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

const LEVEL_COUNT: usize = 96;
static LEVELS: [AtomicU32; LEVEL_COUNT] = [const { AtomicU32::new(0) }; LEVEL_COUNT];
static LEVEL_HEAD: AtomicUsize = AtomicUsize::new(0);
static CHANNELS: AtomicU32 = AtomicU32::new(2);

/// Audio-thread processor: records the peak of each quarter of the buffer.
/// raylib hands processors interleaved 32-bit float samples.
unsafe extern "C" fn level_processor(buffer: *mut c_void, frames: c_uint) {
    if buffer.is_null() || frames == 0 {
        return;
    }
    let channels = CHANNELS.load(Ordering::Relaxed).max(1) as usize;
    let samples =
        unsafe { std::slice::from_raw_parts(buffer as *const f32, frames as usize * channels) };
    for chunk in samples.chunks(samples.len().div_ceil(4).max(1)) {
        let peak = chunk
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        let head = LEVEL_HEAD.fetch_add(1, Ordering::Relaxed) % LEVEL_COUNT;
        LEVELS[head].store(peak.min(1.0).to_bits(), Ordering::Relaxed);
    }
}

/// Recent output levels, oldest first, each 0.0..=1.0.
pub fn levels() -> Vec<f32> {
    let head = LEVEL_HEAD.load(Ordering::Relaxed);
    (0..LEVEL_COUNT)
        .map(|offset| f32::from_bits(LEVELS[(head + offset) % LEVEL_COUNT].load(Ordering::Relaxed)))
        .collect()
}

fn reset_levels() {
    for level in &LEVELS {
        level.store(0, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Stopped,
    Playing,
    Paused,
}

/// Snapshot of the player for scripts and the info line.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(not(feature = "scripting"), allow(dead_code))]
pub struct Snapshot {
    pub track: Option<String>,
    pub state: &'static str,
    pub position: f64,
    pub length: f64,
    pub volume: f64,
    pub shuffle: bool,
    pub repeat: &'static str,
    pub tracks: Vec<String>,
    pub index: Option<usize>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            track: None,
            state: "stopped",
            position: 0.0,
            length: 0.0,
            volume: 0.8,
            shuffle: false,
            repeat: Repeat::default().name(),
            tracks: Vec::new(),
            index: None,
        }
    }
}

pub struct Player {
    audio_ready: bool,
    music: Option<ffi::Music>,
    pub playlist: Playlist,
    state: State,
    volume: f32,
    length: f32,
    /// Set when a new track starts; drained by the app to fire `:track`.
    changed: Option<PathBuf>,
    /// Player bar shown at the bottom of the window.
    pub visible: bool,
    pub playlist_open: bool,
    pub playlist_scroll: usize,
}

impl Player {
    pub fn new(volume: f32, shuffle: bool, repeat: Repeat) -> Self {
        let mut playlist = Playlist::default();
        playlist.set_shuffle(shuffle);
        playlist.repeat = repeat;
        Self {
            audio_ready: false,
            music: None,
            playlist,
            state: State::Stopped,
            volume: volume.clamp(0.0, 1.0),
            length: 0.0,
            changed: None,
            visible: false,
            playlist_open: false,
            playlist_scroll: 0,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    pub fn position(&self) -> f32 {
        match &self.music {
            Some(music) => unsafe { ffi::GetMusicTimePlayed(*music) },
            None => 0.0,
        }
    }

    pub fn has_track(&self) -> bool {
        self.music.is_some()
    }

    pub fn take_changed(&mut self) -> Option<PathBuf> {
        self.changed.take()
    }

    /// Replaces the playlist and starts `start`.
    pub fn play_list(&mut self, tracks: Vec<PathBuf>, start: usize) -> Result<(), String> {
        self.playlist.set(tracks, start);
        self.playlist_scroll = 0;
        self.visible = true;
        self.start_current_or_skip(true)
    }

    /// Adds a track to the end of the playlist (starting it if idle).
    pub fn enqueue(&mut self, path: PathBuf) -> Result<(), String> {
        if !is_playable(&path) {
            return Err(format!("{} is not a playable audio file", path.display()));
        }
        let idle = self.playlist.is_empty() || self.state == State::Stopped && !self.has_track();
        let index = self.playlist.enqueue(path);
        self.visible = true;
        if idle {
            self.playlist.jump(index);
            self.start_current_or_skip(true)
        } else {
            Ok(())
        }
    }

    pub fn jump(&mut self, index: usize) -> Result<(), String> {
        if self.playlist.jump(index).is_none() {
            return Err(format!("no track #{}", index + 1));
        }
        self.start_current_or_skip(true)
    }

    pub fn toggle(&mut self) -> Result<(), String> {
        match self.state {
            State::Playing => self.pause(),
            State::Paused => self.resume(),
            State::Stopped if self.has_track() => self.play_current(),
            State::Stopped if !self.playlist.is_empty() => return self.start_current_or_skip(true),
            State::Stopped => return Err("nothing to play: open an audio cell first".to_owned()),
        }
        Ok(())
    }

    pub fn pause(&mut self) {
        if let (Some(music), State::Playing) = (&self.music, self.state) {
            unsafe { ffi::PauseMusicStream(*music) };
            self.state = State::Paused;
        }
    }

    pub fn resume(&mut self) {
        match (&self.music, self.state) {
            (Some(music), State::Paused) => {
                unsafe { ffi::ResumeMusicStream(*music) };
                self.state = State::Playing;
            }
            (Some(_), State::Stopped) => self.play_current(),
            _ => {}
        }
    }

    fn play_current(&mut self) {
        if let Some(music) = &self.music {
            unsafe { ffi::PlayMusicStream(*music) };
            self.state = State::Playing;
        }
    }

    pub fn stop(&mut self) {
        if let Some(music) = &self.music {
            unsafe { ffi::StopMusicStream(*music) };
        }
        self.state = State::Stopped;
        reset_levels();
    }

    pub fn next(&mut self) -> Result<(), String> {
        match self.playlist.advance(true) {
            Some(_) => self.start_current_or_skip(true),
            None => {
                self.stop();
                Ok(())
            }
        }
    }

    /// Restarts the track if more than 3 s in, else goes to the previous.
    pub fn previous(&mut self) -> Result<(), String> {
        if self.position() > 3.0 {
            self.seek(0.0);
            return Ok(());
        }
        if self.playlist.back().is_some() {
            self.start_current_or_skip(false)
        } else {
            Ok(())
        }
    }

    pub fn seek(&mut self, seconds: f32) {
        if let Some(music) = &self.music {
            let target = seconds.clamp(0.0, (self.length - 0.05).max(0.0));
            unsafe { ffi::SeekMusicStream(*music, target) };
        }
    }

    pub fn seek_by(&mut self, delta: f32) {
        let position = self.position();
        self.seek(position + delta);
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(music) = &self.music {
            unsafe { ffi::SetMusicVolume(*music, self.volume) };
        }
    }

    pub fn set_shuffle(&mut self, shuffle: bool) {
        self.playlist.set_shuffle(shuffle);
    }

    /// Feeds the audio stream; call every frame. Advances on track end.
    pub fn update(&mut self) -> Result<(), String> {
        let Some(music) = self.music else {
            return Ok(());
        };
        if self.state != State::Playing {
            return Ok(());
        }
        unsafe { ffi::UpdateMusicStream(music) };
        if unsafe { ffi::IsMusicStreamPlaying(music) } {
            return Ok(());
        }
        match self.playlist.advance(false) {
            Some(_) => self.start_current_or_skip(true),
            None => {
                self.stop();
                Ok(())
            }
        }
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            track: self
                .playlist
                .current_path()
                .map(|path| path.display().to_string()),
            state: match self.state {
                State::Stopped => "stopped",
                State::Playing => "playing",
                State::Paused => "paused",
            },
            position: self.position() as f64,
            length: self.length as f64,
            volume: self.volume as f64,
            shuffle: self.playlist.shuffle(),
            repeat: self.playlist.repeat.name(),
            tracks: self
                .playlist
                .tracks()
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            index: self.playlist.current(),
        }
    }

    /// Starts the current track; on a decode failure tries the following
    /// tracks (forward or backward) before giving up.
    fn start_current_or_skip(&mut self, forward: bool) -> Result<(), String> {
        let mut first_error = None;
        for _ in 0..self.playlist.len().max(1) {
            let Some(path) = self.playlist.current_path().map(Path::to_path_buf) else {
                self.stop();
                break;
            };
            match self.load(&path) {
                Ok(()) => {
                    return match first_error {
                        Some(error) => Err(format!("{error} (skipped)")),
                        None => Ok(()),
                    };
                }
                Err(error) => {
                    first_error.get_or_insert(error);
                    let moved = if forward {
                        self.playlist.advance(true)
                    } else {
                        self.playlist.back()
                    };
                    if moved.is_none() {
                        break;
                    }
                }
            }
        }
        self.unload();
        self.state = State::Stopped;
        Err(first_error.unwrap_or_else(|| "playlist is empty".to_owned()))
    }

    fn ensure_audio(&mut self) -> Result<(), String> {
        if !self.audio_ready {
            unsafe { ffi::InitAudioDevice() };
            if !unsafe { ffi::IsAudioDeviceReady() } {
                return Err("no audio output device is available".to_owned());
            }
            self.audio_ready = true;
        }
        Ok(())
    }

    fn load(&mut self, path: &Path) -> Result<(), String> {
        self.ensure_audio()?;
        self.unload();
        if !path.is_file() {
            return Err(format!("missing file {}", path.display()));
        }
        if !is_playable(path) {
            return Err(format!(
                "{} is not a format the built-in player decodes",
                path.display()
            ));
        }
        let c_path = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| format!("bad path {}", path.display()))?;
        let mut music = unsafe { ffi::LoadMusicStream(c_path.as_ptr()) };
        if !unsafe { ffi::IsMusicValid(music) } {
            return Err(format!("could not decode {}", path.display()));
        }
        music.looping = false;
        CHANNELS.store(music.stream.channels, Ordering::Relaxed);
        reset_levels();
        unsafe {
            ffi::AttachAudioStreamProcessor(music.stream, Some(level_processor));
            ffi::SetMusicVolume(music, self.volume);
            ffi::PlayMusicStream(music);
            self.length = ffi::GetMusicTimeLength(music);
        }
        self.music = Some(music);
        self.state = State::Playing;
        self.changed = Some(path.to_path_buf());
        Ok(())
    }

    fn unload(&mut self) {
        if let Some(music) = self.music.take() {
            unsafe {
                ffi::StopMusicStream(music);
                ffi::DetachAudioStreamProcessor(music.stream, Some(level_processor));
                ffi::UnloadMusicStream(music);
            }
        }
        self.length = 0.0;
    }

    /// Stops playback and empties the playlist.
    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn clear(&mut self) {
        self.unload();
        self.state = State::Stopped;
        self.playlist.clear();
        self.playlist_scroll = 0;
        reset_levels();
    }

    /// Frees the stream and closes the audio device. Must run before the
    /// raylib window is dropped.
    pub fn shutdown(&mut self) {
        self.unload();
        self.state = State::Stopped;
        if self.audio_ready {
            unsafe { ffi::CloseAudioDevice() };
            self.audio_ready = false;
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn format_time(seconds: f32) -> String {
    let total = if seconds.is_finite() {
        seconds.max(0.0) as u64
    } else {
        0
    };
    if total >= 3600 {
        format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
    } else {
        format!("{}:{:02}", total / 60, total % 60)
    }
}

pub fn track_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("track")
        .to_owned()
}

// ---------------------------------------------------------------- UI ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Previous,
    PlayPause,
    Next,
    Stop,
    Shuffle,
    Repeat,
    Playlist,
    Close,
}

impl Control {
    pub fn tooltip(self) -> &'static str {
        match self {
            Self::Previous => "Previous track, or restart this one if more than 3 s in ([).",
            Self::PlayPause => "Play / pause (Space).",
            Self::Next => "Next track (]).",
            Self::Stop => "Stop playback.",
            Self::Shuffle => "Shuffle the play order.",
            Self::Repeat => "Repeat: off -> all -> one.",
            Self::Playlist => "Show the playlist (P); click a track to play it.",
            Self::Close => "Stop and hide the player.",
        }
    }
}

pub struct BarLayout {
    pub frame: Rectangle,
    pub buttons: Vec<(Control, Rectangle, String)>,
    pub info: Rectangle,
    pub seek: Rectangle,
    pub volume: Rectangle,
    pub font: f32,
    pub scale: f32,
}

pub fn bar_height(screen_width: i32, screen_height: i32) -> f32 {
    let scale = ui::scale(screen_width, screen_height);
    72.0 * scale
}

pub fn bar_layout(player: &Player, screen_width: i32, screen_height: i32) -> BarLayout {
    let scale = ui::scale(screen_width, screen_height);
    let height = bar_height(screen_width, screen_height);
    let width = screen_width as f32;
    let frame = Rectangle::new(0.0, screen_height as f32 - height, width, height);
    let font = 16.0 * scale;
    let pad = 8.0 * scale;
    let icon = 40.0 * scale;
    let icon_y = frame.y + (height - icon) / 2.0;

    let mut buttons = Vec::new();
    let mut x = pad;
    for control in [
        Control::Previous,
        Control::PlayPause,
        Control::Next,
        Control::Stop,
    ] {
        buttons.push((
            control,
            Rectangle::new(x, icon_y, icon, icon),
            String::new(),
        ));
        x += icon + pad * 0.6;
    }
    let left_end = x + pad;

    let text_buttons = [
        (
            Control::Shuffle,
            format!(
                "Shuffle {}",
                if player.playlist.shuffle() {
                    "on"
                } else {
                    "off"
                }
            ),
        ),
        (
            Control::Repeat,
            format!("Repeat {}", player.playlist.repeat.name()),
        ),
        (
            Control::Playlist,
            format!("List ({})", player.playlist.len()),
        ),
    ];
    let close = icon * 0.8;
    let volume_width = (120.0 * scale).min(width * 0.12);
    let mut right = width - pad - close;
    let close_rect = Rectangle::new(right, frame.y + (height - close) / 2.0, close, close);
    right -= pad + volume_width;
    let volume = Rectangle::new(
        right,
        frame.y + height / 2.0 - 4.0 * scale,
        volume_width,
        8.0 * scale,
    );
    right -= pad * 2.0;
    let mut text_rects = Vec::new();
    for (control, label) in text_buttons.into_iter().rev() {
        let button_width = ui::measure(&label, font) + pad * 2.0;
        right -= button_width;
        text_rects.push((
            control,
            Rectangle::new(right, icon_y + icon * 0.15, button_width, icon * 0.7),
            label,
        ));
        right -= pad * 0.6;
    }
    // Narrow windows drop the text buttons (keys still work) before the
    // seek bar gets too small to use.
    if right - left_end >= 160.0 * scale {
        buttons.extend(text_rects.into_iter().rev());
    } else {
        right = width - pad - close - pad - volume_width - pad * 2.0;
    }
    buttons.push((Control::Close, close_rect, String::new()));

    let info = Rectangle::new(
        left_end,
        frame.y + pad * 0.6,
        (right - left_end).max(0.0),
        height - pad * 1.2,
    );
    let seek = Rectangle::new(
        info.x,
        info.y + info.height - 12.0 * scale,
        info.width,
        8.0 * scale,
    );
    BarLayout {
        frame,
        buttons,
        info,
        seek,
        volume,
        font,
        scale,
    }
}

/// Fraction 0..=1 of `rect`'s width under the mouse.
pub fn slider_fraction(rect: Rectangle, mouse: Vector2) -> f32 {
    if rect.width <= 0.0 {
        return 0.0;
    }
    ((mouse.x - rect.x) / rect.width).clamp(0.0, 1.0)
}

/// Grows a thin slider's hit area so it is easy to grab.
pub fn slider_hit(rect: Rectangle, scale: f32) -> Rectangle {
    let grow = 10.0 * scale;
    Rectangle::new(
        rect.x - 4.0,
        rect.y - grow,
        rect.width + 8.0,
        rect.height + grow * 2.0,
    )
}

const BAR_BG: Color = Color::new(16, 16, 24, 240);
const ACCENT: Color = Color::new(160, 110, 230, 255);
const TRACK_BG: Color = Color::new(55, 55, 70, 255);
const TEXT: Color = Color::new(235, 235, 240, 255);
const DIM: Color = Color::new(150, 150, 170, 255);
const HOVER: Color = Color::new(70, 70, 92, 255);
const BUTTON: Color = Color::new(42, 42, 56, 255);

pub fn draw_bar<D: RaylibDraw>(
    d: &mut D,
    player: &Player,
    layout: &BarLayout,
    mouse: Vector2,
    seek_preview: Option<f32>,
) {
    let scale = layout.scale;
    d.draw_rectangle_rec(layout.frame, BAR_BG);
    d.draw_line_ex(
        Vector2::new(0.0, layout.frame.y),
        Vector2::new(layout.frame.width, layout.frame.y),
        2.0 * scale,
        ACCENT,
    );

    // Visualizer behind the track info.
    let levels = levels();
    let info = layout.info;
    if player.state() == State::Playing && info.width > 20.0 {
        let bar_count = levels.len();
        let slot = info.width / bar_count as f32;
        for (index, level) in levels.iter().enumerate() {
            let bar_height = (level.sqrt() * info.height * 0.9).max(1.0);
            let x = info.x + index as f32 * slot;
            d.draw_rectangle_rec(
                Rectangle::new(
                    x,
                    info.y + info.height - bar_height,
                    (slot - 1.0).max(1.0),
                    bar_height,
                ),
                Color::new(160, 110, 230, 45 + (level * 70.0) as u8),
            );
        }
    }

    for (control, rect, label) in &layout.buttons {
        let hovered = point_in(mouse, *rect);
        let active = match control {
            Control::Shuffle => player.playlist.shuffle(),
            Control::Repeat => player.playlist.repeat != Repeat::Off,
            Control::Playlist => player.playlist_open,
            _ => false,
        };
        let bg = if hovered {
            HOVER
        } else if active {
            Color::new(80, 55, 120, 255)
        } else {
            BUTTON
        };
        d.draw_rectangle_rec(*rect, bg);
        if label.is_empty() {
            draw_icon(d, *control, *rect, player.state() == State::Playing);
        } else {
            ui::draw_text_centered(d, label, *rect, layout.font * 0.92, TEXT);
        }
    }

    let title = match player.playlist.current_path() {
        Some(path) => {
            let index = player.playlist.current().unwrap_or(0) + 1;
            format!("{index}/{}  {}", player.playlist.len(), track_name(path))
        }
        None => "No track — open an audio cell or drop music on the grid".to_owned(),
    };
    let state = match player.state() {
        State::Playing => "",
        State::Paused => "  [paused]",
        State::Stopped => "  [stopped]",
    };
    let position = seek_preview
        .map(|fraction| fraction * player.length())
        .unwrap_or_else(|| player.position());
    let time = format!(
        "{} / {}",
        format_time(position),
        format_time(player.length())
    );
    let time_width = ui::measure(&time, layout.font);
    let title_width = (info.width - time_width - 12.0 * scale).max(0.0);
    ui::draw_text(
        d,
        &ui::fit(&format!("{title}{state}"), layout.font, title_width),
        info.x,
        info.y + 4.0 * scale,
        layout.font,
        TEXT,
    );
    ui::draw_text(
        d,
        &time,
        info.x + info.width - time_width,
        info.y + 4.0 * scale,
        layout.font,
        DIM,
    );

    let seek = layout.seek;
    d.draw_rectangle_rec(seek, TRACK_BG);
    let fraction = if player.length() > 0.0 {
        (position / player.length()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    d.draw_rectangle_rec(
        Rectangle::new(seek.x, seek.y, seek.width * fraction, seek.height),
        ACCENT,
    );
    if player.has_track() {
        d.draw_circle_v(
            Vector2::new(seek.x + seek.width * fraction, seek.y + seek.height / 2.0),
            seek.height * 0.95,
            TEXT,
        );
    }

    let volume = layout.volume;
    d.draw_rectangle_rec(volume, TRACK_BG);
    d.draw_rectangle_rec(
        Rectangle::new(
            volume.x,
            volume.y,
            volume.width * player.volume(),
            volume.height,
        ),
        Color::new(110, 190, 140, 255),
    );
    d.draw_circle_v(
        Vector2::new(
            volume.x + volume.width * player.volume(),
            volume.y + volume.height / 2.0,
        ),
        volume.height * 0.95,
        TEXT,
    );
    let label = format!("Vol {:.0}%", player.volume() * 100.0);
    let size = layout.font * 0.8;
    ui::draw_text(
        d,
        &label,
        volume.x,
        volume.y - size - 6.0 * scale,
        size,
        DIM,
    );
}

fn draw_icon<D: RaylibDraw>(d: &mut D, control: Control, rect: Rectangle, playing: bool) {
    let cx = rect.x + rect.width / 2.0;
    let cy = rect.y + rect.height / 2.0;
    let r = rect.width * 0.26;
    let triangle = |d: &mut D, x: f32, dir: f32| {
        // raylib wants counter-clockwise vertices.
        let tip = Vector2::new(x + r * dir, cy);
        let top = Vector2::new(x - r * 0.8 * dir, cy - r);
        let bottom = Vector2::new(x - r * 0.8 * dir, cy + r);
        if dir > 0.0 {
            d.draw_triangle(top, bottom, tip, TEXT);
        } else {
            d.draw_triangle(top, tip, bottom, TEXT);
        }
    };
    let bar = |d: &mut D, x: f32| {
        d.draw_rectangle_rec(
            Rectangle::new(x - r * 0.18, cy - r, r * 0.36, r * 2.0),
            TEXT,
        );
    };
    match control {
        Control::PlayPause if playing => {
            bar(d, cx - r * 0.45);
            bar(d, cx + r * 0.45);
        }
        Control::PlayPause => triangle(d, cx + r * 0.15, 1.0),
        Control::Next => {
            triangle(d, cx - r * 0.2, 1.0);
            bar(d, cx + r * 0.95);
        }
        Control::Previous => {
            triangle(d, cx + r * 0.2, -1.0);
            bar(d, cx - r * 0.95);
        }
        Control::Stop => {
            d.draw_rectangle_rec(
                Rectangle::new(cx - r * 0.8, cy - r * 0.8, r * 1.6, r * 1.6),
                TEXT,
            );
        }
        Control::Close => {
            let k = r * 0.8;
            d.draw_line_ex(
                Vector2::new(cx - k, cy - k),
                Vector2::new(cx + k, cy + k),
                2.5,
                TEXT,
            );
            d.draw_line_ex(
                Vector2::new(cx - k, cy + k),
                Vector2::new(cx + k, cy - k),
                2.5,
                TEXT,
            );
        }
        _ => {}
    }
}

pub struct PlaylistLayout {
    pub frame: Rectangle,
    pub rows: Vec<(usize, Rectangle)>,
    pub header: Rectangle,
    pub font: f32,
    pub visible_rows: usize,
}

pub fn playlist_layout(
    player: &Player,
    screen_width: i32,
    screen_height: i32,
    top: f32,
) -> PlaylistLayout {
    let scale = ui::scale(screen_width, screen_height);
    let font = 16.0 * scale;
    let row_height = font + 10.0 * scale;
    let bottom = screen_height as f32 - bar_height(screen_width, screen_height) - 6.0 * scale;
    let width = (440.0 * scale).min(screen_width as f32 * 0.5);
    let available = (bottom - top - 10.0 * scale).max(row_height * 2.0);
    let visible_rows = (((available - row_height) / row_height).floor() as usize)
        .clamp(1, player.playlist.len().max(1));
    let height = row_height * (visible_rows + 1) as f32 + 6.0 * scale;
    let frame = Rectangle::new(
        screen_width as f32 - width - 8.0 * scale,
        bottom - height,
        width,
        height,
    );
    let header = Rectangle::new(frame.x, frame.y, frame.width, row_height);
    let scroll = player
        .playlist_scroll
        .min(player.playlist.len().saturating_sub(visible_rows));
    let rows = (scroll..player.playlist.len())
        .take(visible_rows)
        .enumerate()
        .map(|(slot, index)| {
            (
                index,
                Rectangle::new(
                    frame.x,
                    frame.y + row_height * (slot + 1) as f32,
                    frame.width,
                    row_height,
                ),
            )
        })
        .collect();
    PlaylistLayout {
        frame,
        rows,
        header,
        font,
        visible_rows,
    }
}

pub fn draw_playlist<D: RaylibDraw>(
    d: &mut D,
    player: &Player,
    layout: &PlaylistLayout,
    mouse: Vector2,
) {
    d.draw_rectangle_rec(layout.frame, BAR_BG);
    d.draw_rectangle_lines_ex(layout.frame, 1.0, ACCENT);
    let header = format!(
        "Playlist  {} track(s)  shuffle {}  repeat {}",
        player.playlist.len(),
        if player.playlist.shuffle() {
            "on"
        } else {
            "off"
        },
        player.playlist.repeat.name()
    );
    ui::draw_text_in(
        d,
        &ui::fit(&header, layout.font * 0.9, layout.header.width - 16.0),
        layout.header,
        8.0,
        layout.font * 0.9,
        DIM,
    );
    let current = player.playlist.current();
    let next = player.playlist.upcoming().first().copied();
    for (index, rect) in &layout.rows {
        if Some(*index) == current {
            d.draw_rectangle_rec(*rect, Color::new(80, 55, 120, 255));
        } else if point_in(mouse, *rect) {
            d.draw_rectangle_rec(*rect, HOVER);
        }
        let marker = if Some(*index) == current {
            ">"
        } else if Some(*index) == next {
            "›"
        } else {
            " "
        };
        let text = format!(
            "{marker} {:>3}. {}",
            index + 1,
            track_name(&player.playlist.tracks()[*index])
        );
        ui::draw_text_in(
            d,
            &ui::fit(&text, layout.font, rect.width - 16.0),
            *rect,
            8.0,
            layout.font,
            TEXT,
        );
    }
}

fn point_in(point: Vector2, rect: Rectangle) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracks(count: usize) -> Vec<PathBuf> {
        (0..count)
            .map(|i| PathBuf::from(format!("{i}.mp3")))
            .collect()
    }

    #[test]
    fn playable_formats() {
        assert!(is_playable(Path::new("a.MP3")));
        assert!(is_playable(Path::new("dir/b.flac")));
        assert!(is_playable(Path::new("c.xm")));
        assert!(!is_playable(Path::new("d.m4a")));
        assert!(!is_playable(Path::new("noext")));
    }

    #[test]
    fn sequential_play_respects_repeat() {
        let mut list = Playlist::with_seed(7);
        list.repeat = Repeat::Off;
        list.set(tracks(3), 1);
        assert_eq!(list.current(), Some(1));
        assert_eq!(list.advance(false), Some(2));
        assert_eq!(list.advance(false), None, "repeat off stops at the end");
        assert_eq!(list.current(), Some(2));
        assert_eq!(list.back(), Some(1));
        assert_eq!(list.back(), Some(0));
        assert_eq!(list.back(), Some(0), "no wrap with repeat off");

        list.repeat = Repeat::All;
        list.jump(2);
        assert_eq!(list.advance(false), Some(0), "repeat all wraps");
        assert_eq!(list.back(), Some(2));

        list.repeat = Repeat::One;
        assert_eq!(
            list.advance(false),
            Some(2),
            "repeat one holds on track end"
        );
        assert_eq!(list.advance(true), Some(0), "a manual next still moves on");
    }

    #[test]
    fn shuffle_visits_every_track_once_per_lap() {
        let mut list = Playlist::with_seed(42);
        list.set_shuffle(true);
        list.set(tracks(10), 4);
        assert_eq!(list.current(), Some(4), "the chosen track plays first");
        let mut seen = vec![4];
        for _ in 0..9 {
            seen.push(list.advance(false).unwrap());
        }
        let mut sorted = seen.clone();
        sorted.sort();
        assert_eq!(sorted, (0..10).collect::<Vec<_>>());
        let first_of_next_lap = list.advance(false).unwrap();
        assert_ne!(first_of_next_lap, *seen.last().unwrap());

        list.set_shuffle(false);
        let current = list.current().unwrap();
        assert_eq!(
            list.upcoming(),
            ((current + 1)..10).collect::<Vec<_>>(),
            "unshuffling continues in file order"
        );
    }

    #[test]
    fn enqueue_and_clear() {
        let mut list = Playlist::with_seed(1);
        assert_eq!(list.current(), None);
        assert_eq!(list.advance(true), None);
        assert_eq!(list.enqueue(PathBuf::from("a.ogg")), 0);
        assert_eq!(list.current(), Some(0));
        assert_eq!(list.enqueue(PathBuf::from("b.ogg")), 1);
        assert_eq!(list.upcoming(), vec![1]);
        assert_eq!(list.jump(5), None);
        list.clear();
        assert!(list.is_empty());
        assert_eq!(list.current_path(), None);
    }

    #[test]
    fn repeat_parsing_and_time_format() {
        assert_eq!(Repeat::parse(":one"), Some(Repeat::One));
        assert_eq!(Repeat::parse("ALL"), Some(Repeat::All));
        assert_eq!(Repeat::parse("off"), Some(Repeat::Off));
        assert_eq!(Repeat::parse("x"), None);
        assert_eq!(Repeat::Off.cycle(), Repeat::All);
        assert_eq!(format_time(65.4), "1:05");
        assert_eq!(format_time(3725.0), "1:02:05");
        assert_eq!(format_time(f32::NAN), "0:00");
        assert_eq!(track_name(Path::new("/m/Song One.mp3")), "Song One");
    }

    #[test]
    fn bar_layout_fits_and_keeps_a_usable_seek_bar() {
        let player = Player::new(0.5, false, Repeat::All);
        for (width, height) in [(1280, 800), (640, 480), (2560, 1440)] {
            let layout = bar_layout(&player, width, height);
            assert!(
                layout.seek.width > 50.0,
                "seek bar too small at {width}x{height}"
            );
            for (_, rect, _) in &layout.buttons {
                assert!(rect.x >= 0.0 && rect.x + rect.width <= width as f32 + 0.5);
                assert!(rect.y >= layout.frame.y);
            }
            assert!(layout.volume.x + layout.volume.width <= width as f32);
        }
        let rect = Rectangle::new(10.0, 0.0, 100.0, 5.0);
        assert_eq!(slider_fraction(rect, Vector2::new(60.0, 0.0)), 0.5);
        assert_eq!(slider_fraction(rect, Vector2::new(-60.0, 0.0)), 0.0);
        assert_eq!(slider_fraction(rect, Vector2::new(600.0, 0.0)), 1.0);
    }
}

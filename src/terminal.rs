//! The native terminal window: a raylib front end for [`term_emu`] that runs a shell on a
//! pseudo-terminal. Started with `selenite --terminal-window`.
//!
//! The text scales with Ctrl +/-/0 and Ctrl+wheel, and the character grid follows the window size.
//! Plain (uncoloured) text is syntax highlighted unless a program draws its own colours.

use std::{
    env,
    ffi::OsString,
    fs,
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        unix::process::{CommandExt, ExitStatusExt},
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use raylib::prelude::*;

use crate::{
    editor::{capture_screen, draw_mono, load_mono_font, measure_mono, token_color, KeyRepeat},
    settings,
    syntax::{self, Token},
    term_emu::{
        encode_mouse, palette_rgb, MouseMode, Row, TermColor, Terminal, BOLD, DIM, HIDDEN, INVERSE,
        STRIKE, UNDERLINE,
    },
    ui,
};

const SCROLLBACK_LINES: usize = 10_000;
const SCROLLBAR_WIDTH: f32 = 12.0;
const SCROLLBAR_MIN_THUMB: f32 = 28.0;
const BASE_FONT: f32 = 18.0;
const THEME_FG: (u8, u8, u8) = (216, 218, 224);
const THEME_BG: (u8, u8, u8) = (20, 21, 26);
const SELECTION: Color = Color::new(58, 90, 160, 170);
const CURSOR: Color = Color::new(250, 210, 60, 255);
const BANNER_BG: Color = Color::new(48, 30, 30, 240);
const BANNER_TEXT: Color = Color::new(250, 200, 170, 255);
const TOAST_BG: Color = Color::new(40, 42, 54, 235);
const TOAST_TEXT: Color = Color::new(225, 227, 233, 255);
/// A shell that dies this soon with an error is probably a failed launch; keep its output visible.
const QUICK_FAILURE: Duration = Duration::from_secs(3);
const DEFAULT_SIZE: (i32, i32) = (1000, 640);

// -------------------------------------------------------------------- pty --

struct Pty {
    master: OwnedFd,
    child: Child,
    output: Receiver<Vec<u8>>,
    input: Sender<Vec<u8>>,
}

fn set_cloexec(fd: RawFd) {
    // SAFETY: `fd` is a descriptor we own; fcntl only reads and updates its flags.
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFD);
        if flags >= 0 {
            libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
        }
    }
}

fn window_size(cols: usize, rows: usize) -> libc::winsize {
    libc::winsize {
        ws_row: rows.min(u16::MAX as usize) as u16,
        ws_col: cols.min(u16::MAX as usize) as u16,
        ws_xpixel: 0,
        ws_ypixel: 0,
    }
}

impl Pty {
    #[cfg(test)]
    fn spawn(program: &Path, args: &[OsString], cols: usize, rows: usize) -> io::Result<Self> {
        Self::spawn_in(program, args, cols, rows, None, None)
    }

    fn spawn_in(program: &Path, args: &[OsString], cols: usize, rows: usize, directory: Option<&Path>, grid: Option<&Path>) -> io::Result<Self> {
        let (mut master_fd, mut slave_fd): (libc::c_int, libc::c_int) = (0, 0);
        let size = window_size(cols, rows);
        // SAFETY: the out-pointers are valid, and the name and termios arguments may be null.
        let status = unsafe {
            libc::openpty(
                &mut master_fd,
                &mut slave_fd,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &size,
            )
        };
        if status != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: openpty returned two fresh descriptors that nothing else owns.
        let (master, slave) = unsafe {
            (
                OwnedFd::from_raw_fd(master_fd),
                OwnedFd::from_raw_fd(slave_fd),
            )
        };
        set_cloexec(master.as_raw_fd());
        set_cloexec(slave.as_raw_fd());
        #[cfg(target_os = "linux")]
        // SAFETY: termios is plain data; the calls only touch the descriptor we own.
        unsafe {
            let mut attributes: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(master.as_raw_fd(), &mut attributes) == 0 {
                attributes.c_iflag |= libc::IUTF8;
                libc::tcsetattr(master.as_raw_fd(), libc::TCSANOW, &attributes);
            }
        }

        let mut command = Command::new(program);
        if let Some(directory) = directory { command.current_dir(directory); }
        if let Some(grid) = grid { command.env("SELENITE_GRID", grid); }
        command
            .args(args)
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor")
            .env("TERM_PROGRAM", "selenite")
            .env_remove("LINES")
            .env_remove("COLUMNS")
            .stdin(Stdio::from(slave.try_clone()?))
            .stdout(Stdio::from(slave.try_clone()?))
            .stderr(Stdio::from(slave));
        // SAFETY: only async-signal-safe calls run between fork and exec.
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                libc::ioctl(0, libc::TIOCSCTTY as _, 0);
                Ok(())
            });
        }
        let child = command.spawn()?;
        // Dropping the command closes our copies of the slave so the reader sees EOF when the child ends.
        drop(command);

        let mut reader = File::from(master.try_clone()?);
        let (output_tx, output) = mpsc::sync_channel::<Vec<u8>>(256);
        thread::spawn(move || {
            let mut buffer = vec![0u8; 64 * 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if output_tx.send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        });
        let mut writer = File::from(master.try_clone()?);
        let (input, input_rx) = mpsc::channel::<Vec<u8>>();
        thread::spawn(move || {
            for chunk in input_rx {
                if writer.write_all(&chunk).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            master,
            child,
            output,
            input,
        })
    }

    fn write(&self, bytes: impl Into<Vec<u8>>) {
        let _ = self.input.send(bytes.into());
    }

    fn resize(&self, cols: usize, rows: usize) {
        let size = window_size(cols, rows);
        // SAFETY: the master descriptor is open and `size` is a valid winsize.
        unsafe {
            libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ as _, &size);
        }
    }

    /// The child's exit status as a shell would report it.
    fn exit_code(&mut self) -> Option<i32> {
        let status = self.child.try_wait().ok().flatten()?;
        Some(
            status
                .code()
                .unwrap_or_else(|| 128 + status.signal().unwrap_or(0)),
        )
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        let pid = self.child.id() as libc::pid_t;
        // SAFETY: signalling our own child's session; failure (already gone) is harmless.
        unsafe {
            libc::kill(-pid, libc::SIGHUP);
            libc::kill(-pid, libc::SIGCONT);
        }
        let _ = self.child.try_wait();
    }
}

// ------------------------------------------------------------------ input --

#[derive(Clone, Copy, Default)]
struct Mods {
    shift: bool,
    alt: bool,
    ctrl: bool,
}

impl Mods {
    /// The xterm modifier parameter: 1 plus shift, alt and ctrl bits.
    fn parameter(self) -> u32 {
        1 + u32::from(self.shift) + 2 * u32::from(self.alt) + 4 * u32::from(self.ctrl)
    }
}

const NAMED_KEYS: &[KeyboardKey] = &[
    KeyboardKey::KEY_UP,
    KeyboardKey::KEY_DOWN,
    KeyboardKey::KEY_LEFT,
    KeyboardKey::KEY_RIGHT,
    KeyboardKey::KEY_HOME,
    KeyboardKey::KEY_END,
    KeyboardKey::KEY_PAGE_UP,
    KeyboardKey::KEY_PAGE_DOWN,
    KeyboardKey::KEY_INSERT,
    KeyboardKey::KEY_DELETE,
    KeyboardKey::KEY_ENTER,
    KeyboardKey::KEY_KP_ENTER,
    KeyboardKey::KEY_BACKSPACE,
    KeyboardKey::KEY_TAB,
    KeyboardKey::KEY_ESCAPE,
    KeyboardKey::KEY_F1,
    KeyboardKey::KEY_F2,
    KeyboardKey::KEY_F3,
    KeyboardKey::KEY_F4,
    KeyboardKey::KEY_F5,
    KeyboardKey::KEY_F6,
    KeyboardKey::KEY_F7,
    KeyboardKey::KEY_F8,
    KeyboardKey::KEY_F9,
    KeyboardKey::KEY_F10,
    KeyboardKey::KEY_F12,
];

const LETTER_KEYS: &[KeyboardKey] = &[
    KeyboardKey::KEY_A,
    KeyboardKey::KEY_B,
    KeyboardKey::KEY_C,
    KeyboardKey::KEY_D,
    KeyboardKey::KEY_E,
    KeyboardKey::KEY_F,
    KeyboardKey::KEY_G,
    KeyboardKey::KEY_H,
    KeyboardKey::KEY_I,
    KeyboardKey::KEY_J,
    KeyboardKey::KEY_K,
    KeyboardKey::KEY_L,
    KeyboardKey::KEY_M,
    KeyboardKey::KEY_N,
    KeyboardKey::KEY_O,
    KeyboardKey::KEY_P,
    KeyboardKey::KEY_Q,
    KeyboardKey::KEY_R,
    KeyboardKey::KEY_S,
    KeyboardKey::KEY_T,
    KeyboardKey::KEY_U,
    KeyboardKey::KEY_V,
    KeyboardKey::KEY_W,
    KeyboardKey::KEY_X,
    KeyboardKey::KEY_Y,
    KeyboardKey::KEY_Z,
];

/// Control codes for Ctrl plus a punctuation or digit key.
const CONTROL_SYMBOLS: &[(KeyboardKey, u8)] = &[
    (KeyboardKey::KEY_SPACE, 0x00),
    (KeyboardKey::KEY_TWO, 0x00),
    (KeyboardKey::KEY_LEFT_BRACKET, 0x1b),
    (KeyboardKey::KEY_BACKSLASH, 0x1c),
    (KeyboardKey::KEY_RIGHT_BRACKET, 0x1d),
    (KeyboardKey::KEY_SLASH, 0x1f),
    (KeyboardKey::KEY_EIGHT, 0x7f),
];

/// What a program receives for a non-text key, or `None` for keys this terminal does not send.
fn named_key_bytes(key: KeyboardKey, mods: Mods, app_cursor: bool) -> Option<Vec<u8>> {
    use KeyboardKey::*;
    let m = mods.parameter();
    let cursor = |letter: char| {
        if m > 1 {
            format!("\x1b[1;{m}{letter}")
        } else if app_cursor {
            format!("\x1bO{letter}")
        } else {
            format!("\x1b[{letter}")
        }
    };
    let function = |letter: char| {
        if m > 1 {
            format!("\x1b[1;{m}{letter}")
        } else {
            format!("\x1bO{letter}")
        }
    };
    let tilde = |number: u32| {
        if m > 1 {
            format!("\x1b[{number};{m}~")
        } else {
            format!("\x1b[{number}~")
        }
    };
    let simple = |text: &str| {
        if mods.alt {
            format!("\x1b{text}")
        } else {
            text.to_owned()
        }
    };
    let text = match key {
        KEY_UP => cursor('A'),
        KEY_DOWN => cursor('B'),
        KEY_RIGHT => cursor('C'),
        KEY_LEFT => cursor('D'),
        KEY_HOME => cursor('H'),
        KEY_END => cursor('F'),
        KEY_PAGE_UP => tilde(5),
        KEY_PAGE_DOWN => tilde(6),
        KEY_INSERT => tilde(2),
        KEY_DELETE => tilde(3),
        KEY_ENTER | KEY_KP_ENTER => simple("\r"),
        KEY_BACKSPACE => simple(if mods.ctrl { "\x08" } else { "\x7f" }),
        KEY_TAB if mods.shift => "\x1b[Z".to_owned(),
        KEY_TAB => simple("\t"),
        KEY_ESCAPE => simple("\x1b"),
        KEY_F1 => function('P'),
        KEY_F2 => function('Q'),
        KEY_F3 => function('R'),
        KEY_F4 => function('S'),
        KEY_F5 => tilde(15),
        KEY_F6 => tilde(17),
        KEY_F7 => tilde(18),
        KEY_F8 => tilde(19),
        KEY_F9 => tilde(20),
        KEY_F10 => tilde(21),
        KEY_F12 => tilde(24),
        _ => return None,
    };
    Some(text.into_bytes())
}

/// Text to send for a paste: line breaks become carriage returns and control characters that
/// can manipulate the terminal (including its flow control) are removed.
fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    let normalized: String = text
        .replace("\r\n", "\r")
        .replace('\n', "\r")
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\r' | '\t'))
        .collect();
    if !bracketed {
        return normalized.into_bytes();
    }
    format!("\x1b[200~{normalized}\x1b[201~").into_bytes()
}

/// The `http(s)` URL around column `col` of `text`, without trailing punctuation.
fn url_at(text: &[char], col: usize) -> Option<String> {
    if col >= text.len() || text[col].is_whitespace() {
        return None;
    }
    let boundary = |c: &char| c.is_whitespace() || "\"'<>`".contains(*c);
    let start = text[..col].iter().rposition(boundary).map_or(0, |i| i + 1);
    let end = text[col..]
        .iter()
        .position(boundary)
        .map_or(text.len(), |i| col + i);
    let mut token: String = text[start..end].iter().collect();
    token.truncate(
        token
            .trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '}'])
            .len(),
    );
    let found = ["https://", "http://"]
        .iter()
        .filter_map(|scheme| token.find(scheme))
        .min()?;
    Some(token[found..].to_owned())
}

// ----------------------------------------------------------------- window --

/// Whole-pixel font size that follows the window size (like every Selenite window) and the user's zoom.
fn font_size(width: i32, height: i32) -> f32 {
    (BASE_FONT * ui::scale(width, height) / 1.25)
        .round()
        .max(8.0)
}

#[derive(Clone, Copy)]
struct Metrics {
    font: f32,
    cell_w: f32,
    cell_h: f32,
    pad: f32,
}

impl Metrics {
    /// Metrics for a window of `width` x `height`: the font grows with the window and the zoom.
    fn new(width: i32, height: i32) -> Self {
        let font = font_size(width, height);
        Self {
            font,
            cell_w: measure_mono("M", font),
            cell_h: font.round(),
            pad: (font * 0.4).round(),
        }
    }

    /// Columns and rows that fit a window of `width` x `height` pixels.
    fn grid(&self, width: i32, height: i32) -> (usize, usize) {
        let cols = ((width as f32 - 2.0 * self.pad) / self.cell_w)
            .floor()
            .max(2.0);
        let rows = ((height as f32 - 2.0 * self.pad) / self.cell_h)
            .floor()
            .max(1.0);
        (cols as usize, rows as usize)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Unit {
    Cell,
    Word,
    Line,
}

/// A selection in (column, absolute row) coordinates, which stay put while the screen scrolls.
#[derive(Clone, Copy)]
struct Selection {
    anchor: (usize, usize),
    head: (usize, usize),
    unit: Unit,
}

fn to_color((r, g, b): (u8, u8, u8)) -> Color {
    Color::new(r, g, b, 255)
}

fn resolve(color: TermColor, default: (u8, u8, u8), brighten: bool) -> (u8, u8, u8) {
    match color {
        TermColor::Default => default,
        TermColor::Indexed(index) => palette_rgb(if brighten && index < 8 {
            index + 8
        } else {
            index
        }),
        TermColor::Rgb(r, g, b) => (r, g, b),
    }
}

fn token_map(row: &Row) -> Vec<Token> {
    let mut map = vec![Token::Plain; row.cells.len()];
    if row
        .cells
        .iter()
        .all(|cell| cell.ch == ' ' || cell.ch == '\0')
    {
        return map;
    }
    let text: String = row
        .cells
        .iter()
        .map(|cell| if cell.ch == '\0' { ' ' } else { cell.ch })
        .collect();
    let length = map.len();
    for span in syntax::highlight_terminal_line(&text) {
        for slot in &mut map[span.start.min(length)..span.end.min(length)] {
            *slot = span.token;
        }
    }
    map
}

struct TermWindow {
    term: Terminal,
    pty: Pty,
    metrics: Metrics,
    /// Lines scrolled back from the live screen.
    offset: usize,
    scrollbar_drag: Option<f32>,
    selection: Option<Selection>,
    selecting: bool,
    last_click: Option<(f64, (usize, usize), u8)>,
    keys: KeyRepeat,
    highlight: bool,
    blink_from: f64,
    toast: Option<(String, f64)>,
    bell_until: f64,
    started: Instant,
    output_closed: bool,
    exited: Option<(i32, f64)>,
    /// Set when a quick failure keeps the window open until a key is pressed.
    hold_open: bool,
    held_button: Option<u32>,
    last_reported: Option<(usize, usize)>,
    focused: bool,
    title: String,
    saw_output: bool,
    pending_paste: Option<(Receiver<Result<String, String>>, Instant)>,
}

impl TermWindow {
    fn say(&mut self, message: impl Into<String>, now: f64) {
        self.toast = Some((message.into(), now));
    }

    fn rows_cols(&self) -> (usize, usize) {
        (self.term.rows(), self.term.cols())
    }

    /// Absolute row shown at the top of the view.
    fn base_abs(&self) -> usize {
        let scrollback = self.term.scrollback_len();
        if self.term.on_alt_screen() {
            scrollback
        } else {
            scrollback - self.offset.min(scrollback)
        }
    }

    fn cell_at(&self, point: Vector2) -> (usize, usize) {
        let (rows, cols) = self.rows_cols();
        let col = ((point.x - self.metrics.pad) / self.metrics.cell_w)
            .floor()
            .max(0.0) as usize;
        let row = ((point.y - self.metrics.pad) / self.metrics.cell_h)
            .floor()
            .max(0.0) as usize;
        (col.min(cols - 1), row.min(rows - 1))
    }

    fn clamp_offset(&mut self) {
        self.offset = self.offset.min(self.term.scrollback_len());
        if self.term.on_alt_screen() {
            self.offset = 0;
            self.scrollbar_drag = None;
        }
    }

    fn scrollbar_mouse(&mut self, mouse: Vector2, width: i32, height: i32, pressed: bool, down: bool) -> bool {
        let scrollback = self.term.scrollback_len();
        let geometry = scrollbar_geometry(height as f32, self.term.rows(), scrollback, self.offset);
        let Some((top, thumb)) = geometry else {
            self.scrollbar_drag = None;
            return false;
        };
        if let Some(grab) = self.scrollbar_drag {
            if down {
                self.offset = scrollbar_offset(mouse.y - grab, height as f32, thumb, scrollback);
                return true;
            }
            self.scrollbar_drag = None;
            return true;
        }
        let left = width as f32 - SCROLLBAR_WIDTH;
        if !pressed || mouse.x < left || mouse.x > width as f32 || mouse.y < 0.0 || mouse.y > height as f32 {
            return false;
        }
        let grab = if mouse.y >= top && mouse.y <= top + thumb {
            mouse.y - top
        } else {
            thumb * 0.5
        };
        self.offset = scrollbar_offset(mouse.y - grab, height as f32, thumb, scrollback);
        self.scrollbar_drag = Some(grab);
        true
    }

    /// Re-fits the character grid to the window (after a resize or a zoom change).
    fn fit(&mut self, width: i32, height: i32) {
        let (cols, rows) = self.metrics.grid(width, height);
        if (cols, rows) == (self.term.cols(), self.term.rows()) {
            return;
        }
        let before = self.term.scrollback_len();
        self.term.resize(cols, rows);
        self.pty.resize(cols, rows);
        let change = self.term.scrollback_len() as isize - before as isize;
        if self.offset > 0 {
            self.offset = (self.offset as isize + change).max(0) as usize;
        }
        self.clamp_offset();
        self.selection = None;
    }

    /// Reads what the program wrote, within a small time budget per frame.
    fn pump(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread, now: f64) {
        let deadline = Instant::now() + Duration::from_millis(8);
        loop {
            match self.pty.output.try_recv() {
                Ok(chunk) => {
                    self.saw_output = true;
                    let before = self.term.scrollback_len();
                    self.term.feed(&chunk);
                    let trimmed = self.term.take_trimmed();
                    let added = (self.term.scrollback_len() + trimmed).saturating_sub(before);
                    if self.offset > 0 {
                        self.offset += added;
                    }
                    if trimmed > 0 {
                        self.selection = self.selection.and_then(|s| {
                            let shift =
                                |p: (usize, usize)| p.1.checked_sub(trimmed).map(|row| (p.0, row));
                            Some(Selection {
                                anchor: shift(s.anchor)?,
                                head: shift(s.head)?,
                                ..s
                            })
                        });
                    }
                    let replies = self.term.take_responses();
                    if !replies.is_empty() {
                        self.pty.write(replies);
                    }
                    if self.term.take_bell() {
                        self.bell_until = now + 0.12;
                    }
                    if let Some(title) = self.term.take_title() {
                        self.title = if title.is_empty() {
                            "Selenite Terminal".to_owned()
                        } else {
                            format!("{title} - Selenite Terminal")
                        };
                        rl.set_window_title(thread, &self.title);
                    }
                    if Instant::now() > deadline {
                        break;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.output_closed = true;
                    break;
                }
            }
        }
        self.clamp_offset();
    }

    fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let selection = self.selection?;
        if selection.unit == Unit::Cell && selection.anchor == selection.head {
            return None;
        }
        let key = |p: (usize, usize)| (p.1, p.0);
        let (first, last) = if key(selection.anchor) <= key(selection.head) {
            (selection.anchor, selection.head)
        } else {
            (selection.head, selection.anchor)
        };
        Some(match selection.unit {
            Unit::Cell => (first, last),
            Unit::Word => (
                (self.term.word_at(first.0, first.1).0, first.1),
                (self.term.word_at(last.0, last.1).1, last.1),
            ),
            Unit::Line => ((0, first.1), (usize::MAX / 2, last.1)),
        })
    }

    fn selected_text(&self) -> Option<String> {
        let (from, to) = self.selection_range()?;
        let text = self.term.text_range(from, to);
        (!text.is_empty()).then_some(text)
    }

    fn copy_selection(&mut self, rl: &mut RaylibHandle, now: f64) -> bool {
        let Some(text) = self.selected_text() else {
            return false;
        };
        if rl.set_clipboard_text(&text).is_ok() {
            self.say(format!("Copied {} characters", text.chars().count()), now);
        }
        true
    }

    fn paste(&mut self) {
        if self.pending_paste.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.pending_paste = Some((receiver, Instant::now()));
        thread::spawn(move || {
            let result = arboard::Clipboard::new()
                .map_err(|error| format!("clipboard unavailable: {error}"))
                .and_then(|mut clipboard| {
                    clipboard
                        .get_text()
                        .map_err(|error| format!("could not read clipboard: {error}"))
                });
            let _ = sender.send(result);
        });
    }

    fn pump_paste(&mut self, now: f64) {
        let Some((receiver, started)) = &self.pending_paste else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(text)) => {
                if !text.is_empty() {
                    self.pty
                        .write(paste_bytes(&text, self.term.modes.bracketed_paste));
                    self.offset = 0;
                }
                self.pending_paste = None;
            }
            Ok(Err(error)) => {
                self.say(error, now);
                self.pending_paste = None;
            }
            Err(TryRecvError::Disconnected) => {
                self.say("clipboard reader stopped unexpectedly", now);
                self.pending_paste = None;
            }
            Err(TryRecvError::Empty) if started.elapsed() >= Duration::from_secs(2) => {
                self.say("Clipboard did not respond", now);
                self.pending_paste = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn row_chars(&self, abs: usize) -> Vec<char> {
        self.term
            .row_abs(abs)
            .map(|row| {
                row.cells
                    .iter()
                    .map(|cell| if cell.ch == '\0' { ' ' } else { cell.ch })
                    .collect()
            })
            .unwrap_or_default()
    }

    // ------------------------------------------------------------ keyboard --

    fn keyboard(&mut self, rl: &mut RaylibHandle, now: f64) {
        let down = |rl: &RaylibHandle, a, b| rl.is_key_down(a) || rl.is_key_down(b);
        let mods = Mods {
            shift: down(
                rl,
                KeyboardKey::KEY_LEFT_SHIFT,
                KeyboardKey::KEY_RIGHT_SHIFT,
            ),
            // Right Alt is AltGr on most layouts and produces characters instead.
            alt: rl.is_key_down(KeyboardKey::KEY_LEFT_ALT),
            ctrl: down(
                rl,
                KeyboardKey::KEY_LEFT_CONTROL,
                KeyboardKey::KEY_RIGHT_CONTROL,
            ),
        };
        let mut out: Vec<u8> = Vec::new();
        let mut app_command = false;

        if mods.ctrl && mods.shift {
            if rl.is_key_pressed(KeyboardKey::KEY_C) {
                app_command = true;
                if !self.copy_selection(rl, now) {
                    self.say("Nothing selected to copy", now);
                }
            }
            if self.keys.fire(rl, KeyboardKey::KEY_V, now) {
                app_command = true;
                self.paste();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_H) {
                app_command = true;
                self.highlight = !self.highlight;
                let state = if self.highlight { "on" } else { "off" };
                self.say(format!("Syntax highlighting {state} (Ctrl+Shift+H)"), now);
            }
        }
        if mods.ctrl && rl.is_key_pressed(KeyboardKey::KEY_INSERT) {
            app_command = true;
            self.copy_selection(rl, now);
        }
        if mods.shift && !mods.ctrl && self.keys.fire(rl, KeyboardKey::KEY_INSERT, now) {
            app_command = true;
            self.paste();
        }

        let (rows, _) = self.rows_cols();
        let page = rows.saturating_sub(1).max(1);
        let scrollback = self.term.scrollback_len();
        let can_scroll = mods.shift && !mods.ctrl && !self.term.on_alt_screen();
        if can_scroll {
            if self.keys.fire(rl, KeyboardKey::KEY_PAGE_UP, now) {
                self.offset = (self.offset + page).min(scrollback);
                app_command = true;
            }
            if self.keys.fire(rl, KeyboardKey::KEY_PAGE_DOWN, now) {
                self.offset = self.offset.saturating_sub(page);
                app_command = true;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_HOME) {
                self.offset = scrollback;
                app_command = true;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_END) {
                self.offset = 0;
                app_command = true;
            }
        }

        if !app_command {
            for &key in NAMED_KEYS {
                if !matches!(key, KeyboardKey::KEY_F8 | KeyboardKey::KEY_F9) && !(mods.ctrl && key == KeyboardKey::KEY_TAB) && self.keys.fire(rl, key, now) {
                    if let Some(bytes) = named_key_bytes(key, mods, self.term.modes.app_cursor) {
                        out.extend(bytes);
                    }
                }
            }
            if mods.ctrl && !mods.alt {
                for (index, &key) in LETTER_KEYS.iter().enumerate() {
                    let reserved = mods.shift
                        && matches!(
                            key,
                            KeyboardKey::KEY_C | KeyboardKey::KEY_V | KeyboardKey::KEY_H | KeyboardKey::KEY_T | KeyboardKey::KEY_W
                        );
                    if self.keys.fire(rl, key, now) && !reserved {
                        out.push(index as u8 + 1);
                    }
                }
                for &(key, code) in CONTROL_SYMBOLS {
                    if self.keys.fire(rl, key, now) {
                        out.push(code);
                    }
                }
            }
        }
        while let Some(c) = rl.get_char_pressed() {
            if mods.ctrl || c.is_control() || app_command {
                continue;
            }
            if mods.alt {
                out.push(0x1b);
            }
            let mut buffer = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
        }
        if !out.is_empty() {
            self.pty.write(out);
            self.offset = 0;
            self.selection = None;
            self.blink_from = now;
        }
    }

    // --------------------------------------------------------------- mouse --

    /// Returns the mouse cursor wanted for this frame.
    fn mouse_at(&mut self, rl: &mut RaylibHandle, now: f64, top: f32) -> MouseCursor {
        let mut mouse = rl.get_mouse_position();
        mouse.y -= top;
        if self.scrollbar_mouse(
            mouse,
            rl.get_screen_width(),
            (rl.get_screen_height() as f32 - top).max(1.0) as i32,
            rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT),
            rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT),
        ) {
            return MouseCursor::MOUSE_CURSOR_DEFAULT;
        }
        if mouse.y < 0.0 {
            return MouseCursor::MOUSE_CURSOR_DEFAULT;
        }
        let (col, view_row) = self.cell_at(mouse);
        let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        let shift = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        let alt = rl.is_key_down(KeyboardKey::KEY_LEFT_ALT);
        let wheel = if ctrl { 0.0 } else { rl.get_mouse_wheel_move() };
        let ticks = wheel.abs().ceil() as usize;
        let reporting = self.term.modes.mouse != MouseMode::Off && !shift;
        let modifier_bits = 8 * u32::from(alt) + 16 * u32::from(ctrl);
        let abs = self.base_abs() + view_row;
        let link = if ctrl {
            url_at(&self.row_chars(abs), col)
        } else {
            None
        };

        if reporting {
            let sgr = self.term.modes.mouse_sgr;
            let mut out = Vec::new();
            let mut emit = |button: u32, release: bool| {
                if let Some(bytes) =
                    encode_mouse(button | modifier_bits, col, view_row, release, sgr)
                {
                    out.extend(bytes);
                }
            };
            for _ in 0..ticks {
                emit(if wheel > 0.0 { 64 } else { 65 }, false);
            }
            for (index, button) in [
                MouseButton::MOUSE_BUTTON_LEFT,
                MouseButton::MOUSE_BUTTON_MIDDLE,
                MouseButton::MOUSE_BUTTON_RIGHT,
            ]
            .into_iter()
            .enumerate()
            {
                if rl.is_mouse_button_pressed(button) {
                    emit(index as u32, false);
                    self.held_button = Some(index as u32);
                    self.last_reported = Some((col, view_row));
                }
                if rl.is_mouse_button_released(button) && self.held_button == Some(index as u32) {
                    emit(index as u32, true);
                    self.held_button = None;
                }
            }
            let moved = self.last_reported != Some((col, view_row));
            let mode = self.term.modes.mouse;
            if moved
                && (mode == MouseMode::Any
                    || (mode == MouseMode::Drag && self.held_button.is_some()))
            {
                emit(32 + self.held_button.unwrap_or(3), false);
                self.last_reported = Some((col, view_row));
            }
            if !out.is_empty() {
                self.pty.write(out);
            }
            return MouseCursor::MOUSE_CURSOR_DEFAULT;
        }
        self.held_button = None;

        if ticks > 0 {
            if self.term.on_alt_screen() {
                if self.term.modes.alt_scroll {
                    let arrow = match (wheel > 0.0, self.term.modes.app_cursor) {
                        (true, true) => "\x1bOA",
                        (true, false) => "\x1b[A",
                        (false, true) => "\x1bOB",
                        (false, false) => "\x1b[B",
                    };
                    self.pty.write(arrow.repeat(3 * ticks));
                }
            } else {
                let lines = 3 * ticks;
                let scrollback = self.term.scrollback_len();
                self.offset = if wheel > 0.0 {
                    (self.offset + lines).min(scrollback)
                } else {
                    self.offset.saturating_sub(lines)
                };
            }
        }

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if let Some(url) = link.clone() {
                match crate::open_with_system(Path::new(&url)) {
                    Ok(()) => self.say(format!("Opening {url}"), now),
                    Err(error) => self.say(error, now),
                }
            } else {
                let clicks = match self.last_click {
                    Some((time, cell, count)) if now - time < 0.4 && cell == (col, view_row) => {
                        (count + 1).min(3)
                    }
                    _ => 1,
                };
                self.last_click = Some((now, (col, view_row), clicks));
                let unit = match clicks {
                    1 => Unit::Cell,
                    2 => Unit::Word,
                    _ => Unit::Line,
                };
                self.selection = Some(Selection {
                    anchor: (col, abs),
                    head: (col, abs),
                    unit,
                });
                self.selecting = true;
            }
        }
        if self.selecting {
            if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
                let scrollback = self.term.scrollback_len();
                if mouse.y < self.metrics.pad {
                    self.offset = (self.offset + 1).min(scrollback);
                } else if mouse.y > rl.get_screen_height() as f32 - top - self.metrics.pad {
                    self.offset = self.offset.saturating_sub(1);
                }
                let head = (col, self.base_abs() + view_row);
                if let Some(selection) = &mut self.selection {
                    selection.head = head;
                }
            } else {
                self.selecting = false;
            }
        }
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
            if self.selection_range().is_some() {
                self.copy_selection(rl, now);
                self.selection = None;
            } else {
                self.paste();
            }
        }
        if link.is_some() {
            MouseCursor::MOUSE_CURSOR_POINTING_HAND
        } else {
            MouseCursor::MOUSE_CURSOR_IBEAM
        }
    }

    // ------------------------------------------------------------- drawing --

    fn draw<D: RaylibDraw>(&self, d: &mut D, width: i32, height: i32, now: f64) {
        let metrics = self.metrics;
        let term = &self.term;
        let (theme_fg, theme_bg) = if term.modes.reverse {
            (THEME_BG, THEME_FG)
        } else {
            (THEME_FG, THEME_BG)
        };
        let theme_bg_color = to_color(theme_bg);
        d.draw_rectangle(0, 0, width, height, theme_bg_color);
        let alt = term.on_alt_screen();
        let highlight = self.highlight && !alt;
        let selection = self.selection_range();
        let base_abs = self.base_abs();
        let cols = term.cols();
        let bold_shift = (metrics.font / 24.0).max(0.6);

        for view_row in 0..term.rows() {
            let Some(row) = term.view_row(view_row, self.offset) else {
                continue;
            };
            let abs = base_abs + view_row;
            let y = metrics.pad + view_row as f32 * metrics.cell_h;
            let count = row.cells.len().min(cols);
            let tokens = if highlight {
                token_map(row)
            } else {
                Vec::new()
            };
            let selected = |col: usize| {
                selection.is_some_and(|(from, to)| {
                    (abs, col) >= (from.1, from.0) && (abs, col) <= (to.1, to.0)
                })
            };
            // (foreground, background, cell attributes) after inverse video and highlighting.
            let style = |col: usize| {
                let cell = &row.cells[col];
                let brighten = cell.attrs & BOLD != 0;
                let mut fg = resolve(cell.fg, theme_fg, brighten);
                let mut bg = resolve(cell.bg, theme_bg, false);
                if cell.attrs & INVERSE != 0 {
                    std::mem::swap(&mut fg, &mut bg);
                }
                let mut color = to_color(fg);
                if highlight
                    && cell.fg == TermColor::Default
                    && cell.bg == TermColor::Default
                    && cell.attrs & INVERSE == 0
                {
                    if let Some(token) = tokens.get(col).filter(|token| **token != Token::Plain) {
                        color = token_color(*token);
                    }
                }
                if cell.attrs & DIM != 0 {
                    color.a = 150;
                }
                (color, to_color(bg), cell.attrs)
            };

            let mut run_start = 0;
            let mut run_color: Option<Color> = None;
            for col in 0..=count {
                let color = (col < count).then(|| {
                    if selected(col) {
                        SELECTION
                    } else {
                        style(col).1
                    }
                });
                if color != run_color {
                    if let Some(fill) = run_color.filter(|fill| *fill != theme_bg_color) {
                        d.draw_rectangle(
                            (metrics.pad + run_start as f32 * metrics.cell_w).floor() as i32,
                            y as i32,
                            ((col - run_start) as f32 * metrics.cell_w).ceil() as i32 + 1,
                            metrics.cell_h as i32,
                            fill,
                        );
                    }
                    run_start = col;
                    run_color = color;
                }
            }

            let mut run = String::new();
            let mut run_origin = 0;
            let mut run_style: Option<(Color, bool)> = None;
            let mut pending = 0;
            let flush = |d: &mut D,
                         run: &mut String,
                         origin: usize,
                         style: Option<(Color, bool)>| {
                if let Some((color, bold)) = style.filter(|_| !run.is_empty()) {
                    let x = metrics.pad + origin as f32 * metrics.cell_w;
                    draw_mono(d, run, x, y, metrics.font, color);
                    if bold {
                        draw_mono(d, run, x + bold_shift, y, metrics.font, color);
                    }
                }
                run.clear();
            };
            #[allow(clippy::needless_range_loop)]
            for col in 0..count {
                let cell = row.cells[col];
                let (color, _, attrs) = style(col);
                let x = metrics.pad + col as f32 * metrics.cell_w;
                if attrs & UNDERLINE != 0 {
                    d.draw_rectangle(
                        x as i32,
                        (y + metrics.cell_h - 2.0) as i32,
                        metrics.cell_w.ceil() as i32,
                        1,
                        color,
                    );
                }
                if attrs & STRIKE != 0 {
                    d.draw_rectangle(
                        x as i32,
                        (y + metrics.cell_h / 2.0) as i32,
                        metrics.cell_w.ceil() as i32,
                        1,
                        color,
                    );
                }
                if cell.ch == '\0' {
                    continue;
                }
                if cell.ch == ' ' || attrs & HIDDEN != 0 {
                    if run_style.is_some() {
                        pending += 1;
                    }
                    continue;
                }
                let bold = attrs & BOLD != 0;
                if cell.ch.is_ascii_graphic() {
                    if run_style != Some((color, bold)) {
                        flush(d, &mut run, run_origin, run_style);
                        run_style = Some((color, bold));
                        run_origin = col;
                        pending = 0;
                    }
                    run.extend(std::iter::repeat_n(' ', pending));
                    pending = 0;
                    run.push(cell.ch);
                } else {
                    flush(d, &mut run, run_origin, run_style);
                    run_style = None;
                    pending = 0;
                    let text = cell.ch.to_string();
                    draw_mono(d, &text, x, y, metrics.font, color);
                    if bold {
                        draw_mono(d, &text, x + bold_shift, y, metrics.font, color);
                    }
                }
            }
            flush(d, &mut run, run_origin, run_style);
        }

        self.draw_cursor(d, theme_bg, now);
        self.draw_scrollbar(d, width, height);
        if now < self.bell_until {
            d.draw_rectangle_lines(0, 0, width, height, CURSOR);
        }
        let size = (metrics.font * 0.8).round();
        if let Some((message, shown)) = &self.toast {
            if now - shown < 2.5 {
                let text_width = ui::measure(message, size);
                let (w, h) = (text_width + size * 1.2, size * 1.8);
                let (x, y) = (
                    width as f32 - w - metrics.pad,
                    height as f32 - h - metrics.pad,
                );
                d.draw_rectangle(x as i32, y as i32, w as i32, h as i32, TOAST_BG);
                ui::draw_text(
                    d,
                    message,
                    x + size * 0.6,
                    y + (h - size) / 2.0,
                    size,
                    TOAST_TEXT,
                );
            }
        }
        if self.hold_open {
            let message = match self.exited {
                Some((code, _)) => {
                    format!("Process exited with status {code}. Press any key to close.")
                }
                None => "Process exited. Press any key to close.".to_owned(),
            };
            let h = size * 2.2;
            d.draw_rectangle(0, (height as f32 - h) as i32, width, h as i32, BANNER_BG);
            ui::draw_text(
                d,
                &message,
                metrics.pad,
                height as f32 - h + (h - size) / 2.0,
                size,
                BANNER_TEXT,
            );
        }
    }

    fn draw_cursor<D: RaylibDraw>(&self, d: &mut D, theme_bg: (u8, u8, u8), now: f64) {
        let term = &self.term;
        if self.offset > 0 || !term.modes.cursor_visible || self.hold_open {
            return;
        }
        let metrics = self.metrics;
        let (x, y) = term.cursor();
        let style = term.cursor_style();
        let blinking = style == 0 || style % 2 == 1;
        let visible = !self.focused || !blinking || (now - self.blink_from) % 1.0 < 0.6;
        if !visible {
            return;
        }
        let left = metrics.pad + x as f32 * metrics.cell_w;
        let top = metrics.pad + y as f32 * metrics.cell_h;
        let cell = term
            .view_row(y, 0)
            .and_then(|row| row.cells.get(x))
            .copied();
        let width = match cell {
            Some(cell) if cell.ch != '\0' && crate::term_emu::char_width(cell.ch) == 2 => {
                metrics.cell_w * 2.0
            }
            _ => metrics.cell_w,
        };
        if !self.focused {
            d.draw_rectangle_lines(
                left as i32,
                top as i32,
                width.ceil() as i32,
                metrics.cell_h as i32,
                CURSOR,
            );
            return;
        }
        match style {
            3 | 4 => d.draw_rectangle(
                left as i32,
                (top + metrics.cell_h - 2.0) as i32,
                width.ceil() as i32,
                2,
                CURSOR,
            ),
            5 | 6 => d.draw_rectangle(left as i32, top as i32, 2, metrics.cell_h as i32, CURSOR),
            _ => {
                d.draw_rectangle(
                    left as i32,
                    top as i32,
                    width.ceil() as i32,
                    metrics.cell_h as i32,
                    CURSOR,
                );
                if let Some(cell) = cell.filter(|cell| cell.ch != ' ' && cell.ch != '\0') {
                    draw_mono(
                        d,
                        &cell.ch.to_string(),
                        left,
                        top,
                        metrics.font,
                        to_color(theme_bg),
                    );
                }
            }
        }
    }

    fn draw_scrollbar<D: RaylibDraw>(&self, d: &mut D, width: i32, height: i32) {
        let scrollback = self.term.scrollback_len();
        let Some((top, thumb)) = scrollbar_geometry(height as f32, self.term.rows(), scrollback, self.offset)
        else {
            return;
        };
        let alpha = if self.offset > 0 { 150 } else { 60 };
        d.draw_rectangle(
            width - SCROLLBAR_WIDTH as i32,
            0,
            SCROLLBAR_WIDTH as i32,
            height,
            Color::new(42, 45, 54, 100),
        );
        d.draw_rectangle(
            width - SCROLLBAR_WIDTH as i32,
            top as i32,
            SCROLLBAR_WIDTH as i32,
            thumb as i32,
            Color::new(160, 165, 185, alpha),
        );
    }
}

fn scrollbar_geometry(height: f32, rows: usize, scrollback: usize, offset: usize) -> Option<(f32, f32)> {
    if height <= 0.0 || rows == 0 || scrollback == 0 {
        return None;
    }
    let thumb = (height * rows as f32 / (scrollback + rows) as f32)
        .max(SCROLLBAR_MIN_THUMB)
        .min(height);
    let travel = height - thumb;
    let top = travel * (1.0 - offset.min(scrollback) as f32 / scrollback as f32);
    Some((top, thumb))
}

fn scrollbar_offset(top: f32, height: f32, thumb: f32, scrollback: usize) -> usize {
    let travel = (height - thumb).max(0.0);
    if travel == 0.0 {
        return 0;
    }
    (scrollback as f32 * (1.0 - top.clamp(0.0, travel) / travel)).round() as usize
}

// -------------------------------------------------------------------- run --

#[cfg(feature = "scripting")]
pub(crate) struct EmbeddedTerminal {
    sessions: Vec<TermWindow>,
    active: usize,
    directory: PathBuf,
    grid: PathBuf,
    scale_requested: bool,
    mouse: Vector2,
}

#[cfg(feature = "scripting")]
impl EmbeddedTerminal {
    pub fn new(directory: &Path, grid: &Path) -> Result<Self, String> {
        let program = env::current_exe().map_err(|error| error.to_string())?;
        let args = vec![OsString::from("--terminal")];
        Ok(Self {
            sessions: vec![new_session(&program, &args, 1000, 640, Some(directory), Some(grid))?],
            active: 0,
            directory: directory.to_owned(),
            grid: grid.to_owned(),
            scale_requested: false,
            mouse: Vector2::new(-1.0, -1.0),
        })
    }

    pub fn take_scale_request(&mut self) -> bool {
        std::mem::take(&mut self.scale_requested)
    }

    pub fn clear(&mut self) {
        let window = &mut self.sessions[self.active];
        window.term.feed(b"\x1b[2J\x1b[3J\x1b[H");
        window.offset = 0;
        window.selection = None;
        window.selecting = false;
    }

    pub fn update(&mut self, rl: &mut RaylibHandle, rect: Rectangle, focused: bool, now: f64) {
        let bar = TabBar::new(rect.width as i32, rect.height as i32, self.sessions.len(), self.active);
        let mouse = rl.get_mouse_position();
        let local = Vector2::new(mouse.x - rect.x, mouse.y - rect.y);
        self.mouse = local;
        let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        let shift = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        let click = focused && local.y >= 0.0 && local.y < bar.height && rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let control = if click { bar.controls.iter().position(|button| button.check_collision_point_rec(local)) } else { None };
        let new_tab = control == Some(2) || focused && ctrl && shift && rl.is_key_pressed(KeyboardKey::KEY_T);
        let close_tab = control == Some(4) || focused && ctrl && shift && rl.is_key_pressed(KeyboardKey::KEY_W);
        let switch = focused && ctrl && rl.is_key_pressed(KeyboardKey::KEY_TAB) || matches!(control, Some(0 | 1));
        if new_tab && self.sessions.len() < 32 || close_tab && self.sessions.len() == 1 {
            let result = env::current_exe().map_err(|error| error.to_string()).and_then(|program| new_session(&program, &["--terminal".into()], rect.width as i32, rect.height as i32, Some(&self.directory), Some(&self.grid)));
            match result {
                Ok(session) => {
                    if close_tab { self.sessions.clear(); }
                    self.sessions.push(session);
                    self.active = self.sessions.len() - 1;
                }
                Err(error) => self.sessions[self.active].say(error, now),
            }
        } else if close_tab {
            self.sessions.remove(self.active);
            self.active = self.active.min(self.sessions.len() - 1);
        }
        if switch {
            self.active = if control == Some(0) || shift && control.is_none() { (self.active + self.sessions.len() - 1) % self.sessions.len() } else { (self.active + 1) % self.sessions.len() };
        }
        if control == Some(3) { self.scale_requested = true; }
        if click {
            if let Some((index, _)) = bar.tabs.iter().find(|(_, button)| button.check_collision_point_rec(local)) { self.active = *index; }
        }
        let content = Rectangle::new(rect.x, rect.y + bar.height, rect.width, (rect.height - bar.height).max(1.0));
        for (index, window) in self.sessions.iter_mut().enumerate() {
            window.metrics = Metrics::new(rl.get_screen_width(), rl.get_screen_height());
            window.fit(content.width as i32, content.height as i32);
            window.focused = focused && index == self.active;
            let deadline = Instant::now() + Duration::from_millis(4);
            while let Ok(bytes) = window.pty.output.try_recv() {
                window.term.feed(&bytes);
                let _ = window.term.take_trimmed();
                window.pty.write(window.term.take_responses());
                let _ = window.term.take_title();
                if Instant::now() > deadline { break; }
            }
            window.clamp_offset();
            if window.exited.is_none() {
                if let Some(code) = window.pty.exit_code() {
                    window.exited = Some((code, now));
                    window.say(format!("Shell exited ({code}); use + for a new terminal"), now);
                }
            }
            window.pump_paste(now);
        }
        let window = &mut self.sessions[self.active];
        if focused && !click && !new_tab && !close_tab && !switch && window.exited.is_none() {
            window.keyboard(rl, now);
        }
        if focused {
            let local = Vector2::new(mouse.x - content.x, mouse.y - content.y);
            let scrollbar_handled = window.scrollbar_mouse(
                local,
                content.width as i32,
                content.height as i32,
                rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT),
                rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT),
            );
            if content.check_collision_point_rec(mouse) && !scrollbar_handled {
                let wheel = rl.get_mouse_wheel_move();
                if wheel > 0.0 {
                    window.offset = (window.offset + wheel.ceil() as usize * 3)
                        .min(window.term.scrollback_len());
                } else if wheel < 0.0 {
                    window.offset = window.offset.saturating_sub((-wheel).ceil() as usize * 3);
                }
                let (col, row) = window.cell_at(Vector2::new(mouse.x - content.x, mouse.y - content.y));
                let point = (col, window.base_abs() + row);
                if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                    window.selection = Some(Selection { anchor: point, head: point, unit: Unit::Cell });
                    window.selecting = true;
                }
                if window.selecting && rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
                    if let Some(selection) = &mut window.selection { selection.head = point; }
                }
                if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
                    if !window.copy_selection(rl, now) { window.paste(); }
                }
            }
        }
        if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) { window.selecting = false; }
    }

    pub fn draw<D: RaylibDraw>(&self, d: &mut D, rect: Rectangle, now: f64) {
        let bar = TabBar::new(rect.width as i32, rect.height as i32, self.sessions.len(), self.active);
        let (x, y, width, height) = crate::rendering::scissor_rect(rect);
        let mut clip = d.begin_scissor_mode(x, y, width, height);
        {
            let camera = Camera2D { offset: Vector2::new(rect.x, rect.y + bar.height), target: Vector2::zero(), rotation: 0.0, zoom: 1.0 };
            let mut translated = clip.begin_mode2D(camera);
            self.sessions[self.active].draw(&mut translated, rect.width as i32, (rect.height - bar.height).max(1.0) as i32, now);
        }
        let camera = Camera2D { offset: Vector2::new(rect.x, rect.y), target: Vector2::zero(), rotation: 0.0, zoom: 1.0 };
        let mut translated = clip.begin_mode2D(camera);
        bar.draw(&mut translated, &self.sessions, self.active, rect.width as i32, self.mouse);
    }
}

fn parse_size(text: &str) -> Option<(i32, i32)> {
    let (width, height) = text.split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

/// The command run inside the terminal: `selenite --terminal ...` or `selenite --terminal-run ...`.
fn child_command(args: &[OsString]) -> Result<(PathBuf, Vec<OsString>), String> {
    let exe = env::current_exe().map_err(|error| error.to_string())?;
    let mode = args.first().and_then(|arg| arg.to_str());
    match mode {
        None => Ok((exe, vec!["--terminal".into()])),
        Some("--terminal" | "--terminal-run") => Ok((exe, args.to_vec())),
        Some(other) => Err(format!("--terminal-window cannot run {other}")),
    }
}

fn terminal_window_options(args: &[OsString]) -> Result<(Option<PathBuf>, Vec<OsString>), String> {
    let mut directory = None;
    let mut command_args = Vec::new();
    let mut index = 0;
    let mut command_started = false;
    while index < args.len() {
        let argument = args[index].to_str();
        if !command_started && argument == Some("--working-directory") {
            index += 1;
            let value = args
                .get(index)
                .ok_or("--working-directory needs a path")?;
            directory = Some(PathBuf::from(value));
        } else if !command_started
            && argument.is_some_and(|argument| argument.starts_with("--working-directory="))
        {
            let value = argument.unwrap().trim_start_matches("--working-directory=");
            directory = Some(PathBuf::from(value));
        } else {
            command_started |= matches!(argument, Some("--terminal" | "--terminal-run"));
            command_args.push(args[index].clone());
        }
        index += 1;
    }
    Ok((directory, command_args))
}

struct TabBar {
    height: f32,
    tabs: Vec<(usize, Rectangle)>,
    controls: [Rectangle; 5],
}

impl TabBar {
    fn new(width: i32, height: i32, count: usize, active: usize) -> Self {
        let scale = ui::scale(width, height);
        let bar = (28.0 * scale).clamp(20.0, (width as f32 / 7.0).min(height as f32 / 4.0).max(20.0));
        let available = (width as f32 - bar * 5.0).max(1.0);
        let visible = ((available / (90.0 * scale)).floor() as usize).max(1).min(count.max(1));
        let first = active / visible * visible;
        let tab_width = (available / visible as f32).min(220.0 * scale);
        Self {
            height: bar,
            tabs: (first..(first + visible).min(count)).enumerate().map(|(slot, index)| (index, Rectangle::new(slot as f32 * tab_width, 0.0, tab_width, bar))).collect(),
            controls: std::array::from_fn(|index| Rectangle::new(available + index as f32 * bar, 0.0, bar, bar)),
        }
    }

    fn draw<D: RaylibDraw>(&self, d: &mut D, sessions: &[TermWindow], active: usize, width: i32, mouse: Vector2) {
        d.draw_rectangle(0, 0, width, self.height as i32, Color::new(17, 18, 23, 255));
        let font = (self.height * 0.43).max(9.0);
        for (index, rect) in &self.tabs {
            d.draw_rectangle_rec(*rect, if *index == active { Color::new(50, 64, 75, 255) } else { Color::new(31, 33, 40, 255) });
            let state = if sessions[*index].exited.is_some() { " (exited)" } else { "" };
            let label = ui::fit(&format!("Terminal {}{state}", index + 1), font, rect.width - 12.0);
            ui::draw_text_in(d, &label, *rect, 6.0, font, Color::RAYWHITE);
        }
        for (rect, label) in self.controls.iter().zip(["<", ">", "+", "Aa", "x"]) {
            ui::draw_text_centered(d, label, *rect, font, Color::RAYWHITE);
        }
        if let Some(index) = self.controls.iter().position(|rect| rect.check_collision_point_rec(mouse)) {
            let label = ["Previous terminal", "Next terminal", "New terminal", "Interface scale", "Close terminal"][index];
            let tip_width = (ui::measure(label, font) + 16.0).min(width as f32);
            let tip = Rectangle::new((self.controls[index].x + self.controls[index].width - tip_width).clamp(0.0, (width as f32 - tip_width).max(0.0)), self.height, tip_width, font + 12.0);
            d.draw_rectangle_rec(tip, Color::new(45, 48, 57, 255));
            ui::draw_text_in(d, label, tip, 8.0, font, Color::RAYWHITE);
        }
    }
}

fn new_session(program: &Path, args: &[OsString], width: i32, height: i32, directory: Option<&Path>, grid: Option<&Path>) -> Result<TermWindow, String> {
    let metrics = Metrics::new(width, height);
    let (cols, rows) = metrics.grid(width, height);
    let pty = Pty::spawn_in(program, args, cols, rows, directory, grid)
        .map_err(|error| format!("could not start the terminal session: {error}"))?;
    let mut term = Terminal::new(cols, rows, SCROLLBACK_LINES);
    term.set_default_colors(THEME_FG, THEME_BG);
    Ok(TermWindow {
        term, pty, metrics, offset: 0, selection: None, selecting: false,
        last_click: None, keys: KeyRepeat::default(),
        highlight: env::var("SELENITE_TERMINAL_HIGHLIGHT").map_or(true, |value| value != "0"),
        blink_from: 0.0, toast: None, bell_until: 0.0, started: Instant::now(),
        output_closed: false, exited: None, hold_open: false, held_button: None,
        last_reported: None, focused: true, title: "Selenite Terminal".to_owned(), saw_output: false,
        scrollbar_drag: None, pending_paste: None,
    })
}

/// Opens the terminal window and returns the exit status of the command it ran.
pub fn run(args: &[OsString]) -> Result<i32, String> {
    let (directory, args) = terminal_window_options(args)?;
    let (program, program_args) = child_command(&args)?;
    let frame_limit: Option<u32> = env::var("SELENITE_TERMINAL_FRAMES")
        .ok()
        .and_then(|value| value.parse().ok());
    let fixed_size = env::var("SELENITE_TERMINAL_SIZE")
        .ok()
        .and_then(|value| parse_size(&value));
    let (width, height) = fixed_size.unwrap_or(DEFAULT_SIZE);
    let (mut rl, thread) = raylib::init()
        .size(width, height)
        .title("Selenite Terminal")
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_exit_key(None);
    rl.set_window_min_size(240, 140);
    if fixed_size.is_none() {
        ui::fit_window_to_display(&mut rl, width, height);
    }
    ui::set_user_scale(settings::saved_ui_scale());
    ui::load_font(&thread);
    load_mono_font(true);

    let mut sessions = vec![new_session(&program, &program_args, rl.get_screen_width(), rl.get_screen_height(), directory.as_deref(), None)?];
    let mut active = 0usize;
    let mut scale_menu = ui::ScaleMenu::default();
    let mut suite_menu = crate::suite::Menu::default();

    let mut applied_cursor = MouseCursor::MOUSE_CURSOR_DEFAULT;
    let mut frames = 0u32;
    let mut code = 0;
    while !rl.window_should_close() {
        let now = rl.get_time();
        let suite_input = suite_menu.update(&mut rl, &thread, None, None);
        if suite_input { scale_menu.open = false; }
        let scale_input = suite_input || scale_menu.update(&rl);
        let mods = Mods {
            ctrl: rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL),
            shift: rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT),
            alt: rl.is_key_down(KeyboardKey::KEY_LEFT_ALT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_ALT),
        };
        let bar = TabBar::new(rl.get_screen_width(), rl.get_screen_height(), sessions.len(), active);
        let click = !scale_input && rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) && rl.get_mouse_position().y < bar.height;
        let control = click.then(|| bar.controls.iter().position(|rect| rect.check_collision_point_rec(rl.get_mouse_position()))).flatten();
        let new_tab = !scale_input && mods.ctrl && mods.shift && rl.is_key_pressed(KeyboardKey::KEY_T) || control == Some(2);
        let close_tab = !scale_input && mods.ctrl && mods.shift && rl.is_key_pressed(KeyboardKey::KEY_W) || control == Some(4);
        let mut tab_command = click || new_tab || close_tab;
        if new_tab && sessions.len() < 32 {
            let directory = fs::read_link(format!("/proc/{}/cwd", sessions[active].pty.child.id())).ok().or_else(|| env::current_dir().ok());
            match new_session(&program, &["--terminal".into()], rl.get_screen_width(), rl.get_screen_height(), directory.as_deref(), None) {
                Ok(session) => { sessions.push(session); active = sessions.len() - 1; }
                Err(error) => sessions[active].say(error, now),
            }
        }
        if close_tab {
            if sessions.len() == 1 { break; }
            sessions.remove(active);
            active = active.min(sessions.len() - 1);
        }
        if !scale_input && mods.ctrl && rl.is_key_pressed(KeyboardKey::KEY_TAB) || matches!(control, Some(0 | 1)) {
            let backwards = control == Some(0) || mods.shift && control.is_none();
            active = if backwards { (active + sessions.len() - 1) % sessions.len() } else { (active + 1) % sessions.len() };
            tab_command = true;
        }
        if control == Some(3) { scale_menu.open = true; }
        if click {
            if let Some((index, _)) = bar.tabs.iter().find(|(_, rect)| rect.check_collision_point_rec(rl.get_mouse_position())) { active = *index; }
        }
        for (index, session) in sessions.iter_mut().enumerate() {
            session.metrics = Metrics::new(rl.get_screen_width(), rl.get_screen_height());
            session.fit(rl.get_screen_width(), rl.get_screen_height() - bar.height as i32);
            let focused = index == active && rl.is_window_focused() && !scale_input;
            if focused != session.focused {
                session.focused = focused;
                if session.term.modes.focus_reporting { session.pty.write(if focused { "\x1b[I" } else { "\x1b[O" }); }
            }
            session.pump(&mut rl, &thread, now);
            session.pump_paste(now);
            if session.exited.is_none() {
                if let Some(status) = session.pty.exit_code() { session.exited = Some((status, now)); }
            }
        }
        let multiple = sessions.len() > 1;
        let window = &mut sessions[active];
        rl.set_window_title(&thread, &window.title);
        if let Some(scale) = ui::handle_scale_keys(&rl).or_else(|| ui::handle_scale_wheel(&rl)) {
            let _ = settings::save_ui_scale(scale);
            window.say(
                format!(
                    "Zoom {:.0}% (Ctrl +/- or Ctrl+wheel, Ctrl 0 resets)",
                    scale * 100.0
                ),
                now,
            );
        }
        let (screen_width, screen_height) = (rl.get_screen_width(), rl.get_screen_height());
        if font_size(screen_width, screen_height) != window.metrics.font {
            window.metrics = Metrics::new(screen_width, screen_height);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            if rl.is_window_maximized() {
                rl.restore_window();
            } else {
                rl.maximize_window();
            }
        }
        if window.hold_open && !scale_input && !tab_command && !multiple {
            if rl.get_char_pressed().is_some()
                || rl.is_key_pressed(KeyboardKey::KEY_ENTER)
                || rl.is_key_pressed(KeyboardKey::KEY_ESCAPE)
            {
                break;
            }
        } else if !scale_input && !tab_command && window.exited.is_none() {
            window.keyboard(&mut rl, now);
            let cursor = if rl.get_mouse_position().y >= bar.height || window.scrollbar_drag.is_some() { window.mouse_at(&mut rl, now, bar.height) } else { MouseCursor::MOUSE_CURSOR_DEFAULT };
            if cursor != applied_cursor {
                rl.set_mouse_cursor(cursor);
                applied_cursor = cursor;
            }
        }

        if window.exited.is_none() {
            if let Some(status) = window.pty.exit_code() {
                window.exited = Some((status, now));
            }
        }
        if let Some((status, since)) = window.exited {
            // Grandchildren can keep the terminal open, so wait only briefly for the output to end.
            if window.output_closed || now - since > 0.3 {
                code = status;
                if status != 0 && (window.started.elapsed() < QUICK_FAILURE || window.term.dump().contains("failed to start shell")) && !window.hold_open {
                    window.hold_open = true;
                } else if !window.hold_open && !multiple {
                    break;
                }
            }
        }

        let mut finished = false;
        let (screen_width, screen_height) = (rl.get_screen_width(), rl.get_screen_height());
        let mouse = rl.get_mouse_position();
        {
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(to_color(THEME_BG));
            {
                let mut clip = d.begin_scissor_mode(0, bar.height as i32, screen_width, screen_height - bar.height as i32);
                let camera = Camera2D { offset: Vector2::new(0.0, bar.height), target: Vector2::zero(), rotation: 0.0, zoom: 1.0 };
                let mut translated = clip.begin_mode2D(camera);
                window.draw(&mut translated, screen_width, screen_height - bar.height as i32, now);
            }
            let bar = TabBar::new(screen_width, screen_height, sessions.len(), active);
            bar.draw(&mut d, &sessions, active, screen_width, mouse);
            scale_menu.draw(&mut d, screen_width, screen_height);
            suite_menu.draw(&mut d, screen_width, screen_height);
            frames += 1;
            if frame_limit.is_some_and(|limit| frames >= limit) {
                if let Ok(target) = env::var("SELENITE_TERMINAL_SCREENSHOT") {
                    if !capture_screen(&target) {
                        eprintln!("selenite: could not save screenshot to {target}");
                    }
                }
                finished = true;
            }
        }
        if frames == 1 {
            if let Some(ready) = env::var_os("SELENITE_TERMINAL_READY") {
                let _ = fs::write(ready, "");
            }
        }
        if finished {
            break;
        }
    }
    if let Some(target) = env::var_os("SELENITE_TERMINAL_DUMP") {
        let _ = fs::write(target, sessions[active].term.dump());
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollbar_thumb_maps_to_history_endpoints() {
        let height = 400.0;
        let scrollback = 500;
        let (oldest, thumb) = scrollbar_geometry(height, 20, scrollback, scrollback).unwrap();
        assert_eq!(oldest, 0.0);
        assert_eq!(scrollbar_offset(oldest, height, thumb, scrollback), scrollback);

        let (live, _) = scrollbar_geometry(height, 20, scrollback, 0).unwrap();
        assert_eq!(scrollbar_offset(live, height, thumb, scrollback), 0);
        assert_eq!(scrollbar_offset(-10.0, height, thumb, scrollback), scrollback);
        assert_eq!(scrollbar_offset(height, height, thumb, scrollback), 0);
        assert!(scrollbar_geometry(height, 20, 0, 0).is_none());
    }

    #[test]
    fn tab_controls_fit_and_keep_the_active_tab_visible() {
        for (width, height) in [(240, 140), (420, 300), (960, 720), (3840, 2160)] {
            for count in [1, 2, 10, 32] {
                for active in 0..count {
                    let bar = TabBar::new(width, height, count, active);
                    assert!(bar.tabs.iter().any(|(index, _)| *index == active));
                    let rects: Vec<_> = bar.tabs.iter().map(|(_, rect)| *rect).chain(bar.controls).collect();
                    for (index, rect) in rects.iter().enumerate() {
                        assert!(rect.x >= 0.0 && rect.x + rect.width <= width as f32 + 0.01);
                        for other in &rects[index + 1..] { assert!(rect.x + rect.width <= other.x + 0.01 || other.x + other.width <= rect.x + 0.01); }
                    }
                }
            }
        }
    }

    fn bytes(key: KeyboardKey, mods: Mods, app: bool) -> String {
        String::from_utf8(named_key_bytes(key, mods, app).unwrap()).unwrap()
    }

    #[test]
    fn encodes_named_keys_like_xterm() {
        let plain = Mods::default();
        assert_eq!(bytes(KeyboardKey::KEY_UP, plain, false), "\x1b[A");
        assert_eq!(bytes(KeyboardKey::KEY_UP, plain, true), "\x1bOA");
        assert_eq!(
            bytes(
                KeyboardKey::KEY_LEFT,
                Mods {
                    ctrl: true,
                    ..plain
                },
                true
            ),
            "\x1b[1;5D"
        );
        assert_eq!(
            bytes(
                KeyboardKey::KEY_HOME,
                Mods {
                    shift: true,
                    alt: true,
                    ctrl: false
                },
                false
            ),
            "\x1b[1;4H"
        );
        assert_eq!(bytes(KeyboardKey::KEY_DELETE, plain, false), "\x1b[3~");
        assert_eq!(
            bytes(KeyboardKey::KEY_PAGE_UP, Mods { alt: true, ..plain }, false),
            "\x1b[5;3~"
        );
        assert_eq!(bytes(KeyboardKey::KEY_F1, plain, false), "\x1bOP");
        assert_eq!(
            bytes(
                KeyboardKey::KEY_F5,
                Mods {
                    ctrl: true,
                    ..plain
                },
                false
            ),
            "\x1b[15;5~"
        );
        assert_eq!(bytes(KeyboardKey::KEY_ENTER, plain, false), "\r");
        assert_eq!(bytes(KeyboardKey::KEY_BACKSPACE, plain, false), "\x7f");
        assert_eq!(
            bytes(
                KeyboardKey::KEY_BACKSPACE,
                Mods { alt: true, ..plain },
                false
            ),
            "\x1b\x7f"
        );
        assert_eq!(
            bytes(
                KeyboardKey::KEY_TAB,
                Mods {
                    shift: true,
                    ..plain
                },
                false
            ),
            "\x1b[Z"
        );
        assert!(named_key_bytes(KeyboardKey::KEY_A, plain, false).is_none());
    }

    #[test]
    fn pastes_are_normalized_and_bracketed_safely() {
        assert_eq!(paste_bytes("a\r\nb\nc", false), b"a\rb\rc");
        assert_eq!(
            paste_bytes("x\x1b[201~y", true),
            b"\x1b[200~x[201~y\x1b[201~"
        );
        assert_eq!(
            paste_bytes("a\x13b\tc\x00d\x7f", false),
            b"ab\tcd"
        );
    }

    #[test]
    fn finds_urls_under_the_pointer() {
        let line: Vec<char> = "see (https://example.com/a?b=1). ok".chars().collect();
        assert_eq!(
            url_at(&line, 10).as_deref(),
            Some("https://example.com/a?b=1")
        );
        assert_eq!(url_at(&line, 1), None);
        assert_eq!(url_at(&line, 100), None);
        let plain: Vec<char> = "ftp://x and http://y.z".chars().collect();
        assert_eq!(url_at(&plain, 2), None);
        assert_eq!(url_at(&plain, 17).as_deref(), Some("http://y.z"));
    }

    #[test]
    fn the_font_scales_with_the_window_and_zoom() {
        let base = font_size(960, 720);
        assert_eq!(base, BASE_FONT);
        assert!(font_size(480, 360) < base);
        assert!(font_size(1920, 1080) > base);
        assert!(font_size(2880, 1620) > font_size(1920, 1080));
        assert!(font_size(100, 100) >= 8.0);
        ui::set_user_scale(2.0);
        assert_eq!(font_size(960, 720), 2.0 * BASE_FONT);
        ui::set_user_scale(1.0);
    }

    #[test]
    fn the_grid_follows_the_window_and_zoom() {
        let small = Metrics {
            font: 10.0,
            cell_w: 5.0,
            cell_h: 10.0,
            pad: 4.0,
        };
        let large = Metrics {
            font: 20.0,
            cell_w: 10.0,
            cell_h: 20.0,
            pad: 8.0,
        };
        assert_eq!(small.grid(1000, 600), (198, 59));
        assert_eq!(large.grid(1000, 600), (98, 29));
        assert_eq!(large.grid(1, 1), (2, 1));
    }

    #[test]
    fn a_session_runs_a_program_on_a_real_pty() {
        let args: Vec<OsString> = [
            "-c",
            "stty size; printf 'a\\033[31mb\\033[0m'; read line; echo got:$line; exit 7",
        ]
        .iter()
        .map(OsString::from)
        .collect();
        let mut pty = Pty::spawn(Path::new("/bin/sh"), &args, 33, 11).unwrap();
        let mut term = Terminal::new(33, 11, 100);
        let mut answered = false;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match pty.output.recv_timeout(Duration::from_millis(100)) {
                Ok(chunk) => term.feed(&chunk),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            // Typing before the program prints would echo ahead of its output.
            if !answered && term.dump().contains('b') {
                pty.write("hello\n");
                answered = true;
            }
            assert!(Instant::now() < deadline, "the program never finished");
        }
        let screen = term.dump();
        assert!(screen.starts_with("11 33\n"), "{screen:?}");
        assert!(screen.contains("got:hello"), "{screen:?}");
        let mut code = pty.exit_code();
        for _ in 0..100 {
            if code.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
            code = pty.exit_code();
        }
        assert_eq!(code, Some(7));
        let b = term.row_abs(term.scrollback_len() + 1).unwrap().cells[1];
        assert_eq!((b.ch, b.fg), ('b', TermColor::Indexed(1)));
    }

    #[test]
    fn rejects_commands_other_than_the_terminal_modes() {
        assert!(child_command(&["--edit".into()]).is_err());
        let (_, args) = child_command(&[]).unwrap();
        assert_eq!(args, ["--terminal"]);
        let (_, args) = child_command(&["--terminal-run".into(), "x".into()]).unwrap();
        assert_eq!(args, ["--terminal-run", "x"]);
    }

    #[test]
    fn consumes_xdg_working_directory_before_terminal_command() {
        let args =
            ["--working-directory", "/tmp", "--terminal", "printf", "ok"].map(OsString::from);
        let (directory, command) = terminal_window_options(&args).unwrap();
        assert_eq!(directory, Some(PathBuf::from("/tmp")));
        assert_eq!(command, ["--terminal", "printf", "ok"].map(OsString::from));

        let args = ["--terminal", "--working-directory", "literal"].map(OsString::from);
        let (directory, command) = terminal_window_options(&args).unwrap();
        assert_eq!(directory, None);
        assert_eq!(command, args);
    }
}

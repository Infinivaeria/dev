//! Terminal emulation: an xterm-compatible parser and screen model.
//!
//! This module knows nothing about windows or pseudo-terminals. Bytes written by a program go
//! into [`Terminal::feed`]; the renderer reads the resulting cells, and anything the terminal
//! must answer (device attributes, cursor reports) is collected in [`Terminal::take_responses`].

use std::collections::VecDeque;

pub const BOLD: u8 = 1;
pub const DIM: u8 = 2;
pub const ITALIC: u8 = 4;
pub const UNDERLINE: u8 = 8;
pub const INVERSE: u8 = 16;
pub const STRIKE: u8 = 32;
pub const HIDDEN: u8 = 64;

const MAX_PARAM: u32 = 1_000_000;
const MAX_OSC: usize = 4096;
const MAX_PARAMS: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TermColor {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    /// `'\0'` marks the right half of a double-width character.
    pub ch: char,
    pub fg: TermColor,
    pub bg: TermColor,
    pub attrs: u8,
}

impl Cell {
    pub const fn blank(bg: TermColor) -> Self {
        Self {
            ch: ' ',
            fg: TermColor::Default,
            bg,
            attrs: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub cells: Vec<Cell>,
    /// The text continues on the next row (soft wrap).
    pub wrapped: bool,
}

impl Row {
    fn new(cols: usize, bg: TermColor) -> Self {
        Self {
            cells: vec![Cell::blank(bg); cols],
            wrapped: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
struct Pen {
    fg: TermColor,
    bg: TermColor,
    attrs: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MouseMode {
    #[default]
    Off,
    /// Button presses and releases (1000).
    Press,
    /// Plus motion while a button is held (1002).
    Drag,
    /// Plus all motion (1003).
    Any,
}

#[derive(Clone, Debug)]
pub struct Modes {
    pub app_cursor: bool,
    pub autowrap: bool,
    pub origin: bool,
    pub insert: bool,
    pub newline: bool,
    pub cursor_visible: bool,
    pub reverse: bool,
    pub bracketed_paste: bool,
    pub focus_reporting: bool,
    pub mouse: MouseMode,
    pub mouse_sgr: bool,
    /// Wheel scrolls arrow keys on the alternate screen when no mouse mode is set.
    pub alt_scroll: bool,
}

impl Default for Modes {
    fn default() -> Self {
        Self {
            app_cursor: false,
            autowrap: true,
            origin: false,
            insert: false,
            newline: false,
            cursor_visible: true,
            reverse: false,
            bracketed_paste: false,
            focus_reporting: false,
            mouse: MouseMode::Off,
            mouse_sgr: false,
            alt_scroll: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Charset {
    Ascii,
    LineDrawing,
}

#[derive(Clone, Copy, Debug)]
struct SavedCursor {
    x: usize,
    y: usize,
    pen: Pen,
    pending_wrap: bool,
    origin: bool,
    charsets: [Charset; 2],
    shift: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    Osc,
    StringIgnore,
}

/// Display width of `c` in cells: 0 for combining marks, 2 for East Asian wide characters.
pub fn char_width(c: char) -> usize {
    let u = c as u32;
    if u < 0x20 || (0x7f..0xa0).contains(&u) {
        return 0;
    }
    if u < 0x300 {
        return 1;
    }
    let zero = matches!(u,
        0x300..=0x36f | 0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0x200b..=0x200f
        | 0x2028..=0x202e | 0x2060..=0x2064 | 0x20d0..=0x20ff | 0xfe00..=0xfe0f
        | 0xfe20..=0xfe2f | 0xfeff);
    if zero {
        return 0;
    }
    let wide = matches!(u,
        0x1100..=0x115f | 0x2e80..=0x303e | 0x3041..=0x33ff | 0x3400..=0x4dbf
        | 0x4e00..=0x9fff | 0xa000..=0xa4cf | 0xac00..=0xd7a3 | 0xf900..=0xfaff
        | 0xfe30..=0xfe6f | 0xff00..=0xff60 | 0xffe0..=0xffe6 | 0x1f300..=0x1f64f
        | 0x1f900..=0x1f9ff | 0x20000..=0x3fffd);
    if wide {
        2
    } else {
        1
    }
}

const LINE_DRAWING: [char; 31] = [
    '◆', '▒', '␉', '␌', '␍', '␊', '°', '±', '␤', '␋', '┘', '┐', '┌', '└', '┼', '⎺', '⎻', '─', '⎼',
    '⎽', '├', '┤', '┴', '┬', '│', '≤', '≥', 'π', '≠', '£', '·',
];

/// The xterm 256-colour palette entry `index` (0-15 are the configurable ANSI colours).
pub fn palette_rgb(index: u8) -> (u8, u8, u8) {
    const ANSI: [(u8, u8, u8); 16] = [
        (60, 64, 80),
        (204, 92, 92),
        (126, 190, 110),
        (222, 190, 100),
        (92, 140, 220),
        (186, 120, 210),
        (86, 190, 200),
        (210, 214, 222),
        (96, 102, 120),
        (240, 120, 120),
        (150, 222, 130),
        (250, 222, 130),
        (130, 170, 250),
        (220, 150, 240),
        (120, 220, 230),
        (245, 247, 250),
    ];
    match index {
        0..=15 => ANSI[index as usize],
        16..=231 => {
            let n = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (level(n / 36), level(n / 6 % 6), level(n % 6))
        }
        _ => {
            let v = 8 + (index - 232) * 10;
            (v, v, v)
        }
    }
}

/// Builds the escape sequence a program receives for a mouse event, or `None` when the
/// legacy encoding cannot express the position. `button` already includes modifier and motion bits.
pub fn encode_mouse(
    button: u32,
    col: usize,
    row: usize,
    release: bool,
    sgr: bool,
) -> Option<Vec<u8>> {
    if sgr {
        let end = if release { 'm' } else { 'M' };
        return Some(format!("\x1b[<{button};{};{}{end}", col + 1, row + 1).into_bytes());
    }
    let (x, y) = (col + 33, row + 33);
    if x > 255 || y > 255 {
        return None;
    }
    let code = if release { 3 | (button & !3) } else { button };
    Some(vec![0x1b, b'[', b'M', (code + 32) as u8, x as u8, y as u8])
}

pub struct Terminal {
    cols: usize,
    rows: usize,
    screen: Vec<Row>,
    other: Vec<Row>,
    alt: bool,
    scrollback: VecDeque<Row>,
    scrollback_limit: usize,
    trimmed: usize,
    x: usize,
    y: usize,
    pending_wrap: bool,
    pen: Pen,
    saved: Option<SavedCursor>,
    saved_main: Option<SavedCursor>,
    top: usize,
    bottom: usize,
    tabs: Vec<bool>,
    pub modes: Modes,
    charsets: [Charset; 2],
    shift: usize,
    cursor_style: u8,
    title: Option<String>,
    responses: Vec<u8>,
    last_char: Option<char>,
    bell: bool,
    default_fg: (u8, u8, u8),
    default_bg: (u8, u8, u8),
    // parser
    state: State,
    params: Vec<Vec<u32>>,
    number: Option<u32>,
    private: u8,
    intermediate: u8,
    osc: Vec<u8>,
    utf8: u32,
    utf8_left: u8,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize, scrollback_limit: usize) -> Self {
        let (cols, rows) = (cols.max(2), rows.max(1));
        let mut terminal = Self {
            cols,
            rows,
            screen: Vec::new(),
            other: Vec::new(),
            alt: false,
            scrollback: VecDeque::new(),
            scrollback_limit,
            trimmed: 0,
            x: 0,
            y: 0,
            pending_wrap: false,
            pen: Pen::default(),
            saved: None,
            saved_main: None,
            top: 0,
            bottom: rows - 1,
            tabs: Vec::new(),
            modes: Modes::default(),
            charsets: [Charset::Ascii; 2],
            shift: 0,
            cursor_style: 0,
            title: None,
            responses: Vec::new(),
            last_char: None,
            bell: false,
            default_fg: (216, 218, 224),
            default_bg: (20, 21, 26),
            state: State::Ground,
            params: Vec::new(),
            number: None,
            private: 0,
            intermediate: 0,
            osc: Vec::new(),
            utf8: 0,
            utf8_left: 0,
        };
        terminal.reset_screens();
        terminal
    }

    fn reset_screens(&mut self) {
        self.screen = (0..self.rows)
            .map(|_| Row::new(self.cols, TermColor::Default))
            .collect();
        self.other = (0..self.rows)
            .map(|_| Row::new(self.cols, TermColor::Default))
            .collect();
        self.tabs = (0..self.cols).map(|c| c % 8 == 0).collect();
    }

    /// The colours answered to OSC 10/11 queries.
    pub fn set_default_colors(&mut self, fg: (u8, u8, u8), bg: (u8, u8, u8)) {
        self.default_fg = fg;
        self.default_bg = bg;
    }

    // ------------------------------------------------------------ queries --

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cursor(&self) -> (usize, usize) {
        (self.x, self.y)
    }

    pub fn cursor_style(&self) -> u8 {
        self.cursor_style
    }

    pub fn on_alt_screen(&self) -> bool {
        self.alt
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Lines dropped from the far end of the scrollback since the last call.
    pub fn take_trimmed(&mut self) -> usize {
        std::mem::take(&mut self.trimmed)
    }

    pub fn take_responses(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.responses)
    }

    pub fn take_title(&mut self) -> Option<String> {
        self.title.take()
    }

    pub fn take_bell(&mut self) -> bool {
        std::mem::take(&mut self.bell)
    }

    /// Row `abs`, counting scrollback first and then the screen.
    pub fn row_abs(&self, abs: usize) -> Option<&Row> {
        if abs < self.scrollback.len() {
            self.scrollback.get(abs)
        } else {
            self.screen.get(abs - self.scrollback.len())
        }
    }

    /// The row shown at `view_row` when scrolled `offset` lines back (the alternate screen never scrolls back).
    pub fn view_row(&self, view_row: usize, offset: usize) -> Option<&Row> {
        let offset = if self.alt {
            0
        } else {
            offset.min(self.scrollback.len())
        };
        self.row_abs(self.scrollback.len() - offset + view_row)
    }

    /// Text between two (column, absolute row) positions, inclusive, in reading order.
    pub fn text_range(&self, from: (usize, usize), to: (usize, usize)) -> String {
        let mut out = String::new();
        for abs in from.1..=to.1 {
            let Some(row) = self.row_abs(abs) else { break };
            let start = if abs == from.1 { from.0 } else { 0 };
            let whole = abs != to.1;
            let end = if whole {
                row.cells.len()
            } else {
                (to.0 + 1).min(row.cells.len())
            };
            let mut line: String = row
                .cells
                .get(start..end.max(start))
                .unwrap_or(&[])
                .iter()
                .filter(|cell| cell.ch != '\0')
                .map(|cell| cell.ch)
                .collect();
            if end >= row.cells.len() {
                line.truncate(line.trim_end().len());
            }
            out.push_str(&line);
            if whole && !row.wrapped {
                out.push('\n');
            }
        }
        out
    }

    /// Inclusive column range of the word (or run of blanks / punctuation) at `col`.
    pub fn word_at(&self, col: usize, abs: usize) -> (usize, usize) {
        let Some(row) = self.row_abs(abs) else {
            return (col, col);
        };
        let class = |cell: &Cell| match cell.ch {
            ' ' | '\0' => 0,
            c if c.is_alphanumeric() || "_-./~:@%+=?&#".contains(c) => 1,
            _ => 2,
        };
        let col = col.min(row.cells.len().saturating_sub(1));
        let Some(here) = row.cells.get(col).map(class) else {
            return (col, col);
        };
        let (mut start, mut end) = (col, col);
        while start > 0 && class(&row.cells[start - 1]) == here {
            start -= 1;
        }
        while end + 1 < row.cells.len() && class(&row.cells[end + 1]) == here {
            end += 1;
        }
        (start, end)
    }

    /// Whole text of the screen and scrollback, for tests and diagnostics.
    pub fn dump(&self) -> String {
        let last = self.scrollback.len() + self.rows;
        let mut text = self.text_range((0, 0), (self.cols.saturating_sub(1), last - 1));
        text.truncate(text.trim_end().len());
        text.push('\n');
        text
    }

    // ----------------------------------------------------------- resizing --

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let (cols, rows) = (cols.max(2), rows.max(1));
        if cols == self.cols && rows == self.rows {
            return;
        }
        for row in self.screen.iter_mut().chain(self.other.iter_mut()) {
            resize_row(row, cols);
        }
        if self.alt {
            // Only the bottom changes on the alternate screen; the saved main screen follows suit.
            self.screen.truncate(rows);
            while self.screen.len() < rows {
                self.screen.push(Row::new(cols, TermColor::Default));
            }
            self.other.truncate(rows);
            while self.other.len() < rows {
                self.other.push(Row::new(cols, TermColor::Default));
            }
        } else {
            while self.screen.len() > rows {
                if self.y + 1 < self.screen.len() {
                    self.screen.pop();
                } else {
                    let row = self.screen.remove(0);
                    self.push_scrollback(row);
                    self.y -= 1;
                }
            }
            while self.screen.len() < rows {
                match self.scrollback.pop_back() {
                    Some(mut row) => {
                        resize_row(&mut row, cols);
                        self.screen.insert(0, row);
                        self.y += 1;
                    }
                    None => self.screen.push(Row::new(cols, TermColor::Default)),
                }
            }
            self.other.truncate(rows);
            while self.other.len() < rows {
                self.other.push(Row::new(cols, TermColor::Default));
            }
        }
        self.cols = cols;
        self.rows = rows;
        self.top = 0;
        self.bottom = rows - 1;
        let known = self.tabs.len();
        self.tabs.resize(cols, false);
        for (column, stop) in self.tabs.iter_mut().enumerate().skip(known) {
            *stop = column % 8 == 0;
        }
        self.x = self.x.min(cols - 1);
        self.y = self.y.min(rows - 1);
        self.pending_wrap = false;
    }

    // ------------------------------------------------------------ parsing --

    pub fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.byte(byte);
        }
    }

    fn byte(&mut self, b: u8) {
        // CAN and SUB abort any sequence; ESC starts a new one (finishing an OSC string first).
        match b {
            0x18 | 0x1a => {
                self.state = State::Ground;
                return;
            }
            0x1b => {
                if self.state == State::Osc {
                    self.finish_osc();
                }
                self.state = State::Escape;
                self.intermediate = 0;
                self.utf8_left = 0;
                return;
            }
            _ => {}
        }
        match self.state {
            State::Ground => self.ground(b),
            State::Escape => match b {
                b'[' => {
                    self.params = vec![Vec::new()];
                    self.number = None;
                    self.private = 0;
                    self.intermediate = 0;
                    self.state = State::CsiParam;
                }
                b']' => {
                    self.osc.clear();
                    self.state = State::Osc;
                }
                b'P' | b'X' | b'^' | b'_' => self.state = State::StringIgnore,
                0x00..=0x1f => self.control(b),
                0x20..=0x2f => {
                    self.intermediate = b;
                    self.state = State::EscapeIntermediate;
                }
                0x7f => {}
                _ => {
                    self.escape(0, b);
                    self.state = State::Ground;
                }
            },
            State::EscapeIntermediate => match b {
                0x00..=0x1f => self.control(b),
                0x20..=0x2f => self.intermediate = b,
                0x30..=0x7e => {
                    self.escape(self.intermediate, b);
                    self.state = State::Ground;
                }
                _ => {}
            },
            State::CsiParam => match b {
                0x00..=0x1f => self.control(b),
                b'0'..=b'9' => {
                    let digit = u32::from(b - b'0');
                    let value = self
                        .number
                        .unwrap_or(0)
                        .saturating_mul(10)
                        .saturating_add(digit);
                    self.number = Some(value.min(MAX_PARAM));
                }
                b':' => {
                    let value = self.number.take().unwrap_or(0);
                    if let Some(group) = self.params.last_mut() {
                        group.push(value);
                    }
                }
                b';' => {
                    if let Some(value) = self.number.take() {
                        if let Some(group) = self.params.last_mut() {
                            group.push(value);
                        }
                    }
                    if self.params.len() < MAX_PARAMS {
                        self.params.push(Vec::new());
                    }
                }
                0x3c..=0x3f => {
                    if self.params.len() == 1 && self.params[0].is_empty() && self.number.is_none()
                    {
                        self.private = b;
                    } else {
                        self.state = State::CsiIgnore;
                    }
                }
                0x20..=0x2f => {
                    self.intermediate = b;
                    self.state = State::CsiIntermediate;
                }
                0x40..=0x7e => {
                    self.finish_params();
                    self.csi(b);
                    self.state = State::Ground;
                }
                _ => {}
            },
            State::CsiIntermediate => match b {
                0x00..=0x1f => self.control(b),
                0x20..=0x2f => self.intermediate = b,
                0x40..=0x7e => {
                    self.finish_params();
                    self.csi(b);
                    self.state = State::Ground;
                }
                _ => self.state = State::CsiIgnore,
            },
            State::CsiIgnore => match b {
                0x00..=0x1f => self.control(b),
                0x40..=0x7e => self.state = State::Ground,
                _ => {}
            },
            State::Osc => match b {
                0x07 => {
                    self.finish_osc();
                    self.state = State::Ground;
                }
                0x00..=0x1f => {}
                _ => {
                    if self.osc.len() < MAX_OSC {
                        self.osc.push(b);
                    }
                }
            },
            State::StringIgnore => {}
        }
    }

    fn finish_params(&mut self) {
        if let Some(value) = self.number.take() {
            if let Some(group) = self.params.last_mut() {
                group.push(value);
            }
        }
    }

    fn ground(&mut self, b: u8) {
        if self.utf8_left > 0 {
            if b & 0xc0 == 0x80 {
                self.utf8 = (self.utf8 << 6) | u32::from(b & 0x3f);
                self.utf8_left -= 1;
                if self.utf8_left == 0 {
                    let c = char::from_u32(self.utf8).unwrap_or('\u{fffd}');
                    self.print(c);
                }
                return;
            }
            self.utf8_left = 0;
            self.print('\u{fffd}');
        }
        match b {
            0x00..=0x1f => self.control(b),
            0x20..=0x7e => self.print(b as char),
            0x7f => {}
            0xc2..=0xdf => (self.utf8, self.utf8_left) = (u32::from(b & 0x1f), 1),
            0xe0..=0xef => (self.utf8, self.utf8_left) = (u32::from(b & 0x0f), 2),
            0xf0..=0xf4 => (self.utf8, self.utf8_left) = (u32::from(b & 0x07), 3),
            _ => self.print('\u{fffd}'),
        }
    }

    fn control(&mut self, b: u8) {
        match b {
            0x07 => self.bell = true,
            0x08 => {
                self.x = self.x.saturating_sub(1);
                self.pending_wrap = false;
            }
            0x09 => self.tab_forward(1),
            0x0a..=0x0c => {
                self.line_feed();
                if self.modes.newline {
                    self.x = 0;
                }
            }
            0x0d => {
                self.x = 0;
                self.pending_wrap = false;
            }
            0x0e => self.shift = 1,
            0x0f => self.shift = 0,
            _ => {}
        }
    }

    // ----------------------------------------------------------- printing --

    fn blank(&self) -> Cell {
        Cell::blank(self.pen.bg)
    }

    fn blank_row(&self) -> Row {
        Row::new(self.cols, self.pen.bg)
    }

    fn print(&mut self, c: char) {
        let c = if self.charsets[self.shift] == Charset::LineDrawing
            && ('\u{60}'..='\u{7e}').contains(&c)
        {
            LINE_DRAWING[c as usize - 0x60]
        } else {
            c
        };
        let width = char_width(c);
        if width == 0 {
            return;
        }
        if self.pending_wrap && self.modes.autowrap {
            self.screen[self.y].wrapped = true;
            self.x = 0;
            self.line_feed();
        }
        self.pending_wrap = false;
        if width == 2 && self.x + 1 >= self.cols {
            if !self.modes.autowrap {
                return;
            }
            let blank = self.blank();
            self.screen[self.y].cells[self.x] = blank;
            self.screen[self.y].wrapped = true;
            self.x = 0;
            self.line_feed();
        }
        if self.modes.insert {
            self.insert_cells(width);
        }
        self.clear_wide_at(self.x);
        if width == 2 {
            self.clear_wide_at(self.x + 1);
        }
        let cell = Cell {
            ch: c,
            fg: self.pen.fg,
            bg: self.pen.bg,
            attrs: self.pen.attrs,
        };
        let row = &mut self.screen[self.y];
        row.cells[self.x] = cell;
        if width == 2 {
            row.cells[self.x + 1] = Cell { ch: '\0', ..cell };
        }
        self.last_char = Some(c);
        self.x += width;
        if self.x >= self.cols {
            self.x = self.cols - 1;
            self.pending_wrap = true;
        }
    }

    /// Overwriting half of a double-width character blanks the other half.
    fn clear_wide_at(&mut self, x: usize) {
        let blank = self.blank();
        let row = &mut self.screen[self.y].cells;
        if x >= row.len() {
            return;
        }
        if row[x].ch == '\0' && x > 0 {
            row[x - 1] = blank;
        }
        if x + 1 < row.len() && row[x + 1].ch == '\0' && row[x].ch != '\0' {
            row[x + 1] = blank;
        }
    }

    // ----------------------------------------------------------- movement --

    fn line_feed(&mut self) {
        if self.y == self.bottom {
            self.scroll_up(1);
        } else if self.y + 1 < self.rows {
            self.y += 1;
        }
        self.pending_wrap = false;
    }

    fn reverse_index(&mut self) {
        if self.y == self.top {
            self.scroll_down(1);
        } else if self.y > 0 {
            self.y -= 1;
        }
        self.pending_wrap = false;
    }

    fn push_scrollback(&mut self, row: Row) {
        if self.scrollback_limit == 0 {
            return;
        }
        if self.scrollback.len() >= self.scrollback_limit {
            self.scrollback.pop_front();
            self.trimmed += 1;
        }
        self.scrollback.push_back(row);
    }

    fn scroll_up(&mut self, n: usize) {
        let n = n.min(self.bottom - self.top + 1);
        for _ in 0..n {
            let row = self.screen.remove(self.top);
            let blank = self.blank_row();
            self.screen.insert(self.bottom, blank);
            if !self.alt && self.top == 0 {
                self.push_scrollback(row);
            }
        }
    }

    fn scroll_down(&mut self, n: usize) {
        let n = n.min(self.bottom - self.top + 1);
        for _ in 0..n {
            self.screen.remove(self.bottom);
            let blank = self.blank_row();
            self.screen.insert(self.top, blank);
        }
    }

    fn set_cursor(&mut self, row: usize, col: usize) {
        let (low, high) = if self.modes.origin {
            (self.top, self.bottom)
        } else {
            (0, self.rows - 1)
        };
        let row = if self.modes.origin {
            row + self.top
        } else {
            row
        };
        self.y = row.clamp(low, high);
        self.x = col.min(self.cols - 1);
        self.pending_wrap = false;
    }

    fn tab_forward(&mut self, n: usize) {
        for _ in 0..n {
            let mut x = self.x + 1;
            while x < self.cols - 1 && !self.tabs[x] {
                x += 1;
            }
            self.x = x.min(self.cols - 1);
        }
        self.pending_wrap = false;
    }

    fn tab_back(&mut self, n: usize) {
        for _ in 0..n {
            let mut x = self.x.saturating_sub(1);
            while x > 0 && !self.tabs[x] {
                x -= 1;
            }
            self.x = x;
        }
        self.pending_wrap = false;
    }

    // ------------------------------------------------------------ editing --

    fn erase_cells(&mut self, y: usize, from: usize, to: usize) {
        let blank = self.blank();
        let cols = self.cols;
        let row = &mut self.screen[y];
        let (from, to) = (from.min(cols), to.min(cols));
        if from >= to {
            return;
        }
        if from > 0 && row.cells[from].ch == '\0' {
            row.cells[from - 1] = blank;
        }
        if to < cols && row.cells[to].ch == '\0' {
            row.cells[to] = blank;
        }
        for cell in &mut row.cells[from..to] {
            *cell = blank;
        }
        if to == cols {
            row.wrapped = false;
        }
    }

    fn insert_cells(&mut self, n: usize) {
        let n = n.min(self.cols - self.x);
        let blank = self.blank();
        let x = self.x;
        let cells = &mut self.screen[self.y].cells;
        for _ in 0..n {
            cells.pop();
            cells.insert(x, blank);
        }
    }

    fn delete_cells(&mut self, n: usize) {
        let n = n.min(self.cols - self.x);
        let blank = self.blank();
        let x = self.x;
        let cells = &mut self.screen[self.y].cells;
        for _ in 0..n {
            cells.remove(x);
            cells.push(blank);
        }
        self.pending_wrap = false;
    }

    fn insert_lines(&mut self, n: usize) {
        if self.y < self.top || self.y > self.bottom {
            return;
        }
        let n = n.min(self.bottom - self.y + 1);
        for _ in 0..n {
            self.screen.remove(self.bottom);
            let blank = self.blank_row();
            self.screen.insert(self.y, blank);
        }
        self.x = 0;
        self.pending_wrap = false;
    }

    fn delete_lines(&mut self, n: usize) {
        if self.y < self.top || self.y > self.bottom {
            return;
        }
        let n = n.min(self.bottom - self.y + 1);
        for _ in 0..n {
            self.screen.remove(self.y);
            let blank = self.blank_row();
            self.screen.insert(self.bottom, blank);
        }
        self.x = 0;
        self.pending_wrap = false;
    }

    fn erase_display(&mut self, mode: u32) {
        match mode {
            0 => {
                self.erase_cells(self.y, self.x, self.cols);
                for y in self.y + 1..self.rows {
                    self.erase_cells(y, 0, self.cols);
                }
            }
            1 => {
                for y in 0..self.y {
                    self.erase_cells(y, 0, self.cols);
                }
                self.erase_cells(self.y, 0, self.x + 1);
            }
            2 => {
                for y in 0..self.rows {
                    self.erase_cells(y, 0, self.cols);
                }
            }
            3 => self.scrollback.clear(),
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: u32) {
        match mode {
            0 => self.erase_cells(self.y, self.x, self.cols),
            1 => self.erase_cells(self.y, 0, self.x + 1),
            2 => self.erase_cells(self.y, 0, self.cols),
            _ => {}
        }
    }

    // ------------------------------------------------------------- state --

    fn save_cursor(&self) -> SavedCursor {
        SavedCursor {
            x: self.x,
            y: self.y,
            pen: self.pen,
            pending_wrap: self.pending_wrap,
            origin: self.modes.origin,
            charsets: self.charsets,
            shift: self.shift,
        }
    }

    fn restore_cursor(&mut self, saved: Option<SavedCursor>) {
        match saved {
            Some(saved) => {
                self.x = saved.x.min(self.cols - 1);
                self.y = saved.y.min(self.rows - 1);
                self.pen = saved.pen;
                self.pending_wrap = saved.pending_wrap;
                self.modes.origin = saved.origin;
                self.charsets = saved.charsets;
                self.shift = saved.shift;
            }
            None => {
                self.x = 0;
                self.y = 0;
                self.pending_wrap = false;
                self.modes.origin = false;
                self.pen = Pen::default();
            }
        }
    }

    fn enter_alt(&mut self, save_cursor: bool) {
        if self.alt {
            return;
        }
        if save_cursor {
            self.saved_main = Some(self.save_cursor());
        }
        std::mem::swap(&mut self.screen, &mut self.other);
        self.alt = true;
        let blank = Row::new(self.cols, TermColor::Default);
        self.screen.iter_mut().for_each(|row| *row = blank.clone());
        self.top = 0;
        self.bottom = self.rows - 1;
    }

    fn leave_alt(&mut self, restore_cursor: bool) {
        if !self.alt {
            return;
        }
        std::mem::swap(&mut self.screen, &mut self.other);
        self.alt = false;
        self.top = 0;
        self.bottom = self.rows - 1;
        if restore_cursor {
            let saved = self.saved_main.take();
            self.restore_cursor(saved);
        }
    }

    fn full_reset(&mut self) {
        let (cols, rows, limit) = (self.cols, self.rows, self.scrollback_limit);
        let (fg, bg) = (self.default_fg, self.default_bg);
        *self = Self::new(cols, rows, limit);
        self.default_fg = fg;
        self.default_bg = bg;
    }

    fn soft_reset(&mut self) {
        self.modes = Modes::default();
        self.pen = Pen::default();
        self.charsets = [Charset::Ascii; 2];
        self.shift = 0;
        self.top = 0;
        self.bottom = self.rows - 1;
        self.saved = None;
        self.cursor_style = 0;
    }

    // ------------------------------------------------------ escape codes --

    fn escape(&mut self, intermediate: u8, b: u8) {
        match (intermediate, b) {
            (0, b'7') => self.saved = Some(self.save_cursor()),
            (0, b'8') => {
                let saved = self.saved;
                self.restore_cursor(saved);
            }
            (0, b'D') => self.line_feed(),
            (0, b'E') => {
                self.line_feed();
                self.x = 0;
            }
            (0, b'H') => {
                if let Some(stop) = self.tabs.get_mut(self.x) {
                    *stop = true;
                }
            }
            (0, b'M') => self.reverse_index(),
            (0, b'c') => self.full_reset(),
            (b'(', _) => {
                self.charsets[0] = if b == b'0' {
                    Charset::LineDrawing
                } else {
                    Charset::Ascii
                }
            }
            (b')', _) => {
                self.charsets[1] = if b == b'0' {
                    Charset::LineDrawing
                } else {
                    Charset::Ascii
                }
            }
            (b'#', b'8') => {
                for y in 0..self.rows {
                    for cell in &mut self.screen[y].cells {
                        *cell = Cell {
                            ch: 'E',
                            ..Cell::blank(TermColor::Default)
                        };
                    }
                }
            }
            _ => {}
        }
    }

    /// Parameter `index` as a count: absent or zero becomes `default`.
    fn arg(&self, index: usize, default: u32) -> u32 {
        self.params
            .get(index)
            .and_then(|group| group.first())
            .copied()
            .filter(|value| *value > 0)
            .unwrap_or(default)
    }

    /// Parameter `index` exactly as given (`None` when absent).
    fn raw(&self, index: usize) -> Option<u32> {
        self.params
            .get(index)
            .and_then(|group| group.first())
            .copied()
    }

    fn respond(&mut self, text: &str) {
        self.responses.extend_from_slice(text.as_bytes());
    }

    fn csi(&mut self, final_byte: u8) {
        let n = self.arg(0, 1) as usize;
        match (self.private, self.intermediate, final_byte) {
            (0, 0, b'@') => self.insert_cells(n),
            (0, 0, b'A') => {
                let limit = if self.y >= self.top { self.top } else { 0 };
                self.y = self.y.saturating_sub(n).max(limit);
                self.pending_wrap = false;
            }
            (0, 0, b'B' | b'e') => {
                let limit = if self.y <= self.bottom {
                    self.bottom
                } else {
                    self.rows - 1
                };
                self.y = (self.y + n).min(limit);
                self.pending_wrap = false;
            }
            (0, 0, b'C' | b'a') => {
                self.x = (self.x + n).min(self.cols - 1);
                self.pending_wrap = false;
            }
            (0, 0, b'D') => {
                self.x = self.x.saturating_sub(n);
                self.pending_wrap = false;
            }
            (0, 0, b'E') => {
                self.y = (self.y + n).min(self.bottom.max(self.y));
                self.x = 0;
                self.pending_wrap = false;
            }
            (0, 0, b'F') => {
                self.y =
                    self.y
                        .saturating_sub(n)
                        .max(if self.y >= self.top { self.top } else { 0 });
                self.x = 0;
                self.pending_wrap = false;
            }
            (0, 0, b'G' | b'`') => {
                self.x = (n - 1).min(self.cols - 1);
                self.pending_wrap = false;
            }
            (0, 0, b'H' | b'f') => {
                let (row, col) = (self.arg(0, 1) as usize - 1, self.arg(1, 1) as usize - 1);
                self.set_cursor(row, col);
            }
            (0, 0, b'I') => self.tab_forward(n),
            (0, 0, b'J') => self.erase_display(self.raw(0).unwrap_or(0)),
            (0, 0, b'K') => self.erase_line(self.raw(0).unwrap_or(0)),
            (0, 0, b'L') => self.insert_lines(n),
            (0, 0, b'M') => self.delete_lines(n),
            (0, 0, b'P') => self.delete_cells(n),
            (0, 0, b'S') => self.scroll_up(n),
            (0, 0, b'T') if self.params.len() <= 1 => self.scroll_down(n),
            (0, 0, b'X') => self.erase_cells(self.y, self.x, self.x + n),
            (0, 0, b'Z') => self.tab_back(n),
            (0, 0, b'b') => {
                if let Some(c) = self.last_char {
                    for _ in 0..n.min(self.cols * self.rows) {
                        self.print(c);
                    }
                }
            }
            (0, 0, b'c') => {
                if self.raw(0).unwrap_or(0) == 0 {
                    self.respond("\x1b[?62;22c");
                }
            }
            (b'>', 0, b'c') => self.respond("\x1b[>1;10;0c"),
            (0, 0, b'd') => {
                let col = self.x;
                self.set_cursor(n - 1, col);
            }
            (0, 0, b'g') => match self.raw(0).unwrap_or(0) {
                0 => {
                    if let Some(stop) = self.tabs.get_mut(self.x) {
                        *stop = false;
                    }
                }
                3 => self.tabs.iter_mut().for_each(|stop| *stop = false),
                _ => {}
            },
            (0, 0, b'h') => self.set_modes(false, true),
            (0, 0, b'l') => self.set_modes(false, false),
            (b'?', 0, b'h') => self.set_modes(true, true),
            (b'?', 0, b'l') => self.set_modes(true, false),
            (0, 0, b'm') => self.sgr(),
            (0, 0, b'n') => match self.raw(0) {
                Some(5) => self.respond("\x1b[0n"),
                Some(6) => {
                    let row = if self.modes.origin {
                        self.y - self.top
                    } else {
                        self.y
                    };
                    self.respond(&format!("\x1b[{};{}R", row + 1, self.x + 1));
                }
                _ => {}
            },
            (0, 0, b'r') => {
                let top = self.arg(0, 1) as usize - 1;
                let bottom = (self.arg(1, self.rows as u32) as usize).min(self.rows) - 1;
                if top < bottom {
                    self.top = top;
                    self.bottom = bottom;
                    self.set_cursor(0, 0);
                }
            }
            (0, 0, b's') => self.saved = Some(self.save_cursor()),
            (0, 0, b'u') => {
                let saved = self.saved;
                self.restore_cursor(saved);
            }
            (0, b' ', b'q') => self.cursor_style = self.raw(0).unwrap_or(0).min(6) as u8,
            (0, b'!', b'p') => self.soft_reset(),
            _ => {}
        }
    }

    fn set_modes(&mut self, private: bool, on: bool) {
        let modes: Vec<u32> = self
            .params
            .iter()
            .filter_map(|group| group.first().copied())
            .collect();
        for mode in modes {
            if private {
                self.set_private_mode(mode, on);
            } else {
                match mode {
                    4 => self.modes.insert = on,
                    20 => self.modes.newline = on,
                    _ => {}
                }
            }
        }
    }

    fn set_private_mode(&mut self, mode: u32, on: bool) {
        match mode {
            1 => self.modes.app_cursor = on,
            5 => self.modes.reverse = on,
            6 => {
                self.modes.origin = on;
                self.set_cursor(0, 0);
            }
            7 => self.modes.autowrap = on,
            25 => self.modes.cursor_visible = on,
            47 | 1047 => {
                if on {
                    self.enter_alt(false);
                } else {
                    self.leave_alt(false);
                }
            }
            1048 => {
                if on {
                    self.saved = Some(self.save_cursor());
                } else {
                    let saved = self.saved;
                    self.restore_cursor(saved);
                }
            }
            1049 => {
                if on {
                    self.enter_alt(true);
                } else {
                    self.leave_alt(true);
                }
            }
            1000 | 1002 | 1003 => {
                let wanted = match mode {
                    1000 => MouseMode::Press,
                    1002 => MouseMode::Drag,
                    _ => MouseMode::Any,
                };
                if on {
                    self.modes.mouse = wanted;
                } else if self.modes.mouse == wanted {
                    self.modes.mouse = MouseMode::Off;
                }
            }
            1004 => self.modes.focus_reporting = on,
            1006 => self.modes.mouse_sgr = on,
            1007 => self.modes.alt_scroll = on,
            2004 => self.modes.bracketed_paste = on,
            _ => {}
        }
    }

    fn sgr(&mut self) {
        let groups = std::mem::take(&mut self.params);
        let mut i = 0;
        if groups.iter().all(Vec::is_empty) {
            self.pen = Pen::default();
            return;
        }
        while i < groups.len() {
            let group = &groups[i];
            let code = group.first().copied().unwrap_or(0);
            match code {
                0 => self.pen = Pen::default(),
                1 => self.pen.attrs |= BOLD,
                2 => self.pen.attrs |= DIM,
                3 => self.pen.attrs |= ITALIC,
                4 => {
                    if group.get(1) == Some(&0) {
                        self.pen.attrs &= !UNDERLINE;
                    } else {
                        self.pen.attrs |= UNDERLINE;
                    }
                }
                7 => self.pen.attrs |= INVERSE,
                8 => self.pen.attrs |= HIDDEN,
                9 => self.pen.attrs |= STRIKE,
                21 => self.pen.attrs |= UNDERLINE,
                22 => self.pen.attrs &= !(BOLD | DIM),
                23 => self.pen.attrs &= !ITALIC,
                24 => self.pen.attrs &= !UNDERLINE,
                27 => self.pen.attrs &= !INVERSE,
                28 => self.pen.attrs &= !HIDDEN,
                29 => self.pen.attrs &= !STRIKE,
                30..=37 => self.pen.fg = TermColor::Indexed(code as u8 - 30),
                39 => self.pen.fg = TermColor::Default,
                40..=47 => self.pen.bg = TermColor::Indexed(code as u8 - 40),
                49 => self.pen.bg = TermColor::Default,
                90..=97 => self.pen.fg = TermColor::Indexed(code as u8 - 90 + 8),
                100..=107 => self.pen.bg = TermColor::Indexed(code as u8 - 100 + 8),
                38 | 48 => {
                    let (color, used) = extended_color(&groups, i);
                    i += used;
                    if let Some(color) = color {
                        if code == 38 {
                            self.pen.fg = color;
                        } else {
                            self.pen.bg = color;
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn finish_osc(&mut self) {
        let text = String::from_utf8_lossy(&std::mem::take(&mut self.osc)).into_owned();
        let (code, rest) = text.split_once(';').unwrap_or((text.as_str(), ""));
        match code {
            "0" | "2" => {
                let title: String = rest.chars().filter(|c| !c.is_control()).take(200).collect();
                self.title = Some(title);
            }
            "10" | "11" if rest == "?" => {
                let (r, g, b) = if code == "10" {
                    self.default_fg
                } else {
                    self.default_bg
                };
                let wide = |v: u8| u16::from(v) * 257;
                self.respond(&format!(
                    "\x1b]{code};rgb:{:04x}/{:04x}/{:04x}\x1b\\",
                    wide(r),
                    wide(g),
                    wide(b)
                ));
            }
            _ => {}
        }
    }
}

fn resize_row(row: &mut Row, cols: usize) {
    if row.cells.len() > cols {
        row.cells.truncate(cols);
        if row
            .cells
            .last()
            .is_some_and(|cell| char_width(cell.ch) == 2)
        {
            if let Some(last) = row.cells.last_mut() {
                *last = Cell::blank(TermColor::Default);
            }
        }
    } else {
        row.cells.resize(cols, Cell::blank(TermColor::Default));
    }
}

/// Parses the colour after an SGR 38/48; returns it and how many extra parameter groups it used.
fn extended_color(groups: &[Vec<u32>], at: usize) -> (Option<TermColor>, usize) {
    let byte = |value: u32| value.min(255) as u8;
    let group = &groups[at];
    if group.len() > 1 {
        let color = match group[1] {
            5 => group.get(2).map(|index| TermColor::Indexed(byte(*index))),
            2 => {
                let rgb = if group.len() >= 6 {
                    &group[3..6]
                } else {
                    group.get(2..5).unwrap_or(&[])
                };
                (rgb.len() == 3).then(|| TermColor::Rgb(byte(rgb[0]), byte(rgb[1]), byte(rgb[2])))
            }
            _ => None,
        };
        return (color, 0);
    }
    let value = |offset: usize| {
        groups
            .get(at + offset)
            .and_then(|group| group.first())
            .copied()
            .unwrap_or(0)
    };
    match value(1) {
        5 if at + 2 < groups.len() => (Some(TermColor::Indexed(byte(value(2)))), 2),
        2 if at + 4 < groups.len() => (
            Some(TermColor::Rgb(
                byte(value(2)),
                byte(value(3)),
                byte(value(4)),
            )),
            4,
        ),
        _ => (None, groups.len() - at - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(cols: usize, rows: usize) -> Terminal {
        Terminal::new(cols, rows, 100)
    }

    fn line(terminal: &Terminal, y: usize) -> String {
        terminal.screen[y]
            .cells
            .iter()
            .map(|cell| cell.ch)
            .collect::<String>()
            .trim_end()
            .to_owned()
    }

    #[test]
    fn prints_wraps_and_handles_newlines() {
        let mut t = term(5, 3);
        t.feed(b"abcdefgh\r\nxy");
        assert_eq!(line(&t, 0), "abcde");
        assert_eq!(line(&t, 1), "fgh");
        assert_eq!(line(&t, 2), "xy");
        assert!(t.screen[0].wrapped && !t.screen[1].wrapped);
        t.feed(b"\r\n1");
        assert_eq!(line(&t, 0), "fgh");
        assert_eq!(t.scrollback_len(), 1);
        assert_eq!(t.dump(), "abcdefgh\nxy\n1\n");
    }

    #[test]
    fn decodes_utf8_split_across_writes_and_wide_characters() {
        let mut t = term(6, 2);
        t.feed(&"é".as_bytes()[..1]);
        t.feed(&"é".as_bytes()[1..]);
        t.feed("日本".as_bytes());
        assert_eq!(t.cursor(), (5, 0));
        assert_eq!(t.screen[0].cells[1].ch, '日');
        assert_eq!(t.screen[0].cells[2].ch, '\0');
        t.feed(b"\x1b[1;2H");
        t.feed(b"x");
        assert_eq!(
            t.screen[0].cells[2].ch, ' ',
            "overwriting half a wide char blanks the rest"
        );
        t.feed(b"\xff!");
        assert!(t.dump().contains('\u{fffd}'));
    }

    #[test]
    fn wide_characters_wrap_instead_of_splitting() {
        let mut t = term(3, 2);
        t.feed("ab日".as_bytes());
        assert_eq!(line(&t, 0), "ab");
        assert_eq!(t.screen[1].cells[0].ch, '日');
    }

    #[test]
    fn cursor_movement_and_erasing() {
        let mut t = term(10, 4);
        t.feed(b"hello\x1b[2;3Hxx\x1b[1;1H\x1b[2K");
        assert_eq!(line(&t, 0), "");
        assert_eq!(line(&t, 1), "  xx");
        t.feed(b"\x1b[2;1Hab\x1b[Cc\x1b[2D!\x1b[K");
        assert_eq!(line(&t, 1), "ab!");
        t.feed(b"\x1b[3;4HQ\x1b[1J");
        assert_eq!(t.dump(), "\n");
        t.feed(b"\x1b[1;1Htop\x1b[3;1Hlow\x1b[2;2H\x1b[J");
        assert_eq!(
            (line(&t, 0), line(&t, 2)),
            ("top".to_owned(), String::new())
        );
        t.feed(b"\x1b[2J");
        assert_eq!(t.dump(), "\n");
    }

    #[test]
    fn colours_attributes_and_resets() {
        let mut t = term(20, 2);
        t.feed(b"\x1b[1;31mA\x1b[0mB\x1b[38;5;200mC\x1b[38;2;1;2;3;48;5;4mD\x1b[38:2::9:8:7mE\x1b[39;49;22mF");
        let cells = &t.screen[0].cells;
        assert_eq!((cells[0].fg, cells[0].attrs), (TermColor::Indexed(1), BOLD));
        assert_eq!((cells[1].fg, cells[1].attrs), (TermColor::Default, 0));
        assert_eq!(cells[2].fg, TermColor::Indexed(200));
        assert_eq!(
            (cells[3].fg, cells[3].bg),
            (TermColor::Rgb(1, 2, 3), TermColor::Indexed(4))
        );
        assert_eq!(cells[4].fg, TermColor::Rgb(9, 8, 7));
        assert_eq!(
            (cells[5].fg, cells[5].bg),
            (TermColor::Default, TermColor::Default)
        );
        t.feed(b"\x1b[7;4;9m\x1b[m ");
        assert_eq!(t.screen[0].cells[6].attrs, 0);
        t.feed(b"\x1b[44m\x1b[K");
        assert_eq!(
            t.screen[0].cells[19].bg,
            TermColor::Indexed(4),
            "erasing keeps the background"
        );
    }

    #[test]
    fn scroll_regions_insert_and_delete_lines() {
        let mut t = term(4, 5);
        for (row, text) in ["a", "b", "c", "d", "e"].iter().enumerate() {
            t.feed(format!("\x1b[{};1H{text}", row + 1).as_bytes());
        }
        t.feed(b"\x1b[2;4r");
        assert_eq!(t.cursor(), (0, 0));
        t.feed(b"\x1b[4;1H\n");
        assert_eq!(
            [
                line(&t, 0),
                line(&t, 1),
                line(&t, 2),
                line(&t, 3),
                line(&t, 4)
            ],
            ["a", "c", "d", "", "e"]
        );
        assert_eq!(
            t.scrollback_len(),
            0,
            "regions that do not start at the top keep no history"
        );
        t.feed(b"\x1b[2;1H\x1b[L");
        assert_eq!([line(&t, 1), line(&t, 2)], ["", "c"]);
        t.feed(b"\x1b[M\x1b[M");
        assert_eq!([line(&t, 1), line(&t, 4)], ["d", "e"]);
        t.feed(b"\x1b[3;1Hz\x1bM\x1bM\x1bMq");
        assert_eq!(line(&t, 1), " q");
    }

    #[test]
    fn insert_delete_and_repeat_characters() {
        let mut t = term(8, 2);
        t.feed(b"abcdef\x1b[1;3H\x1b[2@");
        assert_eq!(line(&t, 0), "ab  cdef");
        t.feed(b"\x1b[3P");
        assert_eq!(line(&t, 0), "abdef");
        t.feed(b"\x1b[1;1H\x1b[2X");
        assert_eq!(line(&t, 0), "  def");
        t.feed(b"\x1b[2;1Hx\x1b[3b");
        assert_eq!(line(&t, 1), "xxxx");
        t.feed(b"\x1b[4h\x1b[2;1Hy");
        assert_eq!(line(&t, 1), "yxxxx");
    }

    #[test]
    fn alternate_screen_saves_and_restores() {
        let mut t = term(6, 3);
        t.feed(b"main\x1b[?1049h");
        assert!(t.on_alt_screen());
        assert_eq!(t.dump(), "\n");
        t.feed(b"alt\r\n\r\n\r\n\r\nmore");
        assert_eq!(t.scrollback_len(), 0);
        t.feed(b"\x1b[?1049l");
        assert!(!t.on_alt_screen());
        assert_eq!(line(&t, 0), "main");
        assert_eq!(t.cursor(), (4, 0));
    }

    #[test]
    fn modes_are_tracked() {
        let mut t = term(10, 3);
        t.feed(b"\x1b[?1h\x1b[?25l\x1b[?2004h\x1b[?1000h\x1b[?1006h\x1b[?1004h");
        assert!(t.modes.app_cursor && !t.modes.cursor_visible && t.modes.bracketed_paste);
        assert!(t.modes.mouse_sgr && t.modes.focus_reporting);
        assert_eq!(t.modes.mouse, MouseMode::Press);
        t.feed(b"\x1b[?1002h\x1b[?1000l");
        assert_eq!(t.modes.mouse, MouseMode::Drag);
        t.feed(b"\x1b[?1002l\x1b[?1h\x1b[?1l\x1b[?7l");
        assert_eq!(t.modes.mouse, MouseMode::Off);
        assert!(!t.modes.app_cursor && !t.modes.autowrap);
        t.feed(b"0123456789AB");
        assert_eq!(line(&t, 0), "012345678B");
    }

    #[test]
    fn answers_device_and_cursor_queries() {
        let mut t = term(10, 5);
        t.feed(b"\x1b[3;4H\x1b[6n\x1b[5n\x1b[c\x1b[>c");
        let reply = String::from_utf8(t.take_responses()).unwrap();
        assert_eq!(reply, "\x1b[3;4R\x1b[0n\x1b[?62;22c\x1b[>1;10;0c");
        assert!(t.take_responses().is_empty());
        t.set_default_colors((255, 0, 128), (0, 0, 0));
        t.feed(b"\x1b]11;?\x07\x1b]10;?\x1b\\");
        let reply = String::from_utf8(t.take_responses()).unwrap();
        assert_eq!(
            reply,
            "\x1b]11;rgb:0000/0000/0000\x1b\\\x1b]10;rgb:ffff/0000/8080\x1b\\"
        );
    }

    #[test]
    fn titles_charsets_and_ignored_strings() {
        let mut t = term(10, 2);
        t.feed(b"\x1b]0;my title\x07\x1b]2;second\x1b\\");
        assert_eq!(t.take_title().as_deref(), Some("second"));
        assert_eq!(t.take_title(), None);
        t.feed(b"\x1b(0lqk\x1b(Bx");
        assert_eq!(line(&t, 0), "┌─┐x");
        t.feed(b"\x1bPignored\x1b\\\x1b_apc\x1b\\ok");
        assert_eq!(line(&t, 0), "┌─┐xok");
        t.feed(b"\x1b[?9999;1$p\x1b[0%xZ");
        assert!(line(&t, 0).ends_with('Z'));
    }

    #[test]
    fn tabs_save_restore_and_reset() {
        let mut t = term(20, 3);
        t.feed(b"a\tb\t\tc");
        assert_eq!(
            line(&t, 0),
            format!("a{}b{}c", " ".repeat(7), " ".repeat(10))
        );
        t.feed(b"\x1b[1;1H\x1b7\x1b[3;5H\x1b8!");
        assert_eq!(&line(&t, 0)[..1], "!");
        t.feed(b"\x1b[3g\x1b[1;1H\tx");
        assert_eq!(t.cursor().0, 19);
        t.feed(b"\x1bc");
        assert_eq!(t.dump(), "\n");
    }

    #[test]
    fn resizing_keeps_content_and_uses_scrollback() {
        let mut t = term(6, 3);
        t.feed(b"one\r\ntwo\r\nthree");
        t.resize(6, 2);
        assert_eq!(t.scrollback_len(), 1);
        assert_eq!(t.cursor(), (5, 1));
        assert_eq!(line(&t, 1), "three");
        t.resize(10, 4);
        assert_eq!(t.cols(), 10);
        assert_eq!(t.scrollback_len(), 0);
        assert_eq!(t.dump(), "one\ntwo\nthree\n");
        t.feed(b"\x1b[?1049h");
        t.resize(4, 2);
        t.feed(b"\x1b[?1049l");
        assert_eq!((t.cols(), t.rows()), (4, 2));
    }

    #[test]
    fn scrollback_is_bounded_and_text_can_be_extracted() {
        let mut t = Terminal::new(8, 2, 3);
        for n in 0..10 {
            t.feed(format!("line{n}\r\n").as_bytes());
        }
        assert_eq!(t.scrollback_len(), 3);
        assert!(t.take_trimmed() > 0);
        let first = t.row_abs(0).unwrap();
        assert!(first
            .cells
            .iter()
            .map(|c| c.ch)
            .collect::<String>()
            .starts_with("line6"));
        assert_eq!(t.text_range((2, 0), (3, 1)), "ne6\nline");
        assert_eq!(t.word_at(1, 0), (0, 4));
        assert_eq!(t.view_row(0, 2).unwrap().cells[0].ch, 'l');
    }

    #[test]
    fn encodes_mouse_reports() {
        assert_eq!(
            encode_mouse(0, 4, 9, false, true).unwrap(),
            b"\x1b[<0;5;10M"
        );
        assert_eq!(encode_mouse(2, 0, 0, true, true).unwrap(), b"\x1b[<2;1;1m");
        assert_eq!(
            encode_mouse(0, 0, 0, false, false).unwrap(),
            vec![0x1b, b'[', b'M', 32, 33, 33]
        );
        assert_eq!(encode_mouse(0, 0, 0, true, false).unwrap()[3], 35);
        assert!(encode_mouse(0, 300, 0, false, false).is_none());
    }

    #[test]
    fn survives_garbage() {
        let mut t = term(7, 3);
        let mut seed = 12345u32;
        let bytes: Vec<u8> = (0..20_000)
            .map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                (seed >> 16) as u8
            })
            .collect();
        t.feed(&bytes);
        t.resize(3, 9);
        t.feed(&bytes);
        assert_eq!((t.cols(), t.rows()), (3, 9));
    }
}

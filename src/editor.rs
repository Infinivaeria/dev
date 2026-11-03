//! The text editor window, started with `selenite --edit FILE`.
//!
//! Plain text, Ruby (with syntax highlighting and block-aware indentation) and
//! Markdown (with highlighting and a live preview beside the source).

use std::{
    cell::Cell,
    env,
    ffi::CString,
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use raylib::{ffi, prelude::*};

use crate::{
    markdown,
    syntax::{self, Language, LineState, Span, Token},
    ui,
};

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const TAB: usize = 4;
const MONO_BYTES: &[u8] = include_bytes!("../assets/fonts/DejaVuSansMono.ttf");
const MONO_ATLAS_SIZE: i32 = 64;
const LONG_LINE: usize = 3000;
const PREVIEW_DELAY: f64 = 0.12;

const BG: Color = Color::new(28, 29, 35, 255);
const GUTTER_BG: Color = Color::new(24, 25, 30, 255);
const BAR_BG: Color = Color::new(18, 18, 24, 255);
const TEXT: Color = Color::new(215, 218, 224, 255);
const DIM: Color = Color::new(110, 114, 130, 255);
const CURRENT_LINE: Color = Color::new(38, 40, 50, 255);
const SELECTION: Color = Color::new(58, 90, 160, 150);
const CARET: Color = Color::new(250, 210, 60, 255);
const DIVIDER: Color = Color::new(60, 62, 76, 255);
const PREVIEW_BG: Color = Color::new(32, 33, 40, 255);
const PREVIEW_TEXT: Color = Color::new(225, 227, 233, 255);
const PREVIEW_ITALIC: Color = Color::new(190, 200, 225, 255);
const CODE_BG: Color = Color::new(22, 23, 28, 255);
const TABLE_HEADER_BG: Color = Color::new(42, 46, 60, 255);
const RULE: Color = Color::new(82, 84, 100, 255);
const QUOTE_BAR: Color = Color::new(90, 120, 170, 255);
const WARNING: Color = Color::new(250, 170, 80, 255);
const FIND_MATCH: Color = Color::new(250, 210, 60, 70);
const FIND_CURRENT: Color = Color::new(250, 210, 60, 150);
const FIELD_BG: Color = Color::new(34, 36, 44, 255);
const DIVIDER_ACTIVE: Color = Color::new(110, 130, 190, 255);

pub fn token_color(token: Token) -> Color {
    match token {
        Token::Plain => TEXT,
        Token::Keyword => Color::new(198, 120, 221, 255),
        Token::Builtin => Color::new(97, 175, 239, 255),
        Token::String => Color::new(152, 195, 121, 255),
        Token::Comment => Color::new(106, 115, 135, 255),
        Token::Number => Color::new(209, 154, 102, 255),
        Token::Symbol => Color::new(86, 182, 194, 255),
        Token::Constant => Color::new(229, 192, 123, 255),
        Token::Variable => Color::new(224, 108, 117, 255),
        Token::Function => Color::new(130, 170, 255, 255),
        Token::Regex => Color::new(255, 160, 122, 255),
        Token::Heading => Color::new(240, 200, 90, 255),
        Token::Marker => Color::new(198, 120, 221, 255),
        Token::Quote => Color::new(140, 160, 190, 255),
        Token::Link => Color::new(100, 170, 255, 255),
        Token::Code => Color::new(229, 192, 123, 255),
        Token::Emphasis => Color::new(224, 150, 170, 255),
    }
}

/// Files the grid opens in this editor on activation (rather than the system viewer).
pub fn is_text_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("md" | "markdown" | "mdown" | "txt")
    )
}

/// First `note.md`, `note-2.md`, … that does not exist in `directory`.
pub fn unique_note_path(directory: &Path) -> PathBuf {
    let mut candidate = directory.join("note.md");
    let mut number = 2;
    while candidate.exists() {
        candidate = directory.join(format!("note-{number}.md"));
        number += 1;
    }
    candidate
}

/// Creates `path` with a title heading unless it already exists.
pub fn create_note_if_missing(path: &Path, title: &str) -> io::Result<()> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            use io::Write;
            write!(file, "# {title}\n\n")
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

// ----------------------------------------------------------------- buffer --

fn byte_index(line: &str, column: usize) -> usize {
    line.char_indices()
        .nth(column)
        .map_or(line.len(), |(index, _)| index)
}

fn char_len(line: &str) -> usize {
    line.chars().count()
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Screen column of character `column`, with tabs advancing to the next stop.
fn visual_col(line: &str, column: usize) -> usize {
    let mut visual = 0;
    for c in line.chars().take(column) {
        visual += if c == '\t' { TAB - visual % TAB } else { 1 };
    }
    visual
}

/// Character column whose cell contains screen position `x` (in cells).
fn column_at(line: &str, x: f32) -> usize {
    let mut visual = 0usize;
    for (column, c) in line.chars().enumerate() {
        let width = if c == '\t' { TAB - visual % TAB } else { 1 };
        if x < visual as f32 + width as f32 / 2.0 {
            return column;
        }
        visual += width;
    }
    char_len(line)
}

/// The cells of `chars` that fall in screen columns `[from, to)`, as
/// `(text, first column, column after the last char)`.
fn expand_range(chars: &[char], start: usize, from: usize, to: usize) -> (String, usize, usize) {
    let mut visual = start;
    let mut out = String::new();
    let mut first = None;
    for &c in chars {
        let width = if c == '\t' { TAB - visual % TAB } else { 1 };
        for cell in 0..width {
            let column = visual + cell;
            if column >= from && column < to {
                first.get_or_insert(column);
                out.push(if c == '\t' { ' ' } else { c });
            }
        }
        visual += width;
        if visual >= to {
            break;
        }
    }
    (out, first.unwrap_or(start), visual)
}

#[derive(Clone)]
struct Snapshot {
    lines: Vec<String>,
    row: usize,
    col: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditKind {
    None,
    Typing,
    Deleting,
    Other,
}

pub struct Buffer {
    pub lines: Vec<String>,
    pub row: usize,
    pub col: usize,
    anchor: Option<(usize, usize)>,
    wanted: usize,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last: EditKind,
    pub revision: u64,
    saved_revision: u64,
    changed_from: Option<usize>,
    crlf: bool,
}

/// What a Markdown list line continues with: the next line's prefix, and
/// whether the item is empty (pressing Enter then ends the list).
fn markdown_continuation(head: &str) -> Option<(String, bool)> {
    let (indent, rest) = head.split_at(head.len() - head.trim_start().len());
    let (marker, content) = if let Some(content) = ["- ", "* ", "+ "]
        .iter()
        .find_map(|marker| rest.strip_prefix(marker))
    {
        (rest[..2].to_owned(), content)
    } else {
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 || digits > 9 {
            return None;
        }
        let after = &rest[digits..];
        let separator = after.chars().next().filter(|c| *c == '.' || *c == ')')?;
        let content = after[1..].strip_prefix(' ')?;
        let next = rest[..digits].parse::<u64>().ok()? + 1;
        (format!("{next}{separator} "), content)
    };
    let task = ["[ ] ", "[x] ", "[X] "]
        .iter()
        .find_map(|prefix| content.strip_prefix(prefix));
    let empty = task.unwrap_or(content).trim().is_empty();
    let marker = if task.is_some() {
        format!("{marker}[ ] ")
    } else {
        marker
    };
    Some((format!("{indent}{marker}"), empty))
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        let crlf = text.contains("\r\n");
        let normalized = text.replace("\r\n", "\n");
        Self {
            lines: normalized.split('\n').map(str::to_owned).collect(),
            row: 0,
            col: 0,
            anchor: None,
            wanted: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            last: EditKind::None,
            revision: 0,
            saved_revision: 0,
            changed_from: None,
            crlf,
        }
    }

    pub fn text(&self) -> String {
        let joined = self.lines.join("\n");
        if self.crlf {
            joined.replace('\n', "\r\n")
        } else {
            joined
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    pub fn mark_saved(&mut self) {
        self.saved_revision = self.revision;
    }

    /// The first line edited since the last call, for invalidating highlight state.
    fn take_changed_from(&mut self) -> Option<usize> {
        self.changed_from.take()
    }

    fn touch(&mut self, row: usize) {
        self.revision += 1;
        self.changed_from = Some(self.changed_from.map_or(row, |first| first.min(row)));
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            row: self.row,
            col: self.col,
        }
    }

    fn checkpoint(&mut self, kind: EditKind) {
        if kind == EditKind::Other || kind != self.last {
            let size: usize = self.lines.iter().map(String::len).sum();
            let limit = (50_000_000 / (size + 1)).clamp(10, 500);
            self.undo.push(self.snapshot());
            if self.undo.len() > limit {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.last = kind;
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.lines = snapshot.lines;
        self.row = snapshot.row.min(self.lines.len() - 1);
        self.col = snapshot.col.min(char_len(&self.lines[self.row]));
        self.wanted = self.col;
        self.anchor = None;
        self.last = EditKind::None;
        self.touch(0);
    }

    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.restore(snapshot);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snapshot());
        self.restore(snapshot);
        true
    }

    fn line_len(&self, row: usize) -> usize {
        char_len(&self.lines[row])
    }

    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.anchor?;
        let cursor = (self.row, self.col);
        match anchor.cmp(&cursor) {
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Less => Some((anchor, cursor)),
            std::cmp::Ordering::Greater => Some((cursor, anchor)),
        }
    }

    pub fn selected_text(&self) -> String {
        let Some(((r0, c0), (r1, c1))) = self.selection() else {
            return String::new();
        };
        if r0 == r1 {
            return self.lines[r0].chars().skip(c0).take(c1 - c0).collect();
        }
        let mut out: String = self.lines[r0].chars().skip(c0).collect();
        for row in r0 + 1..r1 {
            out.push('\n');
            out.push_str(&self.lines[row]);
        }
        out.push('\n');
        out.extend(self.lines[r1].chars().take(c1));
        out
    }

    fn delete_selection(&mut self) -> bool {
        let Some(((r0, c0), (r1, c1))) = self.selection() else {
            self.anchor = None;
            return false;
        };
        let head: String = self.lines[r0].chars().take(c0).collect();
        let tail: String = self.lines[r1].chars().skip(c1).collect();
        self.lines.splice(r0..=r1, [format!("{head}{tail}")]);
        self.row = r0;
        self.col = c0;
        self.wanted = c0;
        self.anchor = None;
        self.touch(r0);
        true
    }

    pub fn select_all(&mut self) {
        self.last = EditKind::None;
        self.anchor = Some((0, 0));
        self.row = self.lines.len() - 1;
        self.col = self.line_len(self.row);
        self.wanted = self.col;
    }

    pub fn select_word_at(&mut self, row: usize, col: usize) {
        let chars: Vec<char> = self.lines[row].chars().collect();
        let col = col.min(chars.len());
        let class = |c: char| (is_word(c), c.is_whitespace());
        let mut start = col;
        let mut end = col;
        if let Some(&c) = chars.get(col).or_else(|| chars.get(col.wrapping_sub(1))) {
            let target = class(c);
            let anchor = if col < chars.len() { col } else { col - 1 };
            start = anchor;
            end = anchor + 1;
            while start > 0 && class(chars[start - 1]) == target {
                start -= 1;
            }
            while end < chars.len() && class(chars[end]) == target {
                end += 1;
            }
        }
        self.last = EditKind::None;
        self.anchor = Some((row, start));
        self.row = row;
        self.col = end;
        self.wanted = end;
    }

    pub fn select_line(&mut self, row: usize) {
        self.last = EditKind::None;
        self.anchor = Some((row, 0));
        self.row = row;
        self.col = self.line_len(row);
        self.wanted = self.col;
    }

    pub fn insert_text(&mut self, text: &str) {
        let text: String = text
            .replace("\r\n", "\n")
            .chars()
            .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
            .collect();
        if text.is_empty() {
            return;
        }
        let kind = if text.chars().count() == 1 && text != "\n" {
            EditKind::Typing
        } else {
            EditKind::Other
        };
        self.checkpoint(kind);
        self.delete_selection();
        let start_row = self.row;
        let mut parts = text.split('\n');
        let first = parts.next().unwrap_or("");
        let rest: Vec<&str> = parts.collect();
        let line = &self.lines[self.row];
        let at = byte_index(line, self.col);
        let (head, tail) = (line[..at].to_owned(), line[at..].to_owned());
        match rest.split_last() {
            None => {
                self.lines[self.row] = format!("{head}{first}{tail}");
                self.col += char_len(first);
            }
            Some((last, middle)) => {
                self.lines[self.row] = format!("{head}{first}");
                let mut row = self.row;
                for part in middle {
                    row += 1;
                    self.lines.insert(row, (*part).to_owned());
                }
                row += 1;
                self.lines.insert(row, format!("{last}{tail}"));
                self.row = row;
                self.col = char_len(last);
            }
        }
        self.wanted = self.col;
        self.touch(start_row);
    }

    pub fn newline(&mut self, language: Language) {
        self.checkpoint(EditKind::Other);
        self.delete_selection();
        let line = self.lines[self.row].clone();
        let at = byte_index(&line, self.col);
        let (head, tail) = (&line[..at], &line[at..]);
        let mut prefix: String = head
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        if language == Language::Ruby && syntax::ruby_opens_block(head) {
            prefix.push_str("  ");
        }
        if language == Language::Markdown {
            if let Some((next, empty)) = markdown_continuation(head) {
                if empty && tail.is_empty() {
                    self.lines[self.row].clear();
                    self.col = 0;
                    self.wanted = 0;
                    self.touch(self.row);
                    return;
                }
                prefix = next;
            }
        }
        let row = self.row;
        self.lines[row] = head.to_owned();
        self.lines.insert(row + 1, format!("{prefix}{tail}"));
        self.row = row + 1;
        self.col = char_len(&prefix);
        self.wanted = self.col;
        self.touch(row);
    }

    /// Re-aligns a just-typed Ruby `end`/`else`/`ensure`/`rescue` with its opener.
    pub fn after_typed(&mut self) {
        let line = &self.lines[self.row];
        let trimmed = line.trim();
        if !matches!(trimmed, "end" | "else" | "ensure" | "rescue") || self.col != char_len(line) {
            return;
        }
        let Some(target) = syntax::ruby_matching_indent(&self.lines, self.row) else {
            return;
        };
        let current = line.len() - line.trim_start().len();
        if current == target {
            return;
        }
        let aligned = format!("{}{trimmed}", " ".repeat(target));
        self.col = char_len(&aligned);
        self.wanted = self.col;
        self.lines[self.row] = aligned;
        self.touch(self.row);
    }

    pub fn backspace(&mut self) {
        if self.selection().is_some() {
            self.checkpoint(EditKind::Other);
            self.delete_selection();
        } else if self.col > 0 {
            self.checkpoint(EditKind::Deleting);
            let line = &mut self.lines[self.row];
            let at = byte_index(line, self.col - 1);
            line.remove(at);
            self.col -= 1;
            self.touch(self.row);
        } else if self.row > 0 {
            self.checkpoint(EditKind::Other);
            let line = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.line_len(self.row);
            self.lines[self.row].push_str(&line);
            self.touch(self.row);
        }
        self.wanted = self.col;
    }

    pub fn delete_forward(&mut self) {
        if self.selection().is_some() {
            self.checkpoint(EditKind::Other);
            self.delete_selection();
        } else if self.col < self.line_len(self.row) {
            self.checkpoint(EditKind::Deleting);
            let line = &mut self.lines[self.row];
            let at = byte_index(line, self.col);
            line.remove(at);
            self.touch(self.row);
        } else if self.row + 1 < self.lines.len() {
            self.checkpoint(EditKind::Other);
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
            self.touch(self.row);
        }
        self.wanted = self.col;
    }

    fn begin_move(&mut self, extend: bool) {
        if extend {
            self.anchor.get_or_insert((self.row, self.col));
        } else {
            self.anchor = None;
        }
        self.last = EditKind::None;
    }

    pub fn move_left(&mut self, extend: bool) {
        if !extend {
            if let Some((start, _)) = self.selection() {
                (self.row, self.col) = start;
                self.wanted = self.col;
                self.anchor = None;
                self.last = EditKind::None;
                return;
            }
        }
        self.begin_move(extend);
        if self.col > 0 {
            self.col -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.line_len(self.row);
        }
        self.wanted = self.col;
    }

    pub fn move_right(&mut self, extend: bool) {
        if !extend {
            if let Some((_, end)) = self.selection() {
                (self.row, self.col) = end;
                self.wanted = self.col;
                self.anchor = None;
                self.last = EditKind::None;
                return;
            }
        }
        self.begin_move(extend);
        if self.col < self.line_len(self.row) {
            self.col += 1;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        }
        self.wanted = self.col;
    }

    pub fn move_vertical(&mut self, delta: isize, extend: bool) {
        self.begin_move(extend);
        let target = self.row as isize + delta;
        if target < 0 {
            self.row = 0;
            self.col = 0;
            self.wanted = 0;
        } else if target as usize >= self.lines.len() {
            self.row = self.lines.len() - 1;
            self.col = self.line_len(self.row);
            self.wanted = self.col;
        } else {
            self.row = target as usize;
            self.col = self.wanted.min(self.line_len(self.row));
        }
    }

    pub fn move_home(&mut self, extend: bool) {
        self.begin_move(extend);
        let indent = self.lines[self.row]
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .count();
        self.col = if self.col == indent { 0 } else { indent };
        self.wanted = self.col;
    }

    pub fn move_end(&mut self, extend: bool) {
        self.begin_move(extend);
        self.col = self.line_len(self.row);
        self.wanted = self.col;
    }

    pub fn move_word(&mut self, forward: bool, extend: bool) {
        self.begin_move(extend);
        let chars: Vec<char> = self.lines[self.row].chars().collect();
        if forward {
            if self.col >= chars.len() {
                if self.row + 1 < self.lines.len() {
                    self.row += 1;
                    self.col = 0;
                }
            } else {
                let mut column = self.col;
                while column < chars.len() && !is_word(chars[column]) {
                    column += 1;
                }
                while column < chars.len() && is_word(chars[column]) {
                    column += 1;
                }
                self.col = column;
            }
        } else if self.col == 0 {
            if self.row > 0 {
                self.row -= 1;
                self.col = self.line_len(self.row);
            }
        } else {
            let mut column = self.col;
            while column > 0 && !is_word(chars[column - 1]) {
                column -= 1;
            }
            while column > 0 && is_word(chars[column - 1]) {
                column -= 1;
            }
            self.col = column;
        }
        self.wanted = self.col;
    }

    pub fn move_document(&mut self, start: bool, extend: bool) {
        self.begin_move(extend);
        self.row = if start { 0 } else { self.lines.len() - 1 };
        self.col = if start { 0 } else { self.line_len(self.row) };
        self.wanted = self.col;
    }

    pub fn set_cursor(&mut self, row: usize, col: usize, extend: bool) {
        self.begin_move(extend);
        self.row = row.min(self.lines.len() - 1);
        self.col = col.min(self.line_len(self.row));
        self.wanted = self.col;
    }

    fn selected_rows(&self) -> (usize, usize) {
        match self.selection() {
            Some(((first, _), (last, last_col))) => (
                first,
                if last_col == 0 && last > first {
                    last - 1
                } else {
                    last
                },
            ),
            None => (self.row, self.row),
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selection().is_some()
    }

    pub fn indent_lines(&mut self, unit: usize) {
        self.checkpoint(EditKind::Other);
        let (first, last) = self.selected_rows();
        let pad = " ".repeat(unit);
        for row in first..=last {
            if self.lines[row].is_empty() {
                continue;
            }
            self.lines[row].insert_str(0, &pad);
            if self.row == row {
                self.col += unit;
            }
            if let Some((anchor_row, anchor_col)) = &mut self.anchor {
                if *anchor_row == row {
                    *anchor_col += unit;
                }
            }
        }
        self.wanted = self.col;
        self.touch(first);
    }

    pub fn unindent_lines(&mut self, unit: usize) {
        self.checkpoint(EditKind::Other);
        let (first, last) = self.selected_rows();
        for row in first..=last {
            let line = &self.lines[row];
            let removed = if line.starts_with('\t') {
                1
            } else {
                line.chars().take(unit).take_while(|c| *c == ' ').count()
            };
            if removed == 0 {
                continue;
            }
            self.lines[row].drain(..removed);
            if self.row == row {
                self.col = self.col.saturating_sub(removed);
            }
            if let Some((anchor_row, anchor_col)) = &mut self.anchor {
                if *anchor_row == row {
                    *anchor_col = anchor_col.saturating_sub(removed);
                }
            }
        }
        self.wanted = self.col;
        self.touch(first);
    }

    /// Every match of `query` as `(row, first column, column after the last)`, capped at [`MAX_MATCHES`].
    pub fn find_matches(&self, query: &str, case_sensitive: bool) -> Vec<(usize, usize, usize)> {
        let mut found = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            for (start, end) in find_in_line(line, query, case_sensitive) {
                found.push((row, start, end));
                if found.len() >= MAX_MATCHES {
                    return found;
                }
            }
        }
        found
    }

    /// Replaces one match and leaves the caret just after the new text.
    pub fn replace_range(&mut self, row: usize, start: usize, end: usize, text: &str) {
        self.checkpoint(EditKind::Other);
        let line = &self.lines[row];
        let (from, to) = (byte_index(line, start), byte_index(line, end));
        self.lines[row].replace_range(from..to, text);
        self.row = row;
        self.col = start + char_len(text);
        self.wanted = self.col;
        self.anchor = None;
        self.touch(row);
    }

    /// Replaces every match in one undoable step; returns how many were replaced.
    pub fn replace_all(&mut self, query: &str, replacement: &str, case_sensitive: bool) -> usize {
        let hits: Vec<Vec<(usize, usize)>> = self
            .lines
            .iter()
            .map(|line| find_in_line(line, query, case_sensitive))
            .collect();
        let total: usize = hits.iter().map(Vec::len).sum();
        let Some(first) = hits.iter().position(|line| !line.is_empty()) else {
            return 0;
        };
        self.checkpoint(EditKind::Other);
        for (row, ranges) in hits.iter().enumerate() {
            for &(start, end) in ranges.iter().rev() {
                let line = &self.lines[row];
                let (from, to) = (byte_index(line, start), byte_index(line, end));
                self.lines[row].replace_range(from..to, replacement);
            }
        }
        self.row = self.row.min(self.lines.len() - 1);
        self.col = self.col.min(self.line_len(self.row));
        self.wanted = self.col;
        self.anchor = None;
        self.touch(first);
        total
    }
}

const MAX_MATCHES: usize = 20_000;

fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Non-overlapping occurrences of `query` in `line`, as character ranges.
fn find_in_line(line: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    let needle: Vec<char> = query.chars().collect();
    let haystack: Vec<char> = line.chars().collect();
    if needle.is_empty() || needle.len() > haystack.len() {
        return Vec::new();
    }
    let same = |a: char, b: char| a == b || (!case_sensitive && fold(a) == fold(b));
    let mut found = Vec::new();
    let mut index = 0;
    while index + needle.len() <= haystack.len() {
        if needle
            .iter()
            .enumerate()
            .all(|(offset, c)| same(haystack[index + offset], *c))
        {
            found.push((index, index + needle.len()));
            index += needle.len();
        } else {
            index += 1;
        }
    }
    found
}

// ------------------------------------------------------------------ fonts --

thread_local! {
    static MONO: Cell<Option<ffi::Font>> = const { Cell::new(None) };
}

fn load_mono_font() {
    let mut codepoints: Vec<i32> = (32..=126).chain(160..=591).collect();
    codepoints.extend([
        0x2026, 0x2013, 0x2014, 0x2022, 0x2190, 0x2191, 0x2192, 0x2193, 0x203A,
    ]);
    let file_type = CString::new(".ttf").expect("static string has no NUL");
    // SAFETY: the window and GL context exist (this runs after `raylib::init`),
    // and every pointer is valid for the duration of the calls.
    unsafe {
        let mut font = ffi::LoadFontFromMemory(
            file_type.as_ptr(),
            MONO_BYTES.as_ptr(),
            MONO_BYTES.len() as i32,
            MONO_ATLAS_SIZE,
            codepoints.as_ptr(),
            codepoints.len() as i32,
        );
        if font.texture.id == 0 || font.glyphCount == 0 {
            return;
        }
        ffi::GenTextureMipmaps(&mut font.texture);
        ffi::SetTextureFilter(
            font.texture,
            ffi::TextureFilter::TEXTURE_FILTER_TRILINEAR as i32,
        );
        MONO.with(|cell| cell.set(Some(font)));
    }
}

fn mono_font() -> (ffi::Font, bool) {
    match MONO.with(Cell::get) {
        Some(font) => (font, true),
        // SAFETY: only reached after window creation, when the default font exists.
        None => (unsafe { ffi::GetFontDefault() }, false),
    }
}

fn mono_spacing(size: f32, custom: bool) -> f32 {
    if custom {
        0.0
    } else {
        (size / 10.0).max(1.0)
    }
}

fn c_text(text: &str) -> CString {
    CString::new(text.replace('\0', "")).expect("NUL bytes were removed")
}

fn measure_mono(text: &str, size: f32) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let (font, custom) = mono_font();
    let text = c_text(text);
    // SAFETY: the font is valid after window creation; `text` outlives the call.
    unsafe { ffi::MeasureTextEx(font, text.as_ptr(), size, mono_spacing(size, custom)).x }
}

fn draw_mono<D: RaylibDraw>(_d: &mut D, text: &str, x: f32, y: f32, size: f32, color: Color) {
    if text.is_empty() || size < 1.0 {
        return;
    }
    let (font, custom) = mono_font();
    let text = c_text(text);
    // SAFETY: called while a draw handle is borrowed, i.e. between
    // BeginDrawing and EndDrawing; `text` outlives the call.
    unsafe {
        ffi::DrawTextEx(
            font,
            text.as_ptr(),
            ffi::Vector2 { x, y },
            size,
            mono_spacing(size, custom),
            color.into(),
        );
    }
}

fn capture_screen(path: &str) -> bool {
    let path = c_text(path);
    // SAFETY: called between BeginDrawing and EndDrawing; the image is freed here.
    unsafe {
        ffi::rlDrawRenderBatchActive();
        let image = ffi::LoadImageFromScreen();
        let saved = ffi::ExportImage(image, path.as_ptr());
        ffi::UnloadImage(image);
        saved
    }
}

// ----------------------------------------------------------------- editor --

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Edit,
    Split,
    Preview,
}

struct PreviewCache {
    revision: u64,
    width: f32,
    size: f32,
    built_at: f64,
    layout: markdown::Layout,
}

struct Geometry {
    font: f32,
    ui_font: f32,
    line_h: f32,
    char_w: f32,
    bar_h: f32,
    status_h: f32,
    gutter: f32,
    editor: Option<Rectangle>,
    preview: Option<Rectangle>,
    /// A Markdown split was too narrow for two panes, so only the source shows.
    collapsed: bool,
    /// Grab area around the split between source and preview.
    divider: Option<Rectangle>,
    /// Strip with the find (and replace) fields, above the source.
    find_bar: Option<Rectangle>,
    min_pane: f32,
    width: f32,
    height: f32,
}

struct Panes {
    editor: Option<Rectangle>,
    preview: Option<Rectangle>,
    collapsed: bool,
}

/// Divides the content area between the source and the preview; a split
/// narrower than two `min_pane`s falls back to the source alone. `ratio` is
/// the source pane's share of the width, kept within `min_pane` of both edges.
fn pane_layout(
    language: Language,
    view: View,
    width: f32,
    top: f32,
    height: f32,
    min_pane: f32,
    ratio: f32,
) -> Panes {
    let area = |x: f32, w: f32| Rectangle::new(x, top, w, height);
    match (language, view) {
        (Language::Markdown, View::Split) if width >= min_pane * 2.0 => {
            let half = (width * ratio).clamp(min_pane, width - min_pane).floor();
            Panes {
                editor: Some(area(0.0, half)),
                preview: Some(area(half, width - half)),
                collapsed: false,
            }
        }
        (Language::Markdown, View::Split) => Panes {
            editor: Some(area(0.0, width)),
            preview: None,
            collapsed: true,
        },
        (Language::Markdown, View::Preview) => Panes {
            editor: None,
            preview: Some(area(0.0, width)),
            collapsed: false,
        },
        _ => Panes {
            editor: Some(area(0.0, width)),
            preview: None,
            collapsed: false,
        },
    }
}

fn parse_size(text: &str) -> Option<(i32, i32)> {
    let (width, height) = text.split_once('x')?;
    let (width, height) = (
        width.trim().parse::<i32>().ok()?,
        height.trim().parse::<i32>().ok()?,
    );
    ((200..=16384).contains(&width) && (150..=16384).contains(&height)).then_some((width, height))
}

/// Top-left of the preview's content, in window coordinates.
fn preview_origin(geo: &Geometry, rect: Rectangle, scroll: f32) -> (f32, f32) {
    let pad = geo.ui_font * 1.2;
    (rect.x + pad, rect.y + pad - scroll)
}

struct Editor {
    path: PathBuf,
    buffer: Buffer,
    language: Language,
    view: View,
    bom: bool,
    loaded: Option<SystemTime>,
    scroll_y: f32,
    scroll_x: f32,
    preview_scroll: f32,
    states: Vec<LineState>,
    preview: Option<PreviewCache>,
    status: Option<(String, f64)>,
    quit: bool,
    discard: bool,
    armed_quit: bool,
    armed_overwrite: bool,
    follow_caret: bool,
    dragging: bool,
    last_click: Option<(f64, usize, u8)>,
    blink_from: f64,
    keys: KeyRepeat,
    /// Share of the width the source pane takes in a Markdown split.
    split: f32,
    divider_drag: bool,
    last_divider_click: f64,
    find: Option<Find>,
    /// The preview link under the mouse.
    hover_link: Option<String>,
    cursor: MouseCursor,
}

/// The find / replace bar: fields, options and the matches it found.
struct Find {
    query: String,
    replacement: String,
    /// Show the replace row.
    replace: bool,
    /// Typing goes to the replacement field.
    in_replace: bool,
    case_sensitive: bool,
    matches: Vec<(usize, usize, usize)>,
    current: Option<usize>,
    /// What `matches` was computed for.
    stamp: Option<(u64, bool, String)>,
}

fn point_in(rect: Rectangle, point: Vector2) -> bool {
    point.x >= rect.x
        && point.x < rect.x + rect.width
        && point.y >= rect.y
        && point.y < rect.y + rect.height
}

/// Key repeat driven by our own clock, so a held key behaves the same everywhere.
#[derive(Default)]
struct KeyRepeat {
    next: Vec<(KeyboardKey, f64)>,
}

impl KeyRepeat {
    const DELAY: f64 = 0.4;
    const INTERVAL: f64 = 0.035;

    fn fire(&mut self, rl: &RaylibHandle, key: KeyboardKey, now: f64) -> bool {
        let slot = self.next.iter().position(|(held, _)| *held == key);
        if rl.is_key_pressed(key) {
            match slot {
                Some(index) => self.next[index].1 = now + Self::DELAY,
                None => self.next.push((key, now + Self::DELAY)),
            }
            return true;
        }
        let Some(index) = slot else { return false };
        if !rl.is_key_down(key) {
            self.next.swap_remove(index);
            return false;
        }
        if now >= self.next[index].1 {
            self.next[index].1 = now + Self::INTERVAL;
            return true;
        }
        false
    }
}

impl Editor {
    fn open(path: &Path) -> Result<Self, String> {
        let (text, loaded) = match fs::metadata(path) {
            Ok(meta) => {
                if !meta.is_file() {
                    return Err(format!("{} is not a file", path.display()));
                }
                if meta.len() > MAX_FILE_BYTES {
                    return Err(format!(
                        "{} is larger than the editor's {} MiB limit",
                        path.display(),
                        MAX_FILE_BYTES / 1024 / 1024
                    ));
                }
                let bytes = fs::read(path)
                    .map_err(|error| format!("could not read {}: {error}", path.display()))?;
                let text = String::from_utf8(bytes)
                    .map_err(|_| format!("{} is not UTF-8 text", path.display()))?;
                (text, meta.modified().ok())
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (String::new(), None),
            Err(error) => return Err(format!("could not open {}: {error}", path.display())),
        };
        let (bom, text) = match text.strip_prefix('\u{feff}') {
            Some(rest) => (true, rest.to_owned()),
            None => (false, text),
        };
        let language = Language::from_path(path);
        Ok(Self {
            path: path.to_path_buf(),
            buffer: Buffer::new(&text),
            language,
            view: if language == Language::Markdown {
                View::Split
            } else {
                View::Edit
            },
            bom,
            loaded,
            scroll_y: 0.0,
            scroll_x: 0.0,
            preview_scroll: 0.0,
            states: vec![LineState::Normal],
            preview: None,
            status: None,
            quit: false,
            discard: false,
            armed_quit: false,
            armed_overwrite: false,
            follow_caret: false,
            dragging: false,
            last_click: None,
            blink_from: 0.0,
            keys: KeyRepeat::default(),
            split: 0.5,
            divider_drag: false,
            last_divider_click: f64::NEG_INFINITY,
            find: None,
            hover_link: None,
            cursor: MouseCursor::MOUSE_CURSOR_DEFAULT,
        })
    }

    fn name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }

    fn title(&self) -> String {
        format!(
            "Selenite Editor - {}{}",
            self.name(),
            if self.buffer.is_dirty() { " *" } else { "" }
        )
    }

    fn indent_width(&self) -> usize {
        if self.language == Language::Plain {
            4
        } else {
            2
        }
    }

    fn say(&mut self, message: impl Into<String>, now: f64) {
        self.status = Some((message.into(), now));
    }

    fn save(&mut self, force: bool, now: f64) -> Result<(), String> {
        let target = fs::canonicalize(&self.path).unwrap_or_else(|_| self.path.clone());
        let on_disk = fs::metadata(&target).and_then(|meta| meta.modified()).ok();
        let changed = match (self.loaded, on_disk) {
            (Some(loaded), Some(current)) => loaded != current,
            (None, Some(_)) => true,
            _ => false,
        };
        if changed && !force && !self.armed_overwrite {
            self.armed_overwrite = true;
            return Err("File changed on disk. Press Ctrl+S again to overwrite it".to_owned());
        }
        let mut bytes = Vec::new();
        if self.bom {
            bytes.extend_from_slice("\u{feff}".as_bytes());
        }
        bytes.extend_from_slice(self.buffer.text().as_bytes());
        write_atomic(&target, &bytes)
            .map_err(|error| format!("could not save {}: {error}", target.display()))?;
        self.loaded = fs::metadata(&target).and_then(|meta| meta.modified()).ok();
        self.buffer.mark_saved();
        self.armed_quit = false;
        self.armed_overwrite = false;
        self.say(format!("Saved {}", self.name()), now);
        Ok(())
    }

    fn ensure_states(&mut self, row: usize) {
        if let Some(changed) = self.buffer.take_changed_from() {
            self.states.truncate(changed + 1);
        }
        if self.states.is_empty() {
            self.states.push(LineState::Normal);
        }
        while self.states.len() <= row && self.states.len() <= self.buffer.lines.len() {
            let index = self.states.len() - 1;
            let line = &self.buffer.lines[index];
            let next = if char_len(line) > LONG_LINE {
                LineState::Normal
            } else {
                syntax::highlight_line(self.language, &self.states[index], line).1
            };
            self.states.push(next);
        }
    }

    fn geometry(&self, width: i32, height: i32) -> Geometry {
        let scale = ui::scale(width, height);
        let font = (13.5 * scale).round().max(8.0);
        let ui_font = (14.0 * scale).round().max(8.0);
        let line_h = (font * 1.5).round();
        let char_w = measure_mono("M", font);
        let bar_h = (ui_font * 2.0).round();
        let status_h = (ui_font * 1.8).round();
        let digits = self.buffer.lines.len().to_string().len().max(3);
        let gutter = char_w * (digits as f32 + 2.0);
        let (width, height) = (width as f32, height as f32);
        let content_h = (height - bar_h - status_h).max(1.0);
        let min_pane = (gutter + char_w * 24.0).max(ui_font * 22.0);
        let panes = pane_layout(
            self.language,
            self.view,
            width,
            bar_h,
            content_h,
            min_pane,
            self.split,
        );
        let mut editor = panes.editor;
        let mut find_bar = None;
        if let (Some(rect), Some(find)) = (&mut editor, &self.find) {
            let rows = if find.replace { 2.0 } else { 1.0 };
            let height = (ui_font * 1.9 * rows).round().min(rect.height / 2.0);
            find_bar = Some(Rectangle::new(rect.x, rect.y, rect.width, height));
            rect.y += height;
            rect.height = (rect.height - height).max(1.0);
        }
        let grab = (ui_font * 0.7).max(6.0);
        let divider = match (panes.editor, panes.preview) {
            (Some(source), Some(_)) => Some(Rectangle::new(
                source.x + source.width - grab / 2.0,
                bar_h,
                grab,
                content_h,
            )),
            _ => None,
        };
        Geometry {
            font,
            ui_font,
            line_h,
            char_w,
            bar_h,
            status_h,
            gutter,
            editor,
            preview: panes.preview,
            collapsed: panes.collapsed,
            divider,
            find_bar,
            min_pane,
            width,
            height,
        }
    }

    fn cycle_view(&mut self, now: f64) {
        if self.language != Language::Markdown {
            self.say("Preview is available for Markdown files", now);
            return;
        }
        self.view = match self.view {
            View::Split => View::Preview,
            View::Preview => View::Edit,
            View::Edit => View::Split,
        };
        self.follow_caret = true;
    }

    fn update(&mut self, rl: &mut RaylibHandle, geo: &Geometry, now: f64) {
        let down = |rl: &RaylibHandle, a, b| rl.is_key_down(a) || rl.is_key_down(b);
        let ctrl = down(
            rl,
            KeyboardKey::KEY_LEFT_CONTROL,
            KeyboardKey::KEY_RIGHT_CONTROL,
        ) || down(
            rl,
            KeyboardKey::KEY_LEFT_SUPER,
            KeyboardKey::KEY_RIGHT_SUPER,
        );
        let shift = down(
            rl,
            KeyboardKey::KEY_LEFT_SHIFT,
            KeyboardKey::KEY_RIGHT_SHIFT,
        );
        let alt = down(rl, KeyboardKey::KEY_LEFT_ALT, KeyboardKey::KEY_RIGHT_ALT);
        let revision = self.buffer.revision;
        let position = (self.buffer.row, self.buffer.col);
        let mut interacted = false;

        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            if self.find.is_some() {
                self.find = None;
                self.follow_caret = true;
            } else if !self.buffer.is_dirty() || self.armed_quit {
                self.discard = true;
                self.quit = true;
            } else {
                self.armed_quit = true;
                self.say(
                    "Unsaved changes. Press Esc again to discard them, or Ctrl+S to save",
                    now,
                );
            }
        }

        let finding = self.find.is_some();
        if ctrl {
            if rl.is_key_pressed(KeyboardKey::KEY_S) {
                if let Err(error) = self.save(false, now) {
                    self.say(error, now);
                }
            }
            if self.keys.fire(rl, KeyboardKey::KEY_Z, now) {
                let done = if shift {
                    self.buffer.redo()
                } else {
                    self.buffer.undo()
                };
                if !done {
                    self.say("Nothing to undo", now);
                }
            }
            if self.keys.fire(rl, KeyboardKey::KEY_Y, now) && !self.buffer.redo() {
                self.say("Nothing to redo", now);
            }
            if !finding && rl.is_key_pressed(KeyboardKey::KEY_A) {
                self.buffer.select_all();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_C) || rl.is_key_pressed(KeyboardKey::KEY_X) {
                let text = if self.buffer.has_selection() {
                    self.buffer.selected_text()
                } else {
                    format!("{}\n", self.buffer.lines[self.buffer.row])
                };
                let cut = rl.is_key_pressed(KeyboardKey::KEY_X) && !finding;
                if rl.set_clipboard_text(&text).is_ok() && cut {
                    if !self.buffer.has_selection() {
                        let row = self.buffer.row;
                        self.buffer.select_line(row);
                    }
                    self.buffer.backspace();
                }
            }
            if self.keys.fire(rl, KeyboardKey::KEY_V, now) {
                if let Ok(text) = rl.get_clipboard_text() {
                    if finding {
                        self.paste_into_find(&text);
                    } else {
                        self.buffer.insert_text(&text);
                    }
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_F) {
                self.open_find(false);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_H) {
                self.open_find(true);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_P) {
                self.cycle_view(now);
            }
        }

        if self.find.is_some() {
            self.find_keys(rl, now, ctrl, shift, alt);
        } else {
            let word = ctrl;
            // Arrow keys: plain moves, Ctrl for words, Shift to extend.
            if self.keys.fire(rl, KeyboardKey::KEY_LEFT, now) {
                if word {
                    self.buffer.move_word(false, shift);
                } else {
                    self.buffer.move_left(shift);
                }
            }
            if self.keys.fire(rl, KeyboardKey::KEY_RIGHT, now) {
                if word {
                    self.buffer.move_word(true, shift);
                } else {
                    self.buffer.move_right(shift);
                }
            }
            if self.keys.fire(rl, KeyboardKey::KEY_UP, now) {
                self.buffer.move_vertical(-1, shift);
            }
            if self.keys.fire(rl, KeyboardKey::KEY_DOWN, now) {
                self.buffer.move_vertical(1, shift);
            }
            let page = ((geo.editor.map_or(geo.height, |rect| rect.height) / geo.line_h) as isize
                - 1)
            .max(1);
            if self.keys.fire(rl, KeyboardKey::KEY_PAGE_UP, now) {
                self.buffer.move_vertical(-page, shift);
            }
            if self.keys.fire(rl, KeyboardKey::KEY_PAGE_DOWN, now) {
                self.buffer.move_vertical(page, shift);
            }
            if self.keys.fire(rl, KeyboardKey::KEY_HOME, now) {
                if ctrl {
                    self.buffer.move_document(true, shift);
                } else {
                    self.buffer.move_home(shift);
                }
            }
            if self.keys.fire(rl, KeyboardKey::KEY_END, now) {
                if ctrl {
                    self.buffer.move_document(false, shift);
                } else {
                    self.buffer.move_end(shift);
                }
            }
            if self.keys.fire(rl, KeyboardKey::KEY_ENTER, now)
                || self.keys.fire(rl, KeyboardKey::KEY_KP_ENTER, now)
            {
                self.buffer.newline(self.language);
            }
            if self.keys.fire(rl, KeyboardKey::KEY_BACKSPACE, now) {
                self.buffer.backspace();
            }
            if self.keys.fire(rl, KeyboardKey::KEY_DELETE, now) {
                self.buffer.delete_forward();
            }
            if self.keys.fire(rl, KeyboardKey::KEY_TAB, now) {
                let unit = self.indent_width();
                if shift {
                    self.buffer.unindent_lines(unit);
                } else if self.buffer.has_selection() {
                    self.buffer.indent_lines(unit);
                } else {
                    let line = &self.buffer.lines[self.buffer.row];
                    let visual = visual_col(line, self.buffer.col);
                    self.buffer.insert_text(&" ".repeat(unit - visual % unit));
                }
            }
            while let Some(c) = rl.get_char_pressed() {
                if (ctrl && !alt) || c.is_control() {
                    continue;
                }
                self.buffer.insert_text(&c.to_string());
                if self.language == Language::Ruby && c.is_alphabetic() {
                    self.buffer.after_typed();
                }
            }
        }

        self.mouse(rl, geo, now, shift);
        self.ensure_find();
        if self.buffer.revision != revision {
            self.armed_quit = false;
            self.armed_overwrite = false;
            interacted = true;
        }
        if position != (self.buffer.row, self.buffer.col) || interacted {
            self.follow_caret = true;
            self.blink_from = now;
        }
        if self.follow_caret {
            self.follow(geo);
            self.follow_caret = false;
        }
    }

    fn mouse(&mut self, rl: &RaylibHandle, geo: &Geometry, now: f64, shift: bool) {
        let mouse = rl.get_mouse_position();
        let wheel = rl.get_mouse_wheel_move();
        let zooming = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        let wheel = if zooming { 0.0 } else { wheel };
        let left = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let held = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        self.cursor = MouseCursor::MOUSE_CURSOR_DEFAULT;
        self.hover_link = None;

        if let Some(divider) = geo.divider {
            let over = point_in(divider, mouse);
            if left && over {
                if now - self.last_divider_click < 0.4 {
                    self.split = 0.5;
                    self.divider_drag = false;
                    self.last_divider_click = f64::NEG_INFINITY;
                } else {
                    self.divider_drag = true;
                    self.last_divider_click = now;
                }
            }
            if self.divider_drag {
                if held {
                    let low = geo.min_pane / geo.width;
                    self.split = (mouse.x / geo.width).clamp(low, 1.0 - low);
                } else {
                    self.divider_drag = false;
                }
            }
            if over || self.divider_drag {
                self.cursor = MouseCursor::MOUSE_CURSOR_RESIZE_EW;
            }
        } else {
            self.divider_drag = false;
        }
        if self.divider_drag {
            return;
        }

        if let (Some(bar), Some(find)) = (geo.find_bar, &mut self.find) {
            if point_in(bar, mouse) {
                self.cursor = MouseCursor::MOUSE_CURSOR_IBEAM;
                if left {
                    find.in_replace = find.replace && mouse.y >= bar.y + bar.height / 2.0;
                }
            }
        }

        if let Some(rect) = geo.editor {
            let text = Rectangle::new(
                rect.x + geo.gutter,
                rect.y,
                rect.width - geo.gutter,
                rect.height,
            );
            let to_cell = |this: &Self, point: Vector2| {
                let row = (((point.y - rect.y - geo.line_h * 0.25 + this.scroll_y) / geo.line_h)
                    .floor()
                    .max(0.0) as usize)
                    .min(this.buffer.lines.len() - 1);
                let x = (point.x - text.x + this.scroll_x) / geo.char_w;
                (row, column_at(&this.buffer.lines[row], x))
            };
            if point_in(text, mouse) {
                self.cursor = MouseCursor::MOUSE_CURSOR_IBEAM;
            }
            if point_in(rect, mouse) && wheel != 0.0 {
                if rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
                    self.scroll_x -= wheel * geo.char_w * 6.0;
                } else {
                    self.scroll_y -= wheel * geo.line_h * 3.0;
                }
            }
            if left && point_in(text, mouse) {
                let (row, col) = to_cell(self, mouse);
                let clicks = match self.last_click {
                    Some((time, last_row, count)) if now - time < 0.4 && last_row == row => {
                        count + 1
                    }
                    _ => 1,
                };
                self.last_click = Some((now, row, clicks));
                match clicks {
                    1 => {
                        self.buffer.set_cursor(row, col, shift);
                        self.dragging = true;
                    }
                    2 => self.buffer.select_word_at(row, col),
                    _ => self.buffer.select_line(row),
                }
            } else if left && point_in(rect, mouse) {
                let (row, _) = to_cell(self, mouse);
                self.buffer.select_line(row);
            }
            if self.dragging {
                if held {
                    let (row, col) = to_cell(self, mouse);
                    if (row, col) != (self.buffer.row, self.buffer.col) {
                        self.buffer.set_cursor(row, col, true);
                        self.follow_caret = true;
                    }
                } else {
                    self.dragging = false;
                }
            }
        }
        if let Some(rect) = geo.preview {
            if point_in(rect, mouse) {
                if wheel != 0.0 {
                    self.preview_scroll -= wheel * geo.line_h * 3.0;
                }
                if let Some(url) = self.link_at(geo, rect, mouse) {
                    self.cursor = MouseCursor::MOUSE_CURSOR_POINTING_HAND;
                    if left {
                        self.open_link(&url, now);
                    }
                    self.hover_link = Some(url);
                }
            }
        }
    }

    // ------------------------------------------------------------ links --

    /// The link target under `point` in the preview, from the last layout.
    fn link_at(&self, geo: &Geometry, rect: Rectangle, point: Vector2) -> Option<String> {
        let cache = self.preview.as_ref()?;
        let (origin_x, origin_y) = preview_origin(geo, rect, self.preview_scroll);
        for item in &cache.layout.items {
            let markdown::Item::Text { y, size, runs } = item else {
                continue;
            };
            let top = origin_y + y;
            if point.y < top || point.y > top + size * 1.3 {
                continue;
            }
            for run in runs {
                let Some(url) = &run.link else { continue };
                let x = origin_x + run.x;
                let font = if run.style.code { size * 0.92 } else { *size };
                let text = run.text.trim_end();
                let width = if run.style.code {
                    measure_mono(text, font)
                } else {
                    ui::measure(text, font)
                };
                if point.x >= x && point.x <= x + width {
                    return Some(url.clone());
                }
            }
        }
        None
    }

    fn open_link(&mut self, url: &str, now: f64) {
        let base = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        match markdown::classify_link(url, base) {
            markdown::LinkTarget::Web(target) => {
                match crate::open_with_system(Path::new(&target)) {
                    Ok(()) => self.say(format!("Opening {target}"), now),
                    Err(error) => self.say(error, now),
                }
            }
            markdown::LinkTarget::Anchor(slug) => {
                let found = self.preview.as_ref().and_then(|cache| {
                    cache
                        .layout
                        .anchors
                        .iter()
                        .find(|(name, _)| *name == slug)
                        .map(|(_, y)| *y)
                });
                match found {
                    Some(y) => self.preview_scroll = y,
                    None => self.say(format!("No heading named #{slug}"), now),
                }
            }
            markdown::LinkTarget::File(path) => {
                let result = if !path.exists() {
                    Err(format!("{} does not exist", path.display()))
                } else if is_text_path(&path) {
                    crate::spawn_editor(&path)
                } else {
                    crate::open_with_system(&path)
                };
                match result {
                    Ok(()) => self.say(format!("Opening {}", path.display()), now),
                    Err(error) => self.say(error, now),
                }
            }
            markdown::LinkTarget::Blocked(scheme) => {
                self.say(format!("Blocked a link with the '{scheme}' scheme"), now);
            }
        }
    }

    // ------------------------------------------------------ find / replace --

    fn open_find(&mut self, replace: bool) {
        if self.language == Language::Markdown && self.view == View::Preview {
            self.view = View::Split;
        }
        let one_line = self
            .buffer
            .selection()
            .is_some_and(|((first, _), (last, _))| first == last);
        let selected = one_line
            .then(|| self.buffer.selected_text())
            .filter(|text| !text.is_empty());
        match &mut self.find {
            Some(find) => {
                find.replace |= replace;
                if let Some(text) = selected {
                    find.query = text;
                }
                find.in_replace = replace && !find.query.is_empty();
            }
            None => {
                self.find = Some(Find {
                    query: selected.unwrap_or_default(),
                    replacement: String::new(),
                    replace,
                    in_replace: false,
                    case_sensitive: false,
                    matches: Vec::new(),
                    current: None,
                    stamp: None,
                });
            }
        }
        self.refresh_find(true);
    }

    fn paste_into_find(&mut self, text: &str) {
        let Some(find) = &mut self.find else { return };
        let line = text.lines().next().unwrap_or("");
        let clean: String = line.chars().filter(|c| !c.is_control()).collect();
        if find.in_replace {
            find.replacement.push_str(&clean);
        } else {
            find.query.push_str(&clean);
            self.refresh_find(true);
        }
    }

    /// Recomputes the matches and picks the first one at or after the caret.
    fn refresh_find(&mut self, jump: bool) {
        let Some(find) = &mut self.find else { return };
        find.matches = self.buffer.find_matches(&find.query, find.case_sensitive);
        find.stamp = Some((
            self.buffer.revision,
            find.case_sensitive,
            find.query.clone(),
        ));
        let caret = self
            .buffer
            .selection()
            .map_or((self.buffer.row, self.buffer.col), |(start, _)| start);
        find.current = find
            .matches
            .iter()
            .position(|&(row, start, _)| (row, start) >= caret)
            .or((!find.matches.is_empty()).then_some(0));
        if jump {
            self.select_current_match();
        }
    }

    /// Brings the matches up to date after the text, query or case option changed.
    fn ensure_find(&mut self) {
        let stale = self.find.as_ref().is_some_and(|find| {
            find.stamp.as_ref()
                != Some(&(
                    self.buffer.revision,
                    find.case_sensitive,
                    find.query.clone(),
                ))
        });
        if stale {
            self.refresh_find(false);
        }
    }

    fn select_current_match(&mut self) {
        let Some(find) = &self.find else { return };
        let Some(&(row, start, end)) = find.current.and_then(|index| find.matches.get(index))
        else {
            return;
        };
        self.buffer.set_cursor(row, start, false);
        self.buffer.set_cursor(row, end, true);
        self.follow_caret = true;
    }

    fn find_step(&mut self, forward: bool, now: f64) {
        self.ensure_find();
        let Some(find) = &mut self.find else { return };
        let count = find.matches.len();
        if count == 0 {
            self.say("No matches", now);
            return;
        }
        find.current = Some(match find.current {
            Some(index) if forward => (index + 1) % count,
            Some(index) => (index + count - 1) % count,
            None => 0,
        });
        self.select_current_match();
    }

    fn replace_current(&mut self, now: f64) {
        self.ensure_find();
        let Some(find) = &self.find else { return };
        let Some(&(row, start, end)) = find.current.and_then(|index| find.matches.get(index))
        else {
            self.say("No matches", now);
            return;
        };
        let replacement = find.replacement.clone();
        self.buffer.replace_range(row, start, end, &replacement);
        self.refresh_find(true);
        if self
            .find
            .as_ref()
            .is_some_and(|find| find.matches.is_empty())
        {
            self.say("No more matches", now);
        }
    }

    fn replace_everything(&mut self, now: f64) {
        let Some(find) = &self.find else { return };
        if !find.replace {
            self.say("Press Ctrl+H to show the replace field", now);
            return;
        }
        let (query, replacement, case) = (
            find.query.clone(),
            find.replacement.clone(),
            find.case_sensitive,
        );
        let replaced = self.buffer.replace_all(&query, &replacement, case);
        self.refresh_find(false);
        self.follow_caret = true;
        let message = if replaced == 0 {
            "No matches".to_owned()
        } else {
            format!("Replaced {replaced} occurrence(s); Ctrl+Z undoes them all")
        };
        self.say(message, now);
    }

    /// Keys while the find bar has focus: typing fills its fields.
    fn find_keys(&mut self, rl: &mut RaylibHandle, now: f64, ctrl: bool, shift: bool, alt: bool) {
        let mut changed = false;
        while let Some(c) = rl.get_char_pressed() {
            if (ctrl && !alt) || (alt && !ctrl) || c.is_control() {
                continue;
            }
            if let Some(find) = &mut self.find {
                if find.in_replace {
                    find.replacement.push(c);
                } else {
                    find.query.push(c);
                    changed = true;
                }
            }
        }
        if self.keys.fire(rl, KeyboardKey::KEY_BACKSPACE, now) {
            if let Some(find) = &mut self.find {
                if find.in_replace {
                    find.replacement.pop();
                } else {
                    find.query.pop();
                    changed = true;
                }
            }
        }
        if self.keys.fire(rl, KeyboardKey::KEY_TAB, now) {
            if let Some(find) = &mut self.find {
                if find.replace {
                    find.in_replace = !find.in_replace;
                }
            }
        }
        if alt && rl.is_key_pressed(KeyboardKey::KEY_C) {
            if let Some(find) = &mut self.find {
                find.case_sensitive = !find.case_sensitive;
                changed = true;
            }
        }
        if alt && rl.is_key_pressed(KeyboardKey::KEY_A) {
            self.replace_everything(now);
        }
        let enter = self.keys.fire(rl, KeyboardKey::KEY_ENTER, now)
            || self.keys.fire(rl, KeyboardKey::KEY_KP_ENTER, now);
        let in_replace = self.find.as_ref().is_some_and(|find| find.in_replace);
        if changed {
            self.refresh_find(true);
        }
        if enter && in_replace {
            self.replace_current(now);
        } else if enter {
            self.find_step(!shift, now);
        } else if self.keys.fire(rl, KeyboardKey::KEY_DOWN, now) {
            self.find_step(true, now);
        } else if self.keys.fire(rl, KeyboardKey::KEY_UP, now) {
            self.find_step(false, now);
        }
        if self.keys.fire(rl, KeyboardKey::KEY_F3, now) {
            self.find_step(!shift, now);
        }
    }

    /// Scrolls just enough to keep the caret on screen.
    fn follow(&mut self, geo: &Geometry) {
        let Some(rect) = geo.editor else { return };
        let pad = geo.line_h * 0.25;
        let top = self.buffer.row as f32 * geo.line_h;
        if top < self.scroll_y {
            self.scroll_y = top;
        } else if top + geo.line_h + pad > self.scroll_y + rect.height {
            self.scroll_y = top + geo.line_h + pad - rect.height;
        }
        let line = &self.buffer.lines[self.buffer.row];
        let x = visual_col(line, self.buffer.col) as f32 * geo.char_w;
        let view = (rect.width - geo.gutter).max(geo.char_w);
        let margin = (geo.char_w * 4.0).min(view / 4.0);
        if x < self.scroll_x + margin {
            self.scroll_x = (x - margin).max(0.0);
        } else if x + geo.char_w > self.scroll_x + view - margin {
            self.scroll_x = x + geo.char_w - view + margin;
        }
    }

    fn clamp_scroll(&mut self, geo: &Geometry) {
        if let Some(rect) = geo.editor {
            let content = self.buffer.lines.len() as f32 * geo.line_h + geo.line_h * 0.5;
            let max_y = (content - rect.height * 0.5).max(0.0);
            self.scroll_y = self.scroll_y.clamp(0.0, max_y);
            let first = (self.scroll_y / geo.line_h) as usize;
            let rows = (rect.height / geo.line_h) as usize + 2;
            let longest = self
                .buffer
                .lines
                .iter()
                .skip(first)
                .take(rows)
                .map(|line| visual_col(line, char_len(line)))
                .max()
                .unwrap_or(0);
            let max_x = ((longest as f32 + 2.0) * geo.char_w - (rect.width - geo.gutter)).max(0.0);
            self.scroll_x = self.scroll_x.clamp(0.0, max_x);
        }
    }

    fn draw<D: RaylibDraw>(&mut self, d: &mut D, geo: &Geometry, now: f64) {
        self.clamp_scroll(geo);
        if let Some(rect) = geo.editor {
            self.draw_editor(d, geo, rect, now);
        }
        if let Some(rect) = geo.preview {
            self.draw_preview(d, geo, rect, now);
        }
        if let Some(divider) = geo.divider {
            let active = self.divider_drag || self.cursor == MouseCursor::MOUSE_CURSOR_RESIZE_EW;
            let thickness = if active { 4 } else { 2 };
            d.draw_rectangle(
                (divider.x + divider.width / 2.0) as i32 - thickness / 2,
                divider.y as i32,
                thickness,
                divider.height as i32,
                if active { DIVIDER_ACTIVE } else { DIVIDER },
            );
        }
        if let Some(bar) = geo.find_bar {
            self.draw_find_bar(d, geo, bar, now);
        }
        self.draw_bars(d, geo, now);
    }

    fn draw_editor<D: RaylibDraw>(&mut self, d: &mut D, geo: &Geometry, rect: Rectangle, now: f64) {
        let (font, line_h, char_w) = (geo.font, geo.line_h, geo.char_w);
        let pad = line_h * 0.25;
        let (scroll_x, scroll_y) = (self.scroll_x, self.scroll_y);
        let total = self.buffer.lines.len();
        let first = (scroll_y / line_h).floor().max(0.0) as usize;
        let last = ((((scroll_y + rect.height) / line_h).ceil() as usize) + 1).min(total);
        self.ensure_states(last.saturating_sub(1));
        let text_rect = Rectangle::new(
            rect.x + geo.gutter,
            rect.y,
            rect.width - geo.gutter,
            rect.height,
        );
        let selection = self.buffer.selection();
        let caret_on = (now - self.blink_from) % 1.0 < 0.6;
        let from_col = (scroll_x / char_w).floor().max(0.0) as usize;
        let to_col = from_col + (text_rect.width / char_w).ceil() as usize + 2;
        let row_y = |row: usize| rect.y + pad + row as f32 * line_h - scroll_y;

        d.draw_rectangle_rec(rect, BG);
        {
            let mut clip = d.begin_scissor_mode(
                text_rect.x as i32,
                text_rect.y as i32,
                text_rect.width as i32,
                text_rect.height as i32,
            );
            for row in first..last {
                let y = row_y(row);
                let line = &self.buffer.lines[row];
                if row == self.buffer.row && selection.is_none() {
                    clip.draw_rectangle(
                        text_rect.x as i32,
                        (y - pad) as i32,
                        text_rect.width as i32,
                        line_h as i32,
                        CURRENT_LINE,
                    );
                }
                if let Some(((r0, c0), (r1, c1))) = selection {
                    if (r0..=r1).contains(&row) {
                        let start = if row == r0 { visual_col(line, c0) } else { 0 };
                        let end = if row == r1 {
                            visual_col(line, c1)
                        } else {
                            visual_col(line, char_len(line)) + 1
                        };
                        clip.draw_rectangle(
                            (text_rect.x + start as f32 * char_w - scroll_x) as i32,
                            (y - pad) as i32,
                            ((end - start) as f32 * char_w).ceil() as i32,
                            line_h as i32,
                            SELECTION,
                        );
                    }
                }
                if let Some(find) = &self.find {
                    let first_match = find.matches.partition_point(|&(r, _, _)| r < row);
                    for (index, &(r, start, end)) in
                        find.matches.iter().enumerate().skip(first_match)
                    {
                        if r != row {
                            break;
                        }
                        let (from, to) = (visual_col(line, start), visual_col(line, end));
                        clip.draw_rectangle(
                            (text_rect.x + from as f32 * char_w - scroll_x) as i32,
                            (y - pad) as i32,
                            ((to - from) as f32 * char_w).ceil() as i32,
                            line_h as i32,
                            if find.current == Some(index) {
                                FIND_CURRENT
                            } else {
                                FIND_MATCH
                            },
                        );
                    }
                }
                let chars: Vec<char> = line.chars().collect();
                let spans: Vec<Span> = if chars.len() > LONG_LINE {
                    Vec::new()
                } else {
                    syntax::highlight_line(self.language, &self.states[row], line).0
                };
                let mut segments: Vec<(usize, usize, Token)> = Vec::new();
                let mut position = 0;
                for span in &spans {
                    if span.start > position {
                        segments.push((position, span.start, Token::Plain));
                    }
                    segments.push((span.start, span.end, span.token));
                    position = span.end;
                }
                if position < chars.len() {
                    segments.push((position, chars.len(), Token::Plain));
                }
                let mut visual = 0;
                for (start, end, token) in segments {
                    let (text, first_col, next) =
                        expand_range(&chars[start..end], visual, from_col, to_col);
                    if !text.is_empty() {
                        draw_mono(
                            &mut clip,
                            &text,
                            text_rect.x + first_col as f32 * char_w - scroll_x,
                            y + (line_h - font) / 2.0 - pad,
                            font,
                            token_color(token),
                        );
                    }
                    visual = next;
                }
            }
            if caret_on && (first..last).contains(&self.buffer.row) {
                let line = &self.buffer.lines[self.buffer.row];
                let x = text_rect.x + visual_col(line, self.buffer.col) as f32 * char_w - scroll_x;
                clip.draw_rectangle(
                    x as i32,
                    (row_y(self.buffer.row) - pad) as i32 + 1,
                    2,
                    line_h as i32 - 2,
                    CARET,
                );
            }
        }
        {
            let mut clip = d.begin_scissor_mode(
                rect.x as i32,
                rect.y as i32,
                geo.gutter as i32,
                rect.height as i32,
            );
            clip.draw_rectangle(
                rect.x as i32,
                rect.y as i32,
                geo.gutter as i32,
                rect.height as i32,
                GUTTER_BG,
            );
            for row in first..last {
                let number = (row + 1).to_string();
                let color = if row == self.buffer.row { TEXT } else { DIM };
                let x = rect.x + geo.gutter - char_w * 1.2 - measure_mono(&number, font);
                draw_mono(
                    &mut clip,
                    &number,
                    x,
                    row_y(row) + (line_h - font) / 2.0 - pad,
                    font,
                    color,
                );
            }
        }
        d.draw_rectangle(
            (rect.x + geo.gutter) as i32 - 1,
            rect.y as i32,
            1,
            rect.height as i32,
            DIVIDER,
        );
    }

    fn ensure_preview(&mut self, width: f32, size: f32, now: f64) {
        let stale = match &self.preview {
            None => true,
            Some(cache) => {
                cache.width != width
                    || cache.size != size
                    || (cache.revision != self.buffer.revision
                        && now - cache.built_at > PREVIEW_DELAY)
            }
        };
        if !stale {
            return;
        }
        let blocks = markdown::parse(&self.buffer.lines.join("\n"));
        let metrics = markdown::Metrics {
            text: &|text, size| ui::measure(text, size),
            mono: &|text, size| measure_mono(text, size),
        };
        self.preview = Some(PreviewCache {
            revision: self.buffer.revision,
            width,
            size,
            built_at: now,
            layout: markdown::layout(&blocks, width, size, &metrics),
        });
    }

    fn draw_preview<D: RaylibDraw>(
        &mut self,
        d: &mut D,
        geo: &Geometry,
        rect: Rectangle,
        now: f64,
    ) {
        let pad = geo.ui_font * 1.2;
        let size = geo.ui_font;
        let width = (rect.width - pad * 2.0).floor().max(size * 4.0);
        self.ensure_preview(width, size, now);
        let height = self
            .preview
            .as_ref()
            .map_or(0.0, |cache| cache.layout.height);
        let max = (height + pad * 2.0 - rect.height).max(0.0);
        self.preview_scroll = self.preview_scroll.clamp(0.0, max);
        let scroll = self.preview_scroll;
        d.draw_rectangle_rec(rect, PREVIEW_BG);
        let Some(cache) = &self.preview else { return };
        let mut clip = d.begin_scissor_mode(
            rect.x as i32,
            rect.y as i32,
            rect.width as i32,
            rect.height as i32,
        );
        let (origin_x, origin_y) = preview_origin(geo, rect, scroll);
        for item in &cache.layout.items {
            match item {
                markdown::Item::Rect {
                    x,
                    y,
                    width,
                    height,
                    kind,
                } => {
                    let top = origin_y + y;
                    if top > rect.y + rect.height || top + height < rect.y {
                        continue;
                    }
                    let color = match kind {
                        markdown::RectKind::CodeBackground => CODE_BG,
                        markdown::RectKind::QuoteBar => QUOTE_BAR,
                        markdown::RectKind::TableHeader => TABLE_HEADER_BG,
                        _ => RULE,
                    };
                    clip.draw_rectangle(
                        (origin_x + x) as i32,
                        top as i32,
                        width.ceil().max(1.0) as i32,
                        height.ceil().max(1.0) as i32,
                        color,
                    );
                }
                markdown::Item::Text { y, size, runs } => {
                    let top = origin_y + y;
                    if top > rect.y + rect.height || top + size * 1.5 < rect.y {
                        continue;
                    }
                    for run in runs {
                        let x = origin_x + run.x;
                        let (color, mono) = if run.token != Token::Plain {
                            (token_color(run.token), run.style.code)
                        } else if run.style.link {
                            (token_color(Token::Link), run.style.code)
                        } else if run.style.code {
                            (TEXT, true)
                        } else if run.style.italic {
                            (PREVIEW_ITALIC, false)
                        } else {
                            (PREVIEW_TEXT, false)
                        };
                        let font = if mono { size * 0.92 } else { *size };
                        let text_width = if mono {
                            measure_mono(&run.text, font)
                        } else {
                            ui::measure(&run.text, font)
                        };
                        if run.style.code && run.token == Token::Code {
                            clip.draw_rectangle(
                                (x - 1.0) as i32,
                                top as i32,
                                (text_width + 2.0) as i32,
                                (size * 1.3) as i32,
                                CODE_BG,
                            );
                        }
                        let y_text = if mono { top + size * 0.04 } else { top };
                        let passes = if run.style.bold { 2 } else { 1 };
                        for pass in 0..passes {
                            let nudge = pass as f32 * (font / 18.0).max(0.6);
                            if mono {
                                draw_mono(&mut clip, &run.text, x + nudge, y_text, font, color);
                            } else {
                                ui::draw_text(&mut clip, &run.text, x + nudge, y_text, font, color);
                            }
                        }
                        let visible = if mono {
                            measure_mono(run.text.trim_end(), font)
                        } else {
                            ui::measure(run.text.trim_end(), font)
                        };
                        if run.style.link {
                            clip.draw_rectangle(
                                x as i32,
                                (top + size * 1.12) as i32,
                                visible as i32,
                                1,
                                color,
                            );
                        }
                        if run.style.strike {
                            clip.draw_rectangle(
                                x as i32,
                                (top + size * 0.58) as i32,
                                visible as i32,
                                1,
                                color,
                            );
                        }
                    }
                }
            }
        }
    }

    fn draw_find_bar<D: RaylibDraw>(&self, d: &mut D, geo: &Geometry, bar: Rectangle, now: f64) {
        let Some(find) = &self.find else { return };
        let size = geo.ui_font;
        let pad = size * 0.6;
        let rows = if find.replace { 2.0 } else { 1.0 };
        let row_h = bar.height / rows;
        d.draw_rectangle_rec(bar, BAR_BG);
        d.draw_rectangle(
            bar.x as i32,
            (bar.y + bar.height) as i32 - 1,
            bar.width as i32,
            1,
            DIVIDER,
        );

        let label_w = ui::measure("Replace", size) + pad;
        let count = if find.query.is_empty() {
            String::new()
        } else if find.matches.is_empty() {
            "No matches".to_owned()
        } else {
            let more = if find.matches.len() >= MAX_MATCHES {
                "+"
            } else {
                ""
            };
            format!(
                "{} of {}{more}",
                find.current.map_or(0, |index| index + 1),
                find.matches.len()
            )
        };
        let case_label = "Aa";
        let case_w = ui::measure(case_label, size);
        let count_w = ui::measure(&count, size);
        let reserved = case_w + count_w + pad * 3.0;
        let caret_on = (now - self.blink_from) % 1.0 < 0.6;

        for row in 0..(rows as usize) {
            let y = bar.y + row as f32 * row_h;
            let (label, text, focused) = if row == 0 {
                ("Find", &find.query, !find.in_replace)
            } else {
                ("Replace", &find.replacement, find.in_replace)
            };
            ui::draw_text(d, label, bar.x + pad, y + (row_h - size) / 2.0, size, DIM);
            let field_x = bar.x + pad + label_w;
            let field_w = (bar.width - label_w - pad * 2.0 - if row == 0 { reserved } else { 0.0 })
                .max(geo.char_w * 4.0);
            let field_y = y + row_h * 0.12;
            let field_h = row_h * 0.76;
            d.draw_rectangle(
                field_x as i32,
                field_y as i32,
                field_w as i32,
                field_h as i32,
                FIELD_BG,
            );
            let border = if focused { CARET } else { DIVIDER };
            d.draw_rectangle_lines(
                field_x as i32,
                field_y as i32,
                field_w as i32,
                field_h as i32,
                border,
            );
            let room = (((field_w - pad) / geo.char_w).floor() as usize).max(1);
            let chars: Vec<char> = text.chars().collect();
            let shown: String = chars[chars.len().saturating_sub(room)..].iter().collect();
            let text_y = field_y + (field_h - geo.font) / 2.0;
            draw_mono(d, &shown, field_x + pad / 2.0, text_y, geo.font, TEXT);
            if focused && caret_on {
                let caret_x = field_x + pad / 2.0 + char_len(&shown) as f32 * geo.char_w;
                d.draw_rectangle(caret_x as i32, text_y as i32, 2, geo.font as i32, CARET);
            }
            if row == 0 {
                let right = bar.x + bar.width - pad;
                let (color, shown_count) = if find.matches.is_empty() && !find.query.is_empty() {
                    (WARNING, &count)
                } else {
                    (DIM, &count)
                };
                ui::draw_text(
                    d,
                    shown_count,
                    right - count_w,
                    y + (row_h - size) / 2.0,
                    size,
                    color,
                );
                ui::draw_text(
                    d,
                    case_label,
                    right - count_w - pad - case_w,
                    y + (row_h - size) / 2.0,
                    size,
                    if find.case_sensitive { CARET } else { DIM },
                );
            }
        }
    }

    fn draw_bars<D: RaylibDraw>(&self, d: &mut D, geo: &Geometry, now: f64) {
        let size = geo.ui_font;
        let pad = size * 0.8;
        d.draw_rectangle(0, 0, geo.width as i32, geo.bar_h as i32, BAR_BG);
        let mut title = self.name();
        if self.buffer.is_dirty() {
            title.push_str(" •");
        }
        let view = match (self.language, self.view) {
            (Language::Markdown, View::Split) if geo.collapsed => " | Split (too narrow)",
            (Language::Markdown, View::Split) => " | Split",
            (Language::Markdown, View::Preview) => " | Preview",
            (Language::Markdown, View::Edit) => " | Source",
            _ => "",
        };
        let right = format!("{}{view}", self.language.name());
        let width = ui::measure(&right, size);
        let show_right = geo.width > width + ui::measure("MMMMMM", size) + pad * 3.0;
        let title_room = geo.width - pad * 2.0 - if show_right { width + pad } else { 0.0 };
        let title = ui::fit(&title, size, title_room);
        ui::draw_text(d, &title, pad, (geo.bar_h - size) / 2.0, size, TEXT);
        if show_right {
            ui::draw_text(
                d,
                &right,
                geo.width - width - pad,
                (geo.bar_h - size) / 2.0,
                size,
                DIM,
            );
        }

        let top = geo.height - geo.status_h;
        d.draw_rectangle(0, top as i32, geo.width as i32, geo.status_h as i32, BAR_BG);
        let position = format!("Ln {}, Col {}", self.buffer.row + 1, self.buffer.col + 1);
        let position_width = ui::measure(&position, size);
        ui::draw_text(
            d,
            &position,
            geo.width - position_width - pad,
            top + (geo.status_h - size) / 2.0,
            size,
            DIM,
        );
        let (message, color) = match (&self.status, &self.hover_link) {
            (Some((message, time)), _) if now - time < 8.0 => (message.clone(), WARNING),
            (_, Some(link)) => (format!("Link: {link}  (click to open)"), DIM),
            _ if self.find.is_some() => (
                "Enter Next  |  Shift+Enter Previous  |  Tab Switch field  |  Alt+C Case  |  Alt+A Replace all  |  Esc Close".to_owned(),
                DIM,
            ),
            _ => (
                "Ctrl+S Save  |  Ctrl+F Find  |  Ctrl+H Replace  |  Ctrl+Z / Ctrl+Y Undo / Redo  |  Ctrl+P Preview  |  Ctrl +/- Zoom  |  F11 Maximize  |  Esc Close".to_owned(),
                DIM,
            ),
        };
        let message = ui::fit(&message, size, geo.width - position_width - pad * 3.0);
        ui::draw_text(
            d,
            &message,
            pad,
            top + (geo.status_h - size) / 2.0,
            size,
            color,
        );
    }
}

fn write_atomic(target: &Path, bytes: &[u8]) -> io::Result<()> {
    let directory = target
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = directory.join(format!(".{name}.selenite-save"));
    fs::write(&temporary, bytes)?;
    if let Ok(meta) = fs::metadata(target) {
        let _ = fs::set_permissions(&temporary, meta.permissions());
    }
    fs::rename(&temporary, target).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

/// Opens the editor window for `path` (created on first save if missing).
pub fn run(path: &Path) -> Result<(), String> {
    let mut editor = Editor::open(path)?;
    let frame_limit: Option<u32> = env::var("SELENITE_EDITOR_FRAMES")
        .ok()
        .and_then(|value| value.parse().ok());
    let (width, height) = env::var("SELENITE_EDITOR_SIZE")
        .ok()
        .and_then(|value| parse_size(&value))
        .unwrap_or((1100, 760));
    let (mut rl, thread) = raylib::init()
        .size(width, height)
        .title(&editor.title())
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_exit_key(None);
    rl.set_window_min_size(420, 300);
    if env::var_os("SELENITE_EDITOR_SIZE").is_none() {
        ui::fit_window_to_display(&mut rl, width, height);
    }
    ui::set_user_scale(crate::settings::saved_ui_scale());
    ui::load_font(&thread);
    load_mono_font();
    let mut title_dirty = false;
    let mut frames = 0u32;
    let mut applied_cursor = MouseCursor::MOUSE_CURSOR_DEFAULT;

    while !rl.window_should_close() {
        let now = rl.get_time();
        if let Some(scale) = ui::handle_scale_keys(&rl).or_else(|| ui::handle_scale_wheel(&rl)) {
            let _ = crate::settings::save_ui_scale(scale);
            editor.follow_caret = true;
            editor.say(
                format!(
                    "UI scale {:.0}% (Ctrl +/- or Ctrl+wheel, Ctrl 0 resets)",
                    scale * 100.0
                ),
                now,
            );
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F11) {
            if rl.is_window_maximized() {
                rl.restore_window();
            } else {
                rl.maximize_window();
            }
        }
        let geo = editor.geometry(rl.get_screen_width(), rl.get_screen_height());
        editor.update(&mut rl, &geo, now);
        if editor.cursor != applied_cursor {
            rl.set_mouse_cursor(editor.cursor);
            applied_cursor = editor.cursor;
        }
        if editor.quit {
            break;
        }
        if editor.buffer.is_dirty() != title_dirty {
            title_dirty = editor.buffer.is_dirty();
            rl.set_window_title(&thread, &editor.title());
        }
        let mut finished = false;
        {
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(BG);
            editor.draw(&mut d, &geo, now);
            frames += 1;
            if frame_limit.is_some_and(|limit| frames >= limit) {
                if let Ok(target) = env::var("SELENITE_EDITOR_SCREENSHOT") {
                    if !capture_screen(&target) {
                        eprintln!("selenite: could not save screenshot to {target}");
                    }
                }
                finished = true;
            }
        }
        if finished {
            editor.discard = true;
            break;
        }
        if frames == 1 {
            if let Some(ready) = env::var_os("SELENITE_EDITOR_READY") {
                let _ = fs::write(ready, "");
            }
        }
    }
    if editor.buffer.is_dirty() && !editor.discard {
        editor.save(true, 0.0)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(text: &str) -> Buffer {
        Buffer::new(text)
    }

    #[test]
    fn text_round_trips_with_line_endings() {
        let unix = buffer("a\nb\n");
        assert_eq!(unix.lines, vec!["a", "b", ""]);
        assert_eq!(unix.text(), "a\nb\n");
        let windows = buffer("a\r\nb");
        assert_eq!(windows.lines, vec!["a", "b"]);
        assert_eq!(windows.text(), "a\r\nb");
        assert_eq!(buffer("").text(), "");
    }

    #[test]
    fn typing_and_deleting_edit_text_and_track_dirtiness() {
        let mut b = buffer("héllo");
        b.set_cursor(0, 2, false);
        b.insert_text("X");
        assert_eq!(b.lines[0], "héXllo");
        assert!(b.is_dirty());
        b.mark_saved();
        assert!(!b.is_dirty());
        b.backspace();
        assert_eq!(b.lines[0], "héllo");
        b.delete_forward();
        assert_eq!(b.lines[0], "hélo");
        b.set_cursor(0, 0, false);
        b.backspace();
        assert_eq!(b.lines, vec!["hélo"]);
        b.insert_text("a\nb\nc");
        assert_eq!(b.lines, vec!["a", "b", "chélo"]);
    }

    #[test]
    fn multiline_insert_splits_and_joins_lines() {
        let mut b = buffer("start end");
        b.set_cursor(0, 6, false);
        b.insert_text("one\ntwo\nthree ");
        assert_eq!(b.lines, vec!["start one", "two", "three end"]);
        assert_eq!((b.row, b.col), (2, 6));
        b.set_cursor(2, 0, false);
        b.backspace();
        assert_eq!(b.lines, vec!["start one", "twothree end"]);
        b.set_cursor(0, 9, false);
        b.delete_forward();
        assert_eq!(b.lines, vec!["start onetwothree end"]);
        b.insert_text("\u{7}\r\n");
        assert_eq!(b.lines.len(), 2);
    }

    #[test]
    fn selection_copy_delete_and_replace() {
        let mut b = buffer("alpha\nbeta\ngamma");
        b.set_cursor(0, 2, false);
        b.set_cursor(2, 2, true);
        assert_eq!(b.selected_text(), "pha\nbeta\nga");
        b.insert_text("-");
        assert_eq!(b.lines, vec!["al-mma"]);
        b.select_all();
        assert_eq!(b.selected_text(), "al-mma");
        b.backspace();
        assert_eq!(b.lines, vec![""]);
        let mut words = buffer("one two_2  three");
        words.select_word_at(0, 5);
        assert_eq!(words.selected_text(), "two_2");
        words.select_word_at(0, 9);
        assert_eq!(words.selected_text(), "  ");
        words.select_line(0);
        assert_eq!(words.selected_text(), "one two_2  three");
    }

    #[test]
    fn cursor_movement_keeps_wanted_column_and_words() {
        let mut b = buffer("long line here\nx\nanother long one");
        b.set_cursor(0, 10, false);
        b.move_vertical(1, false);
        assert_eq!((b.row, b.col), (1, 1));
        b.move_vertical(1, false);
        assert_eq!((b.row, b.col), (2, 10));
        b.move_vertical(5, false);
        assert_eq!((b.row, b.col), (2, 16));
        b.move_vertical(-9, false);
        assert_eq!((b.row, b.col), (0, 0));
        b.move_word(true, false);
        assert_eq!(b.col, 4);
        b.move_word(true, false);
        assert_eq!(b.col, 9);
        b.move_word(false, false);
        assert_eq!(b.col, 5);
        b.move_end(false);
        b.move_right(false);
        assert_eq!((b.row, b.col), (1, 0));
        b.move_left(false);
        assert_eq!((b.row, b.col), (0, 14));
        b.move_document(false, false);
        assert_eq!((b.row, b.col), (2, 16));
        let mut indented = buffer("    code");
        indented.move_end(false);
        indented.move_home(false);
        assert_eq!(indented.col, 4);
        indented.move_home(false);
        assert_eq!(indented.col, 0);
    }

    #[test]
    fn shift_selection_collapses_toward_the_arrow() {
        let mut b = buffer("abcdef");
        b.set_cursor(0, 2, false);
        b.move_right(true);
        b.move_right(true);
        assert_eq!(b.selected_text(), "cd");
        b.move_left(false);
        assert_eq!((b.row, b.col, b.has_selection()), (0, 2, false));
        b.move_right(true);
        b.move_right(true);
        b.move_right(false);
        assert_eq!((b.row, b.col), (0, 4));
    }

    #[test]
    fn undo_redo_coalesces_typing_runs() {
        let mut b = buffer("");
        for c in "abc".chars() {
            b.insert_text(&c.to_string());
        }
        b.move_left(false);
        b.insert_text("X");
        assert_eq!(b.lines[0], "abXc");
        assert!(b.undo());
        assert_eq!(b.lines[0], "abc");
        assert!(b.undo());
        assert_eq!(b.lines[0], "");
        assert!(!b.undo());
        assert!(b.redo());
        assert_eq!(b.lines[0], "abc");
        b.move_end(false);
        b.insert_text("!");
        assert!(!b.redo(), "a new edit clears redo");
        b.newline(Language::Plain);
        assert!(b.undo());
        assert_eq!(b.lines, vec!["abc!"]);
    }

    #[test]
    fn ruby_newline_indents_and_end_aligns() {
        let mut b = buffer("");
        b.insert_text("class Foo");
        b.newline(Language::Ruby);
        assert_eq!(b.lines, vec!["class Foo", "  "]);
        b.insert_text("def bar");
        b.newline(Language::Ruby);
        b.insert_text("1");
        b.newline(Language::Ruby);
        assert_eq!(b.lines[3], "    ");
        for c in "end".chars() {
            b.insert_text(&c.to_string());
            b.after_typed();
        }
        assert_eq!(b.lines[3], "  end");
        assert_eq!(b.col, 5);
        b.newline(Language::Ruby);
        for c in "end".chars() {
            b.insert_text(&c.to_string());
            b.after_typed();
        }
        assert_eq!(b.lines[4], "end");
        let mut plain = buffer("  indented");
        plain.move_end(false);
        plain.newline(Language::Plain);
        assert_eq!(plain.lines[1], "  ");
    }

    #[test]
    fn markdown_lists_continue_and_end() {
        let mut b = buffer("- one");
        b.move_end(false);
        b.newline(Language::Markdown);
        assert_eq!(b.lines, vec!["- one", "- "]);
        b.newline(Language::Markdown);
        assert_eq!(b.lines, vec!["- one", ""]);
        let mut numbered = buffer("  9. item");
        numbered.move_end(false);
        numbered.newline(Language::Markdown);
        assert_eq!(numbered.lines[1], "  10. ");
        let mut task = buffer("- [x] done");
        task.move_end(false);
        task.newline(Language::Markdown);
        assert_eq!(task.lines[1], "- [ ] ");
        let mut plain_text = buffer("text");
        plain_text.move_end(false);
        plain_text.newline(Language::Markdown);
        assert_eq!(plain_text.lines, vec!["text", ""]);
        let mut split = buffer("- ab");
        split.set_cursor(0, 3, false);
        split.newline(Language::Markdown);
        assert_eq!(split.lines, vec!["- a", "- b"]);
    }

    #[test]
    fn indent_and_unindent_selected_lines() {
        let mut b = buffer("a\n\nb\nc");
        b.set_cursor(0, 0, false);
        b.set_cursor(2, 1, true);
        b.indent_lines(2);
        assert_eq!(b.lines, vec!["  a", "", "  b", "c"]);
        assert_eq!(b.selected_text(), "a\n\n  b");
        b.unindent_lines(2);
        assert_eq!(b.lines, vec!["a", "", "b", "c"]);
        let mut single = buffer("\tkeep");
        single.unindent_lines(2);
        assert_eq!(single.lines[0], "keep");
        single.indent_lines(4);
        assert_eq!(single.lines[0], "    keep");
        assert_eq!(single.col, 4);
    }

    #[test]
    fn tabs_use_tab_stops_in_screen_columns() {
        assert_eq!(visual_col("a\tb", 2), 4);
        assert_eq!(visual_col("\t\t", 2), 8);
        assert_eq!(column_at("a\tb", 2.0), 1);
        assert_eq!(column_at("a\tb", 3.9), 2);
        assert_eq!(column_at("abc", 99.0), 3);
        assert_eq!(
            expand_range(&['a', '\t', 'b'], 0, 0, 10),
            ("a   b".to_owned(), 0, 5)
        );
        assert_eq!(
            expand_range(&['a', 'b', 'c', 'd'], 0, 2, 3),
            ("c".to_owned(), 2, 3)
        );
    }

    #[test]
    fn panes_fill_the_window_and_collapse_when_narrow() {
        let fills = |panes: &Panes, width: f32| {
            let editor = panes.editor.map_or(0.0, |rect| rect.width);
            let preview = panes.preview.map_or(0.0, |rect| rect.width);
            (editor + preview - width).abs() < 0.01
        };
        for width in [300.0, 500.0, 960.0, 1920.0, 3840.0, 7680.0] {
            for view in [View::Edit, View::Split, View::Preview] {
                for language in [Language::Plain, Language::Ruby, Language::Markdown] {
                    let panes = pane_layout(language, view, width, 30.0, 400.0, 250.0, 0.5);
                    assert!(fills(&panes, width), "{width} {}", language.name());
                    for rect in [panes.editor, panes.preview].into_iter().flatten() {
                        assert!(rect.x >= 0.0 && rect.x + rect.width <= width + 0.01);
                        assert_eq!((rect.y, rect.height), (30.0, 400.0));
                    }
                    assert!(panes.editor.is_some() || panes.preview.is_some());
                }
            }
        }
        let split = pane_layout(
            Language::Markdown,
            View::Split,
            1000.0,
            0.0,
            10.0,
            250.0,
            0.5,
        );
        assert!(!split.collapsed && split.editor.unwrap().width == 500.0);
        assert_eq!(split.preview.unwrap().x, 500.0);
        let narrow = pane_layout(
            Language::Markdown,
            View::Split,
            499.0,
            0.0,
            10.0,
            250.0,
            0.5,
        );
        assert!(narrow.collapsed && narrow.preview.is_none());
        assert!(narrow.editor.unwrap().width == 499.0);
        let ruby = pane_layout(Language::Ruby, View::Split, 2000.0, 0.0, 10.0, 250.0, 0.5);
        assert!(ruby.preview.is_none() && !ruby.collapsed);
        let preview = pane_layout(
            Language::Markdown,
            View::Preview,
            2000.0,
            0.0,
            10.0,
            250.0,
            0.5,
        );
        assert!(preview.editor.is_none() && preview.preview.unwrap().width == 2000.0);
    }

    #[test]
    fn the_split_ratio_is_honored_and_kept_inside_the_window() {
        let source = |ratio: f32| {
            pane_layout(
                Language::Markdown,
                View::Split,
                1000.0,
                0.0,
                10.0,
                250.0,
                ratio,
            )
            .editor
            .unwrap()
            .width
        };
        assert_eq!(source(0.5), 500.0);
        assert_eq!(source(0.7), 700.0);
        assert_eq!(source(0.0), 250.0, "never narrower than a minimum pane");
        assert_eq!(source(1.0), 750.0, "the preview keeps its minimum too");
        assert_eq!(source(-4.0), 250.0);
        let panes = pane_layout(
            Language::Markdown,
            View::Split,
            1000.0,
            0.0,
            10.0,
            250.0,
            0.3,
        );
        assert_eq!(panes.preview.unwrap().x, 300.0);
        assert_eq!(panes.preview.unwrap().width, 700.0);
    }

    #[test]
    fn find_in_line_matches_literal_text_with_optional_case() {
        assert_eq!(
            find_in_line("Foo foo FOO", "foo", false),
            vec![(0, 3), (4, 7), (8, 11)]
        );
        assert_eq!(find_in_line("Foo foo FOO", "foo", true), vec![(4, 7)]);
        assert_eq!(
            find_in_line("aaaa", "aa", true),
            vec![(0, 2), (2, 4)],
            "no overlaps"
        );
        assert_eq!(
            find_in_line("héllo HÉLLO", "é", false),
            vec![(1, 2), (7, 8)]
        );
        assert_eq!(find_in_line("a.b*c", ".b*", true), vec![(1, 4)], "no regex");
        assert!(find_in_line("abc", "", false).is_empty());
        assert!(find_in_line("ab", "abc", false).is_empty());
        assert!(find_in_line("", "a", false).is_empty());
    }

    #[test]
    fn buffer_finds_across_lines_and_replaces_one_or_all_with_undo() {
        let mut b = buffer("one two\nTwo three\nnothing\ntwo");
        assert_eq!(
            b.find_matches("two", false),
            vec![(0, 4, 7), (1, 0, 3), (3, 0, 3)]
        );
        assert_eq!(b.find_matches("two", true), vec![(0, 4, 7), (3, 0, 3)]);
        b.replace_range(0, 4, 7, "2");
        assert_eq!(b.lines[0], "one 2");
        assert_eq!((b.row, b.col), (0, 5));
        assert!(b.undo());
        assert_eq!(b.lines[0], "one two");

        let replaced = b.replace_all("two", "[2]", false);
        assert_eq!(replaced, 3);
        assert_eq!(b.lines, vec!["one [2]", "[2] three", "nothing", "[2]"]);
        assert!(b.undo(), "one undo restores everything");
        assert_eq!(b.lines, vec!["one two", "Two three", "nothing", "two"]);
        assert_eq!(b.replace_all("zzz", "x", false), 0);
        assert!(b.redo(), "a search with no matches leaves redo alone");
        assert_eq!(b.lines[0], "one [2]");

        let mut grow = buffer("a a a");
        assert_eq!(grow.replace_all("a", "aa", true), 3);
        assert_eq!(
            grow.lines[0], "aa aa aa",
            "replacements are not searched again"
        );
        let mut unicode = buffer("é é");
        assert_eq!(unicode.replace_all("é", "ee", true), 2);
        assert_eq!(unicode.lines[0], "ee ee");
        let mut shrink = buffer("keep delete keep");
        shrink.set_cursor(0, 16, false);
        shrink.replace_all("delete ", "", true);
        assert_eq!((shrink.lines[0].as_str(), shrink.col), ("keep keep", 9));
    }

    #[test]
    fn match_count_is_capped() {
        let b = buffer(&"a\n".repeat(MAX_MATCHES + 50));
        assert_eq!(b.find_matches("a", true).len(), MAX_MATCHES);
    }

    #[test]
    fn window_size_overrides_are_validated() {
        assert_eq!(parse_size("1280x720"), Some((1280, 720)));
        assert_eq!(parse_size(" 800 x 600 "), Some((800, 600)));
        for bad in [
            "",
            "800",
            "800x",
            "x600",
            "10x10",
            "99999x600",
            "-5x600",
            "axb",
            "800X600",
        ] {
            assert_eq!(parse_size(bad), None, "{bad}");
        }
    }

    #[test]
    fn file_helpers_name_and_create_notes() {
        assert!(is_text_path(Path::new("a/Readme.MD")));
        assert!(is_text_path(Path::new("n.txt")));
        assert!(!is_text_path(Path::new("n.rb")));
        let directory =
            env::temp_dir().join(format!("selenite-editor-notes-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let first = unique_note_path(&directory);
        assert_eq!(first, directory.join("note.md"));
        create_note_if_missing(&first, "Title").unwrap();
        assert_eq!(fs::read_to_string(&first).unwrap(), "# Title\n\n");
        fs::write(&first, "kept").unwrap();
        create_note_if_missing(&first, "Other").unwrap();
        assert_eq!(fs::read_to_string(&first).unwrap(), "kept");
        assert_eq!(unique_note_path(&directory), directory.join("note-2.md"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn atomic_save_keeps_permissions_and_cleans_up() {
        let directory =
            env::temp_dir().join(format!("selenite-editor-save-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let target = directory.join("file.rb");
        fs::write(&target, "old").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(0o750)).unwrap();
        }
        write_atomic(&target, b"new").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&target).unwrap().permissions().mode() & 0o777,
                0o750
            );
        }
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn opening_rejects_binary_and_oversized_files_and_keeps_bom() {
        let directory =
            env::temp_dir().join(format!("selenite-editor-open-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let binary = directory.join("b.txt");
        fs::write(&binary, [0xff, 0xfe, 0x00]).unwrap();
        assert!(Editor::open(&binary).err().unwrap().contains("not UTF-8"));
        assert!(Editor::open(&directory)
            .err()
            .unwrap()
            .contains("not a file"));
        let bom = directory.join("bom.md");
        fs::write(&bom, "\u{feff}# T").unwrap();
        let editor = Editor::open(&bom).unwrap();
        assert!(editor.bom && editor.buffer.lines == vec!["# T"]);
        assert!(editor.language == Language::Markdown && editor.view == View::Split);
        let missing = Editor::open(&directory.join("new.rb")).unwrap();
        assert!(missing.buffer.lines == vec![""] && missing.loaded.is_none());
        fs::remove_dir_all(directory).unwrap();
    }
}

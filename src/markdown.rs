//! Markdown parsing and layout for the editor's live preview.
//!
//! Supports headings (ATX and setext), paragraphs with hard breaks, emphasis,
//! strikethrough, inline code, links, images (shown as links), fenced code
//! with Ruby highlighting, block quotes, nested bullet/numbered/task lists,
//! horizontal rules and pipe tables.

use std::path::{Path, PathBuf};

use crate::syntax::{self, Language, LineState, Token};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inline {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub link: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(u8, Vec<Inline>),
    Paragraph(Vec<Inline>),
    Code {
        language: String,
        text: String,
    },
    Quote(Vec<Block>),
    List {
        ordered: bool,
        start: u64,
        items: Vec<ListItem>,
    },
    Rule,
    Table {
        header: Vec<Vec<Inline>>,
        aligns: Vec<Align>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
}

pub fn parse(text: &str) -> Vec<Block> {
    let lines: Vec<String> = text
        .replace("\r\n", "\n")
        .split('\n')
        .map(|line| line.replace('\t', "    "))
        .collect();
    parse_blocks(&lines)
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

fn strip_indent(line: &str, amount: usize) -> String {
    let remove = indent_of(line).min(amount);
    line[remove..].to_owned()
}

fn heading(line: &str) -> Option<(u8, &str)> {
    if indent_of(line) > 3 {
        return None;
    }
    let text = line.trim_start();
    let level = text.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &text[level..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.trim();
    let without = rest.trim_end_matches('#');
    let content = if without.len() != rest.len() && (without.is_empty() || without.ends_with(' ')) {
        without.trim()
    } else {
        rest
    };
    Some((level as u8, content))
}

fn fence(line: &str) -> Option<(char, usize, &str)> {
    if indent_of(line) > 3 {
        return None;
    }
    let text = line.trim_start();
    let marker = text.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let length = text.chars().take_while(|c| *c == marker).count();
    if length < 3 {
        return None;
    }
    let info = text[length..].trim();
    if marker == '`' && info.contains('`') {
        return None;
    }
    Some((marker, length, info))
}

fn is_rule(line: &str) -> bool {
    if indent_of(line) > 3 {
        return false;
    }
    let marks: Vec<char> = line.chars().filter(|c| !c.is_whitespace()).collect();
    marks.len() >= 3 && matches!(marks[0], '-' | '*' | '_') && marks.iter().all(|c| *c == marks[0])
}

fn is_quote(line: &str) -> bool {
    indent_of(line) <= 3 && line.trim_start().starts_with('>')
}

struct ListMarker {
    indent: usize,
    ordered: bool,
    number: u64,
    /// Column where the item's content starts.
    content: usize,
}

fn list_marker(line: &str) -> Option<ListMarker> {
    let indent = indent_of(line);
    let text = &line[indent..];
    let (ordered, number, marker_len) = match text.chars().next()? {
        '-' | '*' | '+' => (false, 0, 1),
        c if c.is_ascii_digit() => {
            let digits = text.chars().take_while(char::is_ascii_digit).count();
            if digits > 9 || !matches!(text[digits..].chars().next(), Some('.' | ')')) {
                return None;
            }
            (true, text[..digits].parse().ok()?, digits + 1)
        }
        _ => return None,
    };
    let rest = &text[marker_len..];
    if !rest.is_empty() && !rest.starts_with(' ') || is_rule(line) {
        return None;
    }
    let spaces = rest.len() - rest.trim_start_matches(' ').len();
    let spaces = if rest.trim().is_empty() || spaces > 4 {
        1
    } else {
        spaces
    };
    Some(ListMarker {
        indent,
        ordered,
        number,
        content: indent + marker_len + spaces,
    })
}

fn split_row(line: &str) -> Vec<String> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = line
        .strip_suffix('|')
        .filter(|_| !line.ends_with("\\|"))
        .unwrap_or(line);
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'|') {
            current.push('|');
            chars.next();
        } else if c == '|' {
            cells.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(c);
        }
    }
    cells.push(current.trim().to_owned());
    cells
}

fn table_alignments(line: &str, columns: usize) -> Option<Vec<Align>> {
    if !line.contains('-') {
        return None;
    }
    let cells = split_row(line);
    if cells.len() != columns {
        return None;
    }
    cells
        .iter()
        .map(|cell| {
            let body = cell.trim_matches(':');
            if body.is_empty() || !body.chars().all(|c| c == '-') {
                return None;
            }
            Some(match (cell.starts_with(':'), cell.ends_with(':')) {
                (true, true) => Align::Center,
                (false, true) => Align::Right,
                _ => Align::Left,
            })
        })
        .collect()
}

/// Whether `line` can interrupt a paragraph.
fn interrupts_paragraph(line: &str) -> bool {
    is_blank(line)
        || heading(line).is_some()
        || fence(line).is_some()
        || is_quote(line)
        || is_rule(line)
        || list_marker(line).is_some_and(|marker| !marker.ordered || marker.number == 1)
}

fn setext_level(line: &str) -> Option<u8> {
    if indent_of(line) > 3 {
        return None;
    }
    let text = line.trim();
    if !text.is_empty() && text.chars().all(|c| c == '=') {
        Some(1)
    } else if !text.is_empty() && text.chars().all(|c| c == '-') {
        Some(2)
    } else {
        None
    }
}

fn parse_blocks(lines: &[String]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = &lines[index];
        if is_blank(line) {
            index += 1;
        } else if let Some((marker, length, info)) = fence(line) {
            let language = info.split_whitespace().next().unwrap_or("").to_owned();
            let base = indent_of(line);
            let mut body = Vec::new();
            index += 1;
            while index < lines.len() {
                if let Some((m, l, rest)) = fence(&lines[index]) {
                    if m == marker && l >= length && rest.is_empty() {
                        index += 1;
                        break;
                    }
                }
                body.push(strip_indent(&lines[index], base));
                index += 1;
            }
            blocks.push(Block::Code {
                language,
                text: body.join("\n"),
            });
        } else if let Some((level, content)) = heading(line) {
            blocks.push(Block::Heading(level, parse_inline(content)));
            index += 1;
        } else if is_rule(line) {
            blocks.push(Block::Rule);
            index += 1;
        } else if is_quote(line) {
            let mut inner = Vec::new();
            while index < lines.len() && is_quote(&lines[index]) {
                let text = lines[index].trim_start();
                let text = &text[1..];
                inner.push(text.strip_prefix(' ').unwrap_or(text).to_owned());
                index += 1;
            }
            blocks.push(Block::Quote(parse_blocks(&inner)));
        } else if list_marker(line).is_some() {
            blocks.push(parse_list(lines, &mut index));
        } else if let Some(table) = parse_table(lines, &mut index) {
            blocks.push(table);
        } else {
            let mut paragraph = vec![line.as_str()];
            let mut level = None;
            index += 1;
            while index < lines.len() {
                let next = &lines[index];
                if let Some(setext) = setext_level(next) {
                    level = Some(setext);
                    index += 1;
                    break;
                }
                if interrupts_paragraph(next) {
                    break;
                }
                paragraph.push(next.as_str());
                index += 1;
            }
            let mut text = String::new();
            for (position, raw) in paragraph.iter().enumerate() {
                let hard = raw.ends_with("  ") || raw.ends_with('\\');
                text.push_str(raw.trim().trim_end_matches('\\').trim_end());
                if position + 1 < paragraph.len() {
                    text.push(if hard { '\n' } else { ' ' });
                }
            }
            let inlines = parse_inline(&text);
            blocks.push(match level {
                Some(level) => Block::Heading(level, inlines),
                None => Block::Paragraph(inlines),
            });
        }
    }
    blocks
}

fn parse_table(lines: &[String], index: &mut usize) -> Option<Block> {
    let header_line = &lines[*index];
    let separator = lines.get(*index + 1)?;
    if !header_line.contains('|') {
        return None;
    }
    let header = split_row(header_line);
    let aligns = table_alignments(separator, header.len())?;
    let mut rows = Vec::new();
    let mut next = *index + 2;
    while next < lines.len() && !is_blank(&lines[next]) && lines[next].contains('|') {
        let mut cells = split_row(&lines[next]);
        cells.resize(header.len(), String::new());
        rows.push(cells.iter().map(|cell| parse_inline(cell)).collect());
        next += 1;
    }
    *index = next;
    Some(Block::Table {
        header: header.iter().map(|cell| parse_inline(cell)).collect(),
        aligns,
        rows,
    })
}

fn parse_list(lines: &[String], index: &mut usize) -> Block {
    let first = list_marker(&lines[*index]).expect("caller checked the marker");
    let (ordered, start) = (first.ordered, first.number);
    let mut items = Vec::new();
    while let Some(marker) = lines.get(*index).and_then(|line| list_marker(line)) {
        let mut item_lines =
            vec![lines[*index][marker.content.min(lines[*index].len())..].to_owned()];
        *index += 1;
        while *index < lines.len() {
            let line = &lines[*index];
            if is_blank(line) {
                let mut probe = *index;
                while probe < lines.len() && is_blank(&lines[probe]) {
                    probe += 1;
                }
                if probe < lines.len() && indent_of(&lines[probe]) >= marker.content {
                    item_lines.extend(std::iter::repeat_n(String::new(), probe - *index));
                    *index = probe;
                    continue;
                }
                break;
            }
            if indent_of(line) >= marker.content {
                item_lines.push(strip_indent(line, marker.content));
            } else if item_lines.last().is_some_and(|last| !is_blank(last))
                && list_marker(line).is_none()
                && heading(line).is_none()
                && fence(line).is_none()
                && !is_rule(line)
                && !is_quote(line)
            {
                item_lines.push(line.trim().to_owned());
            } else {
                break;
            }
            *index += 1;
        }
        let mut task = None;
        let first_line = item_lines[0].clone();
        for (prefix, done) in [("[ ] ", false), ("[x] ", true), ("[X] ", true)] {
            if let Some(rest) = first_line.strip_prefix(prefix) {
                task = Some(done);
                item_lines[0] = rest.to_owned();
                break;
            }
        }
        items.push(ListItem {
            task,
            blocks: parse_blocks(&item_lines),
        });
        let mut probe = *index;
        while probe < lines.len() && is_blank(&lines[probe]) {
            probe += 1;
        }
        match lines.get(probe).and_then(|line| list_marker(line)) {
            Some(next) if next.ordered == ordered && next.indent < marker.content => *index = probe,
            _ => break,
        }
    }
    Block::List {
        ordered,
        start,
        items,
    }
}

#[derive(Clone, Default)]
struct Style {
    bold: bool,
    italic: bool,
    strike: bool,
    link: Option<String>,
}

pub fn parse_inline(text: &str) -> Vec<Inline> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    inline_into(&chars, &Style::default(), &mut out);
    out
}

fn push_inline(out: &mut Vec<Inline>, text: String, style: &Style, code: bool) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if !code
            && !last.code
            && last.text != "\n"
            && text != "\n"
            && last.bold == style.bold
            && last.italic == style.italic
            && last.strike == style.strike
            && last.link == style.link
        {
            last.text.push_str(&text);
            return;
        }
    }
    out.push(Inline {
        text,
        bold: style.bold,
        italic: style.italic,
        strike: style.strike,
        code,
        link: style.link.clone(),
    });
}

/// `[label](url)` starting at `start`: the indices of `]` and `)`, and the URL.
fn parse_link(chars: &[char], start: usize) -> Option<(usize, usize, String)> {
    let mut depth = 0usize;
    let mut close = None;
    for (offset, c) in chars[start..].iter().enumerate() {
        match c {
            '\\' => {}
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(start + offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let mut depth = 0usize;
    for (offset, c) in chars[close + 2..].iter().enumerate() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => {
                let end = close + 2 + offset;
                let target: String = chars[close + 2..end].iter().collect();
                let url = target.split_whitespace().next().unwrap_or("").to_owned();
                return Some((close, end, url));
            }
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn find_closer(chars: &[char], from: usize, delimiter: char, run: usize) -> Option<usize> {
    let mut probe = from;
    while probe < chars.len() {
        match chars[probe] {
            '\\' => probe += 2,
            '`' => {
                let ticks = chars[probe..].iter().take_while(|c| **c == '`').count();
                probe += ticks;
            }
            c if c == delimiter => {
                let length = chars[probe..]
                    .iter()
                    .take_while(|c| **c == delimiter)
                    .count();
                let boundary = delimiter != '_'
                    || chars
                        .get(probe + length)
                        .is_none_or(|next| !next.is_alphanumeric());
                if length == run && probe > from && !chars[probe - 1].is_whitespace() && boundary {
                    return Some(probe);
                }
                probe += length;
            }
            _ => probe += 1,
        }
    }
    None
}

fn inline_into(chars: &[char], style: &Style, out: &mut Vec<Inline>) {
    let n = chars.len();
    let mut buffer = String::new();
    let mut index = 0;
    macro_rules! flush {
        () => {
            push_inline(out, std::mem::take(&mut buffer), style, false)
        };
    }
    while index < n {
        let c = chars[index];
        match c {
            '\\' if chars.get(index + 1).is_some_and(char::is_ascii_punctuation) => {
                buffer.push(chars[index + 1]);
                index += 2;
            }
            '\n' => {
                flush!();
                out.push(Inline {
                    text: "\n".to_owned(),
                    ..Inline::default()
                });
                index += 1;
            }
            '`' => {
                let run = chars[index..].iter().take_while(|c| **c == '`').count();
                let mut probe = index + run;
                let mut end = None;
                while probe < n {
                    let length = chars[probe..].iter().take_while(|c| **c == '`').count();
                    if length == run {
                        end = Some(probe);
                        break;
                    }
                    probe += length.max(1);
                }
                match end {
                    Some(end) => {
                        flush!();
                        let code: String = chars[index + run..end].iter().collect();
                        let code = if code.starts_with(' ') && code.ends_with(' ') && code.len() > 1
                        {
                            code[1..code.len() - 1].to_owned()
                        } else {
                            code
                        };
                        push_inline(out, code, style, true);
                        index = end + run;
                    }
                    None => {
                        buffer.extend(std::iter::repeat_n('`', run));
                        index += run;
                    }
                }
            }
            '!' if chars.get(index + 1) == Some(&'[') => match parse_link(chars, index + 1) {
                Some((close, end, url)) => {
                    flush!();
                    let alt: String = chars[index + 2..close].iter().collect();
                    let mut image = style.clone();
                    image.link = Some(url);
                    image.italic = true;
                    push_inline(out, format!("[image: {alt}]"), &image, false);
                    index = end + 1;
                }
                None => {
                    buffer.push('!');
                    index += 1;
                }
            },
            '[' => match parse_link(chars, index) {
                Some((close, end, url)) => {
                    flush!();
                    let mut linked = style.clone();
                    linked.link = Some(url);
                    inline_into(&chars[index + 1..close], &linked, out);
                    index = end + 1;
                }
                None => {
                    buffer.push('[');
                    index += 1;
                }
            },
            '<' => {
                let end = chars[index..]
                    .iter()
                    .position(|c| *c == '>')
                    .map(|p| p + index);
                let target = end.map(|end| chars[index + 1..end].iter().collect::<String>());
                match (end, target) {
                    (Some(end), Some(url)) if url.contains("://") && !url.contains(' ') => {
                        flush!();
                        let mut linked = style.clone();
                        linked.link = Some(url.clone());
                        push_inline(out, url, &linked, false);
                        index = end + 1;
                    }
                    _ => {
                        buffer.push('<');
                        index += 1;
                    }
                }
            }
            '~' if chars.get(index + 1) == Some(&'~') => {
                match find_closer(chars, index + 2, '~', 2) {
                    Some(end) if chars.get(index + 2).is_some_and(|c| !c.is_whitespace()) => {
                        flush!();
                        let mut struck = style.clone();
                        struck.strike = true;
                        inline_into(&chars[index + 2..end], &struck, out);
                        index = end + 2;
                    }
                    _ => {
                        buffer.push_str("~~");
                        index += 2;
                    }
                }
            }
            '*' | '_' => {
                let run = chars[index..].iter().take_while(|x| **x == c).count();
                let usable = run.min(3);
                let opens = chars
                    .get(index + run)
                    .is_some_and(|next| !next.is_whitespace())
                    && !(c == '_' && index > 0 && chars[index - 1].is_alphanumeric())
                    && run <= 3;
                let end = if opens {
                    find_closer(chars, index + run, c, run)
                } else {
                    None
                };
                match end {
                    Some(end) => {
                        flush!();
                        let mut emphasized = style.clone();
                        emphasized.bold |= usable >= 2;
                        emphasized.italic |= usable == 1 || usable == 3;
                        inline_into(&chars[index + run..end], &emphasized, out);
                        index = end + run;
                    }
                    None => {
                        buffer.extend(std::iter::repeat_n(c, run));
                        index += run;
                    }
                }
            }
            'h' if (chars[index..].starts_with(&['h', 't', 't', 'p', ':', '/', '/'])
                || chars[index..].starts_with(&['h', 't', 't', 'p', 's', ':', '/', '/']))
                && (index == 0 || !chars[index - 1].is_alphanumeric()) =>
            {
                let mut end = index;
                while end < n && !chars[end].is_whitespace() {
                    end += 1;
                }
                while end > index && ".,;:!?)]'\"".contains(chars[end - 1]) {
                    end -= 1;
                }
                flush!();
                let url: String = chars[index..end].iter().collect();
                let mut linked = style.clone();
                linked.link = Some(url.clone());
                push_inline(out, url, &linked, false);
                index = end;
            }
            _ => {
                buffer.push(c);
                index += 1;
            }
        }
    }
    flush!();
}

pub fn plain_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|inline| {
            if inline.text == "\n" {
                " "
            } else {
                inline.text.as_str()
            }
        })
        .collect()
}

// ----------------------------------------------------------------- links --

/// Where following a link goes. Only web, mail, anchor and local-file targets are honored.
#[derive(Debug, PartialEq, Eq)]
pub enum LinkTarget {
    Web(String),
    Anchor(String),
    File(PathBuf),
    /// A scheme Selenite refuses to launch (`javascript:`, `file:`, `data:`, ...).
    Blocked(String),
}

/// GitHub-style heading anchor: lower case, spaces to hyphens, punctuation dropped.
pub fn slugify(text: &str) -> String {
    text.trim()
        .chars()
        .filter_map(|c| match c {
            ' ' | '-' => Some('-'),
            '_' => Some('_'),
            c if c.is_alphanumeric() => Some(c.to_lowercase().next().unwrap_or(c)),
            _ => None,
        })
        .collect()
}

fn unique_anchor(existing: &[(String, f32)], slug: String) -> String {
    let taken = |name: &str| existing.iter().any(|(known, _)| known == name);
    if !taken(&slug) {
        return slug;
    }
    (1..)
        .map(|number| format!("{slug}-{number}"))
        .find(|candidate| !taken(candidate))
        .expect("an unbounded range always yields a free name")
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let decoded = (bytes[index] == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match decoded {
            Some(byte) => {
                out.push(byte);
                index += 3;
            }
            None => {
                out.push(bytes[index]);
                index += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

/// Decides what a clicked link may do; relative paths resolve against `base`.
pub fn classify_link(url: &str, base: &Path) -> LinkTarget {
    let url = url.trim();
    if let Some(anchor) = url.strip_prefix('#') {
        return LinkTarget::Anchor(slugify(&percent_decode(anchor)));
    }
    let scheme_end = url.find(':').filter(|end| {
        *end >= 2
            && url[..*end].starts_with(|c: char| c.is_ascii_alphabetic())
            && url[..*end]
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    });
    if let Some(end) = scheme_end {
        let scheme = url[..end].to_ascii_lowercase();
        return match scheme.as_str() {
            "http" | "https" | "mailto" => LinkTarget::Web(url.to_owned()),
            _ => LinkTarget::Blocked(scheme),
        };
    }
    let path = url.split(['#', '?']).next().unwrap_or("");
    let path = percent_decode(path);
    if path.is_empty() || path.contains('\0') {
        return LinkTarget::Blocked("empty".to_owned());
    }
    LinkTarget::File(base.join(path))
}

// ---------------------------------------------------------------- layout --

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunStyle {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub link: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub x: f32,
    pub text: String,
    pub style: RunStyle,
    /// Syntax colour inside fenced code.
    pub token: Token,
    /// Where a click goes, for link text.
    pub link: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectKind {
    CodeBackground,
    QuoteBar,
    Rule,
    HeadingRule,
    TableBorder,
    TableHeader,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Text {
        y: f32,
        size: f32,
        runs: Vec<Run>,
    },
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        kind: RectKind,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub items: Vec<Item>,
    pub height: f32,
    /// Heading anchors (`#slug`) with their vertical position.
    pub anchors: Vec<(String, f32)>,
}

const HEADING_SCALE: [f32; 6] = [1.9, 1.55, 1.3, 1.15, 1.0, 0.9];
const LINE_HEIGHT: f32 = 1.45;
const CODE_SCALE: f32 = 0.92;

/// Text widths: `text` for proportional text, `mono` for code.
pub struct Metrics<'a> {
    pub text: &'a dyn Fn(&str, f32) -> f32,
    pub mono: &'a dyn Fn(&str, f32) -> f32,
}

pub fn layout(blocks: &[Block], width: f32, base: f32, metrics: &Metrics) -> Layout {
    let mut context = Context {
        items: Vec::new(),
        y: 0.0,
        base,
        metrics,
        depth: 0,
        anchors: Vec::new(),
    };
    context.blocks(blocks, 0.0, width.max(base * 4.0), 0.6);
    Layout {
        height: context.y,
        items: context.items,
        anchors: context.anchors,
    }
}

struct Context<'a> {
    items: Vec<Item>,
    y: f32,
    base: f32,
    metrics: &'a Metrics<'a>,
    depth: usize,
    anchors: Vec<(String, f32)>,
}

struct Word {
    text: String,
    style: RunStyle,
    space_after: bool,
    link: Option<String>,
}

enum Piece {
    Word(Word),
    Break,
}

fn fit_text(text: &str, max: f32, measure: &dyn Fn(&str) -> f32) -> String {
    if measure(text) <= max {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut length = chars.len();
    while length > 0 {
        let candidate: String = chars[..length].iter().chain(['…'].iter()).collect();
        if measure(&candidate) <= max {
            return candidate;
        }
        length -= 1;
    }
    String::new()
}

impl Context<'_> {
    fn measure(&self, text: &str, size: f32, style: RunStyle) -> f32 {
        if style.code {
            (self.metrics.mono)(text, size * CODE_SCALE)
        } else {
            (self.metrics.text)(text, size)
        }
    }

    fn blocks(&mut self, blocks: &[Block], x: f32, width: f32, gap: f32) {
        for (index, block) in blocks.iter().enumerate() {
            if index > 0 {
                self.y += self.base * gap;
                if matches!(block, Block::Heading(..)) {
                    self.y += self.base * 0.4;
                }
            }
            self.block(block, x, width);
        }
    }

    fn block(&mut self, block: &Block, x: f32, width: f32) {
        let base = self.base;
        match block {
            Block::Heading(level, inlines) => {
                let size = base * HEADING_SCALE[usize::from(*level - 1)];
                let slug = unique_anchor(&self.anchors, slugify(&plain_text(inlines)));
                self.anchors.push((slug, self.y));
                self.paragraph(inlines, x, width, size, true);
                if *level <= 2 {
                    self.items.push(Item::Rect {
                        x,
                        y: self.y + base * 0.1,
                        width,
                        height: 1.0,
                        kind: RectKind::HeadingRule,
                    });
                    self.y += base * 0.3;
                }
            }
            Block::Paragraph(inlines) => self.paragraph(inlines, x, width, base, false),
            Block::Code { language, text } => self.code(language, text, x, width),
            Block::Quote(children) => {
                let indent = base * 1.0;
                let top = self.y;
                self.blocks(children, x + indent, (width - indent).max(base * 2.0), 0.6);
                self.items.push(Item::Rect {
                    x: x + 2.0,
                    y: top,
                    width: 3.0,
                    height: (self.y - top).max(base),
                    kind: RectKind::QuoteBar,
                });
            }
            Block::List {
                ordered,
                start,
                items,
            } => self.list(*ordered, *start, items, x, width),
            Block::Rule => {
                self.items.push(Item::Rect {
                    x,
                    y: self.y + base * 0.4,
                    width,
                    height: 1.0,
                    kind: RectKind::Rule,
                });
                self.y += base * 0.8;
            }
            Block::Table {
                header,
                aligns,
                rows,
            } => self.table(header, aligns, rows, x, width),
        }
    }

    fn paragraph(&mut self, inlines: &[Inline], x: f32, width: f32, size: f32, heading: bool) {
        let mut pieces: Vec<Piece> = Vec::new();
        for inline in inlines {
            if inline.text == "\n" {
                pieces.push(Piece::Break);
                continue;
            }
            let style = RunStyle {
                bold: inline.bold || heading,
                italic: inline.italic,
                strike: inline.strike,
                code: inline.code,
                link: inline.link.is_some(),
            };
            let mut current = String::new();
            let finish = |current: &mut String, pieces: &mut Vec<Piece>| {
                if !current.is_empty() {
                    pieces.push(Piece::Word(Word {
                        text: std::mem::take(current),
                        style,
                        space_after: false,
                        link: inline.link.clone(),
                    }));
                }
            };
            for c in inline.text.chars() {
                if c.is_whitespace() {
                    finish(&mut current, &mut pieces);
                    if let Some(Piece::Word(word)) = pieces.last_mut() {
                        word.space_after = true;
                    }
                } else {
                    current.push(c);
                }
            }
            finish(&mut current, &mut pieces);
        }

        // Words glued together (no whitespace between) may not be split across lines.
        let mut chunks: Vec<Vec<Word>> = Vec::new();
        let mut open: Vec<Word> = Vec::new();
        for piece in pieces {
            match piece {
                Piece::Break => {
                    if !open.is_empty() {
                        chunks.push(std::mem::take(&mut open));
                    }
                    chunks.push(Vec::new());
                }
                Piece::Word(word) => {
                    let ends = word.space_after;
                    open.push(word);
                    if ends {
                        chunks.push(std::mem::take(&mut open));
                    }
                }
            }
        }
        if !open.is_empty() {
            chunks.push(open);
        }

        let space = (self.metrics.text)(" ", size);
        let code_space = (self.metrics.mono)(" ", size * CODE_SCALE);
        let mut runs: Vec<(Run, bool)> = Vec::new();
        let mut cursor = 0.0f32;
        let flush = |this: &mut Self, runs: &mut Vec<(Run, bool)>| {
            if runs.is_empty() {
                return;
            }
            let mut merged: Vec<Run> = Vec::new();
            let mut previous_space = false;
            for (run, space_after) in runs.drain(..) {
                match merged.last_mut() {
                    Some(last)
                        if previous_space && last.style == run.style && last.link == run.link =>
                    {
                        last.text.push(' ');
                        last.text.push_str(&run.text);
                    }
                    _ => merged.push(run),
                }
                previous_space = space_after;
            }
            this.items.push(Item::Text {
                y: this.y,
                size,
                runs: merged,
            });
            this.y += size * LINE_HEIGHT;
        };
        let mut start_of_paragraph = true;
        // Style of the previous word when whitespace followed it.
        let mut pending: Option<RunStyle> = None;
        let gap = |pending: Option<RunStyle>, next: RunStyle| match pending {
            None => 0.0,
            Some(previous) if previous == next && next.code => code_space,
            Some(_) => space,
        };
        for chunk in chunks {
            if chunk.is_empty() {
                // Hard line break.
                if runs.is_empty() && !start_of_paragraph {
                    self.y += size * LINE_HEIGHT;
                }
                flush(self, &mut runs);
                cursor = 0.0;
                pending = None;
                continue;
            }
            start_of_paragraph = false;
            let widths: Vec<f32> = chunk
                .iter()
                .map(|word| self.measure(&word.text, size, word.style))
                .collect();
            let total: f32 = widths.iter().sum();
            let lead = gap(pending, chunk[0].style);
            if cursor > 0.0 && cursor + lead + total > width {
                flush(self, &mut runs);
                cursor = 0.0;
                pending = None;
            }
            if cursor > 0.0 {
                cursor += gap(pending, chunk[0].style);
            }
            for (word, word_width) in chunk.into_iter().zip(widths) {
                let style = word.style;
                let space_after = word.space_after;
                runs.push((
                    Run {
                        x: x + cursor,
                        text: word.text,
                        style,
                        token: if style.code {
                            Token::Code
                        } else {
                            Token::Plain
                        },
                        link: word.link,
                    },
                    space_after,
                ));
                cursor += word_width;
                pending = space_after.then_some(style);
            }
        }
        flush(self, &mut runs);
    }

    fn code(&mut self, language: &str, text: &str, x: f32, width: f32) {
        let size = self.base * CODE_SCALE;
        let line_height = size * 1.35;
        let pad = self.base * 0.5;
        let syntax = match language {
            "ruby" | "rb" => Language::Ruby,
            _ => Language::Plain,
        };
        let lines: Vec<&str> = text.split('\n').collect();
        self.items.push(Item::Rect {
            x,
            y: self.y,
            width,
            height: lines.len() as f32 * line_height + pad * 2.0,
            kind: RectKind::CodeBackground,
        });
        let mut state = LineState::Normal;
        let mut y = self.y + pad;
        for line in lines {
            let (spans, next) = syntax::highlight_line(syntax, &state, line);
            state = next;
            let chars: Vec<char> = line.chars().collect();
            let mut runs = Vec::new();
            let mut position = 0;
            let mut segments: Vec<(usize, usize, Token)> = Vec::new();
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
            for (start, end, token) in segments {
                let prefix: String = chars[..start].iter().collect();
                runs.push(Run {
                    x: x + pad + (self.metrics.mono)(&prefix, size),
                    text: chars[start..end].iter().collect(),
                    style: RunStyle {
                        code: true,
                        ..RunStyle::default()
                    },
                    token,
                    link: None,
                });
            }
            if !runs.is_empty() {
                self.items.push(Item::Text { y, size, runs });
            }
            y += line_height;
        }
        self.y = y + pad;
    }

    fn list(&mut self, ordered: bool, start: u64, items: &[ListItem], x: f32, width: f32) {
        let base = self.base;
        let markers: Vec<String> = items
            .iter()
            .enumerate()
            .map(|(number, item)| match item.task {
                Some(true) => "[x]".to_owned(),
                Some(false) => "[ ]".to_owned(),
                None if ordered => format!("{}.", start + number as u64),
                None if self.depth % 2 == 0 => "•".to_owned(),
                None => "–".to_owned(),
            })
            .collect();
        let widest = markers
            .iter()
            .map(|marker| (self.metrics.text)(marker, base))
            .fold(0.0f32, f32::max);
        let indent = widest + base * 0.7;
        for (index, (item, marker)) in items.iter().zip(markers).enumerate() {
            if index > 0 {
                self.y += base * 0.2;
            }
            let top = self.y;
            self.items.push(Item::Text {
                y: top,
                size: base,
                runs: vec![Run {
                    x: x + base * 0.1,
                    text: marker,
                    style: RunStyle::default(),
                    token: Token::Marker,
                    link: None,
                }],
            });
            self.depth += 1;
            self.blocks(
                &item.blocks,
                x + indent,
                (width - indent).max(base * 2.0),
                0.2,
            );
            self.depth -= 1;
            if self.y == top {
                self.y += base * LINE_HEIGHT;
            }
        }
    }

    fn table(
        &mut self,
        header: &[Vec<Inline>],
        aligns: &[Align],
        rows: &[Vec<Vec<Inline>>],
        x: f32,
        width: f32,
    ) {
        let base = self.base;
        let pad = base * 0.5;
        let columns = header.len().max(1);
        let cell_text = |cell: &Vec<Inline>| plain_text(cell);
        let mut natural = vec![0.0f32; columns];
        for row in std::iter::once(header).chain(rows.iter().map(Vec::as_slice)) {
            for (column, cell) in row.iter().enumerate().take(columns) {
                natural[column] = natural[column].max((self.metrics.text)(&cell_text(cell), base));
            }
        }
        for value in &mut natural {
            *value += pad * 2.0;
        }
        let total: f32 = natural.iter().sum();
        if total > width {
            let scale = width / total;
            for value in &mut natural {
                *value *= scale;
            }
        }
        let table_width: f32 = natural.iter().sum();
        let row_height = base * LINE_HEIGHT + pad;
        let top = self.y;
        let height = row_height * (rows.len() + 1) as f32;
        self.items.push(Item::Rect {
            x,
            y: top,
            width: table_width,
            height: row_height,
            kind: RectKind::TableHeader,
        });
        for (index, row) in std::iter::once(header)
            .chain(rows.iter().map(Vec::as_slice))
            .enumerate()
        {
            let y = top + row_height * index as f32;
            let mut runs = Vec::new();
            let mut column_x = x;
            for (column, cell) in row.iter().enumerate().take(columns) {
                let available = (natural[column] - pad * 2.0).max(0.0);
                let text = fit_text(&cell_text(cell), available, &|candidate| {
                    (self.metrics.text)(candidate, base)
                });
                let text_width = (self.metrics.text)(&text, base);
                let offset = match aligns.get(column).copied().unwrap_or(Align::Left) {
                    Align::Left => 0.0,
                    Align::Center => (available - text_width) / 2.0,
                    Align::Right => available - text_width,
                };
                if !text.is_empty() {
                    runs.push(Run {
                        x: column_x + pad + offset.max(0.0),
                        text,
                        style: RunStyle {
                            bold: index == 0,
                            ..RunStyle::default()
                        },
                        token: Token::Plain,
                        link: None,
                    });
                }
                column_x += natural[column];
            }
            self.items.push(Item::Text {
                y: y + pad / 2.0,
                size: base,
                runs,
            });
            self.items.push(Item::Rect {
                x,
                y,
                width: table_width,
                height: 1.0,
                kind: RectKind::TableBorder,
            });
        }
        self.items.push(Item::Rect {
            x,
            y: top + height,
            width: table_width,
            height: 1.0,
            kind: RectKind::TableBorder,
        });
        let mut column_x = x;
        for value in std::iter::once(0.0).chain(natural.iter().copied()) {
            column_x += value;
            self.items.push(Item::Rect {
                x: column_x,
                y: top,
                width: 1.0,
                height,
                kind: RectKind::TableBorder,
            });
        }
        self.y = top + height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inline(text: &str) -> Vec<(String, &'static str)> {
        parse_inline(text)
            .into_iter()
            .map(|part| {
                let kind = match (
                    part.bold,
                    part.italic,
                    part.strike,
                    part.code,
                    part.link.is_some(),
                ) {
                    (false, false, false, false, false) => "plain",
                    (true, false, false, false, false) => "bold",
                    (false, true, false, false, false) => "italic",
                    (true, true, false, false, false) => "bold-italic",
                    (false, false, true, false, false) => "strike",
                    (false, false, false, true, false) => "code",
                    (_, _, _, _, true) => "link",
                    _ => "other",
                };
                (part.text, kind)
            })
            .collect()
    }

    #[test]
    fn inline_styles() {
        assert_eq!(
            inline("a **b** *c* ***d*** ~~e~~ `f g`"),
            vec![
                ("a ".into(), "plain"),
                ("b".into(), "bold"),
                (" ".into(), "plain"),
                ("c".into(), "italic"),
                (" ".into(), "plain"),
                ("d".into(), "bold-italic"),
                (" ".into(), "plain"),
                ("e".into(), "strike"),
                (" ".into(), "plain"),
                ("f g".into(), "code"),
            ]
        );
        assert_eq!(
            inline("snake_case_word and 2 * 3 * 4 and \\*literal\\*"),
            vec![(
                "snake_case_word and 2 * 3 * 4 and *literal*".into(),
                "plain"
            )]
        );
        assert_eq!(
            inline("*a **b** c*"),
            vec![
                ("a ".into(), "italic"),
                ("b".into(), "bold-italic"),
                (" c".into(), "italic")
            ]
        );
        assert_eq!(
            inline("unclosed **bold"),
            vec![("unclosed **bold".into(), "plain")]
        );
        assert_eq!(inline("`` a`b ``"), vec![("a`b".into(), "code")]);
    }

    #[test]
    fn inline_links_images_and_urls() {
        let parts = parse_inline(
            "see [the **docs**](https://x.test/a_(b) \"t\"), ![logo](l.png) and https://y.test/z.",
        );
        let links: Vec<(&str, &str, bool)> = parts
            .iter()
            .filter_map(|part| {
                part.link
                    .as_deref()
                    .map(|url| (part.text.as_str(), url, part.bold))
            })
            .collect();
        assert_eq!(
            links,
            vec![
                ("the ", "https://x.test/a_(b)", false),
                ("docs", "https://x.test/a_(b)", true),
                ("[image: logo]", "l.png", false),
                ("https://y.test/z", "https://y.test/z", false),
            ]
        );
        assert_eq!(plain_text(&parse_inline("a\\\nb")), "a\\ b");
        assert_eq!(
            inline("<https://z.test>"),
            vec![("https://z.test".into(), "link")]
        );
    }

    #[test]
    fn headings_paragraphs_and_breaks() {
        let blocks = parse("# One #\n\nTwo\nlines  \nhard\n\nSetext\n======\n\n#nospace");
        assert_eq!(blocks.len(), 4);
        assert!(matches!(&blocks[0], Block::Heading(1, text) if text[0].text == "One"));
        let Block::Paragraph(text) = &blocks[1] else {
            panic!("paragraph expected")
        };
        assert_eq!(
            text.iter()
                .map(|part| part.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Two lines", "\n", "hard"]
        );
        assert!(matches!(&blocks[2], Block::Heading(1, text) if text[0].text == "Setext"));
        assert!(matches!(&blocks[3], Block::Paragraph(_)));
        assert!(matches!(&parse("Sub\n---")[0], Block::Heading(2, _)));
        assert_eq!(parse("***"), vec![Block::Rule]);
        assert_eq!(parse("one\n\n---\n\ntwo").len(), 3);
    }

    #[test]
    fn code_blocks_and_quotes() {
        let blocks = parse(
            "```ruby\nputs 1\n\n# not heading\n```\n\n> quoted\n> # inner\n\n~~~\nunterminated",
        );
        assert_eq!(
            blocks[0],
            Block::Code {
                language: "ruby".into(),
                text: "puts 1\n\n# not heading".into()
            }
        );
        let Block::Quote(inner) = &blocks[1] else {
            panic!("quote expected")
        };
        assert!(matches!(&inner[0], Block::Paragraph(_)));
        assert!(matches!(&inner[1], Block::Heading(1, _)));
        assert_eq!(
            blocks[2],
            Block::Code {
                language: String::new(),
                text: "unterminated".into()
            }
        );
    }

    #[test]
    fn nested_and_task_lists() {
        let blocks = parse("- a\n  - b\n  - c\n- d\n\n1. one\n2. two\n\n- [x] done\n- [ ] todo");
        let Block::List { ordered, items, .. } = &blocks[0] else {
            panic!("list expected")
        };
        assert!(!ordered);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].blocks.len(), 2);
        let Block::List { items: nested, .. } = &items[0].blocks[1] else {
            panic!("nested list expected")
        };
        assert_eq!(nested.len(), 2);
        let Block::List {
            ordered,
            start,
            items,
        } = &blocks[1]
        else {
            panic!("ordered list expected")
        };
        assert!(*ordered && *start == 1 && items.len() == 2);
        let Block::List { items, .. } = &blocks[2] else {
            panic!("task list expected")
        };
        assert_eq!(items[0].task, Some(true));
        assert_eq!(items[1].task, Some(false));
        assert_eq!(items[1].blocks.len(), 1);
        assert_eq!(parse("- a\n\n  continued\n- b").len(), 1);
        assert!(matches!(
            &parse("text\n2. not a list")[0],
            Block::Paragraph(_)
        ));
    }

    #[test]
    fn tables() {
        let blocks = parse("| Name | Qty |\n|:--|--:|\n| a \\| b | 1 |\n| c |\n\nafter");
        let Block::Table {
            header,
            aligns,
            rows,
        } = &blocks[0]
        else {
            panic!("table expected")
        };
        assert_eq!(header.len(), 2);
        assert_eq!(aligns, &[Align::Left, Align::Right]);
        assert_eq!(rows.len(), 2);
        assert_eq!(plain_text(&rows[0][0]), "a | b");
        assert_eq!(plain_text(&rows[1][1]), "");
        assert!(matches!(&blocks[1], Block::Paragraph(_)));
        assert!(matches!(
            &parse("a | b\nnot separator")[0],
            Block::Paragraph(_)
        ));
    }

    fn measure(text: &str, size: f32) -> f32 {
        text.chars().count() as f32 * size * 0.5
    }

    fn lay_out(text: &str, width: f32) -> Layout {
        layout(
            &parse(text),
            width,
            10.0,
            &Metrics {
                text: &measure,
                mono: &measure,
            },
        )
    }

    fn lines(layout: &Layout) -> Vec<String> {
        layout
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Text { runs, .. } => Some(
                    runs.iter()
                        .map(|run| run.text.as_str())
                        .collect::<Vec<_>>()
                        .join("|"),
                ),
                Item::Rect { .. } => None,
            })
            .collect()
    }

    #[test]
    fn layout_wraps_words_and_keeps_glued_text_together() {
        let wrapped = lay_out("aaa bbb **ccc**, ddd", 40.0);
        assert_eq!(lines(&wrapped), vec!["aaa bbb", "ccc|, ddd"]);
        let Item::Text { runs, .. } = &wrapped.items[1] else {
            panic!("text expected")
        };
        assert!(runs[0].style.bold && !runs[1].style.bold);
        assert_eq!(runs[1].x, runs[0].x + 15.0);
        assert!(wrapped.height >= 2.0 * 10.0 * LINE_HEIGHT - 0.01);
        assert_eq!(lines(&lay_out("one\\\ntwo", 200.0)), vec!["one", "two"]);
        assert_eq!(lines(&lay_out("one  \ntwo", 200.0)), vec!["one", "two"]);
        assert_eq!(lines(&lay_out("averyveryverylongword", 20.0)).len(), 1);
    }

    #[test]
    fn inline_code_keeps_monospace_gaps_and_does_not_collide_with_text() {
        let text = |text: &str, size: f32| text.chars().count() as f32 * size * 0.5;
        let mono = |text: &str, size: f32| text.chars().count() as f32 * size * 0.6;
        let layout = layout(
            &parse("a `b c` d"),
            400.0,
            10.0,
            &Metrics {
                text: &text,
                mono: &mono,
            },
        );
        let Item::Text { runs, .. } = &layout.items[0] else {
            panic!("text expected")
        };
        let code_width = 0.6 * 10.0 * CODE_SCALE;
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
            vec!["a", "b c", "d"]
        );
        assert_eq!(runs[1].token, Token::Code);
        assert_eq!(runs[0].token, Token::Plain);
        let code_end = runs[1].x + 3.0 * code_width;
        assert!(
            (runs[2].x - (code_end + 5.0)).abs() < 0.01,
            "{} vs {}",
            runs[2].x,
            code_end + 5.0
        );
    }

    #[test]
    fn layout_places_lists_quotes_code_and_tables() {
        let list = lay_out("- a\n  - b\n1. x", 200.0);
        assert_eq!(lines(&list), vec!["•", "a", "–", "b", "1.", "x"]);
        let Item::Text { runs: nested, .. } = &list.items[2] else {
            panic!("marker expected")
        };
        let Item::Text { runs: top, .. } = &list.items[0] else {
            panic!("marker expected")
        };
        assert!(nested[0].x > top[0].x);

        let quote = lay_out("> hi", 200.0);
        assert!(quote.items.iter().any(|item| matches!(
            item,
            Item::Rect {
                kind: RectKind::QuoteBar,
                ..
            }
        )));

        let code = lay_out("```ruby\nputs 'x' # c\n```", 200.0);
        assert!(matches!(
            &code.items[0],
            Item::Rect {
                kind: RectKind::CodeBackground,
                ..
            }
        ));
        let Item::Text { runs, .. } = &code.items[1] else {
            panic!("code line expected")
        };
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.token))
                .collect::<Vec<_>>(),
            vec![
                ("puts", Token::Builtin),
                (" ", Token::Plain),
                ("'x'", Token::String),
                (" ", Token::Plain),
                ("# c", Token::Comment)
            ]
        );

        let table = lay_out("| a | b |\n|---|--:|\n| long cell text | 1 |", 60.0);
        let rects = table
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item,
                    Item::Rect {
                        kind: RectKind::TableBorder,
                        ..
                    }
                )
            })
            .count();
        assert!(rects >= 3 + 3);
        for item in &table.items {
            if let Item::Rect {
                x,
                width,
                kind: RectKind::TableBorder,
                ..
            } = item
            {
                assert!(x + width <= 61.5, "table fits the pane");
            }
        }
        assert!(lines(&table).iter().any(|line| line.contains('…')));

        let heading = lay_out("# T\n\ntext", 200.0);
        assert!(matches!(
            &heading.items[1],
            Item::Rect {
                kind: RectKind::HeadingRule,
                ..
            }
        ));
    }

    #[test]
    fn link_runs_carry_their_target_and_never_merge_across_links() {
        let page = lay_out("see [docs](https://x.test) and [a](u1) [b](u2) end", 400.0);
        let Item::Text { runs, .. } = &page.items[0] else {
            panic!("text expected")
        };
        let links: Vec<(&str, Option<&str>)> = runs
            .iter()
            .map(|run| (run.text.as_str(), run.link.as_deref()))
            .collect();
        assert_eq!(
            links,
            vec![
                ("see", None),
                ("docs", Some("https://x.test")),
                ("and", None),
                ("a", Some("u1")),
                ("b", Some("u2")),
                ("end", None),
            ]
        );
    }

    #[test]
    fn headings_get_unique_github_style_anchors() {
        assert_eq!(slugify("Hello, World! 2"), "hello-world-2");
        assert_eq!(slugify("snake_case Name"), "snake_case-name");
        assert_eq!(slugify("Über uns"), "über-uns");
        let page = lay_out("# Intro\n\ntext\n\n## Intro\n\n### Intro\n\n# Other", 400.0);
        let names: Vec<&str> = page.anchors.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, vec!["intro", "intro-1", "intro-2", "other"]);
        assert!(page.anchors.windows(2).all(|pair| pair[0].1 < pair[1].1));
        assert_eq!(page.anchors[0].1, 0.0);
    }

    #[test]
    fn links_are_classified_and_dangerous_schemes_blocked() {
        let base = Path::new("/notes");
        let web = |url: &str| LinkTarget::Web(url.to_owned());
        assert_eq!(
            classify_link("https://a.test/x?y=1#z", base),
            web("https://a.test/x?y=1#z")
        );
        assert_eq!(classify_link("HTTP://a.test", base), web("HTTP://a.test"));
        assert_eq!(
            classify_link("mailto:me@a.test", base),
            web("mailto:me@a.test")
        );
        assert_eq!(
            classify_link("#My%20Heading!", base),
            LinkTarget::Anchor("my-heading".into())
        );
        assert_eq!(
            classify_link("other%20file.md#part", base),
            LinkTarget::File(PathBuf::from("/notes/other file.md"))
        );
        assert_eq!(
            classify_link("../up/x.rb?raw", base),
            LinkTarget::File(PathBuf::from("/notes/../up/x.rb"))
        );
        assert_eq!(
            classify_link("C:\\docs\\a.md", base),
            LinkTarget::File(PathBuf::from("/notes/C:\\docs\\a.md"))
        );
        for (url, scheme) in [
            ("javascript:alert(1)", "javascript"),
            (" JavaScript:alert(1)", "javascript"),
            ("file:///etc/passwd", "file"),
            ("data:text/html;base64,AAAA", "data"),
            ("vbscript:x", "vbscript"),
            ("ssh://host", "ssh"),
        ] {
            assert_eq!(
                classify_link(url, base),
                LinkTarget::Blocked(scheme.into()),
                "{url}"
            );
        }
        assert_eq!(classify_link("", base), LinkTarget::Blocked("empty".into()));
        assert_eq!(
            classify_link("?only=query", base),
            LinkTarget::Blocked("empty".into())
        );
        assert_eq!(
            classify_link("a%00b", base),
            LinkTarget::Blocked("empty".into())
        );
        assert_eq!(percent_decode("%zz%4"), "%zz%4");
        assert_eq!(percent_decode("%C3%A9"), "é");
    }

    #[test]
    fn layout_handles_empty_and_degenerate_input() {
        assert_eq!(lay_out("", 100.0).height, 0.0);
        assert!(lay_out("-", 100.0).height > 0.0);
        assert!(lay_out("|a|\n|-|", 5.0).height > 0.0);
        assert!(lay_out("> ", 1.0).height >= 0.0);
    }
}

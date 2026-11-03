//! Line-based syntax highlighting for Ruby and Markdown source.
//!
//! A line is highlighted from the state the previous line ended in, so only
//! multi-line constructs (block comments, heredocs, fenced code) carry state.

use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    Plain,
    Keyword,
    Builtin,
    String,
    Comment,
    Number,
    Symbol,
    Constant,
    Variable,
    Function,
    Regex,
    Heading,
    Marker,
    Quote,
    Link,
    Code,
    Emphasis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Plain,
    Ruby,
    Markdown,
}

impl Language {
    pub fn from_path(path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        match extension.as_deref() {
            Some("rb" | "rbw" | "rake" | "gemspec" | "ru") => Self::Ruby,
            Some("md" | "markdown" | "mdown") => Self::Markdown,
            _ if matches!(name.as_str(), "rakefile" | "gemfile" | "guardfile") => Self::Ruby,
            _ => Self::Plain,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Plain => "Plain text",
            Self::Ruby => "Ruby",
            Self::Markdown => "Markdown",
        }
    }
}

/// A highlighted range in characters (not bytes); unlisted ranges are plain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub token: Token,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LineState {
    #[default]
    Normal,
    BlockComment,
    Heredoc {
        terminator: String,
        indented: bool,
    },
    Fence {
        marker: char,
        length: usize,
        /// Ruby's own state inside a ```ruby fence.
        ruby: Option<Box<LineState>>,
    },
}

pub fn highlight_line(language: Language, state: &LineState, line: &str) -> (Vec<Span>, LineState) {
    let chars: Vec<char> = line.chars().collect();
    match language {
        Language::Plain => (Vec::new(), LineState::Normal),
        Language::Ruby => ruby_line(state, &chars),
        Language::Markdown => markdown_line(state, &chars),
    }
}

fn whole(chars: &[char], token: Token) -> Vec<Span> {
    if chars.is_empty() {
        Vec::new()
    } else {
        vec![Span {
            start: 0,
            end: chars.len(),
            token,
        }]
    }
}

fn starts_with(chars: &[char], prefix: &str) -> bool {
    prefix
        .chars()
        .enumerate()
        .all(|(index, expected)| chars.get(index) == Some(&expected))
}

const RUBY_KEYWORDS: &[&str] = &[
    "alias",
    "and",
    "begin",
    "BEGIN",
    "break",
    "case",
    "class",
    "def",
    "defined?",
    "do",
    "else",
    "elsif",
    "end",
    "END",
    "ensure",
    "false",
    "for",
    "if",
    "in",
    "module",
    "next",
    "nil",
    "not",
    "or",
    "redo",
    "rescue",
    "retry",
    "return",
    "self",
    "super",
    "then",
    "true",
    "undef",
    "unless",
    "until",
    "when",
    "while",
    "yield",
    "__FILE__",
    "__LINE__",
    "__method__",
];

const RUBY_BUILTINS: &[&str] = &[
    "puts",
    "print",
    "p",
    "pp",
    "require",
    "require_relative",
    "load",
    "attr_reader",
    "attr_writer",
    "attr_accessor",
    "include",
    "extend",
    "prepend",
    "raise",
    "lambda",
    "proc",
    "loop",
    "private",
    "public",
    "protected",
    "module_function",
    "catch",
    "throw",
    "format",
    "printf",
    "gets",
    "sleep",
    "freeze",
    "block_given?",
];

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Index just past the closing quote of the string starting at `start`.
fn skip_quoted(chars: &[char], start: usize, interpolate: bool) -> usize {
    let quote = chars[start];
    let mut index = start + 1;
    let mut depth = 0usize;
    while index < chars.len() {
        match chars[index] {
            '\\' => index += 2,
            '#' if interpolate && chars.get(index + 1) == Some(&'{') => {
                depth += 1;
                index += 2;
            }
            '}' if depth > 0 => {
                depth -= 1;
                index += 1;
            }
            c if c == quote && depth == 0 => return index + 1,
            _ => index += 1,
        }
    }
    chars.len()
}

fn scan_number(chars: &[char], start: usize) -> usize {
    let n = chars.len();
    let mut index = start;
    if chars[index] == '0'
        && matches!(
            chars.get(index + 1),
            Some('x' | 'X' | 'b' | 'B' | 'o' | 'O')
        )
    {
        index += 2;
        while index < n && (chars[index].is_ascii_alphanumeric() || chars[index] == '_') {
            index += 1;
        }
        return index;
    }
    while index < n && (chars[index].is_ascii_digit() || chars[index] == '_') {
        index += 1;
    }
    if chars.get(index) == Some(&'.') && chars.get(index + 1).is_some_and(char::is_ascii_digit) {
        index += 1;
        while index < n && (chars[index].is_ascii_digit() || chars[index] == '_') {
            index += 1;
        }
    }
    if matches!(chars.get(index), Some('e' | 'E')) {
        let mut probe = index + 1;
        if matches!(chars.get(probe), Some('+' | '-')) {
            probe += 1;
        }
        if chars.get(probe).is_some_and(char::is_ascii_digit) {
            index = probe;
            while index < n && chars[index].is_ascii_digit() {
                index += 1;
            }
        }
    }
    index
}

/// End of a `%w[...]`-style literal whose delimiter is at `open`.
fn skip_percent_literal(chars: &[char], open: usize) -> usize {
    let opener = chars[open];
    let closer = match opener {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        other => other,
    };
    let nests = closer != opener;
    let mut depth = 0usize;
    let mut index = open + 1;
    while index < chars.len() {
        match chars[index] {
            '\\' => index += 2,
            c if nests && c == opener => {
                depth += 1;
                index += 1;
            }
            c if c == closer => {
                if depth == 0 {
                    return index + 1;
                }
                depth -= 1;
                index += 1;
            }
            _ => index += 1,
        }
    }
    chars.len()
}

fn ruby_line(state: &LineState, chars: &[char]) -> (Vec<Span>, LineState) {
    match state {
        LineState::BlockComment => {
            let ended = starts_with(chars, "=end");
            let next = if ended {
                LineState::Normal
            } else {
                LineState::BlockComment
            };
            return (whole(chars, Token::Comment), next);
        }
        LineState::Heredoc {
            terminator,
            indented,
        } => {
            let text: String = chars.iter().collect();
            let finished = if *indented {
                text.trim() == terminator
            } else {
                text == *terminator
            };
            let next = if finished {
                LineState::Normal
            } else {
                state.clone()
            };
            return (whole(chars, Token::String), next);
        }
        _ => {}
    }
    if starts_with(chars, "=begin") {
        return (whole(chars, Token::Comment), LineState::BlockComment);
    }
    scan_ruby(chars)
}

fn scan_ruby(chars: &[char]) -> (Vec<Span>, LineState) {
    let n = chars.len();
    let mut spans: Vec<Span> = Vec::new();
    let mut state = LineState::Normal;
    let mut index = 0;
    // After a value (name, literal, closing bracket) a `/` divides instead of starting a regex.
    let mut after_value = false;
    let mut after_def = false;
    let mut after_dot = false;
    let mut push = |start: usize, end: usize, token: Token| {
        if end > start {
            spans.push(Span { start, end, token });
        }
    };
    while index < n {
        let c = chars[index];
        if c.is_whitespace() {
            index += 1;
            continue;
        }
        let start = index;
        match c {
            '#' => {
                push(start, n, Token::Comment);
                index = n;
            }
            '"' | '`' => {
                index = skip_quoted(chars, index, true).min(n);
                push(start, index, Token::String);
                after_value = true;
                after_def = false;
                after_dot = false;
            }
            '\'' => {
                index = skip_quoted(chars, index, false).min(n);
                push(start, index, Token::String);
                after_value = true;
                after_def = false;
                after_dot = false;
            }
            '@' => {
                while index < n && chars[index] == '@' {
                    index += 1;
                }
                while index < n && is_ident(chars[index]) {
                    index += 1;
                }
                push(start, index, Token::Variable);
                after_value = true;
                after_def = false;
                after_dot = false;
            }
            '$' => {
                index += 1;
                if index < n && is_ident(chars[index]) {
                    while index < n && is_ident(chars[index]) {
                        index += 1;
                    }
                } else if index < n && !chars[index].is_whitespace() {
                    index += 1;
                }
                push(start, index, Token::Variable);
                after_value = true;
                after_def = false;
                after_dot = false;
            }
            '0'..='9' => {
                index = scan_number(chars, index);
                push(start, index, Token::Number);
                after_value = true;
                after_def = false;
                after_dot = false;
            }
            ':' => {
                let next = chars.get(index + 1).copied();
                let attached = index > 0 && is_ident(chars[index - 1]);
                if next == Some(':') {
                    index += 2;
                    after_dot = true;
                    after_value = false;
                } else if next == Some('"') && !attached {
                    index = skip_quoted(chars, index + 1, true).min(n);
                    push(start, index, Token::Symbol);
                    after_value = true;
                } else if next.is_some_and(is_ident_start) && !attached {
                    index += 1;
                    while index < n && is_ident(chars[index]) {
                        index += 1;
                    }
                    if matches!(chars.get(index), Some('?' | '!' | '='))
                        && chars.get(index + 1) != Some(&'>')
                    {
                        index += 1;
                    }
                    push(start, index, Token::Symbol);
                    after_value = true;
                } else {
                    index += 1;
                    after_value = false;
                }
                if next != Some(':') {
                    after_dot = false;
                }
                after_def = false;
            }
            '.' => {
                index += 1;
                after_dot = !after_def;
                after_value = false;
            }
            '/' => {
                if after_value {
                    index += 1;
                    after_value = false;
                } else {
                    index += 1;
                    while index < n && chars[index] != '/' {
                        index += if chars[index] == '\\' { 2 } else { 1 };
                    }
                    index = (index + 1).min(n);
                    while index < n && chars[index].is_ascii_alphabetic() {
                        index += 1;
                    }
                    push(start, index, Token::Regex);
                    after_value = true;
                }
                after_def = false;
                after_dot = false;
            }
            '%' if !after_value => {
                let kind = chars.get(index + 1).copied();
                let (open, regex) = match kind {
                    Some(k) if "wWiIqQrsx".contains(k) => (index + 2, k == 'r'),
                    _ => (index + 1, false),
                };
                if chars.get(open).is_some_and(|d| "([{<|!/".contains(*d)) {
                    index = skip_percent_literal(chars, open).min(n);
                    push(
                        start,
                        index,
                        if regex { Token::Regex } else { Token::String },
                    );
                    after_value = true;
                } else {
                    index += 1;
                }
                after_def = false;
                after_dot = false;
            }
            '<' if chars.get(index + 1) == Some(&'<') => {
                let mut probe = index + 2;
                let marker = chars.get(probe).copied().filter(|m| *m == '~' || *m == '-');
                if marker.is_some() {
                    probe += 1;
                }
                let quote = chars
                    .get(probe)
                    .copied()
                    .filter(|q| *q == '"' || *q == '\'');
                if quote.is_some() {
                    probe += 1;
                }
                let name_start = probe;
                while probe < n && is_ident(chars[probe]) {
                    probe += 1;
                }
                let valid = probe > name_start
                    && (marker.is_some()
                        || (chars[name_start].is_uppercase()
                            && (index == 0 || chars[index - 1].is_whitespace())));
                if valid {
                    let terminator: String = chars[name_start..probe].iter().collect();
                    if let Some(q) = quote {
                        if chars.get(probe) == Some(&q) {
                            probe += 1;
                        }
                    }
                    push(start, probe, Token::String);
                    state = LineState::Heredoc {
                        terminator,
                        indented: marker.is_some(),
                    };
                    index = probe;
                    after_value = true;
                } else {
                    index += 2;
                    after_value = false;
                }
                after_def = false;
                after_dot = false;
            }
            c if is_ident_start(c) => {
                while index < n && is_ident(chars[index]) {
                    index += 1;
                }
                if matches!(chars.get(index), Some('?' | '!')) && chars.get(index + 1) != Some(&'=')
                {
                    index += 1;
                }
                let word: String = chars[start..index].iter().collect();
                let label = chars.get(index) == Some(&':')
                    && chars.get(index + 1) != Some(&':')
                    && chars.get(index + 1).is_none_or(|next| next.is_whitespace())
                    && !after_dot;
                if label {
                    index += 1;
                    push(start, index, Token::Symbol);
                    after_value = false;
                    after_def = false;
                    continue;
                }
                let self_receiver = after_def && word == "self" && chars.get(index) == Some(&'.');
                let token = if after_dot {
                    if c.is_uppercase() {
                        Token::Constant
                    } else {
                        Token::Plain
                    }
                } else if self_receiver {
                    Token::Keyword
                } else if after_def {
                    Token::Function
                } else if RUBY_KEYWORDS.contains(&word.as_str()) {
                    Token::Keyword
                } else if RUBY_BUILTINS.contains(&word.as_str()) {
                    Token::Builtin
                } else if c.is_uppercase() {
                    Token::Constant
                } else {
                    Token::Plain
                };
                if token != Token::Plain {
                    push(start, index, token);
                }
                after_def = self_receiver || (!after_dot && !after_def && word == "def");
                after_dot = false;
                after_value = !matches!(
                    word.as_str(),
                    "if" | "unless"
                        | "while"
                        | "until"
                        | "and"
                        | "or"
                        | "not"
                        | "when"
                        | "in"
                        | "return"
                        | "then"
                        | "do"
                        | "else"
                        | "elsif"
                        | "case"
                );
            }
            ')' | ']' | '}' => {
                index += 1;
                after_value = true;
                after_def = false;
                after_dot = false;
            }
            _ => {
                index += 1;
                after_value = false;
                after_def = false;
                after_dot = false;
            }
        }
    }
    (spans, state)
}

fn is_heading(chars: &[char]) -> bool {
    let level = chars.iter().take_while(|c| **c == '#').count();
    (1..=6).contains(&level) && chars.get(level).is_none_or(|c| *c == ' ')
}

fn fence_open(chars: &[char]) -> Option<(char, usize, String)> {
    let marker = *chars.first()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let length = chars.iter().take_while(|c| **c == marker).count();
    if length < 3 {
        return None;
    }
    let info: String = chars[length..].iter().collect();
    if marker == '`' && info.contains('`') {
        return None;
    }
    Some((marker, length, info.trim().to_owned()))
}

fn markdown_line(state: &LineState, chars: &[char]) -> (Vec<Span>, LineState) {
    let indent = chars.iter().take_while(|c| **c == ' ').count();
    let body = &chars[indent..];
    if let LineState::Fence {
        marker,
        length,
        ruby,
    } = state
    {
        let closes = indent <= 3
            && body.iter().take_while(|c| *c == marker).count() >= *length
            && body.iter().all(|c| c == marker || c.is_whitespace());
        if closes {
            return (whole(chars, Token::Marker), LineState::Normal);
        }
        return match ruby {
            Some(inner) => {
                let (spans, next) = ruby_line(inner, chars);
                (
                    spans,
                    LineState::Fence {
                        marker: *marker,
                        length: *length,
                        ruby: Some(Box::new(next)),
                    },
                )
            }
            None => (whole(chars, Token::Code), state.clone()),
        };
    }
    if indent <= 3 {
        if let Some((marker, length, info)) = fence_open(body) {
            let ruby = matches!(info.split_whitespace().next(), Some("ruby" | "rb"));
            return (
                whole(chars, Token::Marker),
                LineState::Fence {
                    marker,
                    length,
                    ruby: ruby.then(|| Box::new(LineState::Normal)),
                },
            );
        }
        if is_heading(body) {
            return (whole(chars, Token::Heading), LineState::Normal);
        }
        if body.first() == Some(&'>') {
            return (whole(chars, Token::Quote), LineState::Normal);
        }
        let marks: Vec<char> = body
            .iter()
            .copied()
            .filter(|c| !c.is_whitespace())
            .collect();
        if marks.len() >= 3
            && matches!(marks[0], '-' | '*' | '_')
            && marks.iter().all(|c| *c == marks[0])
        {
            return (whole(chars, Token::Marker), LineState::Normal);
        }
    }
    let mut spans = Vec::new();
    let mut offset = indent;
    let digits = body.iter().take_while(|c| c.is_ascii_digit()).count();
    let marker_len = if matches!(body.first(), Some('-' | '*' | '+')) && body.get(1) == Some(&' ') {
        1
    } else if digits > 0
        && matches!(body.get(digits), Some('.' | ')'))
        && body.get(digits + 1) == Some(&' ')
    {
        digits + 1
    } else {
        0
    };
    if marker_len > 0 {
        spans.push(Span {
            start: indent,
            end: indent + marker_len,
            token: Token::Marker,
        });
        offset += marker_len;
    }
    markdown_inline(chars, offset, &mut spans);
    (spans, LineState::Normal)
}

fn markdown_inline(chars: &[char], from: usize, spans: &mut Vec<Span>) {
    let n = chars.len();
    let mut index = from;
    while index < n {
        match chars[index] {
            '`' => {
                let run = chars[index..].iter().take_while(|c| **c == '`').count();
                let mut probe = index + run;
                let mut found = None;
                while probe < n {
                    let length = chars[probe..].iter().take_while(|c| **c == '`').count();
                    if length == run {
                        found = Some(probe + run);
                        break;
                    }
                    probe += length.max(1);
                }
                match found {
                    Some(end) => {
                        spans.push(Span {
                            start: index,
                            end,
                            token: Token::Code,
                        });
                        index = end;
                    }
                    None => index += run,
                }
            }
            '[' => {
                let close = chars[index..]
                    .iter()
                    .position(|c| *c == ']')
                    .map(|p| p + index);
                let end = close
                    .filter(|close| chars.get(close + 1) == Some(&'('))
                    .and_then(|close| {
                        chars[close..]
                            .iter()
                            .position(|c| *c == ')')
                            .map(|p| p + close + 1)
                    });
                match end {
                    Some(end) => {
                        spans.push(Span {
                            start: index,
                            end,
                            token: Token::Link,
                        });
                        index = end;
                    }
                    None => index += 1,
                }
            }
            delimiter @ ('*' | '_') => {
                let run = chars[index..]
                    .iter()
                    .take_while(|c| **c == delimiter)
                    .count();
                let opens = chars.get(index + run).is_some_and(|c| !c.is_whitespace())
                    && !(delimiter == '_' && index > 0 && chars[index - 1].is_alphanumeric());
                let mut probe = index + run;
                let mut found = None;
                while opens && probe < n {
                    if chars[probe] == delimiter {
                        let length = chars[probe..]
                            .iter()
                            .take_while(|c| **c == delimiter)
                            .count();
                        if length == run && !chars[probe - 1].is_whitespace() {
                            found = Some(probe + run);
                            break;
                        }
                        probe += length;
                    } else {
                        probe += 1;
                    }
                }
                match found {
                    Some(end) => {
                        spans.push(Span {
                            start: index,
                            end,
                            token: Token::Emphasis,
                        });
                        index = end;
                    }
                    None => index += run,
                }
            }
            _ => index += 1,
        }
    }
}

/// Whether a Ruby line opens a block that a later `end` closes.
pub fn ruby_opens_block(line: &str) -> bool {
    let code = line.split('#').next().unwrap_or("").trim();
    if code.is_empty() {
        return false;
    }
    let first = code.split(|c: char| !is_ident(c)).next().unwrap_or("");
    let last = code.rsplit(|c: char| !is_ident(c)).next().unwrap_or("");
    if last == "end" {
        return false;
    }
    let block_do = last == "do" || (code.ends_with('|') && code.contains(" do |"));
    block_do
        || matches!(
            first,
            "def"
                | "class"
                | "module"
                | "if"
                | "unless"
                | "while"
                | "until"
                | "for"
                | "case"
                | "begin"
        )
}

/// Indent a closing `end`/`else`/`ensure`/`rescue` on `row` should use,
/// from the nearest unmatched opener above it.
pub fn ruby_matching_indent(lines: &[String], row: usize) -> Option<usize> {
    let mut depth = 1usize;
    for index in (0..row).rev() {
        let line = &lines[index];
        let word = line
            .trim_start()
            .split(|c: char| !is_ident(c))
            .next()
            .unwrap_or("");
        let indent = line.len() - line.trim_start().len();
        if word == "end" {
            depth += 1;
        } else if ruby_opens_block(line) {
            depth -= 1;
            if depth == 0 {
                return Some(indent);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(language: Language, line: &str) -> Vec<(String, Token)> {
        let chars: Vec<char> = line.chars().collect();
        highlight_line(language, &LineState::Normal, line)
            .0
            .into_iter()
            .map(|span| (chars[span.start..span.end].iter().collect(), span.token))
            .collect()
    }

    #[test]
    fn languages_follow_file_names() {
        assert_eq!(Language::from_path(Path::new("a/game.RB")), Language::Ruby);
        assert_eq!(Language::from_path(Path::new("Rakefile")), Language::Ruby);
        assert_eq!(
            Language::from_path(Path::new("notes.md")),
            Language::Markdown
        );
        assert_eq!(Language::from_path(Path::new("notes.txt")), Language::Plain);
    }

    #[test]
    fn ruby_definitions_keywords_and_literals() {
        assert_eq!(
            tokens(Language::Ruby, "def greet(name = \"hi\") # say"),
            vec![
                ("def".into(), Token::Keyword),
                ("greet".into(), Token::Function),
                ("\"hi\"".into(), Token::String),
                ("# say".into(), Token::Comment),
            ]
        );
        assert_eq!(
            tokens(Language::Ruby, "def self.build; @count += 1; end"),
            vec![
                ("def".into(), Token::Keyword),
                ("self".into(), Token::Keyword),
                ("build".into(), Token::Function),
                ("@count".into(), Token::Variable),
                ("1".into(), Token::Number),
                ("end".into(), Token::Keyword),
            ]
        );
        assert_eq!(
            tokens(Language::Ruby, "x = Foo::Bar.new(:ok, key: 0x1F, r: 2.5e3)"),
            vec![
                ("Foo".into(), Token::Constant),
                ("Bar".into(), Token::Constant),
                (":ok".into(), Token::Symbol),
                ("key:".into(), Token::Symbol),
                ("0x1F".into(), Token::Number),
                ("r:".into(), Token::Symbol),
                ("2.5e3".into(), Token::Number),
            ]
        );
    }

    #[test]
    fn ruby_strings_regexes_and_division() {
        assert_eq!(
            tokens(Language::Ruby, "puts \"a #{b \"c\"} d\" + 'e\\'f'"),
            vec![
                ("puts".into(), Token::Builtin),
                ("\"a #{b \"c\"} d\"".into(), Token::String),
                ("'e\\'f'".into(), Token::String),
            ]
        );
        assert_eq!(
            tokens(Language::Ruby, "if line =~ /ab+c/i then a / b end"),
            vec![
                ("if".into(), Token::Keyword),
                ("/ab+c/i".into(), Token::Regex),
                ("then".into(), Token::Keyword),
                ("end".into(), Token::Keyword),
            ]
        );
        assert_eq!(
            tokens(Language::Ruby, "list = %w[a b] + %i(c d)"),
            vec![
                ("%w[a b]".into(), Token::String),
                ("%i(c d)".into(), Token::String),
            ]
        );
        assert_eq!(
            tokens(Language::Ruby, "obj.class.end # not keywords"),
            vec![("# not keywords".into(), Token::Comment)]
        );
    }

    #[test]
    fn ruby_block_comments_and_heredocs_carry_state() {
        let (_, state) = highlight_line(Language::Ruby, &LineState::Normal, "=begin");
        assert_eq!(state, LineState::BlockComment);
        let (spans, state) = highlight_line(Language::Ruby, &state, "def not_code");
        assert_eq!(spans[0].token, Token::Comment);
        let (_, state) = highlight_line(Language::Ruby, &state, "=end");
        assert_eq!(state, LineState::Normal);

        let (spans, state) =
            highlight_line(Language::Ruby, &LineState::Normal, "text = <<~EOS.strip");
        assert_eq!(spans.last().unwrap().token, Token::String);
        assert_eq!(
            state,
            LineState::Heredoc {
                terminator: "EOS".into(),
                indented: true
            }
        );
        let (spans, state) = highlight_line(Language::Ruby, &state, "  def inside");
        assert_eq!(spans[0].token, Token::String);
        let (_, state) = highlight_line(Language::Ruby, &state, "  EOS");
        assert_eq!(state, LineState::Normal);

        let (_, state) = highlight_line(Language::Ruby, &LineState::Normal, "a = <<EOS");
        let (_, state) = highlight_line(Language::Ruby, &state, "  EOS");
        assert!(
            matches!(state, LineState::Heredoc { .. }),
            "plain heredoc ends at column 0"
        );
        let (_, state) = highlight_line(Language::Ruby, &state, "EOS");
        assert_eq!(state, LineState::Normal);
        assert!(tokens(Language::Ruby, "a << b").is_empty());
    }

    #[test]
    fn markdown_source_highlighting() {
        assert_eq!(
            tokens(Language::Markdown, "## Title"),
            vec![("## Title".into(), Token::Heading)]
        );
        assert_eq!(
            tokens(
                Language::Markdown,
                "- item with `code`, **bold** and [a](b)"
            ),
            vec![
                ("-".into(), Token::Marker),
                ("`code`".into(), Token::Code),
                ("**bold**".into(), Token::Emphasis),
                ("[a](b)".into(), Token::Link),
            ]
        );
        assert!(tokens(Language::Markdown, "snake_case_name").is_empty());
        assert_eq!(
            tokens(Language::Markdown, "> quoted"),
            vec![("> quoted".into(), Token::Quote)]
        );
    }

    #[test]
    fn markdown_fences_highlight_ruby_inside() {
        let (_, state) = highlight_line(Language::Markdown, &LineState::Normal, "```ruby");
        assert!(matches!(state, LineState::Fence { ruby: Some(_), .. }));
        let (spans, state) = highlight_line(Language::Markdown, &state, "class Foo");
        assert_eq!(spans[0].token, Token::Keyword);
        let (_, state) = highlight_line(Language::Markdown, &state, "```");
        assert_eq!(state, LineState::Normal);

        let (_, state) = highlight_line(Language::Markdown, &LineState::Normal, "~~~");
        let (spans, state) = highlight_line(Language::Markdown, &state, "# not a heading");
        assert_eq!(spans[0].token, Token::Code);
        let (_, state) = highlight_line(Language::Markdown, &state, "~~~");
        assert_eq!(state, LineState::Normal);
    }

    #[test]
    fn ruby_block_structure_helpers() {
        assert!(ruby_opens_block("  def run(x)"));
        assert!(ruby_opens_block("items.each do |item|"));
        assert!(ruby_opens_block("if ready # wait"));
        assert!(!ruby_opens_block("def one; 1; end"));
        assert!(!ruby_opens_block("puts 1 if ready"));
        let lines: Vec<String> = ["class A", "  def b", "    if c", "      d", "    end", "  "]
            .iter()
            .map(|line| line.to_string())
            .collect();
        assert_eq!(ruby_matching_indent(&lines, 5), Some(2));
        assert_eq!(ruby_matching_indent(&lines, 4), Some(4));
        assert_eq!(ruby_matching_indent(&lines[..1], 1), Some(0));
        assert_eq!(ruby_matching_indent(&[], 0), None);
    }
}

//! Resolution-independent text rendering and UI scaling.
//!
//! Selenite embeds DejaVu Sans and renders it through a mipmapped, trilinear
//! filtered atlas so text stays crisp from tiny grid labels to very large,
//! zoomed-in cell captions. UI chrome (toolbar, dialogs, console, status bar)
//! scales with the window size and a user multiplier (Ctrl +/-/0).

use std::{cell::Cell, ffi::CString};

use raylib::{ffi, prelude::*};

/// Reference window size the base UI metrics were designed for.
const BASE_WIDTH: f32 = 960.0;
const BASE_HEIGHT: f32 = 720.0;
/// Makes the default UI noticeably larger than raylib's stock sizes.
const BASE_BOOST: f32 = 1.25;
const MIN_USER_SCALE: f32 = 0.5;
const MAX_USER_SCALE: f32 = 3.0;
const USER_SCALE_STEP: f32 = 0.1;

const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
/// Glyph atlas resolution; larger sizes are upscaled, smaller ones use mipmaps.
const ATLAS_FONT_SIZE: i32 = 96;

thread_local! {
    static FONT: Cell<Option<ffi::Font>> = const { Cell::new(None) };
    static USER_SCALE: Cell<f32> = const { Cell::new(1.0) };
}

/// Loads the embedded UI font. Must be called once per process after the
/// raylib window exists; falls back to raylib's default font on failure.
pub fn load_font(_thread: &RaylibThread) {
    let mut codepoints: Vec<i32> = (32..=126).chain(160..=591).collect();
    codepoints.extend([
        0x2026, 0x2013, 0x2014, 0x2022, 0x2190, 0x2191, 0x2192, 0x2193, 0x203A,
    ]);
    let file_type = CString::new(".ttf").expect("static string has no NUL");
    // SAFETY: the window/GL context exists (caller holds the RaylibThread
    // produced by `raylib::init().build()`), and all pointers are valid for
    // the duration of the calls.
    unsafe {
        let mut font = ffi::LoadFontFromMemory(
            file_type.as_ptr(),
            FONT_BYTES.as_ptr(),
            FONT_BYTES.len() as i32,
            ATLAS_FONT_SIZE,
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
        // The font intentionally lives for the rest of the process.
        FONT.with(|cell| cell.set(Some(font)));
    }
}

fn current_font() -> (ffi::Font, bool) {
    match FONT.with(Cell::get) {
        Some(font) => (font, true),
        // SAFETY: only reached after window creation, when the default font exists.
        None => (unsafe { ffi::GetFontDefault() }, false),
    }
}

fn spacing(size: f32, custom_font: bool) -> f32 {
    if custom_font {
        0.0
    } else {
        (size / 10.0).max(1.0)
    }
}

fn c_text(text: &str) -> CString {
    CString::new(text.replace('\0', "")).expect("NUL bytes were removed")
}

/// Draws `text` with its top-left corner at (`x`, `y`) at `size` pixels.
pub fn draw_text<D: RaylibDraw>(_d: &mut D, text: &str, x: f32, y: f32, size: f32, color: Color) {
    if text.is_empty() || size < 1.0 {
        return;
    }
    let (font, custom) = current_font();
    let text = c_text(text);
    // SAFETY: called while a draw handle is borrowed, i.e. inside
    // BeginDrawing/EndDrawing; `text` outlives the call.
    unsafe {
        ffi::DrawTextEx(
            font,
            text.as_ptr(),
            ffi::Vector2 { x, y },
            size,
            spacing(size, custom),
            color.into(),
        );
    }
}

/// Draws `text` vertically centred inside `rect`, starting `pad_x` from the left.
pub fn draw_text_in<D: RaylibDraw>(
    d: &mut D,
    text: &str,
    rect: Rectangle,
    pad_x: f32,
    size: f32,
    color: Color,
) {
    draw_text(
        d,
        text,
        rect.x + pad_x,
        rect.y + (rect.height - size) / 2.0,
        size,
        color,
    );
}

/// Draws `text` centred horizontally and vertically inside `rect`.
pub fn draw_text_centered<D: RaylibDraw>(
    d: &mut D,
    text: &str,
    rect: Rectangle,
    size: f32,
    color: Color,
) {
    let width = measure(text, size);
    draw_text(
        d,
        text,
        rect.x + (rect.width - width) / 2.0,
        rect.y + (rect.height - size) / 2.0,
        size,
        color,
    );
}

/// Width in pixels of `text` rendered at `size`.
pub fn measure(text: &str, size: f32) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let (font, custom) = current_font();
    let text = c_text(text);
    // SAFETY: font is valid after window creation; `text` outlives the call.
    unsafe { ffi::MeasureTextEx(font, text.as_ptr(), size, spacing(size, custom)).x }
}

/// Shortens `text` with a trailing ellipsis so it fits within `max_width`.
pub fn fit(text: &str, size: f32, max_width: f32) -> String {
    fit_with(text, max_width, |candidate| measure(candidate, size))
}

/// Word-wraps `text` into lines no wider than `max_width` (explicit
/// newlines are kept; over-long words are truncated with an ellipsis).
pub fn wrap(text: &str, size: f32, max_width: f32) -> Vec<String> {
    wrap_with(text, max_width, |candidate| measure(candidate, size))
}

fn wrap_with(text: &str, max_width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if measure(&candidate) <= max_width || line.is_empty() {
                line = candidate;
            } else {
                lines.push(std::mem::take(&mut line));
                line = word.to_owned();
            }
            if measure(&line) > max_width {
                lines.push(fit_with(&line, max_width, &measure));
                line.clear();
            }
        }
        if !line.is_empty() || paragraph.trim().is_empty() {
            lines.push(line);
        }
    }
    lines
}

fn fit_with(text: &str, max_width: f32, measure: impl Fn(&str) -> f32) -> String {
    if measure(text) <= max_width {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let (mut low, mut high) = (0usize, chars.len());
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate: String = chars[..mid].iter().chain(['…'].iter()).collect();
        if measure(&candidate) <= max_width {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    if low == 0 {
        String::new()
    } else {
        chars[..low].iter().chain(['…'].iter()).collect()
    }
}

/// Overall UI scale factor for a window of the given size.
pub fn scale(screen_width: i32, screen_height: i32) -> f32 {
    scale_for(
        screen_width as f32,
        screen_height as f32,
        USER_SCALE.with(Cell::get),
    )
}

fn scale_for(width: f32, height: f32, user_scale: f32) -> f32 {
    let window = (width / BASE_WIDTH)
        .min(height / BASE_HEIGHT)
        .clamp(0.75, 3.0);
    window * BASE_BOOST * user_scale
}

/// Handles Ctrl+= / Ctrl+- / Ctrl+0 to grow, shrink, or reset the UI scale.
/// Returns the new user scale when it changed.
pub fn handle_scale_keys(rl: &RaylibHandle) -> Option<f32> {
    let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
        || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
    if !ctrl {
        return None;
    }
    let current = USER_SCALE.with(Cell::get);
    let next = if rl.is_key_pressed(KeyboardKey::KEY_EQUAL)
        || rl.is_key_pressed(KeyboardKey::KEY_KP_ADD)
    {
        current + USER_SCALE_STEP
    } else if rl.is_key_pressed(KeyboardKey::KEY_MINUS)
        || rl.is_key_pressed(KeyboardKey::KEY_KP_SUBTRACT)
    {
        current - USER_SCALE_STEP
    } else if rl.is_key_pressed(KeyboardKey::KEY_ZERO) || rl.is_key_pressed(KeyboardKey::KEY_KP_0) {
        1.0
    } else {
        return None;
    };
    let next = (next.clamp(MIN_USER_SCALE, MAX_USER_SCALE) * 10.0).round() / 10.0;
    USER_SCALE.with(|cell| cell.set(next));
    Some(next)
}

/// Font size for text drawn inside a grid cell, proportional to the cell's
/// on-screen size so labels follow the camera zoom.
pub fn cell_text_size(cell_height: f32, ratio: f32, max: f32) -> f32 {
    (cell_height * ratio).min(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_grows_with_window_and_user_multiplier() {
        let base = scale_for(BASE_WIDTH, BASE_HEIGHT, 1.0);
        assert!((base - BASE_BOOST).abs() < f32::EPSILON);
        assert!(scale_for(BASE_WIDTH * 2.0, BASE_HEIGHT * 2.0, 1.0) > base * 1.9);
        assert!(scale_for(BASE_WIDTH * 2.0, BASE_HEIGHT, 1.0) - base < f32::EPSILON);
        assert!(scale_for(100.0, 100.0, 1.0) >= 0.75 * BASE_BOOST);
        assert!(scale_for(BASE_WIDTH, BASE_HEIGHT, 1.5) > base);
    }

    #[test]
    fn fit_truncates_with_ellipsis() {
        let measure = |text: &str| text.chars().count() as f32 * 10.0;
        assert_eq!(fit_with("short", 100.0, measure), "short");
        assert_eq!(fit_with("a_very_long_name.mp3", 60.0, measure), "a_ver…");
        assert_eq!(fit_with("abc", 5.0, measure), "");
    }

    #[test]
    fn wrap_breaks_on_words_and_keeps_paragraphs() {
        let measure = |text: &str| text.chars().count() as f32 * 10.0;
        assert_eq!(
            wrap_with("one two three four", 90.0, measure),
            vec!["one two", "three", "four"]
        );
        assert_eq!(wrap_with("a\n\nb", 100.0, measure), vec!["a", "", "b"]);
        assert_eq!(
            wrap_with("supercalifragilistic x", 60.0, measure),
            vec!["super…", "x"]
        );
    }

    #[test]
    fn cell_text_follows_zoom_but_is_capped() {
        assert!(cell_text_size(200.0, 0.15, 48.0) > cell_text_size(100.0, 0.15, 48.0));
        assert_eq!(cell_text_size(10_000.0, 0.15, 48.0), 48.0);
    }
}

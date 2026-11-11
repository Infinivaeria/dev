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
/// Window-size scaling keeps growing up to this factor, so maximized windows on large displays stay readable.
const MAX_WINDOW_SCALE: f32 = 6.0;
/// Measured toolbar footprint at scale 1 (button area in px²) and the window share zoom may let it use.
const TOOLBAR_AREA: f32 = 65_000.0;
const TOOLBAR_SHARE: f32 = 0.25;

const FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
/// Glyph atlas resolution; larger sizes are upscaled, smaller ones use mipmaps.
const ATLAS_FONT_SIZE: i32 = 96;

thread_local! {
    static FONT: Cell<Option<ffi::Font>> = const { Cell::new(None) };
    static USER_SCALE: Cell<f32> = const { Cell::new(1.0) };
    static TOOLBAR_LIMIT: Cell<bool> = const { Cell::new(false) };
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
    if measure(text) <= max_width + 0.01 {
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
    let (width, height) = (screen_width as f32, screen_height as f32);
    let scaled = scale_for(width, height, USER_SCALE.with(Cell::get));
    if TOOLBAR_LIMIT.with(Cell::get) {
        limit_for_toolbar(width, height, scaled)
    } else {
        scaled
    }
}

/// Windows with a toolbar stop zooming once the wrapped toolbar would crowd out the content.
pub fn limit_to_toolbar() {
    TOOLBAR_LIMIT.with(|cell| cell.set(true));
}

fn scale_for(width: f32, height: f32, user_scale: f32) -> f32 {
    let window = (width / BASE_WIDTH)
        .min(height / BASE_HEIGHT)
        .clamp(0.75, MAX_WINDOW_SCALE);
    window * BASE_BOOST * user_scale
}

/// The wrapped toolbar needs about TOOLBAR_AREA * scale² / width pixels of height; the
/// default (zoom 1) scale is always allowed, extra zoom only while the toolbar fits.
fn limit_for_toolbar(width: f32, height: f32, scaled: f32) -> f32 {
    let default = scale_for(width, height, 1.0);
    let fit = (TOOLBAR_SHARE * width * height / TOOLBAR_AREA)
        .max(0.0)
        .sqrt();
    scaled.min(fit.max(default))
}

fn clamp_user_scale(value: f32) -> f32 {
    if !value.is_finite() {
        return 1.0;
    }
    (value.clamp(MIN_USER_SCALE, MAX_USER_SCALE) * 100.0).round() / 100.0
}

/// The user's Ctrl +/- zoom multiplier (1.0 by default).
#[allow(dead_code)]
pub fn user_scale() -> f32 {
    USER_SCALE.with(Cell::get)
}

/// Restores a saved zoom multiplier; out-of-range or invalid values are corrected.
#[allow(dead_code)]
pub fn set_user_scale(value: f32) {
    USER_SCALE.with(|cell| cell.set(clamp_user_scale(value)));
}

#[derive(Default)]
pub(crate) struct ScaleMenu {
    pub open: bool,
    toolbar_enabled: Option<bool>,
}

impl ScaleMenu {
    fn geometry(width: i32, height: i32) -> (Rectangle, Vec<Rectangle>) {
        let row = (height as f32 / 9.0).clamp(20.0, 40.0);
        let menu_width = (width as f32 - 16.0).clamp(1.0, 400.0);
        let bounds = Rectangle::new((width as f32 - menu_width) / 2.0, (height as f32 - row * 7.0) / 2.0, menu_width, row * 7.0);
        let rows = (0..6).map(|index| Rectangle::new(bounds.x + 8.0, bounds.y + row * (index + 1) as f32, bounds.width - 16.0, row - 1.0)).collect();
        (bounds, rows)
    }

    fn toolbar_toggle(bounds: Rectangle, row_height: f32) -> Rectangle {
        let width = bounds.width * 0.42;
        Rectangle::new(bounds.x + bounds.width - width - 8.0, bounds.y, width, row_height)
    }

    pub fn update(&mut self, rl: &RaylibHandle) -> bool {
        if rl.is_key_pressed(KeyboardKey::KEY_F9) { self.open = !self.open; }
        if !self.open { return false; }
        let path = crate::settings::Settings::path_in(&crate::profiles::default_base_dir());
        let mut settings = crate::settings::Settings::load(&path);
        self.toolbar_enabled = Some(settings.toolbar_enabled);
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) { self.open = false; return true; }
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let (bounds, rows) = Self::geometry(rl.get_screen_width(), rl.get_screen_height());
            let mouse = rl.get_mouse_position();
            if Self::toolbar_toggle(bounds, rows[0].height).check_collision_point_rec(mouse) {
                settings.toolbar_enabled = !settings.toolbar_enabled;
                if settings.save(&path).is_ok() { self.toolbar_enabled = Some(settings.toolbar_enabled); }
            } else if let Some(index) = rows.iter().position(|row| row.check_collision_point_rec(mouse)) {
                if index == 5 { self.open = false; }
                else {
                    set_user_scale([0.75, 1.0, 1.25, 1.5, 2.0][index]);
                    let _ = crate::settings::save_ui_scale(user_scale());
                }
            } else if !bounds.check_collision_point_rec(mouse) { self.open = false; }
        }
        true
    }

    pub fn draw<D: RaylibDraw>(&self, d: &mut D, width: i32, height: i32) {
        if !self.open { return; }
        let (bounds, rows) = Self::geometry(width, height);
        let font = (rows[0].height * 0.45).clamp(9.0, 18.0);
        d.draw_rectangle(0, 0, width, height, Color::new(0, 0, 0, 170));
        d.draw_rectangle_rec(bounds, Color::new(28, 29, 35, 255));
        let toggle = Self::toolbar_toggle(bounds, rows[0].height);
        let title = fit("Interface Scale", font, toggle.x - bounds.x - 20.0);
        draw_text_in(d, &title, Rectangle::new(bounds.x, bounds.y, toggle.x - bounds.x, rows[0].height), 12.0, font, Color::RAYWHITE);
        let box_size = (rows[0].height * 0.5).clamp(10.0, 18.0);
        let checkbox = Rectangle::new(toggle.x, toggle.y + (toggle.height - box_size) / 2.0, box_size, box_size);
        d.draw_rectangle_lines_ex(checkbox, 1.0, Color::RAYWHITE);
        if self.toolbar_enabled.unwrap_or(true) {
            d.draw_rectangle_rec(Rectangle::new(checkbox.x + 3.0, checkbox.y + 3.0, checkbox.width - 6.0, checkbox.height - 6.0), Color::SKYBLUE);
        }
        draw_text_in(d, "Toolbar", toggle, box_size + 6.0, font, Color::RAYWHITE);
        for (index, row) in rows.iter().enumerate() {
            let selected = index < 5 && (user_scale() - [0.75, 1.0, 1.25, 1.5, 2.0][index.min(4)]).abs() < 0.01;
            d.draw_rectangle_rec(*row, if selected { Color::new(50, 80, 95, 255) } else { Color::new(38, 40, 48, 255) });
            draw_text_in(d, ["75%", "100%", "125%", "150%", "200%", "Close"][index], *row, 12.0, font, Color::RAYWHITE);
        }
    }
}

/// Ctrl + mouse wheel zooms the UI like Ctrl +/-. Returns the new scale when it changed.
#[allow(dead_code)]
pub fn handle_scale_wheel(rl: &RaylibHandle) -> Option<f32> {
    let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
        || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
    let wheel = rl.get_mouse_wheel_move();
    if !ctrl || wheel == 0.0 {
        return None;
    }
    let current = user_scale();
    let next = clamp_user_scale(current + USER_SCALE_STEP * wheel.signum());
    set_user_scale(next);
    (next != current).then_some(next)
}

/// Sizes a new window for the display: `width` x `height` scaled by the
/// monitor's DPI factor, never larger than the monitor.
#[allow(dead_code)]
pub fn fit_window_to_display(rl: &mut RaylibHandle, width: i32, height: i32) {
    let dpi = rl.get_window_scale_dpi();
    let factor = if dpi.x.is_finite() && dpi.y.is_finite() {
        dpi.x.max(dpi.y).clamp(1.0, 4.0)
    } else {
        1.0
    };
    let monitor = raylib::core::window::get_current_monitor();
    let (monitor_width, monitor_height) = (
        raylib::core::window::get_monitor_width(monitor),
        raylib::core::window::get_monitor_height(monitor),
    );
    let (wanted_width, wanted_height) = window_size_for(
        width,
        height,
        factor,
        (monitor_width > 0).then_some((monitor_width, monitor_height)),
    );
    if (wanted_width, wanted_height) != (width, height) {
        rl.set_window_size(wanted_width, wanted_height);
    }
}

#[allow(dead_code)]
fn window_size_for(
    width: i32,
    height: i32,
    factor: f32,
    monitor: Option<(i32, i32)>,
) -> (i32, i32) {
    let scaled = (
        (width as f32 * factor).round() as i32,
        (height as f32 * factor).round() as i32,
    );
    match monitor {
        Some((monitor_width, monitor_height)) => (
            scaled.0.min((monitor_width - 80).max(320)),
            scaled.1.min((monitor_height - 80).max(240)),
        ),
        None => scaled,
    }
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
    fn scale_menu_options_fit_small_and_large_windows() {
        for (width, height) in [(240, 140), (420, 300), (1100, 760), (3840, 2160)] {
            let (bounds, rows) = ScaleMenu::geometry(width, height);
            assert!(bounds.x >= 0.0 && bounds.y >= 0.0);
            assert!(bounds.x + bounds.width <= width as f32);
            assert!(bounds.y + bounds.height <= height as f32);
            for (index, row) in rows.iter().enumerate() {
                assert!(row.x >= bounds.x && row.x + row.width <= bounds.x + bounds.width);
                assert!(row.y + row.height <= bounds.y + bounds.height);
                for other in &rows[index + 1..] { assert!(!row.check_collision_recs(*other)); }
            }
        }
    }

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
    fn maximized_windows_on_large_displays_keep_scaling() {
        let hd = scale_for(1920.0, 1080.0, 1.0);
        let uhd = scale_for(3840.0, 2160.0, 1.0);
        let huge = scale_for(7680.0, 4320.0, 1.0);
        assert!((uhd / hd - 2.0).abs() < 0.01, "4K is twice 1080p");
        assert!(huge > uhd, "8K keeps growing past the old 3x cap");
        assert!((huge - MAX_WINDOW_SCALE * BASE_BOOST).abs() < 0.01);
        assert!(scale_for(1e9, 1e9, 1.0) <= MAX_WINDOW_SCALE * BASE_BOOST + 0.001);
    }

    #[test]
    fn saved_zoom_cannot_bury_the_window_under_the_toolbar() {
        for (width, height) in [(960.0, 720.0), (1920.0, 1080.0), (6010.0, 3318.0)] {
            let base = scale_for(width, height, 1.0);
            let zoomed = limit_for_toolbar(width, height, scale_for(width, height, MAX_USER_SCALE));
            assert!(zoomed >= base, "zoom never shrinks the default");
            let toolbar_height = TOOLBAR_AREA * zoomed * zoomed / width;
            assert!(zoomed == base || toolbar_height <= height * TOOLBAR_SHARE * 1.001);
        }
        assert!(limit_for_toolbar(960.0, 720.0, scale_for(960.0, 720.0, 2.3)) < 2.0);
    }

    #[test]
    fn user_scale_is_clamped_rounded_and_survives_bad_input() {
        assert_eq!(clamp_user_scale(1.04), 1.04);
        assert_eq!(clamp_user_scale(0.75), 0.75);
        assert_eq!(clamp_user_scale(1.25), 1.25);
        assert_eq!(clamp_user_scale(0.01), MIN_USER_SCALE);
        assert_eq!(clamp_user_scale(99.0), MAX_USER_SCALE);
        assert_eq!(clamp_user_scale(f32::NAN), 1.0);
        assert_eq!(clamp_user_scale(f32::INFINITY), 1.0);
        set_user_scale(1.5);
        assert_eq!(user_scale(), 1.5);
        assert!(scale(960, 720) > BASE_BOOST);
        set_user_scale(f32::NAN);
        assert_eq!(user_scale(), 1.0);
    }

    #[test]
    fn new_windows_follow_dpi_but_stay_on_the_monitor() {
        assert_eq!(
            window_size_for(960, 720, 1.0, Some((1920, 1080))),
            (960, 720)
        );
        assert_eq!(
            window_size_for(960, 720, 2.0, Some((3840, 2160))),
            (1920, 1440)
        );
        assert_eq!(
            window_size_for(960, 720, 2.0, Some((1920, 1080))),
            (1840, 1000)
        );
        assert_eq!(window_size_for(960, 720, 1.5, None), (1440, 1080));
        assert_eq!(window_size_for(960, 720, 1.0, Some((200, 100))), (320, 240));
    }

    #[test]
    fn fit_truncates_with_ellipsis() {
        let measure = |text: &str| text.chars().count() as f32 * 10.0;
        assert_eq!(fit_with("short", 100.0, measure), "short");
        assert_eq!(fit_with("a_very_long_name.mp3", 60.0, measure), "a_ver…");
        assert_eq!(fit_with("abc", 5.0, measure), "");
    }

    #[test]
    fn fit_preserves_short_button_labels_at_float_rounding_boundaries() {
        let measure = |text: &str| text.chars().count() as f32 * 10.0;
        assert_eq!(fit_with("In", 20.0_f32.next_down(), measure), "In");
        assert_eq!(fit_with("Out", 30.0_f32.next_down(), measure), "Out");
        assert_eq!(fit_with("In", 5.0, measure), "");
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

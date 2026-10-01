//! Game development API: a raylib-bindings–style `Raylib` Ruby module
//! (CamelCase functions, `include Raylib`, `KEY_SPACE`, `RAYWHITE`, …)
//! backed directly by raylib's C API, plus `Selenite::Game` (see
//! `ruby/raylib_game.rb`).
//!
//! Window/drawing/audio calls only work in game mode (`selenite --game
//! FILE.rb`), where Ruby owns the process's single raylib window. Inside the
//! Selenite app the module still loads (so scripts can share helpers,
//! collision math and constants) but window calls raise.
//!
//! Textures, sounds and music streams live in thread-local tables; Ruby
//! sees small integer handles wrapped in `Raylib::Texture2D` etc.

use std::{
    cell::RefCell,
    ffi::CString,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use magnus::{
    function, prelude::*, value::ReprValue, Error, Integer, RArray, RModule, Ruby, Value,
};
use raylib::ffi;

static GAME_MODE: AtomicBool = AtomicBool::new(false);
static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
static AUDIO_OPEN: AtomicBool = AtomicBool::new(false);
static FRAMES: AtomicU64 = AtomicU64::new(0);

/// `SELENITE_GAME_FRAMES=N` makes `WindowShouldClose` return true after N
/// frames, and `SELENITE_GAME_SCREENSHOT=name.png` captures the last of
/// them — for smoke-testing games unattended (CI, packaging checks).
fn frame_limit() -> Option<u64> {
    std::env::var("SELENITE_GAME_FRAMES")
        .ok()
        .and_then(|value| value.trim().parse().ok())
}

const RUBY_SIDE: &str = include_str!("ruby/raylib_game.rb");

thread_local! {
    static TEXTURES: RefCell<Vec<Option<ffi::Texture2D>>> = const { RefCell::new(Vec::new()) };
    static SOUNDS: RefCell<Vec<Option<ffi::Sound>>> = const { RefCell::new(Vec::new()) };
    static MUSIC: RefCell<Vec<Option<ffi::Music>>> = const { RefCell::new(Vec::new()) };
}

/// Allows window/audio calls (only `selenite --game` does this).
pub fn enable() {
    GAME_MODE.store(true, Ordering::SeqCst);
    // raylib's INFO chatter drowns out a game's own output; scripts can
    // raise it again with SetTraceLogLevel(LOG_INFO).
    unsafe { ffi::SetTraceLogLevel(ffi::TraceLogLevel::LOG_WARNING as i32) };
}

/// Releases anything a game script left open (textures, audio, window).
pub fn shutdown() {
    unload_all();
    if AUDIO_OPEN.swap(false, Ordering::SeqCst) {
        unsafe { ffi::CloseAudioDevice() };
    }
    if WINDOW_OPEN.swap(false, Ordering::SeqCst) {
        unsafe { ffi::CloseWindow() };
    }
}

fn unload_all() {
    MUSIC.with_borrow_mut(|table| {
        for music in table.drain(..).flatten() {
            unsafe { ffi::UnloadMusicStream(music) };
        }
    });
    SOUNDS.with_borrow_mut(|table| {
        for sound in table.drain(..).flatten() {
            unsafe { ffi::UnloadSound(sound) };
        }
    });
    TEXTURES.with_borrow_mut(|table| {
        for texture in table.drain(..).flatten() {
            unsafe { ffi::UnloadTexture(texture) };
        }
    });
}

fn error(message: impl Into<String>) -> Error {
    let ruby = Ruby::get().expect("Ruby is running");
    Error::new(ruby.exception_runtime_error(), message.into())
}

fn arg_error(message: impl Into<String>) -> Error {
    let ruby = Ruby::get().expect("Ruby is running");
    Error::new(ruby.exception_arg_error(), message.into())
}

fn need_game() -> Result<(), Error> {
    if GAME_MODE.load(Ordering::SeqCst) {
        Ok(())
    } else {
        Err(error(
            "Raylib window functions only run in game mode: selenite --game FILE.rb \
             (or the \"Run as game\" menu item)",
        ))
    }
}

fn need_window() -> Result<(), Error> {
    need_game()?;
    if WINDOW_OPEN.load(Ordering::SeqCst) {
        Ok(())
    } else {
        Err(error("call InitWindow first"))
    }
}

fn need_audio() -> Result<(), Error> {
    need_game()?;
    if AUDIO_OPEN.load(Ordering::SeqCst) {
        Ok(())
    } else {
        Err(error("call InitAudioDevice first"))
    }
}

fn c_string(text: &str) -> CString {
    CString::new(text.replace('\0', "")).unwrap_or_default()
}

// ---- argument conversion: arrays, structs (x/y/…) or numbers -------------

fn float(value: Value) -> Result<f32, Error> {
    f64::try_convert(value).map(|number| number as f32)
}

fn field(value: Value, name: &str) -> Result<f32, Error> {
    float(value.funcall::<_, _, Value>(name, ())?)
}

fn floats(value: Value, count: usize, what: &str) -> Result<Option<Vec<f32>>, Error> {
    let Some(array) = RArray::from_value(value) else {
        return Ok(None);
    };
    if array.len() != count {
        return Err(arg_error(format!(
            "{what} array needs {count} numbers, got {}",
            array.len()
        )));
    }
    array
        .into_iter()
        .map(float)
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn vec2(value: Value) -> Result<ffi::Vector2, Error> {
    if let Some(parts) = floats(value, 2, "Vector2")? {
        return Ok(ffi::Vector2 {
            x: parts[0],
            y: parts[1],
        });
    }
    Ok(ffi::Vector2 {
        x: field(value, "x")?,
        y: field(value, "y")?,
    })
}

fn rect(value: Value) -> Result<ffi::Rectangle, Error> {
    if let Some(parts) = floats(value, 4, "Rectangle")? {
        return Ok(ffi::Rectangle {
            x: parts[0],
            y: parts[1],
            width: parts[2],
            height: parts[3],
        });
    }
    Ok(ffi::Rectangle {
        x: field(value, "x")?,
        y: field(value, "y")?,
        width: field(value, "width")?,
        height: field(value, "height")?,
    })
}

fn channel(value: Value) -> Result<u8, Error> {
    Ok(f64::try_convert(value)?.round().clamp(0.0, 255.0) as u8)
}

/// A Color struct, `[r, g, b]` / `[r, g, b, a]`, or `0xRRGGBBAA`.
fn color(value: Value) -> Result<ffi::Color, Error> {
    if let Some(number) = Integer::from_value(value) {
        let packed = number.to_i64()? as u32;
        return Ok(ffi::Color {
            r: (packed >> 24) as u8,
            g: (packed >> 16) as u8,
            b: (packed >> 8) as u8,
            a: packed as u8,
        });
    }
    if let Some(array) = RArray::from_value(value) {
        let parts: Vec<Value> = array.into_iter().collect();
        if !(3..=4).contains(&parts.len()) {
            return Err(arg_error("a color array needs 3 or 4 numbers"));
        }
        return Ok(ffi::Color {
            r: channel(parts[0])?,
            g: channel(parts[1])?,
            b: channel(parts[2])?,
            a: parts.get(3).copied().map_or(Ok(255), channel)?,
        });
    }
    Ok(ffi::Color {
        r: channel(value.funcall("r", ())?)?,
        g: channel(value.funcall("g", ())?)?,
        b: channel(value.funcall("b", ())?)?,
        a: channel(value.funcall("a", ())?)?,
    })
}

fn camera2d(value: Value) -> Result<ffi::Camera2D, Error> {
    Ok(ffi::Camera2D {
        offset: vec2(value.funcall("offset", ())?)?,
        target: vec2(value.funcall("target", ())?)?,
        rotation: field(value, "rotation")?,
        zoom: field(value, "zoom")?,
    })
}

/// Integer handle, or any object with an `id` (Texture2D, Sound, Music).
fn handle(value: Value) -> Result<usize, Error> {
    let id = match Integer::from_value(value) {
        Some(number) => number.to_i64()?,
        None => i64::try_convert(value.funcall("id", ())?)?,
    };
    usize::try_from(id)
        .ok()
        .filter(|id| *id > 0)
        .map(|id| id - 1)
        .ok_or_else(|| arg_error(format!("invalid resource handle {id}")))
}

fn texture(value: Value) -> Result<ffi::Texture2D, Error> {
    let index = handle(value)?;
    TEXTURES
        .with_borrow(|table| table.get(index).copied().flatten())
        .ok_or_else(|| arg_error("texture was unloaded or never loaded"))
}

fn sound(value: Value) -> Result<ffi::Sound, Error> {
    let index = handle(value)?;
    SOUNDS
        .with_borrow(|table| table.get(index).copied().flatten())
        .ok_or_else(|| arg_error("sound was unloaded or never loaded"))
}

fn music(value: Value) -> Result<ffi::Music, Error> {
    let index = handle(value)?;
    MUSIC
        .with_borrow(|table| table.get(index).copied().flatten())
        .ok_or_else(|| arg_error("music stream was unloaded or never loaded"))
}

fn store<T>(table: &'static std::thread::LocalKey<RefCell<Vec<Option<T>>>>, item: T) -> i64 {
    table.with_borrow_mut(|table| {
        table.push(Some(item));
        table.len() as i64
    })
}

fn take<T>(
    table: &'static std::thread::LocalKey<RefCell<Vec<Option<T>>>>,
    value: Value,
) -> Result<Option<T>, Error> {
    let index = handle(value)?;
    Ok(table.with_borrow_mut(|table| table.get_mut(index).and_then(Option::take)))
}

// ---- window & timing --------------------------------------------------------

fn init_window(width: i32, height: i32, title: String) -> Result<(), Error> {
    need_game()?;
    if WINDOW_OPEN.load(Ordering::SeqCst) {
        return Err(error("the game window is already open"));
    }
    if width <= 0 || height <= 0 {
        return Err(arg_error("window size must be positive"));
    }
    let title = c_string(&title);
    unsafe { ffi::InitWindow(width, height, title.as_ptr()) };
    if !unsafe { ffi::IsWindowReady() } {
        return Err(error("could not open a window (is a display available?)"));
    }
    WINDOW_OPEN.store(true, Ordering::SeqCst);
    Ok(())
}

fn close_window() -> Result<(), Error> {
    need_game()?;
    unload_all();
    if WINDOW_OPEN.swap(false, Ordering::SeqCst) {
        unsafe { ffi::CloseWindow() };
    }
    Ok(())
}

fn window_should_close() -> Result<bool, Error> {
    need_window()?;
    if frame_limit().is_some_and(|limit| FRAMES.load(Ordering::SeqCst) >= limit) {
        return Ok(true);
    }
    Ok(unsafe { ffi::WindowShouldClose() })
}

fn is_window_ready() -> bool {
    WINDOW_OPEN.load(Ordering::SeqCst)
}

fn game_mode() -> bool {
    GAME_MODE.load(Ordering::SeqCst)
}

fn set_target_fps(fps: i32) -> Result<(), Error> {
    need_game()?;
    unsafe { ffi::SetTargetFPS(fps) };
    Ok(())
}

fn set_config_flags(flags: u32) -> Result<(), Error> {
    need_game()?;
    unsafe { ffi::SetConfigFlags(flags) };
    Ok(())
}

fn set_trace_log_level(level: i32) {
    unsafe { ffi::SetTraceLogLevel(level) };
}

fn get_frame_time() -> Result<f32, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetFrameTime() })
}

fn get_time() -> Result<f64, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetTime() })
}

fn get_fps() -> Result<i32, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetFPS() })
}

fn get_screen_width() -> Result<i32, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetScreenWidth() })
}

fn get_screen_height() -> Result<i32, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetScreenHeight() })
}

fn set_window_title(title: String) -> Result<(), Error> {
    need_window()?;
    let title = c_string(&title);
    unsafe { ffi::SetWindowTitle(title.as_ptr()) };
    Ok(())
}

fn set_window_size(width: i32, height: i32) -> Result<(), Error> {
    need_window()?;
    unsafe { ffi::SetWindowSize(width, height) };
    Ok(())
}

fn toggle_fullscreen() -> Result<(), Error> {
    need_window()?;
    unsafe { ffi::ToggleFullscreen() };
    Ok(())
}

fn set_exit_key(key: i32) -> Result<(), Error> {
    need_game()?;
    unsafe { ffi::SetExitKey(key) };
    Ok(())
}

fn show_cursor() -> Result<(), Error> {
    need_window()?;
    unsafe { ffi::ShowCursor() };
    Ok(())
}

fn hide_cursor() -> Result<(), Error> {
    need_window()?;
    unsafe { ffi::HideCursor() };
    Ok(())
}

/// Saves the current framebuffer. Unlike raylib's TakeScreenshot (which
/// always prefixes the working directory) this accepts absolute paths.
fn capture_screen(path: &str) -> bool {
    let path = c_string(path);
    unsafe {
        let image = ffi::LoadImageFromScreen();
        let ok = ffi::ExportImage(image, path.as_ptr());
        ffi::UnloadImage(image);
        ok
    }
}

fn take_screenshot(path: String) -> Result<bool, Error> {
    need_window()?;
    unsafe { ffi::rlDrawRenderBatchActive() };
    Ok(capture_screen(&path))
}

// ---- drawing ------------------------------------------------------------------

fn begin_drawing() -> Result<(), Error> {
    need_window()?;
    unsafe { ffi::BeginDrawing() };
    Ok(())
}

fn end_drawing() -> Result<(), Error> {
    need_window()?;
    let frame = FRAMES.fetch_add(1, Ordering::SeqCst) + 1;
    if frame_limit() == Some(frame) {
        if let Ok(name) = std::env::var("SELENITE_GAME_SCREENSHOT") {
            // Flush batched shapes/text so the capture sees the full frame.
            unsafe { ffi::rlDrawRenderBatchActive() };
            if !capture_screen(&name) {
                eprintln!("selenite: could not save screenshot to {name}");
            }
        }
    }
    unsafe { ffi::EndDrawing() };
    Ok(())
}

fn clear_background(tint: Value) -> Result<(), Error> {
    need_window()?;
    let tint = color(tint)?;
    unsafe { ffi::ClearBackground(tint) };
    Ok(())
}

fn begin_mode_2d(camera: Value) -> Result<(), Error> {
    need_window()?;
    let camera = camera2d(camera)?;
    unsafe { ffi::BeginMode2D(camera) };
    Ok(())
}

fn end_mode_2d() -> Result<(), Error> {
    need_window()?;
    unsafe { ffi::EndMode2D() };
    Ok(())
}

fn draw_pixel(x: i32, y: i32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let tint = color(tint)?;
    unsafe { ffi::DrawPixel(x, y, tint) };
    Ok(())
}

fn draw_line(x1: i32, y1: i32, x2: i32, y2: i32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let tint = color(tint)?;
    unsafe { ffi::DrawLine(x1, y1, x2, y2, tint) };
    Ok(())
}

fn draw_line_ex(start: Value, end: Value, thick: f32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let (start, end, tint) = (vec2(start)?, vec2(end)?, color(tint)?);
    unsafe { ffi::DrawLineEx(start, end, thick, tint) };
    Ok(())
}

fn draw_circle(x: i32, y: i32, radius: f32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let tint = color(tint)?;
    unsafe { ffi::DrawCircle(x, y, radius, tint) };
    Ok(())
}

fn draw_circle_v(center: Value, radius: f32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let (center, tint) = (vec2(center)?, color(tint)?);
    unsafe { ffi::DrawCircleV(center, radius, tint) };
    Ok(())
}

fn draw_circle_lines(x: i32, y: i32, radius: f32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let tint = color(tint)?;
    unsafe { ffi::DrawCircleLines(x, y, radius, tint) };
    Ok(())
}

fn draw_ellipse(x: i32, y: i32, radius_h: f32, radius_v: f32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let tint = color(tint)?;
    unsafe { ffi::DrawEllipse(x, y, radius_h, radius_v, tint) };
    Ok(())
}

fn draw_rectangle(x: i32, y: i32, width: i32, height: i32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let tint = color(tint)?;
    unsafe { ffi::DrawRectangle(x, y, width, height, tint) };
    Ok(())
}

fn draw_rectangle_v(position: Value, size: Value, tint: Value) -> Result<(), Error> {
    need_window()?;
    let (position, size, tint) = (vec2(position)?, vec2(size)?, color(tint)?);
    unsafe { ffi::DrawRectangleV(position, size, tint) };
    Ok(())
}

fn draw_rectangle_rec(rec: Value, tint: Value) -> Result<(), Error> {
    need_window()?;
    let (rec, tint) = (rect(rec)?, color(tint)?);
    unsafe { ffi::DrawRectangleRec(rec, tint) };
    Ok(())
}

fn draw_rectangle_lines(x: i32, y: i32, width: i32, height: i32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let tint = color(tint)?;
    unsafe { ffi::DrawRectangleLines(x, y, width, height, tint) };
    Ok(())
}

fn draw_rectangle_lines_ex(rec: Value, thick: f32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let (rec, tint) = (rect(rec)?, color(tint)?);
    unsafe { ffi::DrawRectangleLinesEx(rec, thick, tint) };
    Ok(())
}

fn draw_rectangle_rounded(
    rec: Value,
    roundness: f32,
    segments: i32,
    tint: Value,
) -> Result<(), Error> {
    need_window()?;
    let (rec, tint) = (rect(rec)?, color(tint)?);
    unsafe { ffi::DrawRectangleRounded(rec, roundness, segments, tint) };
    Ok(())
}

fn draw_rectangle_gradient_v(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    top: Value,
    bottom: Value,
) -> Result<(), Error> {
    need_window()?;
    let (top, bottom) = (color(top)?, color(bottom)?);
    unsafe { ffi::DrawRectangleGradientV(x, y, width, height, top, bottom) };
    Ok(())
}

fn draw_triangle(v1: Value, v2: Value, v3: Value, tint: Value) -> Result<(), Error> {
    need_window()?;
    let (v1, v2, v3, tint) = (vec2(v1)?, vec2(v2)?, vec2(v3)?, color(tint)?);
    unsafe { ffi::DrawTriangle(v1, v2, v3, tint) };
    Ok(())
}

fn draw_poly(
    center: Value,
    sides: i32,
    radius: f32,
    rotation: f32,
    tint: Value,
) -> Result<(), Error> {
    need_window()?;
    let (center, tint) = (vec2(center)?, color(tint)?);
    unsafe { ffi::DrawPoly(center, sides, radius, rotation, tint) };
    Ok(())
}

fn draw_text(text: String, x: i32, y: i32, size: i32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let (text, tint) = (c_string(&text), color(tint)?);
    unsafe { ffi::DrawText(text.as_ptr(), x, y, size, tint) };
    Ok(())
}

fn measure_text(text: String, size: i32) -> Result<i32, Error> {
    need_window()?;
    let text = c_string(&text);
    Ok(unsafe { ffi::MeasureText(text.as_ptr(), size) })
}

fn draw_fps(x: i32, y: i32) -> Result<(), Error> {
    need_window()?;
    unsafe { ffi::DrawFPS(x, y) };
    Ok(())
}

// ---- textures -----------------------------------------------------------------

fn load_texture(path: String) -> Result<RArray, Error> {
    need_window()?;
    let c_path = c_string(&path);
    let loaded = unsafe { ffi::LoadTexture(c_path.as_ptr()) };
    if !unsafe { ffi::IsTextureValid(loaded) } {
        return Err(error(format!("could not load texture {path}")));
    }
    let id = store(&TEXTURES, loaded);
    let ruby = Ruby::get().expect("Ruby is running");
    Ok(ruby.ary_new_from_values(&[
        ruby.integer_from_i64(id).as_value(),
        ruby.integer_from_i64(i64::from(loaded.width)).as_value(),
        ruby.integer_from_i64(i64::from(loaded.height)).as_value(),
    ]))
}

fn unload_texture(value: Value) -> Result<bool, Error> {
    need_game()?;
    Ok(match take(&TEXTURES, value)? {
        Some(loaded) => {
            unsafe { ffi::UnloadTexture(loaded) };
            true
        }
        None => false,
    })
}

fn draw_texture(value: Value, x: i32, y: i32, tint: Value) -> Result<(), Error> {
    need_window()?;
    let (loaded, tint) = (texture(value)?, color(tint)?);
    unsafe { ffi::DrawTexture(loaded, x, y, tint) };
    Ok(())
}

fn draw_texture_v(value: Value, position: Value, tint: Value) -> Result<(), Error> {
    need_window()?;
    let (loaded, position, tint) = (texture(value)?, vec2(position)?, color(tint)?);
    unsafe { ffi::DrawTextureV(loaded, position, tint) };
    Ok(())
}

fn draw_texture_ex(
    value: Value,
    position: Value,
    rotation: f32,
    scale: f32,
    tint: Value,
) -> Result<(), Error> {
    need_window()?;
    let (loaded, position, tint) = (texture(value)?, vec2(position)?, color(tint)?);
    unsafe { ffi::DrawTextureEx(loaded, position, rotation, scale, tint) };
    Ok(())
}

fn draw_texture_rec(
    value: Value,
    source: Value,
    position: Value,
    tint: Value,
) -> Result<(), Error> {
    need_window()?;
    let (loaded, source, position, tint) = (
        texture(value)?,
        rect(source)?,
        vec2(position)?,
        color(tint)?,
    );
    unsafe { ffi::DrawTextureRec(loaded, source, position, tint) };
    Ok(())
}

fn draw_texture_pro(
    value: Value,
    source: Value,
    dest: Value,
    origin: Value,
    rotation: f32,
    tint: Value,
) -> Result<(), Error> {
    need_window()?;
    let (loaded, source, dest, origin, tint) = (
        texture(value)?,
        rect(source)?,
        rect(dest)?,
        vec2(origin)?,
        color(tint)?,
    );
    unsafe { ffi::DrawTexturePro(loaded, source, dest, origin, rotation, tint) };
    Ok(())
}

// ---- input --------------------------------------------------------------------

fn is_key_pressed(key: i32) -> Result<bool, Error> {
    need_window()?;
    Ok(unsafe { ffi::IsKeyPressed(key) })
}

fn is_key_down(key: i32) -> Result<bool, Error> {
    need_window()?;
    Ok(unsafe { ffi::IsKeyDown(key) })
}

fn is_key_released(key: i32) -> Result<bool, Error> {
    need_window()?;
    Ok(unsafe { ffi::IsKeyReleased(key) })
}

fn is_key_up(key: i32) -> Result<bool, Error> {
    need_window()?;
    Ok(unsafe { ffi::IsKeyUp(key) })
}

fn get_key_pressed() -> Result<i32, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetKeyPressed() })
}

fn get_char_pressed() -> Result<i32, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetCharPressed() })
}

fn is_mouse_button_pressed(button: i32) -> Result<bool, Error> {
    need_window()?;
    Ok(unsafe { ffi::IsMouseButtonPressed(button) })
}

fn is_mouse_button_down(button: i32) -> Result<bool, Error> {
    need_window()?;
    Ok(unsafe { ffi::IsMouseButtonDown(button) })
}

fn is_mouse_button_released(button: i32) -> Result<bool, Error> {
    need_window()?;
    Ok(unsafe { ffi::IsMouseButtonReleased(button) })
}

fn get_mouse_x() -> Result<i32, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetMouseX() })
}

fn get_mouse_y() -> Result<i32, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetMouseY() })
}

fn get_mouse_position() -> Result<(f32, f32), Error> {
    need_window()?;
    let position = unsafe { ffi::GetMousePosition() };
    Ok((position.x, position.y))
}

fn get_mouse_wheel_move() -> Result<f32, Error> {
    need_window()?;
    Ok(unsafe { ffi::GetMouseWheelMove() })
}

fn set_mouse_position(x: i32, y: i32) -> Result<(), Error> {
    need_window()?;
    unsafe { ffi::SetMousePosition(x, y) };
    Ok(())
}

fn get_random_value(min: i32, max: i32) -> i32 {
    unsafe { ffi::GetRandomValue(min, max) }
}

fn set_random_seed(seed: u32) {
    unsafe { ffi::SetRandomSeed(seed) };
}

// ---- audio --------------------------------------------------------------------

fn init_audio_device() -> Result<bool, Error> {
    need_game()?;
    if !AUDIO_OPEN.load(Ordering::SeqCst) {
        unsafe { ffi::InitAudioDevice() };
        AUDIO_OPEN.store(unsafe { ffi::IsAudioDeviceReady() }, Ordering::SeqCst);
    }
    Ok(AUDIO_OPEN.load(Ordering::SeqCst))
}

fn close_audio_device() -> Result<(), Error> {
    need_game()?;
    MUSIC.with_borrow_mut(|table| {
        for music in table.drain(..).flatten() {
            unsafe { ffi::UnloadMusicStream(music) };
        }
    });
    SOUNDS.with_borrow_mut(|table| {
        for sound in table.drain(..).flatten() {
            unsafe { ffi::UnloadSound(sound) };
        }
    });
    if AUDIO_OPEN.swap(false, Ordering::SeqCst) {
        unsafe { ffi::CloseAudioDevice() };
    }
    Ok(())
}

fn is_audio_device_ready() -> bool {
    AUDIO_OPEN.load(Ordering::SeqCst)
}

fn set_master_volume(volume: f32) -> Result<(), Error> {
    need_audio()?;
    unsafe { ffi::SetMasterVolume(volume) };
    Ok(())
}

fn load_sound(path: String) -> Result<i64, Error> {
    need_audio()?;
    let c_path = c_string(&path);
    let loaded = unsafe { ffi::LoadSound(c_path.as_ptr()) };
    if !unsafe { ffi::IsSoundValid(loaded) } {
        return Err(error(format!("could not load sound {path}")));
    }
    Ok(store(&SOUNDS, loaded))
}

fn unload_sound(value: Value) -> Result<bool, Error> {
    need_game()?;
    Ok(match take(&SOUNDS, value)? {
        Some(loaded) => {
            unsafe { ffi::UnloadSound(loaded) };
            true
        }
        None => false,
    })
}

fn play_sound(value: Value) -> Result<(), Error> {
    need_audio()?;
    let loaded = sound(value)?;
    unsafe { ffi::PlaySound(loaded) };
    Ok(())
}

fn stop_sound(value: Value) -> Result<(), Error> {
    need_audio()?;
    let loaded = sound(value)?;
    unsafe { ffi::StopSound(loaded) };
    Ok(())
}

fn is_sound_playing(value: Value) -> Result<bool, Error> {
    need_audio()?;
    let loaded = sound(value)?;
    Ok(unsafe { ffi::IsSoundPlaying(loaded) })
}

fn set_sound_volume(value: Value, volume: f32) -> Result<(), Error> {
    need_audio()?;
    let loaded = sound(value)?;
    unsafe { ffi::SetSoundVolume(loaded, volume) };
    Ok(())
}

fn load_music_stream(path: String) -> Result<i64, Error> {
    need_audio()?;
    let c_path = c_string(&path);
    let loaded = unsafe { ffi::LoadMusicStream(c_path.as_ptr()) };
    if !unsafe { ffi::IsMusicValid(loaded) } {
        return Err(error(format!("could not load music {path}")));
    }
    Ok(store(&MUSIC, loaded))
}

fn unload_music_stream(value: Value) -> Result<bool, Error> {
    need_game()?;
    Ok(match take(&MUSIC, value)? {
        Some(loaded) => {
            unsafe { ffi::UnloadMusicStream(loaded) };
            true
        }
        None => false,
    })
}

macro_rules! music_call {
    ($name:ident, $ffi:ident) => {
        fn $name(value: Value) -> Result<(), Error> {
            need_audio()?;
            let loaded = music(value)?;
            unsafe { ffi::$ffi(loaded) };
            Ok(())
        }
    };
}

music_call!(play_music_stream, PlayMusicStream);
music_call!(update_music_stream, UpdateMusicStream);
music_call!(stop_music_stream, StopMusicStream);
music_call!(pause_music_stream, PauseMusicStream);
music_call!(resume_music_stream, ResumeMusicStream);

fn is_music_stream_playing(value: Value) -> Result<bool, Error> {
    need_audio()?;
    let loaded = music(value)?;
    Ok(unsafe { ffi::IsMusicStreamPlaying(loaded) })
}

fn set_music_volume(value: Value, volume: f32) -> Result<(), Error> {
    need_audio()?;
    let loaded = music(value)?;
    unsafe { ffi::SetMusicVolume(loaded, volume) };
    Ok(())
}

fn get_music_time_length(value: Value) -> Result<f32, Error> {
    need_audio()?;
    let loaded = music(value)?;
    Ok(unsafe { ffi::GetMusicTimeLength(loaded) })
}

fn get_music_time_played(value: Value) -> Result<f32, Error> {
    need_audio()?;
    let loaded = music(value)?;
    Ok(unsafe { ffi::GetMusicTimePlayed(loaded) })
}

/// Defines the native `Raylib` functions, then the Ruby layer (structs,
/// constants, snake_case aliases, collision/vector helpers, Selenite::Game).
pub fn register(ruby: &Ruby) -> Result<(), Error> {
    let module: RModule = ruby.define_module("Raylib")?;
    macro_rules! def {
        ($name:literal, $func:expr, $arity:tt) => {
            module.define_module_function($name, function!($func, $arity))?;
        };
    }
    // window & timing
    def!("InitWindow", init_window, 3);
    def!("CloseWindow", close_window, 0);
    def!("WindowShouldClose", window_should_close, 0);
    def!("IsWindowReady", is_window_ready, 0);
    def!("GameMode?", game_mode, 0);
    def!("SetTargetFPS", set_target_fps, 1);
    def!("SetConfigFlags", set_config_flags, 1);
    def!("SetTraceLogLevel", set_trace_log_level, 1);
    def!("GetFrameTime", get_frame_time, 0);
    def!("GetTime", get_time, 0);
    def!("GetFPS", get_fps, 0);
    def!("GetScreenWidth", get_screen_width, 0);
    def!("GetScreenHeight", get_screen_height, 0);
    def!("SetWindowTitle", set_window_title, 1);
    def!("SetWindowSize", set_window_size, 2);
    def!("ToggleFullscreen", toggle_fullscreen, 0);
    def!("SetExitKey", set_exit_key, 1);
    def!("ShowCursor", show_cursor, 0);
    def!("HideCursor", hide_cursor, 0);
    def!("TakeScreenshot", take_screenshot, 1);
    def!("GetRandomValue", get_random_value, 2);
    def!("SetRandomSeed", set_random_seed, 1);
    // drawing
    def!("BeginDrawing", begin_drawing, 0);
    def!("EndDrawing", end_drawing, 0);
    def!("ClearBackground", clear_background, 1);
    def!("BeginMode2D", begin_mode_2d, 1);
    def!("EndMode2D", end_mode_2d, 0);
    def!("DrawPixel", draw_pixel, 3);
    def!("DrawLine", draw_line, 5);
    def!("DrawLineEx", draw_line_ex, 4);
    def!("DrawCircle", draw_circle, 4);
    def!("DrawCircleV", draw_circle_v, 3);
    def!("DrawCircleLines", draw_circle_lines, 4);
    def!("DrawEllipse", draw_ellipse, 5);
    def!("DrawRectangle", draw_rectangle, 5);
    def!("DrawRectangleV", draw_rectangle_v, 3);
    def!("DrawRectangleRec", draw_rectangle_rec, 2);
    def!("DrawRectangleLines", draw_rectangle_lines, 5);
    def!("DrawRectangleLinesEx", draw_rectangle_lines_ex, 3);
    def!("DrawRectangleRounded", draw_rectangle_rounded, 4);
    def!("DrawRectangleGradientV", draw_rectangle_gradient_v, 6);
    def!("DrawTriangle", draw_triangle, 4);
    def!("DrawPoly", draw_poly, 5);
    def!("DrawText", draw_text, 5);
    def!("MeasureText", measure_text, 2);
    def!("DrawFPS", draw_fps, 2);
    // textures
    def!("__LoadTexture", load_texture, 1);
    def!("UnloadTexture", unload_texture, 1);
    def!("DrawTexture", draw_texture, 4);
    def!("DrawTextureV", draw_texture_v, 3);
    def!("DrawTextureEx", draw_texture_ex, 5);
    def!("DrawTextureRec", draw_texture_rec, 4);
    def!("DrawTexturePro", draw_texture_pro, 6);
    // input
    def!("IsKeyPressed", is_key_pressed, 1);
    def!("IsKeyDown", is_key_down, 1);
    def!("IsKeyReleased", is_key_released, 1);
    def!("IsKeyUp", is_key_up, 1);
    def!("GetKeyPressed", get_key_pressed, 0);
    def!("GetCharPressed", get_char_pressed, 0);
    def!("IsMouseButtonPressed", is_mouse_button_pressed, 1);
    def!("IsMouseButtonDown", is_mouse_button_down, 1);
    def!("IsMouseButtonReleased", is_mouse_button_released, 1);
    def!("GetMouseX", get_mouse_x, 0);
    def!("GetMouseY", get_mouse_y, 0);
    def!("__GetMousePosition", get_mouse_position, 0);
    def!("GetMouseWheelMove", get_mouse_wheel_move, 0);
    def!("SetMousePosition", set_mouse_position, 2);
    // audio
    def!("InitAudioDevice", init_audio_device, 0);
    def!("CloseAudioDevice", close_audio_device, 0);
    def!("IsAudioDeviceReady", is_audio_device_ready, 0);
    def!("SetMasterVolume", set_master_volume, 1);
    def!("__LoadSound", load_sound, 1);
    def!("UnloadSound", unload_sound, 1);
    def!("PlaySound", play_sound, 1);
    def!("StopSound", stop_sound, 1);
    def!("IsSoundPlaying", is_sound_playing, 1);
    def!("SetSoundVolume", set_sound_volume, 2);
    def!("__LoadMusicStream", load_music_stream, 1);
    def!("UnloadMusicStream", unload_music_stream, 1);
    def!("PlayMusicStream", play_music_stream, 1);
    def!("UpdateMusicStream", update_music_stream, 1);
    def!("StopMusicStream", stop_music_stream, 1);
    def!("PauseMusicStream", pause_music_stream, 1);
    def!("ResumeMusicStream", resume_music_stream, 1);
    def!("IsMusicStreamPlaying", is_music_stream_playing, 1);
    def!("SetMusicVolume", set_music_volume, 2);
    def!("GetMusicTimeLength", get_music_time_length, 1);
    def!("GetMusicTimePlayed", get_music_time_played, 1);

    let _: Value = ruby.eval(RUBY_SIDE)?;
    Ok(())
}

use mic_freq::analysis::{self, Analysis, RhythmAnalysis, RhythmTracker};
use mic_freq::audio::MicCapture;
use mic_freq::mic_tools::MicTools;
use mic_freq::scripting::{ScriptInput, ScriptRuntime};
#[path = "../../src/portable.rs"]
#[allow(dead_code)]
mod portable;
use mic_freq::studio::{self, BlinkState, Settings};
use mic_freq::voice::{self, AvatarMotion, AvatarState, VoiceBand, VoiceBandTracker, VoiceState};
use raylib::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};

const WINDOW_SAMPLES: usize = 2048;
const MIC_CAPACITY: usize = 32_768;
const VOICE_WINDOW_SECONDS: f32 = 0.08;
const BASE_WINDOW_WIDTH: f32 = 1280.0;
const BASE_WINDOW_HEIGHT: f32 = 960.0;
const SIDE_PANEL_WIDTH: f32 = 340.0;
// Render at twice the width and height, then average four samples per pixel.
// This also smooths textures/text and avoids the driver's GLX MSAA crash.
const SUPERSAMPLE_SCALE: u32 = 2;
const IMAGE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "bmp", "gif", "qoi", "tga"];

fn image_file(dir: &Path, state: AvatarState) -> Result<Option<PathBuf>, Box<dyn Error>> {
    image_named(dir, state.file_stem())
}

fn capture_window_flag() -> WindowState {
    WindowState::default().set_window_undecorated(true)
}

fn set_capture_window(rl: &mut RaylibHandle, enabled: bool) {
    if rl.get_window_state().window_undecorated() == enabled {
        return;
    }
    if enabled {
        rl.set_window_state(capture_window_flag());
    } else {
        rl.clear_window_state(capture_window_flag());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CaptureTransition {
    Stable(bool),
    Pending,
}

impl CaptureTransition {
    fn request(current: bool, applied: bool, elapsed_since_apply: f32) -> Self {
        if current != applied && elapsed_since_apply < 0.25 {
            return Self::Pending;
        }
        Self::Stable(current)
    }
}

fn image_named(dir: &Path, stem: &str) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let files = match std::fs::read_dir(dir) {
        Ok(files) => files,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut found = None;
    for entry in files {
        let path = entry?.path();
        if !path.is_file()
            || !path
                .file_stem()
                .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(stem))
            || !path.extension().is_some_and(|e| {
                IMAGE_EXTENSIONS.contains(&e.to_string_lossy().to_ascii_lowercase().as_str())
            })
        {
            continue;
        }
        if found.replace(path).is_some() {
            return Err(format!(
                "multiple {} images in {} (keep only one)",
                stem,
                dir.display()
            )
            .into());
        }
    }
    Ok(found)
}

struct AvatarImageSet {
    poses: [Texture2D; 3],
    blinks: [Option<Texture2D>; 3],
}

struct FrequencyImageSet {
    poses: [Option<Texture2D>; 3],
    blinks: [Option<Texture2D>; 3],
}

struct AvatarImages {
    base: AvatarImageSet,
    frequencies: [FrequencyImageSet; 3],
}

impl AvatarImages {
    fn texture(&self, band: VoiceBand, state: AvatarState, closed: bool) -> &Texture2D {
        let state_index = state.index();
        if let Some(index) = band.index() {
            let profile = &self.frequencies[index];
            if profile.poses[state_index].is_some() {
                if closed {
                    return profile.blinks[state_index]
                        .as_ref()
                        .unwrap_or_else(|| profile.poses[state_index].as_ref().unwrap());
                }
                return profile.poses[state_index].as_ref().unwrap();
            }
        }
        if closed {
            self.base.blinks[state_index]
                .as_ref()
                .unwrap_or(&self.base.poses[state_index])
        } else {
            &self.base.poses[state_index]
        }
    }
}

fn placeholder(state: AvatarState, closed: bool) -> Image {
    let mut image = Image::gen_image_color(512, 512, Color::BLANK);
    image.draw_circle(256, 310, 155, state.placeholder_color());
    image.draw_circle(256, 210, 150, Color::RAYWHITE);
    for eye in [205, 307] {
        if closed {
            image.draw_rectangle(eye - 16, 194, 32, 6, Color::DARKGRAY);
        } else {
            image.draw_circle(eye, 194, 15, Color::DARKGRAY);
            image.draw_circle(eye + 4, 189, 4, Color::WHITE);
        }
    }
    match state {
        AvatarState::Idle => image.draw_rectangle(234, 250, 44, 6, Color::DARKGRAY),
        AvatarState::Talking => image.draw_circle(256, 258, 20, Color::DARKGRAY),
        AvatarState::Loud => image.draw_circle(256, 258, 34, Color::DARKGRAY),
    }
    image
}

fn texture_from_image(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    image: &Image,
) -> Result<Texture2D, Box<dyn Error>> {
    let mut texture = rl.load_texture_from_image(thread, image)?;
    texture.gen_texture_mipmaps();
    texture.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_TRILINEAR);
    Ok(texture)
}

fn load_images(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    dir: &Path,
) -> Result<AvatarImages, Box<dyn Error>> {
    let mut textures = Vec::new();
    let mut blinks = Vec::new();
    for state in AvatarState::ALL {
        let path = image_file(dir, state)?;
        let missing = path.is_none();
        let image = if let Some(path) = path {
            Image::load_image(&path.to_string_lossy())
                .map_err(|e| format!("could not load {}: {e}", path.display()))?
        } else {
            eprintln!(
                "No {} image in {}; using placeholder (add {}.png and press R to reload)",
                state.file_stem(),
                dir.display(),
                state.file_stem()
            );
            placeholder(state, false)
        };
        textures.push(texture_from_image(rl, thread, &image)?);
        let blink_image =
            if let Some(path) = image_named(dir, &format!("{}_blink", state.file_stem()))? {
                let blink = Image::load_image(&path.to_string_lossy())
                    .map_err(|error| format!("could not load {}: {error}", path.display()))?;
                if blink.width != image.width || blink.height != image.height {
                    return Err(
                        format!("{} must match its base pose dimensions", path.display()).into(),
                    );
                }
                Some(blink)
            } else if missing {
                Some(placeholder(state, true))
            } else {
                None
            };
        blinks.push(
            blink_image
                .as_ref()
                .map(|image| texture_from_image(rl, thread, image))
                .transpose()?,
        );
    }
    let base = AvatarImageSet {
        poses: textures.try_into().unwrap_or_else(|_| unreachable!()),
        blinks: blinks.try_into().unwrap_or_else(|_| unreachable!()),
    };
    let mut frequencies = Vec::new();
    for band in VoiceBand::ALL {
        let mut poses = Vec::new();
        let mut frequency_blinks = Vec::new();
        for state in AvatarState::ALL {
            let open_stem = format!("{}_{}", band.cli_name(), state.file_stem());
            let open = image_named(dir, &open_stem)?;
            let blink_stem = format!("{open_stem}_blink");
            let blink_path = image_named(dir, &blink_stem)?;
            if blink_path.is_some() && open.is_none() {
                return Err(format!(
                    "{} needs a matching {open_stem} image",
                    dir.join(blink_stem).display()
                )
                .into());
            }
            let open_image = if let Some(path) = &open {
                Some(
                    Image::load_image(&path.to_string_lossy())
                        .map_err(|error| format!("could not load {}: {error}", path.display()))?,
                )
            } else {
                None
            };
            let blink_image = if let Some(path) = &blink_path {
                Some(
                    Image::load_image(&path.to_string_lossy())
                        .map_err(|error| format!("could not load {}: {error}", path.display()))?,
                )
            } else {
                None
            };
            if let (Some(open), Some(blink)) = (&open_image, &blink_image) {
                if open.width != blink.width || open.height != blink.height {
                    return Err(format!(
                        "{} must match its base pose dimensions",
                        blink_path.as_ref().expect("blink path for image").display()
                    )
                    .into());
                }
            }
            poses.push(
                open_image
                    .as_ref()
                    .map(|image| texture_from_image(rl, thread, image))
                    .transpose()?,
            );
            frequency_blinks.push(
                blink_image
                    .as_ref()
                    .map(|image| texture_from_image(rl, thread, image))
                    .transpose()?,
            );
        }
        frequencies.push(FrequencyImageSet {
            poses: poses.try_into().unwrap_or_else(|_| unreachable!()),
            blinks: frequency_blinks
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
        });
    }
    Ok(AvatarImages {
        base,
        frequencies: frequencies.try_into().unwrap_or_else(|_| unreachable!()),
    })
}

fn assets_beside_executable(executable: &Path) -> Result<PathBuf, Box<dyn Error>> {
    Ok(executable
        .parent()
        .ok_or("executable has no parent directory")?
        .join("assets"))
}

fn resolve_assets(
    executable: &Path,
    explicit: Option<PathBuf>,
    saved: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn Error>> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    let adjacent = assets_beside_executable(executable)?;
    if adjacent.is_dir() {
        return Ok(adjacent);
    }
    Ok(saved.filter(|path| path.is_dir()).unwrap_or(adjacent))
}

fn ui_scale(width: f32, height: f32) -> f32 {
    (width.max(1.0) / BASE_WINDOW_WIDTH).min(height.max(1.0) / BASE_WINDOW_HEIGHT)
}

fn avatar_rect(width: f32, height: f32, texture_width: f32, texture_height: f32) -> Rectangle {
    let ui = ui_scale(width, height);
    let scale = ((width - 60.0 * ui).max(1.0) / texture_width)
        .min((height - 150.0 * ui).max(1.0) / texture_height);
    let (w, h) = (texture_width * scale, texture_height * scale);
    Rectangle::new((width - w) / 2.0, (height - h) / 2.0, w, h)
}

fn scene_target(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    width: u32,
    height: u32,
) -> Result<RenderTexture2D, Box<dyn Error>> {
    let target = rl.load_render_texture(thread, width, height)?;
    if !target.is_render_texture_valid() {
        return Err(format!("could not create {width}x{height} anti-aliasing buffer").into());
    }
    target
        .texture()
        .set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    Ok(target)
}

#[derive(Default)]
struct Options {
    assets: Option<PathBuf>,
    device: Option<String>,
    capture: bool,
    list_devices: bool,
    help: bool,
    version: bool,
    preset: Option<String>,
    save_preset: Option<String>,
    list_presets: bool,
    self_test: bool,
    artwork: bool,
    frequency: Option<VoiceBand>,
    script: Option<PathBuf>,
    check_script: Option<PathBuf>,
}

fn options(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Options, Box<dyn Error>> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => options.help = true,
            Some("--version" | "-V") => options.version = true,
            Some("--capture") => options.capture = true,
            Some("--list-devices") => options.list_devices = true,
            Some("--list-presets") => options.list_presets = true,
            Some("--self-test") => options.self_test = true,
            Some("--artwork") => options.artwork = true,
            Some("--script") => {
                options.script = Some(PathBuf::from(
                    args.next().ok_or("--script requires a Rhai script")?,
                ));
            }
            Some("--check-script") => {
                options.check_script = Some(PathBuf::from(
                    args.next().ok_or("--check-script requires a Rhai script")?,
                ));
            }
            Some("--frequency") => {
                let name = args
                    .next()
                    .ok_or("--frequency requires low, mid, or high")?
                    .to_string_lossy()
                    .into_owned();
                options.frequency = Some(
                    VoiceBand::from_cli_name(&name)
                        .ok_or(format!("unknown frequency profile: {name}"))?,
                );
            }
            Some("--preset" | "--save-preset") => {
                let name = args
                    .next()
                    .ok_or("preset option requires a name")?
                    .into_string()
                    .map_err(|_| "preset name must be UTF-8")?;
                if arg == "--preset" {
                    options.preset = Some(name);
                } else {
                    options.save_preset = Some(name);
                }
            }
            Some("--device") => {
                options.device = Some(
                    args.next()
                        .ok_or("--device requires an input name")?
                        .into_string()
                        .map_err(|_| "device name must be UTF-8")?,
                );
            }
            Some(flag) if flag.starts_with('-') => {
                return Err(format!("unknown option: {flag}").into());
            }
            _ if options.assets.is_none() => options.assets = Some(PathBuf::from(arg)),
            _ => return Err("only one avatar folder may be supplied".into()),
        }
    }
    Ok(options)
}

fn open_mic(settings: &Settings) -> Result<MicCapture, Box<dyn Error>> {
    MicCapture::start_device(MIC_CAPACITY, settings.device.as_deref())
}

#[derive(Default)]
struct PresetMenu {
    open: bool,
    name: String,
    names: Vec<String>,
    offset: usize,
    message: String,
}

#[derive(Default)]
struct HelpTab {
    open: bool,
    offset: usize,
}

impl HelpTab {
    const LINES: [&'static str; 23] = [
        "Selenite Audio Studio / mic_freq Microphone Suite",
        "",
        "The microphone chooses Idle, Talking, or Loud artwork. Pitch also selects",
        "the Low (<160 Hz), Mid (160-300 Hz), or High (>300 Hz) animation profile.",
        "",
        "Artwork",
        "Use Artwork to browse and replace the six global open/closed-eye poses.",
        "Each Low/Mid/High Edit window has the same image slots and controls.",
        "Supported files: PNG, JPG/JPEG, BMP, GIF, QOI and TGA.",
        "",
        "Controls",
        "C: PNGTuber box / studio controls. M: mute. R: reload artwork.",
        "F3: microphone tools and pitch tuner.",
        "Up/Down: gate. B: bounce. X: mirror. 0: auto. 1/2/3: force pose.",
        "Esc: close the open Help/Presets panel, or quit from the main studio.",
        "",
        "PNGTuber box",
        "Click PNGTuber at the bottom, or press C, to hide every control,",
        "meter, error, panel and the operating-system window frame. The remaining",
        "window is a clean avatar-only capture box.",
        "",
        "Press C or click the avatar box once to restore its frame and controls.",
        "In OBS, capture this window and chroma-key its green/magenta background.",
    ];

    fn update(
        &mut self,
        rl: &mut RaylibHandle,
        mouse: Vector2,
        clicked: bool,
        width: f32,
        height: f32,
    ) -> bool {
        if rl.is_key_pressed(KeyboardKey::KEY_F1) {
            self.open = !self.open;
        }
        if !self.open {
            return false;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.open = false;
            return false;
        }
        self.offset = (self.offset as isize - rl.get_mouse_wheel_move() as isize)
            .clamp(0, Self::LINES.len().saturating_sub(18) as isize) as usize;
        let bounds = Rectangle::new((width - 760.0) / 2.0, (height - 620.0) / 2.0, 760.0, 620.0);
        let launch = Rectangle::new(
            bounds.x + bounds.width - 212.0,
            bounds.y + bounds.height - 54.0,
            194.0,
            36.0,
        );
        if clicked && launch.check_collision_point_rec(mouse) {
            self.open = false;
            return true;
        }
        false
    }

    fn draw(&self, d: &mut impl RaylibDraw, width: f32, height: f32, mouse: Vector2) {
        if !self.open {
            return;
        }
        d.draw_rectangle(0, 0, width as i32, height as i32, Color::new(0, 0, 0, 185));
        let bounds = Rectangle::new((width - 760.0) / 2.0, (height - 620.0) / 2.0, 760.0, 620.0);
        d.draw_rectangle_rec(bounds, Color::new(28, 29, 35, 255));
        d.draw_text(
            "mic_freq Help",
            bounds.x as i32 + 18,
            bounds.y as i32 + 14,
            24,
            Color::RAYWHITE,
        );
        d.draw_text(
            "F1 or Esc closes · Scroll for more",
            bounds.x as i32 + 18,
            bounds.y as i32 + 48,
            16,
            Color::GRAY,
        );
        for (index, line) in Self::LINES.iter().skip(self.offset).take(18).enumerate() {
            d.draw_text(
                line,
                bounds.x as i32 + 18,
                bounds.y as i32 + 84 + index as i32 * 27,
                if index == 0 && self.offset == 0 {
                    19
                } else {
                    17
                },
                if line.ends_with("Controls")
                    || line.ends_with("Artwork")
                    || line.ends_with("PNGTuber box")
                {
                    Color::new(100, 220, 160, 255)
                } else {
                    Color::LIGHTGRAY
                },
            );
        }
        button(
            d,
            Rectangle::new(
                bounds.x + bounds.width - 212.0,
                bounds.y + bounds.height - 54.0,
                194.0,
                36.0,
            ),
            "Open PNGTuber box",
            false,
            mouse,
        );
    }
}

impl PresetMenu {
    fn bounds(width: f32, height: f32) -> Rectangle {
        Rectangle::new((width - 620.0) / 2.0, (height - 420.0) / 2.0, 620.0, 420.0)
    }

    fn update(
        &mut self,
        rl: &mut RaylibHandle,
        mouse: Vector2,
        bounds: Rectangle,
        config: &Path,
        settings: &mut Settings,
    ) -> bool {
        if rl.is_key_pressed(KeyboardKey::KEY_F2) {
            self.open = !self.open;
        }
        if !self.open {
            return false;
        }
        self.names = Settings::presets(config).unwrap_or_default();
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.open = false;
            return false;
        }
        let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_A) {
            self.name.clear();
        }
        while let Some(character) = rl.get_char_pressed() {
            if !ctrl && !character.is_control() && self.name.chars().count() < 64 {
                self.name.push(character);
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
            self.name.pop();
        }
        self.offset = (self.offset as isize - rl.get_mouse_wheel_move() as isize)
            .clamp(0, self.names.len().saturating_sub(6) as isize) as usize;
        let mut action = rl.is_key_pressed(KeyboardKey::KEY_ENTER).then_some(1);
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            for index in 0..3 {
                let rect = Rectangle::new(
                    bounds.x + 16.0 + index as f32 * 196.0,
                    bounds.y + 88.0,
                    188.0,
                    34.0,
                );
                if rect.check_collision_point_rec(mouse) {
                    action = Some(index);
                }
            }
            for (slot, name) in self.names.iter().skip(self.offset).take(6).enumerate() {
                let rect = Rectangle::new(
                    bounds.x + 16.0,
                    bounds.y + 136.0 + slot as f32 * 36.0,
                    bounds.width - 32.0,
                    34.0,
                );
                if rect.check_collision_point_rec(mouse) {
                    self.name = name.clone();
                }
            }
        }
        match action {
            Some(0) => match settings.save_preset(config, &self.name) {
                Ok(()) => self.message = format!("Saved {}", self.name),
                Err(error) => self.message = error.to_string(),
            },
            Some(1) => match settings.load_preset(config, &self.name) {
                Ok(()) => {
                    self.message = format!("Loaded {}", self.name);
                    return true;
                }
                Err(error) => self.message = error.to_string(),
            },
            Some(2) => self.open = false,
            _ => {}
        }
        false
    }

    fn draw(
        &self,
        d: &mut impl RaylibDraw,
        bounds: Rectangle,
        width: f32,
        height: f32,
        mouse: Vector2,
    ) {
        if !self.open {
            return;
        }
        d.draw_rectangle(0, 0, width as i32, height as i32, Color::new(0, 0, 0, 170));
        d.draw_rectangle_rec(bounds, Color::new(28, 29, 35, 255));
        d.draw_text(
            "Voice Presets",
            bounds.x as i32 + 16,
            bounds.y as i32 + 12,
            22,
            Color::RAYWHITE,
        );
        let field = Rectangle::new(bounds.x + 16.0, bounds.y + 44.0, bounds.width - 32.0, 32.0);
        d.draw_rectangle_rec(field, Color::new(40, 42, 48, 255));
        let name: String = self
            .name
            .chars()
            .skip(self.name.chars().count().saturating_sub(48))
            .collect();
        d.draw_text(
            &name,
            field.x as i32 + 8,
            field.y as i32 + 6,
            18,
            Color::RAYWHITE,
        );
        for (index, label) in ["Save", "Load", "Close"].into_iter().enumerate() {
            button(
                d,
                Rectangle::new(
                    bounds.x + 16.0 + index as f32 * 196.0,
                    bounds.y + 88.0,
                    188.0,
                    34.0,
                ),
                label,
                false,
                mouse,
            );
        }
        for (slot, name) in self.names.iter().skip(self.offset).take(6).enumerate() {
            let label: String = name.chars().take(48).collect();
            button(
                d,
                Rectangle::new(
                    bounds.x + 16.0,
                    bounds.y + 136.0 + slot as f32 * 36.0,
                    bounds.width - 32.0,
                    34.0,
                ),
                &label,
                *name == self.name,
                mouse,
            );
        }
        let message: String = self.message.chars().take(64).collect();
        d.draw_text(
            &message,
            bounds.x as i32 + 16,
            (bounds.y + bounds.height - 32.0) as i32,
            16,
            Color::GOLD,
        );
    }
}

fn control_rect(panel: f32, top: f32) -> Rectangle {
    Rectangle::new(panel, top, 298.0, 34.0)
}

fn frequency_slider_rect(panel: f32, top: f32) -> Rectangle {
    Rectangle::new(panel, top, 238.0, 34.0)
}

fn slider_amount(bounds: Rectangle, x: f32) -> f32 {
    ((x - bounds.x) / bounds.width).clamp(0.0, 1.0)
}

fn range_value(amount: f32, min: f32, max: f32) -> f32 {
    min + amount * (max - min)
}

fn range_amount(value: f32, min: f32, max: f32) -> f32 {
    (value - min) / (max - min)
}

fn calibrated_gate(peak_db: f32) -> f32 {
    (peak_db + 6.0).clamp(studio::GATE_MIN_DB, studio::GATE_MAX_DB)
}

fn apply_microphone_recommendation(
    settings: &mut Settings,
    recommendation: mic_freq::mic_tools::Recommendation,
) {
    settings.gate_db = recommendation.gate_db;
    settings.loud_delta_db = recommendation.loud_delta_db;
    settings.low_voice_gain_db = 0.0;
    settings.mid_voice_gain_db = 0.0;
    settings.high_voice_gain_db = 0.0;
}

fn frequency_button_rect(panel: f32, top: f32) -> Rectangle {
    Rectangle::new(panel + 246.0, top, 52.0, 34.0)
}

fn button(d: &mut impl RaylibDraw, rect: Rectangle, text: &str, selected: bool, mouse: Vector2) {
    let hover = rect.check_collision_point_rec(mouse);
    d.draw_rectangle_rec(
        rect,
        if selected {
            Color::new(40, 111, 85, 255)
        } else if hover {
            Color::new(65, 67, 72, 255)
        } else {
            Color::new(44, 46, 51, 255)
        },
    );
    d.draw_text(
        text,
        rect.x as i32 + 10,
        rect.y as i32 + 8,
        18,
        Color::RAYWHITE,
    );
}

fn toggle(d: &mut impl RaylibDraw, rect: Rectangle, text: &str, selected: bool) {
    d.draw_rectangle_lines(rect.x as i32, rect.y as i32 + 6, 22, 22, Color::GRAY);
    if selected {
        d.draw_rectangle(
            rect.x as i32 + 5,
            rect.y as i32 + 11,
            12,
            12,
            Color::new(100, 220, 160, 255),
        );
    }
    d.draw_text(
        text,
        rect.x as i32 + 34,
        rect.y as i32 + 8,
        18,
        Color::RAYWHITE,
    );
}

fn slider(d: &mut impl RaylibDraw, rect: Rectangle, amount: f32) {
    d.draw_rectangle(
        rect.x as i32,
        rect.y as i32 + 14,
        rect.width as i32,
        6,
        Color::new(65, 67, 72, 255),
    );
    let position = rect.x + amount.clamp(0.0, 1.0) * rect.width;
    d.draw_rectangle(
        rect.x as i32,
        rect.y as i32 + 14,
        (position - rect.x) as i32,
        6,
        Color::new(100, 220, 160, 255),
    );
    d.draw_circle(position as i32, rect.y as i32 + 17, 7.0, Color::RAYWHITE);
}

fn framed_avatar(width: f32, height: f32, tw: f32, th: f32, scale: f32, bounce: f32) -> Rectangle {
    let mut rect = avatar_rect(width, height, tw, th);
    let center = Vector2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    rect.width *= scale;
    rect.height *= scale;
    rect.x = center.x - rect.width / 2.0;
    rect.y = center.y - rect.height / 2.0 + bounce * ui_scale(width, height);
    rect
}

fn draw_live_analysis(
    d: &mut impl RaylibDraw,
    x: f32,
    y: f32,
    width: f32,
    analysis: &Analysis,
    rhythm: &RhythmAnalysis,
) {
    d.draw_rectangle(
        x as i32,
        y as i32,
        width as i32,
        104,
        Color::new(18, 22, 28, 220),
    );
    let pitch = analysis
        .midi_note
        .map(mic_freq::composer::note_name)
        .map_or_else(
            || "—".into(),
            |name| {
                format!(
                    "{name}  {:.1} Hz  {:+.0} cents",
                    analysis.fundamental_hz.unwrap_or(0.0),
                    analysis.cents
                )
            },
        );
    d.draw_text(
        "SIGNAL ANALYSIS",
        x as i32 + 10,
        y as i32 + 8,
        16,
        Color::RAYWHITE,
    );
    d.draw_text(
        &pitch,
        x as i32 + 10,
        y as i32 + 28,
        15,
        Color::new(100, 220, 160, 255),
    );
    d.draw_text(
        &format!(
            "Level {:.0} dB  Harmonic {:.0}%  Tonal {:.0}%  BPM {}",
            analysis.rms_db,
            analysis.harmonicity * 100.0,
            analysis.tonal_ratio * 100.0,
            rhythm
                .bpm
                .map_or_else(|| "—".into(), |bpm| format!("{bpm:.0}"))
        ),
        x as i32 + 10,
        y as i32 + 48,
        13,
        Color::LIGHTGRAY,
    );
    let bar_width = ((width - 20.0) / 8.0 - 3.0).max(2.0);
    for (index, amplitude) in analysis.harmonic_amplitudes.iter().enumerate() {
        let height = (*amplitude * 24.0).clamp(1.0, 24.0);
        d.draw_rectangle(
            (x + 10.0 + index as f32 * (bar_width + 3.0)) as i32,
            (y + 94.0 - height) as i32,
            bar_width as i32,
            height as i32,
            Color::new(73, 202, 178, 255),
        );
    }
}

fn audio_studio_self_test() -> Result<(), Box<dyn Error>> {
    use mic_freq::composer::{self, Note, Project, Scale, Step, Waveform};
    let mut project = Project::default();
    project.tracks[0].instrument.waveform = Waveform::Square;
    project.tracks[0].patterns[0].steps[0] = Step::Notes {
        notes: vec![Note {
            midi: 69,
            velocity: 1.0,
            duration_steps: 1,
        }],
    };
    project.validate()?;
    if Project::from_json(&project.to_json()?)? != project {
        return Err("project JSON self-test changed data".into());
    }
    if composer::scale_notes(60, Scale::Major, 1) != [60, 62, 64, 65, 67, 69, 71] {
        return Err("music-theory self-test failed".into());
    }
    let audio = composer::render(&project)?;
    let mono = analysis::downmix(&audio)?;
    let signal =
        analysis::try_analyze_samples(&mono[..mono.len().min(8_192)], audio.sample_rate as f32)?;
    if signal.midi_note != Some(69) {
        return Err(format!("pitch self-test expected A4, got {:?}", signal.midi_note).into());
    }
    let directory = std::env::temp_dir().join(format!("mic-freq-self-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let wav = directory.join("self-test.wav");
    composer::export_wav(&audio, &wav)?;
    let imported = composer::import_wav(&wav)?;
    std::fs::remove_dir_all(&directory)?;
    if imported.sample_rate != audio.sample_rate
        || imported.channels != audio.channels
        || imported.samples.len() != audio.samples.len()
    {
        return Err("WAV round-trip self-test changed the audio shape".into());
    }
    let mut rhythm = analysis::RhythmTracker::try_new(composer::SAMPLE_RATE as f32)?;
    let click = (0..composer::SAMPLE_RATE as usize / 2)
        .map(|index| {
            if index < 32 {
                1.0 - index as f32 / 32.0
            } else {
                0.0
            }
        })
        .collect::<Vec<_>>();
    for _ in 0..8 {
        rhythm.update(&click);
    }
    let mut runtime = ScriptRuntime::from_source(
        "fn update(input) { let n = 0; while n < 10 { n += 1; } #{ pose: \"auto\" } }",
    )?;
    runtime.update(ScriptInput::default())?;
    println!(
        "Selenite mic_freq Microphone Suite self-test: OK (A4 {:.2} Hz, WAV, theory, render, analysis)",
        signal.fundamental_hz.unwrap_or(0.0)
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    portable::initialize().map_err(std::io::Error::other)?;
    let options = options(std::env::args_os().skip(1))?;
    if options.help {
        println!(
            "mic_freq [avatar-folder] [--device NAME] [--capture] [--list-devices]\n\
            C: PNGTuber box  M: mute  0/1/2/3: auto/idle/talking/loud\n\
            F3: microphone tools and pitch tuner\n\
            Up/Down: gate  B: bounce  X: mirror  R: reload  F1: help  F2: presets  Esc: quit\n\
            --artwork opens the browsable global pose editor\n\
            --frequency low|mid|high opens that pitch-animation editor\n\
            --script FILE controls live poses and presentation with Rhai\n\
            --check-script FILE validates a Rhai script without microphone or display\n\
            --self-test validates synthesis, media, scripting, theory and analysis offline\n\
            --preset NAME  --save-preset NAME  --list-presets\n\
            --version prints the mic_freq package version\n\
            Drop an avatar folder or pose image to switch sets. Settings save automatically."
        );
        return Ok(());
    }
    if options.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if options.self_test {
        return audio_studio_self_test();
    }
    if let Some(path) = options.check_script.as_ref() {
        let mut runtime = ScriptRuntime::load(path)?;
        runtime.update(ScriptInput::default())?;
        println!("mic_freq script OK: {}", path.display());
        return Ok(());
    }
    if options.list_devices {
        for device in MicCapture::devices()? {
            println!("{device}");
        }
        return Ok(());
    }
    let config_path = studio::settings_path();
    let mut error = None;
    let mut settings = Settings::load(&config_path).unwrap_or_else(|failure| {
        error = Some(format!("Settings: {failure}"));
        Settings::default()
    });
    if options.list_presets {
        for name in Settings::presets(&config_path)? {
            println!("{name}");
        }
        return Ok(());
    }
    if let Some(name) = options.preset {
        settings.load_preset(&config_path, &name)?;
    }
    if let Some(name) = options.save_preset {
        settings.save_preset(&config_path, &name)?;
        println!("Saved preset: {name}");
        return Ok(());
    }
    if let Some(device) = options.device {
        settings.device = Some(device);
    }
    let mut asset_dir = resolve_assets(
        &std::env::current_exe()?,
        options.assets,
        settings.assets.clone(),
    )?;
    asset_dir = std::fs::canonicalize(&asset_dir).unwrap_or(asset_dir);
    settings.assets = Some(asset_dir.clone());
    println!("Avatar images: {}", asset_dir.display());
    if options.artwork {
        return mic_freq::frequency_ui::run_avatar(asset_dir);
    }
    if let Some(band) = options.frequency {
        return mic_freq::frequency_ui::run(band, asset_dir, &config_path);
    }
    let mut script_runtime = options
        .script
        .as_ref()
        .map(|path| ScriptRuntime::load(path))
        .transpose()?;
    let mut mic = open_mic(&settings)
        .map_err(|failure| {
            error = Some(format!("Microphone: {failure}"));
        })
        .ok();

    let (mut rl, thread) = raylib::init()
        .size(BASE_WINDOW_WIDTH as i32, BASE_WINDOW_HEIGHT as i32)
        .title("mic_freq - Selenite Microphone Suite")
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_window_min_size(960, 720);
    rl.set_exit_key(None);
    rl.get_font_default()
        .texture()
        .set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    let mut images = load_images(&mut rl, &thread, &asset_dir)?;
    let mut scene: Option<RenderTexture2D> = None;
    let mut voice = VoiceState::default();
    voice.gate_db = settings.gate_db;
    let mut motion = AvatarMotion::default();
    let mut blink = BlinkState::default();
    let mut capture = options.capture;
    let mut muted = false;
    let mut forced: Option<AvatarState> = None;
    let mut calibration: Option<(f32, Option<f32>)> = None;
    let mut save_delay: Option<f32> = None;
    let mut active_slider = None;
    let mut samples = Vec::with_capacity(MIC_CAPACITY);
    let mut rhythm_samples = Vec::with_capacity(WINDOW_SAMPLES);
    let mut live_analysis = Analysis::default();
    let mut rhythm: Option<RhythmTracker> = mic
        .as_ref()
        .and_then(|input| RhythmTracker::try_new(input.sample_rate).ok());
    let mut rhythm_sequence = 0;
    let mut rhythm_analysis = RhythmAnalysis::default();
    let mut analysis_delay = 0.0;
    let mut pitch_delay = 0.0;
    let mut voice_pitch = None;
    let mut analysis_visible = true;
    let mut mic_tools = MicTools::default();
    let mut presets = PresetMenu::default();
    let mut help = HelpTab::default();
    let mut frequency_children = Vec::<std::process::Child>::new();
    let mut voice_band_tracker = VoiceBandTracker::default();
    let mut script_frame = 0_u64;
    let mut suite_elapsed = 0.0_f32;
    set_capture_window(&mut rl, capture);
    let mut applied_capture = capture;
    let mut capture_apply_elapsed = 0.25;

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        suite_elapsed += dt;
        capture_apply_elapsed = (capture_apply_elapsed + dt).min(0.25);
        let (width, height) = (rl.get_screen_width().max(1), rl.get_screen_height().max(1));
        let (w, h) = (width as f32, height as f32);
        let ui = ui_scale(w, h);
        let panel = w / ui - SIDE_PANEL_WIDTH + 22.0;
        let position = rl.get_mouse_position();
        let mouse = Vector2::new(position.x / ui, position.y / ui);
        let mouse_clicked = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let presets_were_open = presets.open;
        let help_was_open = help.open;
        let preset_button = Rectangle::new(panel, 56.0, 94.0, 28.0);
        let artwork_button = Rectangle::new(panel + 100.0, 56.0, 132.0, 28.0);
        let help_button = Rectangle::new(panel + 238.0, 56.0, 60.0, 28.0);
        let analysis_button = Rectangle::new(panel, 916.0, 94.0, 34.0);
        let tools_button = Rectangle::new(panel + 98.0, 916.0, 96.0, 34.0);
        let capture_button = Rectangle::new(panel + 198.0, 916.0, 100.0, 34.0);
        let mut frequency_editor_finished = false;
        frequency_children.retain_mut(|child| match child.try_wait() {
            Ok(Some(_)) => {
                frequency_editor_finished = true;
                false
            }
            Ok(None) => true,
            Err(failure) => {
                error = Some(format!("Frequency editor: {failure}"));
                false
            }
        });
        if frequency_editor_finished {
            let mut reload_failed = false;
            match Settings::load(&config_path) {
                Ok(updated) => settings = updated,
                Err(failure) => {
                    error = Some(format!("Settings reload failed: {failure}"));
                    reload_failed = true;
                }
            }
            match load_images(&mut rl, &thread, &asset_dir) {
                Ok(updated) => images = updated,
                Err(failure) => {
                    error = Some(format!("Frequency artwork reload failed: {failure}"));
                    reload_failed = true;
                }
            }
            if !reload_failed {
                error = None;
            }
            voice.gate_db = settings.gate_db;
            voice.loud_delta_db = settings.loud_delta_db;
        }
        if !capture && mouse_clicked && preset_button.check_collision_point_rec(mouse) {
            presets.open = true;
            help.open = false;
        }
        if !capture && mouse_clicked && help_button.check_collision_point_rec(mouse) {
            help.open = true;
            presets.open = false;
        }
        if !capture && mouse_clicked && artwork_button.check_collision_point_rec(mouse) {
            match std::process::Command::new(std::env::current_exe()?)
                .arg("--artwork")
                .arg(&asset_dir)
                .spawn()
            {
                Ok(child) => frequency_children.push(child),
                Err(failure) => error = Some(format!("Could not open Artwork editor: {failure}")),
            }
        }
        let mut changed = presets.update(
            &mut rl,
            mouse,
            PresetMenu::bounds(w / ui, h / ui),
            &config_path,
            &mut settings,
        );
        let open_capture_from_help = help.update(&mut rl, mouse, mouse_clicked, w / ui, h / ui);
        let preset_active = presets.open || presets_were_open;
        let help_active = help.open || help_was_open;
        if !preset_active && !help_active && rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            break;
        }
        if presets.open {
            capture = false;
            active_slider = None;
        }
        if help.open {
            capture = false;
            active_slider = None;
        }
        if open_capture_from_help {
            capture = true;
        }
        let clicked = !capture && mouse_clicked;
        let hit = |top| clicked && control_rect(panel, top).check_collision_point_rec(mouse);
        if !preset_active && !help_active {
            if rl.is_key_pressed(KeyboardKey::KEY_C)
                || (!capture && clicked && capture_button.check_collision_point_rec(mouse))
                || (capture && mouse_clicked)
            {
                capture = !capture;
            }
            if !capture && clicked && analysis_button.check_collision_point_rec(mouse) {
                if mic_tools.open {
                    mic_tools.open = false;
                    analysis_visible = true;
                } else {
                    analysis_visible = !analysis_visible;
                }
            }
            if !capture
                && (rl.is_key_pressed(KeyboardKey::KEY_F3)
                    || (clicked && tools_button.check_collision_point_rec(mouse)))
            {
                mic_tools.open = !mic_tools.open;
            }
            if !capture && mic_tools.open && clicked {
                if let Some(recommendation) = mic_tools.click(mouse) {
                    apply_microphone_recommendation(&mut settings, recommendation);
                    changed = true;
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_M) || hit(605.0) {
                muted = !muted;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_X) || hit(645.0) {
                settings.mirror = !settings.mirror;
                changed = true;
            }
            if hit(685.0) {
                settings.blink = !settings.blink;
                changed = true;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_B) {
                settings.bounce = if settings.bounce > 0.0 { 0.0 } else { 1.0 };
                changed = true;
            }
            for (index, key) in [
                KeyboardKey::KEY_ZERO,
                KeyboardKey::KEY_ONE,
                KeyboardKey::KEY_TWO,
                KeyboardKey::KEY_THREE,
            ]
            .into_iter()
            .enumerate()
            {
                let rect = Rectangle::new(panel + index as f32 * 74.0, 130.0, 72.0, 34.0);
                if rl.is_key_pressed(key) || (clicked && rect.check_collision_point_rec(mouse)) {
                    forced = index.checked_sub(1).map(|index| AvatarState::ALL[index]);
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_UP) {
                settings.gate_db = (settings.gate_db + 2.0).min(studio::GATE_MAX_DB);
                changed = true;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
                settings.gate_db = (settings.gate_db - 2.0).max(studio::GATE_MIN_DB);
                changed = true;
            }
            for (index, top) in [246.0, 306.0, 354.0, 402.0, 450.0, 504.0, 557.0]
                .into_iter()
                .enumerate()
            {
                let bounds = if (1..=3).contains(&index) {
                    frequency_slider_rect(panel, top)
                } else {
                    control_rect(panel, top)
                };
                if clicked && bounds.check_collision_point_rec(mouse) {
                    active_slider = Some(index);
                }
                if (1..=3).contains(&index)
                    && clicked
                    && frequency_button_rect(panel, top).check_collision_point_rec(mouse)
                {
                    let band = VoiceBand::ALL[index - 1];
                    match std::process::Command::new(std::env::current_exe()?)
                        .arg("--frequency")
                        .arg(band.cli_name())
                        .arg(&asset_dir)
                        .spawn()
                    {
                        Ok(child) => frequency_children.push(child),
                        Err(failure) => {
                            error = Some(format!(
                                "Could not open {} frequency editor: {failure}",
                                band.label()
                            ));
                        }
                    }
                    active_slider = None;
                }
            }
            if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) || capture {
                active_slider = None;
            }
            if let Some(index) = active_slider {
                let bounds = if (1..=3).contains(&index) {
                    frequency_slider_rect(panel, 0.0)
                } else {
                    control_rect(panel, 0.0)
                };
                let amount = slider_amount(bounds, mouse.x);
                match index {
                    0 => {
                        settings.gate_db =
                            range_value(amount, studio::GATE_MIN_DB, studio::GATE_MAX_DB)
                    }
                    1 => settings.low_voice_gain_db = -18.0 + amount * 36.0,
                    2 => settings.mid_voice_gain_db = -18.0 + amount * 36.0,
                    3 => settings.high_voice_gain_db = -18.0 + amount * 36.0,
                    4 => {
                        settings.loud_delta_db = range_value(
                            amount,
                            studio::LOUD_OFFSET_MIN_DB,
                            studio::LOUD_OFFSET_MAX_DB,
                        )
                    }
                    5 => settings.bounce = amount * 2.0,
                    6 => settings.scale = 0.2 + amount * 0.8,
                    _ => unreachable!(),
                }
                changed = true;
            }
            for index in 0..3 {
                let rect = Rectangle::new(panel + index as f32 * 101.0, 757.0, 96.0, 30.0);
                if clicked && rect.check_collision_point_rec(mouse) {
                    settings.background = index;
                    changed = true;
                }
            }
            if hit(831.0) {
                match MicCapture::devices() {
                    Ok(devices) => {
                        let next = settings
                            .device
                            .as_ref()
                            .and_then(|name| devices.iter().position(|device| device == name))
                            .map_or(0, |index| index + 1);
                        let device = devices.get(next).cloned();
                        let mut candidate = settings.clone();
                        candidate.device = device;
                        drop(mic.take());
                        match open_mic(&candidate) {
                            Ok(input) => {
                                mic = Some(input);
                                rhythm = mic.as_ref().and_then(|input| {
                                    RhythmTracker::try_new(input.sample_rate).ok()
                                });
                                rhythm_sequence = 0;
                                live_analysis = Analysis::default();
                                rhythm_analysis = RhythmAnalysis::default();
                                settings = candidate;
                                error = None;
                                changed = true;
                            }
                            Err(failure) => {
                                error = Some(format!("Microphone: {failure}"));
                                mic = open_mic(&settings).ok();
                                rhythm = mic.as_ref().and_then(|input| {
                                    RhythmTracker::try_new(input.sample_rate).ok()
                                });
                                rhythm_sequence = 0;
                                live_analysis = Analysis::default();
                                rhythm_analysis = RhythmAnalysis::default();
                            }
                        }
                        voice = VoiceState::default();
                        voice_band_tracker.reset();
                        voice_pitch = None;
                        pitch_delay = 0.0;
                    }
                    Err(failure) => error = Some(format!("Devices: {failure}")),
                }
            }
            if hit(871.0) && mic.as_ref().is_some_and(|mic| !mic.has_failed()) {
                calibration = Some((0.0, None));
            }
            let mut reload = rl.is_key_pressed(KeyboardKey::KEY_R);
            let mut new_dir = None;
            if rl.is_file_dropped() {
                let files = rl.load_dropped_files();
                if files.paths().len() == 1 {
                    let path = PathBuf::from(files.paths()[0]);
                    new_dir = if path.is_dir() {
                        Some(path)
                    } else {
                        path.parent().map(Path::to_path_buf)
                    };
                    reload = true;
                } else {
                    error = Some("Drop one avatar folder or pose image at a time".into());
                }
            }
            if reload {
                let candidate = new_dir.as_ref().unwrap_or(&asset_dir);
                match load_images(&mut rl, &thread, candidate) {
                    Ok(new_images) => {
                        images = new_images;
                        asset_dir =
                            std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.clone());
                        settings.assets = Some(asset_dir.clone());
                        changed = true;
                        error = None;
                    }
                    Err(e) => {
                        eprintln!("reload failed: {e}");
                        error = Some(format!("Reload failed: {e}"));
                    }
                }
            }
        }

        samples.clear();
        rhythm_samples.clear();
        if let Some(input) = &mic {
            if !input.has_failed() {
                let window = (input.sample_rate * VOICE_WINDOW_SECONDS).ceil() as usize;
                input.latest(window.min(MIC_CAPACITY), &mut samples);
                input.since(&mut rhythm_sequence, &mut rhythm_samples);
                if let Some(tracker) = &mut rhythm {
                    rhythm_analysis = tracker.update(&rhythm_samples);
                }
            }
        }
        let level_samples = &samples[samples.len().saturating_sub(WINDOW_SAMPLES)..];
        pitch_delay -= dt;
        if pitch_delay <= 0.0 {
            let sample_rate = mic.as_ref().map_or(44_100.0, |input| input.sample_rate);
            match analysis::try_voice_pitch(&samples, sample_rate) {
                Ok(pitch) => voice_pitch = pitch,
                Err(failure) => {
                    voice_pitch = None;
                    error = Some(format!("Voice pitch analysis: {failure}"));
                }
            }
            pitch_delay = 0.05;
        }
        analysis_delay -= dt;
        if analysis_delay <= 0.0 {
            let sample_rate = mic.as_ref().map_or(44_100.0, |input| input.sample_rate);
            live_analysis = analysis::analyze_samples(level_samples, sample_rate);
            analysis_delay = 0.1;
        }
        let db = voice::amplitude_to_db(voice::rms(level_samples));
        if let Some((elapsed, peak)) = calibration.as_mut() {
            *elapsed += dt;
            if !samples.is_empty() {
                *peak = Some(peak.map_or(db, |previous| previous.max(db)));
            }
            if *elapsed >= 3.0 {
                if let Some(peak) = *peak {
                    settings.gate_db = calibrated_gate(peak);
                    changed = true;
                } else {
                    error = Some("Noise gate calibration failed: no microphone samples".into());
                }
                calibration = None;
            }
        }
        let pitch = if muted || samples.is_empty() {
            voice_band_tracker.reset();
            None
        } else {
            voice_pitch
        };
        let voice_band = voice_band_tracker.update(pitch, dt);
        let voice_gain = voice_band.gain_db(
            settings.low_voice_gain_db,
            settings.mid_voice_gain_db,
            settings.high_voice_gain_db,
        );
        let mut script_controls = mic_freq::scripting::ScriptControls::default();
        if let Some(runtime) = script_runtime.as_mut() {
            match runtime.update(ScriptInput {
                frame: script_frame,
                time: suite_elapsed,
                dt,
                db,
                adjusted_db: db + voice_gain,
                pitch_hz: voice_pitch,
                band: voice_band,
                pose: voice.current(),
                muted,
                gate_db: settings.gate_db,
                loud_delta_db: settings.loud_delta_db,
            }) {
                Ok(controls) => script_controls = controls,
                Err(failure) => {
                    error = Some(format!("Script disabled: {failure}"));
                    script_runtime = None;
                }
            }
        }
        script_frame = script_frame.saturating_add(1);
        voice.gate_db = script_controls.gate_db.unwrap_or(settings.gate_db);
        voice.loud_delta_db = script_controls
            .loud_delta_db
            .unwrap_or(settings.loud_delta_db);
        let effective_gain = script_controls.gain_db.unwrap_or(voice_gain);
        let voice_db = db + effective_gain;
        if !capture {
            if let Err(failure) = mic_tools.update(
                dt,
                &samples,
                &rhythm_samples,
                mic.as_ref().map_or(44_100.0, |input| input.sample_rate),
                0.0,
            ) {
                error = Some(format!("Microphone tools: {failure}"));
            }
        }
        let detected = voice.update(if muted { -180.0 } else { voice_db }, dt);
        let tracked_voice_band = voice_band_tracker.active_for_state(detected, muted);
        let active_voice_band = if muted {
            VoiceBand::Unknown
        } else {
            script_controls.band.unwrap_or(tracked_voice_band)
        };
        let state = if muted {
            AvatarState::Idle
        } else {
            forced.or(script_controls.pose).unwrap_or(detected)
        };
        let frequency_profile = settings
            .frequency_profile(active_voice_band)
            .filter(|profile| profile.enabled);
        let bounce = script_controls
            .bounce
            .unwrap_or_else(|| frequency_profile.map_or(settings.bounce, |profile| profile.bounce));
        let avatar_scale = script_controls
            .scale
            .unwrap_or_else(|| frequency_profile.map_or(settings.scale, |profile| profile.scale));
        let mirror = script_controls
            .mirror
            .unwrap_or_else(|| frequency_profile.map_or(settings.mirror, |profile| profile.mirror));
        let blinking = script_controls
            .blink
            .unwrap_or_else(|| frequency_profile.map_or(settings.blink, |profile| profile.blink));
        let offset = motion.update(state, dt, bounce);
        let closed = blink.update(dt, blinking);
        if changed {
            save_delay = Some(0.5);
        }
        if let Some(delay) = save_delay.as_mut() {
            *delay -= dt;
            if *delay <= 0.0 {
                if let Err(failure) = settings.save(&config_path) {
                    error = Some(format!("Save failed: {failure}"));
                }
                save_delay = None;
            }
        }
        let (render_width, render_height) = (
            width as u32 * SUPERSAMPLE_SCALE,
            height as u32 * SUPERSAMPLE_SCALE,
        );
        if scene.as_ref().is_none_or(|target| {
            target.texture().width() != render_width as i32
                || target.texture().height() != render_height as i32
        }) {
            scene = Some(scene_target(&mut rl, &thread, render_width, render_height)?);
        }
        let scene = scene.as_mut().expect("scene target initialized above");
        let texture = images.texture(active_voice_band, state, closed);
        let (tw, th) = (texture.width() as f32, texture.height() as f32);
        let canvas_width = if capture {
            w
        } else {
            w - SIDE_PANEL_WIDTH * ui
        };
        let avatar = framed_avatar(canvas_width, h, tw, th, avatar_scale, offset);
        let backgrounds = [
            Color::new(0, 255, 0, 255),
            Color::new(255, 0, 255, 255),
            Color::new(18, 18, 24, 255),
        ];
        {
            let mut target = rl.begin_texture_mode(&thread, scene);
            let mut d = target.begin_mode2D(Camera2D {
                offset: Vector2::zero(),
                target: Vector2::zero(),
                rotation: 0.0,
                zoom: SUPERSAMPLE_SCALE as f32,
            });
            d.clear_background(backgrounds[settings.background]);
            d.draw_texture_pro(
                texture,
                Rectangle::new(0.0, 0.0, if mirror { -tw } else { tw }, th),
                avatar,
                Vector2::zero(),
                0.0,
                Color::WHITE,
            );
        }
        if !capture {
            let mut target = rl.begin_texture_mode(&thread, scene);
            let mut d = target.begin_mode2D(Camera2D {
                offset: Vector2::zero(),
                target: Vector2::zero(),
                rotation: 0.0,
                zoom: ui * SUPERSAMPLE_SCALE as f32,
            });
            d.draw_rectangle(
                (panel - 22.0) as i32,
                0,
                SIDE_PANEL_WIDTH as i32,
                (h / ui).ceil() as i32,
                Color::new(25, 27, 31, 255),
            );
            d.draw_text("Mic Frequency Suite", panel as i32, 24, 24, Color::RAYWHITE);
            button(&mut d, preset_button, "Presets", presets.open, mouse);
            button(&mut d, artwork_button, "Artwork", false, mouse);
            button(&mut d, help_button, "Help", help.open, mouse);
            d.draw_text(
                &format!(
                    "{} · {}{}",
                    state.label(),
                    if active_voice_band == VoiceBand::Unknown {
                        "Global"
                    } else {
                        active_voice_band.label()
                    },
                    if active_voice_band == VoiceBand::Unknown || voice_gain == 0.0 {
                        String::new()
                    } else {
                        format!(" {voice_gain:+.0} dB")
                    }
                ),
                panel as i32,
                94,
                20,
                Color::new(100, 220, 160, 255),
            );
            for (index, label) in ["Auto", "Idle", "Talk", "Loud"].into_iter().enumerate() {
                let selected = forced.map_or(0, |state| state.index() + 1) == index;
                button(
                    &mut d,
                    Rectangle::new(panel + index as f32 * 74.0, 130.0, 72.0, 34.0),
                    label,
                    selected,
                    mouse,
                );
            }
            d.draw_text(
                &format!(
                    "Input  {db:.0} dBFS  ·  Pitch  {}",
                    voice_pitch.map_or("—".to_owned(), |frequency| format!("{frequency:.0} Hz"))
                ),
                panel as i32,
                180,
                18,
                Color::LIGHTGRAY,
            );
            slider(
                &mut d,
                control_rect(panel, 194.0),
                range_amount(voice_db, studio::GATE_MIN_DB, 18.0),
            );
            for (index, (top, label, amount)) in [
                (
                    246.0,
                    format!("Noise gate  {:.0} dB", settings.gate_db),
                    range_amount(settings.gate_db, studio::GATE_MIN_DB, studio::GATE_MAX_DB),
                ),
                (
                    306.0,
                    format!("Low voice gain  {:+.0} dB", settings.low_voice_gain_db),
                    (settings.low_voice_gain_db + 18.0) / 36.0,
                ),
                (
                    354.0,
                    format!("Mid voice gain  {:+.0} dB", settings.mid_voice_gain_db),
                    (settings.mid_voice_gain_db + 18.0) / 36.0,
                ),
                (
                    402.0,
                    format!("High voice gain  {:+.0} dB", settings.high_voice_gain_db),
                    (settings.high_voice_gain_db + 18.0) / 36.0,
                ),
                (
                    450.0,
                    format!(
                        "Loud threshold  {:.0} dB",
                        settings.gate_db + settings.loud_delta_db
                    ),
                    range_amount(
                        settings.loud_delta_db,
                        studio::LOUD_OFFSET_MIN_DB,
                        studio::LOUD_OFFSET_MAX_DB,
                    ),
                ),
                (
                    504.0,
                    format!("Bounce  {:.0}%", settings.bounce * 100.0),
                    settings.bounce / 2.0,
                ),
                (
                    557.0,
                    format!("Avatar size  {:.0}%", settings.scale * 100.0),
                    (settings.scale - 0.2) / 0.8,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                d.draw_text(&label, panel as i32, top as i32 - 22, 18, Color::LIGHTGRAY);
                if (1..=3).contains(&index) {
                    slider(&mut d, frequency_slider_rect(panel, top), amount);
                    button(
                        &mut d,
                        frequency_button_rect(panel, top),
                        "Edit",
                        false,
                        mouse,
                    );
                } else {
                    slider(&mut d, control_rect(panel, top), amount);
                }
            }
            toggle(&mut d, control_rect(panel, 605.0), "Mute microphone", muted);
            toggle(
                &mut d,
                control_rect(panel, 645.0),
                "Mirror avatar",
                settings.mirror,
            );
            toggle(&mut d, control_rect(panel, 685.0), "Blink", settings.blink);
            d.draw_text("Background", panel as i32, 735, 18, Color::LIGHTGRAY);
            for (index, color) in backgrounds.into_iter().enumerate() {
                let rect = Rectangle::new(panel + index as f32 * 101.0, 757.0, 96.0, 30.0);
                d.draw_rectangle_rec(rect, color);
                if settings.background == index {
                    d.draw_rectangle_lines_ex(rect, 3.0, Color::RAYWHITE);
                }
            }
            d.draw_text("Microphone", panel as i32, 800, 18, Color::LIGHTGRAY);
            let input_name = mic.as_ref().map_or("Unavailable", |input| {
                if input.has_failed() {
                    "Disconnected"
                } else {
                    &input.device_name
                }
            });
            let short_name: String = input_name.chars().take(30).collect();
            d.draw_text(&short_name, panel as i32, 819, 16, Color::GRAY);
            button(
                &mut d,
                control_rect(panel, 831.0),
                "Next input / reconnect",
                false,
                mouse,
            );
            button(
                &mut d,
                control_rect(panel, 871.0),
                if calibration.is_some() {
                    "Measuring room noise..."
                } else {
                    "Calibrate noise gate"
                },
                calibration.is_some(),
                mouse,
            );
            button(&mut d, analysis_button, "Analyzer", analysis_visible, mouse);
            button(&mut d, tools_button, "Mic tools", mic_tools.open, mouse);
            button(&mut d, capture_button, "PNGTuber", false, mouse);
            if analysis_visible && !mic_tools.open {
                draw_live_analysis(
                    &mut d,
                    18.0,
                    18.0,
                    (canvas_width / ui - 36.0).min(390.0),
                    &live_analysis,
                    &rhythm_analysis,
                );
            }
            if mic_tools.open {
                mic_tools.draw(
                    &mut d,
                    &samples,
                    if samples.is_empty() {
                        None
                    } else {
                        voice_pitch
                    },
                    mic.as_ref().map(|input| {
                        (
                            input.device_name.as_str(),
                            input.sample_rate,
                            input.has_failed(),
                        )
                    }),
                );
            }
            if let Some(message) = &error {
                let max_chars = ((canvas_width / ui - 32.0) / 10.0).max(1.0) as usize;
                let chars: Vec<char> = message.chars().collect();
                for (index, line) in chars.chunks(max_chars).take(3).enumerate() {
                    let line: String = line.iter().collect();
                    d.draw_text(
                        &line,
                        16,
                        (h / ui) as i32 - 78 + index as i32 * 22,
                        18,
                        Color::RED,
                    );
                }
            }
            presets.draw(
                &mut d,
                PresetMenu::bounds(w / ui, h / ui),
                w / ui,
                h / ui,
                mouse,
            );
            help.draw(&mut d, w / ui, h / ui, mouse);
        }
        {
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::BLACK);
            // Render textures are vertically flipped in OpenGL.
            d.draw_texture_pro(
                scene.texture(),
                Rectangle::new(0.0, 0.0, render_width as f32, -(render_height as f32)),
                Rectangle::new(0.0, 0.0, w, h),
                Vector2::zero(),
                0.0,
                Color::WHITE,
            );
        }
        let transition =
            CaptureTransition::request(capture, applied_capture, capture_apply_elapsed);
        if let CaptureTransition::Stable(target) = transition {
            if target != applied_capture {
                set_capture_window(&mut rl, target);
                applied_capture = target;
                capture_apply_elapsed = 0.0;
            }
        }
    }
    if let Err(failure) = settings.save(&config_path) {
        eprintln!("Could not save settings: {failure}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_gain_slider_reaches_its_full_range() {
        let bounds = frequency_slider_rect(100.0, 306.0);
        assert_eq!(slider_amount(bounds, bounds.x), 0.0);
        assert_eq!(slider_amount(bounds, bounds.x + bounds.width / 2.0), 0.5);
        assert_eq!(slider_amount(bounds, bounds.x + bounds.width), 1.0);
        let global = control_rect(100.0, 246.0);
        assert_eq!(slider_amount(global, global.x + global.width), 1.0);
    }

    #[test]
    fn expanded_threshold_controls_and_calibration_reach_both_ends() {
        for (min, max) in [
            (studio::GATE_MIN_DB, studio::GATE_MAX_DB),
            (studio::LOUD_OFFSET_MIN_DB, studio::LOUD_OFFSET_MAX_DB),
        ] {
            assert_eq!(range_value(0.0, min, max), min);
            assert_eq!(range_value(1.0, min, max), max);
            assert_eq!(range_amount(min, min, max), 0.0);
            assert_eq!(range_amount(max, min, max), 1.0);
        }
        assert_eq!(calibrated_gate(-150.0), -120.0);
        assert_eq!(calibrated_gate(-110.0), -104.0);
        assert_eq!(calibrated_gate(-3.0), 0.0);
    }

    #[test]
    fn applying_microphone_calibration_restores_neutral_band_gains() {
        let mut settings = Settings {
            low_voice_gain_db: -18.0,
            mid_voice_gain_db: 9.0,
            high_voice_gain_db: 18.0,
            ..Default::default()
        };
        apply_microphone_recommendation(
            &mut settings,
            mic_freq::mic_tools::Recommendation {
                gate_db: -48.0,
                loud_delta_db: 26.0,
            },
        );
        assert_eq!(settings.gate_db, -48.0);
        assert_eq!(settings.loud_delta_db, 26.0);
        assert_eq!(settings.low_voice_gain_db, 0.0);
        assert_eq!(settings.mid_voice_gain_db, 0.0);
        assert_eq!(settings.high_voice_gain_db, 0.0);
    }

    #[test]
    fn live_voice_switches_bands_and_returns_to_global_when_quiet() {
        let sample_rate = 96_000.0;
        let mut tracker = VoiceBandTracker::default();
        let mut voice = VoiceState::default();
        for (frequency, expected) in [
            (100.0, VoiceBand::Low),
            (220.0, VoiceBand::Mid),
            (400.0, VoiceBand::High),
        ] {
            let samples: Vec<f32> = (0..(sample_rate * VOICE_WINDOW_SECONDS) as usize)
                .map(|index| {
                    let phase = std::f32::consts::TAU * frequency * index as f32 / sample_rate;
                    0.25 * phase.sin() + 0.15 * (phase * 2.0).sin()
                })
                .collect();
            let pitch = analysis::try_voice_pitch(&samples, sample_rate).unwrap();
            for _ in 0..10 {
                let band = tracker.update(pitch, 0.05);
                let state = voice.update(
                    voice::amplitude_to_db(voice::rms(&samples)) + band.gain_db(3.0, -2.0, 5.0),
                    0.05,
                );
                assert_eq!(tracker.active_for_state(state, false), expected);
            }
        }
        for _ in 0..20 {
            tracker.update(None, 0.05);
            let state = voice.update(-90.0, 0.05);
            tracker.active_for_state(state, false);
        }
        assert_eq!(
            tracker.active_for_state(AvatarState::Idle, false),
            VoiceBand::Unknown
        );
    }

    #[test]
    fn launch_options_validate_arguments() {
        let parsed =
            options(["/tmp/avatar", "--device", "USB mic", "--capture"].map(Into::into)).unwrap();
        assert_eq!(parsed.assets.unwrap(), PathBuf::from("/tmp/avatar"));
        assert_eq!(parsed.device.as_deref(), Some("USB mic"));
        assert!(parsed.capture);
        let artwork = options(["--artwork", "/tmp/avatar"].map(Into::into)).unwrap();
        assert!(artwork.artwork);
        assert_eq!(artwork.assets, Some(PathBuf::from("/tmp/avatar")));
        assert!(options(["--self-test"].map(Into::into)).unwrap().self_test);
        assert!(options(["--version"].map(Into::into)).unwrap().version);
        assert!(capture_window_flag().window_undecorated());
        assert_eq!(
            CaptureTransition::request(true, false, 0.1),
            CaptureTransition::Pending
        );
        assert_eq!(
            CaptureTransition::request(true, false, 0.25),
            CaptureTransition::Stable(true)
        );
        assert!(options(["--composer"].map(Into::into)).is_err());
        assert_eq!(
            options(["--frequency", "MID"].map(Into::into))
                .unwrap()
                .frequency,
            Some(VoiceBand::Mid)
        );
        assert!(options(["--frequency", "noise"].map(Into::into)).is_err());
        assert!(options(["--device"].map(Into::into)).is_err());
        assert_eq!(
            options(["--script", "voice.rhai"].map(Into::into))
                .unwrap()
                .script,
            Some(PathBuf::from("voice.rhai"))
        );
        assert_eq!(
            options(["--check-script", "voice.rhai"].map(Into::into))
                .unwrap()
                .check_script,
            Some(PathBuf::from("voice.rhai"))
        );
        assert!(options(["--script"].map(Into::into)).is_err());
        assert!(options(["--unknown"].map(Into::into)).is_err());
        assert!(options(["one", "two"].map(Into::into)).is_err());
    }

    #[test]
    fn maximum_bounce_and_scaling_stay_inside_canvas() {
        for (width, height) in [(600.0, 900.0), (1920.0, 1080.0), (240.0, 180.0)] {
            for (tw, th) in [(512.0, 512.0), (256.0, 32000.0)] {
                for scale in [0.2, 1.0] {
                    let rect = framed_avatar(width, height, tw, th, scale, -60.0);
                    assert!(rect.x >= 0.0 && rect.y >= 0.0);
                    assert!(rect.x + rect.width <= width && rect.y + rect.height <= height);
                }
            }
        }
    }

    #[test]
    fn layout_scales_proportionally_when_window_grows() {
        assert_eq!(ui_scale(BASE_WINDOW_WIDTH, BASE_WINDOW_HEIGHT), 1.0);
        assert_eq!(
            ui_scale(BASE_WINDOW_WIDTH * 2.0, BASE_WINDOW_HEIGHT * 2.0),
            2.0
        );
        assert_eq!(
            ui_scale(BASE_WINDOW_WIDTH / 2.0, BASE_WINDOW_HEIGHT / 2.0),
            0.5
        );
        assert_eq!(ui_scale(960.0, 720.0), 0.75);
        assert!(916.0 + 34.0 <= BASE_WINDOW_HEIGHT);
        let normal = avatar_rect(BASE_WINDOW_WIDTH, BASE_WINDOW_HEIGHT, 512.0, 512.0);
        let large = avatar_rect(
            BASE_WINDOW_WIDTH * 2.0,
            BASE_WINDOW_HEIGHT * 2.0,
            512.0,
            512.0,
        );
        assert_eq!(normal, Rectangle::new(235.0, 75.0, 810.0, 810.0));
        assert_eq!(large, Rectangle::new(470.0, 150.0, 1620.0, 1620.0));
    }

    #[test]
    fn avatar_stays_centered_and_uncropped_at_different_aspect_ratios() {
        for (width, height) in [(4986.0, 2742.0), (700.0, 1400.0), (240.0, 180.0)] {
            for (tw, th) in [(512.0, 512.0), (1024.0, 256.0), (256.0, 32000.0)] {
                let rect = avatar_rect(width, height, tw, th);
                assert!(rect.x >= 0.0 && rect.y >= 0.0);
                assert!(rect.width <= width && rect.height <= height);
                assert!((rect.x + rect.width / 2.0 - width / 2.0).abs() < 0.001);
                assert!((rect.y + rect.height / 2.0 - height / 2.0).abs() < 0.001);
                assert!((rect.width / rect.height - tw / th).abs() < 0.001);
            }
        }
    }

    #[test]
    fn copied_executable_prefers_adjacent_assets_over_saved_folder() {
        let dir =
            std::env::temp_dir().join(format!("mic-freq-asset-routing-{}", std::process::id()));
        let adjacent = dir.join("copied/assets");
        let saved = dir.join("original/assets");
        std::fs::create_dir_all(&adjacent).unwrap();
        std::fs::create_dir_all(&saved).unwrap();
        let executable = dir.join("copied/mic_freq");
        assert_eq!(
            resolve_assets(&executable, None, Some(saved.clone())).unwrap(),
            adjacent
        );
        let explicit = dir.join("custom");
        assert_eq!(
            resolve_assets(&executable, Some(explicit.clone()), Some(saved.clone())).unwrap(),
            explicit
        );
        std::fs::remove_dir(&adjacent).unwrap();
        assert_eq!(
            resolve_assets(&executable, None, Some(saved.clone())).unwrap(),
            saved
        );
        std::fs::remove_dir(&saved).unwrap();
        assert_eq!(
            resolve_assets(&executable, None, Some(saved)).unwrap(),
            adjacent
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn default_assets_are_beside_executable() {
        assert_eq!(
            assets_beside_executable(Path::new("/app/target/release/mic_freq")).unwrap(),
            Path::new("/app/target/release/assets")
        );
    }

    #[test]
    fn finds_images_by_state_and_rejects_duplicates() {
        let dir = std::env::temp_dir().join(format!("mic-freq-images-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("IDLE.PNG"), []).unwrap();
        std::fs::write(dir.join("talking.jpg"), []).unwrap();
        std::fs::write(dir.join("low_idle.qoi"), []).unwrap();
        std::fs::write(dir.join("high_talking_blink.tga"), []).unwrap();
        assert_eq!(
            image_file(&dir, AvatarState::Idle).unwrap(),
            Some(dir.join("IDLE.PNG"))
        );
        assert_eq!(
            image_file(&dir, AvatarState::Talking).unwrap(),
            Some(dir.join("talking.jpg"))
        );
        assert!(image_file(&dir, AvatarState::Loud).unwrap().is_none());
        assert_eq!(
            image_named(&dir, "low_idle").unwrap(),
            Some(dir.join("low_idle.qoi"))
        );
        assert_eq!(
            image_named(&dir, "high_talking_blink").unwrap(),
            Some(dir.join("high_talking_blink.tga"))
        );
        std::fs::write(dir.join("idle.jpg"), []).unwrap();
        assert!(image_file(&dir, AvatarState::Idle).is_err());
        for file in [
            "IDLE.PNG",
            "talking.jpg",
            "idle.jpg",
            "low_idle.qoi",
            "high_talking_blink.tga",
        ] {
            std::fs::remove_file(dir.join(file)).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }
}

use mic_freq::audio::MicCapture;
use mic_freq::studio::{self, BlinkState, Settings};
use mic_freq::voice::{self, AvatarMotion, AvatarState, VoiceState};
use raylib::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};

const WINDOW_SAMPLES: usize = 2048;
const BASE_WINDOW_SIZE: f32 = 900.0;
// Render at twice the width and height, then average four samples per pixel.
// This also smooths textures/text and avoids the driver's GLX MSAA crash.
const SUPERSAMPLE_SCALE: u32 = 2;
const IMAGE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "bmp", "gif", "qoi", "tga"];

fn image_file(dir: &Path, state: AvatarState) -> Result<Option<PathBuf>, Box<dyn Error>> {
    image_named(dir, state.file_stem())
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

struct AvatarImages {
    poses: [Texture2D; 3],
    blinks: [Option<Texture2D>; 3],
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
    Ok(AvatarImages {
        poses: textures.try_into().unwrap_or_else(|_| unreachable!()),
        blinks: blinks.try_into().unwrap_or_else(|_| unreachable!()),
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
    width.max(1.0).min(height.max(1.0)) / BASE_WINDOW_SIZE
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
}

fn options(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Options, Box<dyn Error>> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => options.help = true,
            Some("--capture") => options.capture = true,
            Some("--list-devices") => options.list_devices = true,
            Some("--device") => {
                options.device = Some(
                    args.next()
                        .ok_or("--device requires an input name")?
                        .into_string()
                        .map_err(|_| "device name must be UTF-8")?,
                );
            }
            Some(flag) if flag.starts_with('-') => {
                return Err(format!("unknown option: {flag}").into())
            }
            _ if options.assets.is_none() => options.assets = Some(PathBuf::from(arg)),
            _ => return Err("only one avatar folder may be supplied".into()),
        }
    }
    Ok(options)
}

fn open_mic(settings: &Settings) -> Result<MicCapture, Box<dyn Error>> {
    MicCapture::start_device(WINDOW_SAMPLES * 2, settings.device.as_deref())
}

fn control_rect(panel: f32, top: f32) -> Rectangle {
    Rectangle::new(panel, top, 258.0, 34.0)
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

fn main() -> Result<(), Box<dyn Error>> {
    let options = options(std::env::args_os().skip(1))?;
    if options.help {
        println!(
            "mic_freq [avatar-folder] [--device NAME] [--capture] [--list-devices]\n\
            C: clean capture  M: mute  0/1/2/3: auto/idle/talking/loud\n\
            Up/Down: gate  B: bounce  X: mirror  R: reload  Esc: quit\n\
            Drop an avatar folder or pose image to switch sets. Settings save automatically."
        );
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
    let mut mic = open_mic(&settings)
        .map_err(|failure| {
            error = Some(format!("Microphone: {failure}"));
        })
        .ok();

    let (mut rl, thread) = raylib::init()
        .size(900, 900)
        .title("mic_freq - PNGTuber Studio")
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_window_min_size(720, 600);
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
    let mut calibration: Option<(f32, f32)> = None;
    let mut save_delay: Option<f32> = None;
    let mut active_slider = None;
    let mut samples = Vec::with_capacity(WINDOW_SAMPLES);

    while !rl.window_should_close() && !rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
        let dt = rl.get_frame_time();
        let (width, height) = (rl.get_screen_width().max(1), rl.get_screen_height().max(1));
        let (w, h) = (width as f32, height as f32);
        let ui = ui_scale(w, h);
        let panel = w / ui - 278.0;
        let position = rl.get_mouse_position();
        let mouse = Vector2::new(position.x / ui, position.y / ui);
        let clicked = !capture && rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let hit = |top| clicked && control_rect(panel, top).check_collision_point_rec(mouse);
        let mut changed = false;
        if rl.is_key_pressed(KeyboardKey::KEY_C) || hit(748.0) {
            capture = !capture;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_M) || hit(410.0) {
            muted = !muted;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_X) || hit(450.0) {
            settings.mirror = !settings.mirror;
            changed = true;
        }
        if hit(490.0) {
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
            let rect = Rectangle::new(panel + index as f32 * 65.0, 130.0, 63.0, 34.0);
            if rl.is_key_pressed(key) || (clicked && rect.check_collision_point_rec(mouse)) {
                forced = index.checked_sub(1).map(|index| AvatarState::ALL[index]);
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            settings.gate_db = (settings.gate_db + 2.0).min(-26.0);
            changed = true;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            settings.gate_db = (settings.gate_db - 2.0).max(-90.0);
            changed = true;
        }
        for (index, top) in [246.0, 309.0, 372.0, 830.0].into_iter().enumerate() {
            if hit(top) {
                active_slider = Some(index);
            }
        }
        if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) || capture {
            active_slider = None;
        }
        if let Some(index) = active_slider {
            let amount = ((mouse.x - panel) / 258.0).clamp(0.0, 1.0);
            match index {
                0 => settings.gate_db = -90.0 + amount * 64.0,
                1 => settings.bounce = amount * 2.0,
                2 => settings.scale = 0.2 + amount * 0.8,
                _ => settings.loud_delta_db = 8.0 + amount * 28.0,
            }
            changed = true;
        }
        for index in 0..3 {
            let rect = Rectangle::new(panel + index as f32 * 86.0, 554.0, 76.0, 30.0);
            if clicked && rect.check_collision_point_rec(mouse) {
                settings.background = index;
                changed = true;
            }
        }
        if hit(657.0) {
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
                            settings = candidate;
                            error = None;
                            changed = true;
                        }
                        Err(failure) => {
                            error = Some(format!("Microphone: {failure}"));
                            mic = open_mic(&settings).ok();
                        }
                    }
                    voice = VoiceState::default();
                }
                Err(failure) => error = Some(format!("Devices: {failure}")),
            }
        }
        if hit(704.0) && mic.as_ref().is_some_and(|mic| !mic.has_failed()) {
            calibration = Some((0.0, -90.0));
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

        samples.clear();
        if let Some(input) = &mic {
            if !input.has_failed() {
                input.latest(WINDOW_SAMPLES, &mut samples);
            }
        }
        let db = voice::amplitude_to_db(voice::rms(&samples));
        if let Some((elapsed, peak)) = calibration.as_mut() {
            *elapsed += dt;
            if !samples.is_empty() {
                *peak = peak.max(db);
            }
            if *elapsed >= 3.0 {
                settings.gate_db = (*peak + 6.0).clamp(-90.0, -26.0);
                calibration = None;
                changed = true;
            }
        }
        voice.gate_db = settings.gate_db;
        voice.loud_delta_db = settings.loud_delta_db;
        let detected = voice.update(if muted { -180.0 } else { db }, dt);
        let state = if muted {
            AvatarState::Idle
        } else {
            forced.unwrap_or(detected)
        };
        let offset = motion.update(state, dt, settings.bounce);
        let closed = blink.update(dt, settings.blink);
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
        let texture = if closed {
            images.blinks[state.index()]
                .as_ref()
                .unwrap_or(&images.poses[state.index()])
        } else {
            &images.poses[state.index()]
        };
        let (tw, th) = (texture.width() as f32, texture.height() as f32);
        let canvas_width = if capture { w } else { w - 300.0 * ui };
        let avatar = framed_avatar(canvas_width, h, tw, th, settings.scale, offset);
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
                Rectangle::new(0.0, 0.0, if settings.mirror { -tw } else { tw }, th),
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
                300,
                (h / ui).ceil() as i32,
                Color::new(25, 27, 31, 255),
            );
            d.draw_text("PNGTuber Studio", panel as i32, 24, 24, Color::RAYWHITE);
            d.draw_text("mic_freq", panel as i32, 58, 16, Color::GRAY);
            d.draw_text(
                state.label(),
                panel as i32,
                94,
                20,
                Color::new(100, 220, 160, 255),
            );
            for (index, label) in ["Auto", "Idle", "Talk", "Loud"].into_iter().enumerate() {
                let selected = forced.map_or(0, |state| state.index() + 1) == index;
                button(
                    &mut d,
                    Rectangle::new(panel + index as f32 * 65.0, 130.0, 63.0, 34.0),
                    label,
                    selected,
                    mouse,
                );
            }
            d.draw_text(
                &format!("Input  {db:.0} dBFS"),
                panel as i32,
                180,
                18,
                Color::LIGHTGRAY,
            );
            slider(&mut d, control_rect(panel, 194.0), (db + 90.0) / 90.0);
            for (top, label, amount) in [
                (
                    246.0,
                    format!("Noise gate  {:.0} dB", settings.gate_db),
                    (settings.gate_db + 90.0) / 64.0,
                ),
                (
                    309.0,
                    format!("Bounce  {:.0}%", settings.bounce * 100.0),
                    settings.bounce / 2.0,
                ),
                (
                    372.0,
                    format!("Avatar size  {:.0}%", settings.scale * 100.0),
                    (settings.scale - 0.2) / 0.8,
                ),
                (
                    830.0,
                    format!(
                        "Loud threshold  {:.0} dB",
                        settings.gate_db + settings.loud_delta_db
                    ),
                    (settings.loud_delta_db - 8.0) / 28.0,
                ),
            ] {
                d.draw_text(&label, panel as i32, top as i32 - 22, 18, Color::LIGHTGRAY);
                slider(&mut d, control_rect(panel, top), amount);
            }
            toggle(&mut d, control_rect(panel, 410.0), "Mute microphone", muted);
            toggle(
                &mut d,
                control_rect(panel, 450.0),
                "Mirror avatar",
                settings.mirror,
            );
            toggle(&mut d, control_rect(panel, 490.0), "Blink", settings.blink);
            d.draw_text("Background", panel as i32, 532, 18, Color::LIGHTGRAY);
            for (index, color) in backgrounds.into_iter().enumerate() {
                let rect = Rectangle::new(panel + index as f32 * 86.0, 554.0, 76.0, 30.0);
                d.draw_rectangle_rec(rect, color);
                if settings.background == index {
                    d.draw_rectangle_lines_ex(rect, 3.0, Color::RAYWHITE);
                }
            }
            d.draw_text("Microphone", panel as i32, 598, 18, Color::LIGHTGRAY);
            let input_name = mic.as_ref().map_or("Unavailable", |input| {
                if input.has_failed() {
                    "Disconnected"
                } else {
                    &input.device_name
                }
            });
            let short_name: String = input_name.chars().take(23).collect();
            d.draw_text(&short_name, panel as i32, 626, 18, Color::GRAY);
            button(
                &mut d,
                control_rect(panel, 657.0),
                "Next input / reconnect",
                false,
                mouse,
            );
            button(
                &mut d,
                control_rect(panel, 704.0),
                if calibration.is_some() {
                    "Measuring room noise..."
                } else {
                    "Calibrate noise gate"
                },
                calibration.is_some(),
                mouse,
            );
            button(
                &mut d,
                control_rect(panel, 748.0),
                "Clean capture",
                false,
                mouse,
            );
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
        }
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
    if let Err(failure) = settings.save(&config_path) {
        eprintln!("Could not save settings: {failure}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_options_validate_arguments() {
        let parsed =
            options(["/tmp/avatar", "--device", "USB mic", "--capture"].map(Into::into)).unwrap();
        assert_eq!(parsed.assets.unwrap(), PathBuf::from("/tmp/avatar"));
        assert_eq!(parsed.device.as_deref(), Some("USB mic"));
        assert!(parsed.capture);
        assert!(options(["--device"].map(Into::into)).is_err());
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
        assert_eq!(ui_scale(900.0, 900.0), 1.0);
        assert_eq!(ui_scale(1800.0, 1800.0), 2.0);
        assert_eq!(ui_scale(450.0, 450.0), 0.5);
        let normal = avatar_rect(900.0, 900.0, 512.0, 512.0);
        let large = avatar_rect(1800.0, 1800.0, 512.0, 512.0);
        assert_eq!(normal, Rectangle::new(75.0, 75.0, 750.0, 750.0));
        assert_eq!(large, Rectangle::new(150.0, 150.0, 1500.0, 1500.0));
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
        assert_eq!(
            image_file(&dir, AvatarState::Idle).unwrap(),
            Some(dir.join("IDLE.PNG"))
        );
        assert_eq!(
            image_file(&dir, AvatarState::Talking).unwrap(),
            Some(dir.join("talking.jpg"))
        );
        assert!(image_file(&dir, AvatarState::Loud).unwrap().is_none());
        std::fs::write(dir.join("idle.jpg"), []).unwrap();
        assert!(image_file(&dir, AvatarState::Idle).is_err());
        for file in ["IDLE.PNG", "talking.jpg", "idle.jpg"] {
            std::fs::remove_file(dir.join(file)).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }
}

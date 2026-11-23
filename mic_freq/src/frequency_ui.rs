use crate::studio::{FrequencyProfile, Settings};
use crate::voice::{AvatarState, VoiceBand};
use raylib::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};

const WIDTH: i32 = 900;
const HEIGHT: i32 = 760;
const IMAGE_EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "bmp", "gif", "qoi", "tga"];
const CONTROL_TOPS: [f32; 3] = [110.0, 176.0, 242.0];

#[derive(Default)]
struct ImageBrowser {
    open: bool,
    directory: PathBuf,
    entries: Vec<(PathBuf, bool)>,
    offset: usize,
    message: String,
}

impl ImageBrowser {
    fn open_at(&mut self, directory: &Path) {
        self.open = true;
        self.directory = directory.to_path_buf();
        self.refresh();
    }

    fn refresh(&mut self) {
        self.offset = 0;
        let entries = std::fs::read_dir(&self.directory).and_then(|entries| {
            entries
                .map(|entry| {
                    entry.and_then(|entry| {
                        let path = entry.path();
                        let is_directory = entry.file_type()?.is_dir();
                        Ok((path, is_directory))
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        });
        match entries {
            Ok(mut entries) => {
                entries.retain(|(path, is_directory)| {
                    *is_directory
                        || path
                            .extension()
                            .and_then(|value| value.to_str())
                            .is_some_and(|value| {
                                IMAGE_EXTENSIONS.contains(&value.to_ascii_lowercase().as_str())
                            })
                });
                entries.sort_by_key(|(path, is_directory)| {
                    (
                        !*is_directory,
                        path.file_name()
                            .map(|name| name.to_string_lossy().to_ascii_lowercase())
                            .unwrap_or_default(),
                    )
                });
                self.entries = entries;
                self.message.clear();
            }
            Err(error) => {
                self.entries.clear();
                self.message = error.to_string();
            }
        }
    }

    fn update(&mut self, rl: &mut RaylibHandle, mouse: Vector2, clicked: bool) -> Option<PathBuf> {
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.open = false;
            return None;
        }
        self.offset = (self.offset as isize - rl.get_mouse_wheel_move() as isize)
            .clamp(0, self.entries.len().saturating_sub(12) as isize)
            as usize;
        if !clicked {
            return None;
        }
        let bounds = Rectangle::new(90.0, 70.0, 720.0, 620.0);
        let close = Rectangle::new(bounds.x + bounds.width - 100.0, bounds.y + 16.0, 84.0, 36.0);
        if close.check_collision_point_rec(mouse) {
            self.open = false;
            return None;
        }
        let parent = Rectangle::new(bounds.x + 16.0, bounds.y + 62.0, 110.0, 36.0);
        if parent.check_collision_point_rec(mouse) {
            if let Some(parent) = self.directory.parent() {
                self.directory = parent.to_path_buf();
                self.refresh();
            }
            return None;
        }
        for (row, (path, is_directory)) in
            self.entries.iter().skip(self.offset).take(12).enumerate()
        {
            let item = Rectangle::new(
                bounds.x + 16.0,
                bounds.y + 110.0 + row as f32 * 40.0,
                bounds.width - 32.0,
                36.0,
            );
            if item.check_collision_point_rec(mouse) {
                if *is_directory {
                    self.directory = path.clone();
                    self.refresh();
                    return None;
                }
                self.open = false;
                return Some(path.clone());
            }
        }
        None
    }

    fn draw(&self, d: &mut impl RaylibDraw, mouse: Vector2) {
        let bounds = Rectangle::new(90.0, 70.0, 720.0, 620.0);
        d.draw_rectangle_rec(
            Rectangle::new(0.0, 0.0, WIDTH as f32, HEIGHT as f32),
            Color::new(0, 0, 0, 190),
        );
        d.draw_rectangle_rec(bounds, Color::new(28, 31, 38, 255));
        d.draw_text("Choose image", 106, 88, 24, Color::RAYWHITE);
        button(
            d,
            Rectangle::new(bounds.x + bounds.width - 100.0, bounds.y + 16.0, 84.0, 36.0),
            "Close",
            false,
            mouse,
        );
        let path = self.directory.to_string_lossy();
        let visible: String = path
            .chars()
            .skip(path.chars().count().saturating_sub(70))
            .collect();
        d.draw_text(&visible, 106, 132, 16, Color::LIGHTGRAY);
        button(
            d,
            Rectangle::new(bounds.x + 16.0, bounds.y + 62.0, 110.0, 36.0),
            "Parent",
            false,
            mouse,
        );
        for (row, (path, is_directory)) in
            self.entries.iter().skip(self.offset).take(12).enumerate()
        {
            let item = Rectangle::new(
                bounds.x + 16.0,
                bounds.y + 110.0 + row as f32 * 40.0,
                bounds.width - 32.0,
                36.0,
            );
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "/".into());
            button(
                d,
                item,
                &format!("{}{}", if *is_directory { "[dir] " } else { "" }, name),
                false,
                mouse,
            );
        }
        d.draw_text(&self.message, 106, 674, 16, Color::RED);
    }
}

fn logical_point(point: Vector2, screen_width: f32, screen_height: f32) -> Vector2 {
    let scale = (screen_width / WIDTH as f32).min(screen_height / HEIGHT as f32);
    let offset = Vector2::new(
        (screen_width - WIDTH as f32 * scale) * 0.5,
        (screen_height - HEIGHT as f32 * scale) * 0.5,
    );
    (point - offset) * (1.0 / scale.max(f32::EPSILON))
}

fn logical_mouse(rl: &RaylibHandle) -> Vector2 {
    logical_point(
        rl.get_mouse_position(),
        rl.get_screen_width() as f32,
        rl.get_screen_height() as f32,
    )
}

fn slider_rect(y: f32) -> Rectangle {
    Rectangle::new(40.0, y, 520.0, 34.0)
}

fn button(d: &mut impl RaylibDraw, bounds: Rectangle, label: &str, selected: bool, mouse: Vector2) {
    let hover = bounds.check_collision_point_rec(mouse);
    d.draw_rectangle_rec(
        bounds,
        if selected {
            Color::new(40, 111, 85, 255)
        } else if hover {
            Color::new(65, 67, 72, 255)
        } else {
            Color::new(44, 46, 51, 255)
        },
    );
    d.draw_text(
        label,
        bounds.x as i32 + 10,
        bounds.y as i32 + 8,
        18,
        Color::RAYWHITE,
    );
}

fn slider(d: &mut impl RaylibDraw, bounds: Rectangle, amount: f32) {
    d.draw_rectangle(
        bounds.x as i32,
        bounds.y as i32 + 14,
        bounds.width as i32,
        6,
        Color::new(65, 67, 72, 255),
    );
    let position = bounds.x + amount.clamp(0.0, 1.0) * bounds.width;
    d.draw_rectangle(
        bounds.x as i32,
        bounds.y as i32 + 14,
        (position - bounds.x) as i32,
        6,
        Color::new(100, 220, 160, 255),
    );
    d.draw_circle(position as i32, bounds.y as i32 + 17, 7.0, Color::RAYWHITE);
}

#[derive(Clone, Copy)]
enum ArtworkSet {
    Global,
    Frequency(VoiceBand),
}

fn stem(set: ArtworkSet, state: AvatarState, closed: bool) -> String {
    let prefix = match set {
        ArtworkSet::Global => String::new(),
        ArtworkSet::Frequency(band) => format!("{}_", band.cli_name()),
    };
    format!(
        "{prefix}{}{}",
        state.file_stem(),
        if closed { "_blink" } else { "" }
    )
}

fn image_named(dir: &Path, name: &str) -> Result<Option<PathBuf>, Box<dyn Error>> {
    if !dir.exists() {
        return Ok(None);
    }
    let mut found = None;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase);
        if path.is_file()
            && path
                .file_stem()
                .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case(name))
            && extension
                .as_deref()
                .is_some_and(|value| IMAGE_EXTENSIONS.contains(&value))
        {
            if found.replace(path).is_some() {
                return Err(format!("multiple {name} images in {}", dir.display()).into());
            }
        }
    }
    Ok(found)
}

fn image_dimensions(path: &Path) -> Result<(i32, i32), Box<dyn Error>> {
    let image = Image::load_image(&path.to_string_lossy())
        .map_err(|error| format!("could not load {}: {error}", path.display()))?;
    Ok((image.width, image.height))
}

fn install_image(
    directory: &Path,
    source: &Path,
    set: ArtworkSet,
    state: AvatarState,
    closed: bool,
) -> Result<(), Box<dyn Error>> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or("Dropped artwork needs an image extension")?;
    if !IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        return Err(format!("Unsupported image extension: .{extension}").into());
    }
    let dimensions = image_dimensions(source)?;
    let counterpart = stem(set, state, !closed);
    let counterpart_path = image_named(directory, &counterpart)?;
    if closed && counterpart_path.is_none() {
        return Err(format!(
            "Add the open image for {} before its closed-eye image",
            state.label()
        )
        .into());
    }
    if let Some(path) = counterpart_path {
        if image_dimensions(&path)? != dimensions {
            return Err(format!(
                "Open and closed-eye images for {} must have matching dimensions",
                state.label()
            )
            .into());
        }
    }
    let bytes = std::fs::read(source)?;
    let name = stem(set, state, closed);
    std::fs::create_dir_all(directory)?;
    if directory.exists() {
        for entry in std::fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_file()
                && path
                    .file_stem()
                    .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case(&name))
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| {
                        IMAGE_EXTENSIONS.contains(&value.to_ascii_lowercase().as_str())
                    })
            {
                std::fs::remove_file(path)?;
            }
        }
    }
    std::fs::write(directory.join(format!("{name}.{extension}")), bytes)?;
    Ok(())
}

fn remove_image(
    directory: &Path,
    set: ArtworkSet,
    state: AvatarState,
    closed: bool,
) -> Result<(), Box<dyn Error>> {
    let name = stem(set, state, closed);
    if !directory.exists() {
        return Ok(());
    }
    if !closed && image_named(directory, &stem(set, state, true))?.is_some() {
        return Err("Remove the closed-eye image before removing its open image".into());
    }
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_file()
            && path
                .file_stem()
                .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case(&name))
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| {
                    IMAGE_EXTENSIONS.contains(&value.to_ascii_lowercase().as_str())
                })
        {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn save_profile(
    config: &Path,
    band: VoiceBand,
    gain_db: f32,
    profile: &FrequencyProfile,
) -> Result<(), Box<dyn Error>> {
    let mut current = Settings::load(config)?;
    current.set_frequency_gain(band, gain_db);
    if let Some(target) = current.frequency_profile_mut(band) {
        *target = profile.clone();
    }
    current.save(config)
}

pub fn run(band: VoiceBand, asset_dir: PathBuf, config: &Path) -> Result<(), Box<dyn Error>> {
    if band == VoiceBand::Unknown {
        return Err("Unpitched input does not have a frequency window".into());
    }
    let initial = Settings::load(config)?;
    let mut profile = initial
        .frequency_profile(band)
        .expect("known frequency band")
        .clone();
    if !profile.enabled {
        profile.bounce = initial.bounce;
        profile.scale = initial.scale;
        profile.mirror = initial.mirror;
        profile.blink = initial.blink;
    }
    let mut gain_db = initial.frequency_gain(band).expect("known frequency band");
    let mut selected = (AvatarState::Idle, false);
    let mut active_slider: Option<usize> = None;
    let mut save_delay: Option<f32> = None;
    let mut status = String::new();
    let mut browser = ImageBrowser::default();

    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title(&format!(
            "mic_freq - {} frequency ({})",
            band.label(),
            band.range_label()
        ))
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_window_min_size(760, 700);
    rl.set_exit_key(None);

    while !rl.window_should_close() {
        let dt = rl.get_frame_time();
        let mouse = logical_mouse(&rl);
        let clicked = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let mut changed = false;

        let browser_was_open = browser.open;
        if browser.open {
            if let Some(source) = browser.update(&mut rl, mouse, clicked) {
                match install_image(
                    &asset_dir,
                    &source,
                    ArtworkSet::Frequency(band),
                    selected.0,
                    selected.1,
                ) {
                    Ok(()) => {
                        status = format!(
                            "Saved {} {} image",
                            selected.0.label(),
                            if selected.1 { "closed-eye" } else { "open" }
                        );
                        changed = true;
                    }
                    Err(error) => status = error.to_string(),
                }
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) && !browser_was_open {
            break;
        }
        if clicked && !browser_was_open {
            for (index, top) in CONTROL_TOPS.into_iter().enumerate() {
                if slider_rect(top).check_collision_point_rec(mouse) {
                    active_slider = Some(index);
                }
            }
            if Rectangle::new(600.0, CONTROL_TOPS[0], 260.0, 34.0).check_collision_point_rec(mouse)
            {
                profile.mirror = !profile.mirror;
                profile.enabled = true;
                changed = true;
            }
            if Rectangle::new(600.0, 156.0, 260.0, 34.0).check_collision_point_rec(mouse) {
                profile.blink = !profile.blink;
                profile.enabled = true;
                changed = true;
            }
            if Rectangle::new(600.0, 202.0, 260.0, 34.0).check_collision_point_rec(mouse) {
                profile.enabled = !profile.enabled;
                changed = true;
            }
            for (state_index, state) in AvatarState::ALL.into_iter().enumerate() {
                for closed in [false, true] {
                    let column = usize::from(closed);
                    let bounds = Rectangle::new(
                        40.0 + column as f32 * 420.0,
                        334.0 + state_index as f32 * 104.0,
                        390.0,
                        84.0,
                    );
                    if bounds.check_collision_point_rec(mouse) {
                        selected = (state, closed);
                    }
                }
            }
            if Rectangle::new(640.0, 676.0, 220.0, 42.0).check_collision_point_rec(mouse) {
                match remove_image(
                    &asset_dir,
                    ArtworkSet::Frequency(band),
                    selected.0,
                    selected.1,
                ) {
                    Ok(()) => {
                        status = "Selected image removed; the global pose will be used".into();
                        changed = true;
                    }
                    Err(error) => status = error.to_string(),
                }
            }
            if Rectangle::new(400.0, 676.0, 220.0, 42.0).check_collision_point_rec(mouse) {
                browser.open_at(&asset_dir);
            }
        }
        if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) || browser.open {
            active_slider = None;
        }
        if let Some(index) = active_slider {
            let bounds = slider_rect(CONTROL_TOPS[index]);
            let amount = ((mouse.x - bounds.x) / bounds.width).clamp(0.0, 1.0);
            match index {
                0 => gain_db = -18.0 + amount * 36.0,
                1 => {
                    profile.bounce = amount * 2.0;
                    profile.enabled = true;
                }
                2 => {
                    profile.scale = 0.2 + amount * 0.8;
                    profile.enabled = true;
                }
                _ => unreachable!(),
            }
            changed = true;
        }

        if rl.is_file_dropped() && !browser_was_open {
            let files = rl.load_dropped_files();
            if files.paths().len() != 1 {
                status = "Drop exactly one open or closed-eye image".into();
            } else {
                let source = PathBuf::from(files.paths()[0]);
                match install_image(
                    &asset_dir,
                    &source,
                    ArtworkSet::Frequency(band),
                    selected.0,
                    selected.1,
                ) {
                    Ok(()) => {
                        status = format!(
                            "Saved {} {} image",
                            selected.0.label(),
                            if selected.1 { "closed-eye" } else { "open" }
                        );
                        changed = true;
                    }
                    Err(error) => status = error.to_string(),
                }
            }
        }

        if changed {
            save_delay = Some(0.25);
        }
        if let Some(delay) = save_delay.as_mut() {
            *delay -= dt;
            if *delay <= 0.0 {
                match save_profile(config, band, gain_db, &profile) {
                    Ok(()) => status.push_str(" · Saved"),
                    Err(error) => status = format!("Save failed: {error}"),
                }
                save_delay = None;
            }
        }

        let (screen_width, screen_height) =
            (rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let scale = (screen_width / WIDTH as f32).min(screen_height / HEIGHT as f32);
        let camera = Camera2D {
            offset: Vector2::new(
                (screen_width - WIDTH as f32 * scale) * 0.5,
                (screen_height - HEIGHT as f32 * scale) * 0.5,
            ),
            target: Vector2::zero(),
            rotation: 0.0,
            zoom: scale,
        };
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(12, 13, 17, 255));
        let mut d = d.begin_mode2D(camera);
        d.clear_background(Color::new(24, 25, 30, 255));
        d.draw_text(
            &format!("{} voice animation", band.label()),
            40,
            20,
            28,
            Color::RAYWHITE,
        );
        d.draw_text(
            &format!(
                "{} · Changes apply when Audio Studio detects this pitch",
                band.range_label()
            ),
            40,
            54,
            18,
            Color::LIGHTGRAY,
        );
        for (top, label, amount) in [
            (
                CONTROL_TOPS[0],
                format!("Voice gain  {gain_db:+.0} dB"),
                (gain_db + 18.0) / 36.0,
            ),
            (
                CONTROL_TOPS[1],
                format!("Animation bounce  {:.0}%", profile.bounce * 100.0),
                profile.bounce / 2.0,
            ),
            (
                CONTROL_TOPS[2],
                format!("Avatar size  {:.0}%", profile.scale * 100.0),
                (profile.scale - 0.2) / 0.8,
            ),
        ] {
            d.draw_text(&label, 40, top as i32 - 22, 18, Color::LIGHTGRAY);
            slider(&mut d, slider_rect(top), amount);
        }
        button(
            &mut d,
            Rectangle::new(600.0, CONTROL_TOPS[0], 260.0, 34.0),
            if profile.mirror {
                "Mirrored: On"
            } else {
                "Mirrored: Off"
            },
            profile.mirror,
            mouse,
        );
        button(
            &mut d,
            Rectangle::new(600.0, 156.0, 260.0, 34.0),
            if profile.blink {
                "Blink images: On"
            } else {
                "Blink images: Off"
            },
            profile.blink,
            mouse,
        );
        button(
            &mut d,
            Rectangle::new(600.0, 202.0, 260.0, 34.0),
            if profile.enabled {
                "Custom animation: On"
            } else {
                "Custom animation: Off"
            },
            profile.enabled,
            mouse,
        );
        d.draw_text("Artwork", 40, 286, 22, Color::RAYWHITE);
        d.draw_text(
            "Select a slot, then drop PNG/JPG/BMP/GIF/QOI/TGA artwork",
            40,
            310,
            17,
            Color::GRAY,
        );
        for (state_index, state) in AvatarState::ALL.into_iter().enumerate() {
            for closed in [false, true] {
                let column = usize::from(closed);
                let bounds = Rectangle::new(
                    40.0 + column as f32 * 420.0,
                    334.0 + state_index as f32 * 104.0,
                    390.0,
                    84.0,
                );
                let is_selected = selected == (state, closed);
                let file = match image_named(
                    &asset_dir,
                    &stem(ArtworkSet::Frequency(band), state, closed),
                ) {
                    Ok(file) => file,
                    Err(error) => {
                        status = error.to_string();
                        None
                    }
                };
                let name = file
                    .and_then(|path| {
                        path.file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| "Using global artwork".into());
                button(
                    &mut d,
                    bounds,
                    &format!(
                        "{} · {}  |  {name}",
                        state.label(),
                        if closed { "closed eye" } else { "open" }
                    ),
                    is_selected,
                    mouse,
                );
            }
        }
        button(
            &mut d,
            Rectangle::new(640.0, 676.0, 220.0, 42.0),
            "Remove selected image",
            false,
            mouse,
        );
        button(
            &mut d,
            Rectangle::new(400.0, 676.0, 220.0, 42.0),
            "Browse selected image",
            false,
            mouse,
        );
        d.draw_text(&status, 40, 716, 17, Color::LIGHTGRAY);
        if browser.open {
            browser.draw(&mut d, mouse);
        }
    }
    if let Err(error) = save_profile(config, band, gain_db, &profile) {
        return Err(format!("Could not save {} frequency profile: {error}", band.label()).into());
    }
    Ok(())
}

pub fn run_avatar(asset_dir: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut selected = (AvatarState::Idle, false);
    let mut status = String::new();
    let mut browser = ImageBrowser::default();
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("mic_freq - Avatar Artwork")
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_window_min_size(760, 700);
    rl.set_exit_key(None);

    while !rl.window_should_close() {
        let mouse = logical_mouse(&rl);
        let clicked = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let browser_was_open = browser.open;
        if browser.open {
            if let Some(source) = browser.update(&mut rl, mouse, clicked) {
                match install_image(
                    &asset_dir,
                    &source,
                    ArtworkSet::Global,
                    selected.0,
                    selected.1,
                ) {
                    Ok(()) => {
                        status = format!(
                            "Saved {} {} image",
                            selected.0.label(),
                            if selected.1 { "closed-eye" } else { "open" }
                        );
                    }
                    Err(error) => status = error.to_string(),
                }
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) && !browser_was_open {
            break;
        }
        if clicked && !browser_was_open {
            for (state_index, state) in AvatarState::ALL.into_iter().enumerate() {
                for closed in [false, true] {
                    let column = usize::from(closed);
                    let bounds = Rectangle::new(
                        40.0 + column as f32 * 420.0,
                        110.0 + state_index as f32 * 126.0,
                        390.0,
                        106.0,
                    );
                    if bounds.check_collision_point_rec(mouse) {
                        selected = (state, closed);
                    }
                }
            }
            if Rectangle::new(400.0, 526.0, 220.0, 42.0).check_collision_point_rec(mouse) {
                browser.open_at(&asset_dir);
            }
            if Rectangle::new(640.0, 526.0, 220.0, 42.0).check_collision_point_rec(mouse) {
                match remove_image(&asset_dir, ArtworkSet::Global, selected.0, selected.1) {
                    Ok(()) => status = "Selected image removed; the demo pose will be used".into(),
                    Err(error) => status = error.to_string(),
                }
            }
        }
        if rl.is_file_dropped() && !browser_was_open {
            let files = rl.load_dropped_files();
            if files.paths().len() != 1 {
                status = "Drop exactly one image".into();
            } else {
                let source = PathBuf::from(files.paths()[0]);
                match install_image(
                    &asset_dir,
                    &source,
                    ArtworkSet::Global,
                    selected.0,
                    selected.1,
                ) {
                    Ok(()) => status = "Selected image replaced".into(),
                    Err(error) => status = error.to_string(),
                }
            }
        }

        let (screen_width, screen_height) =
            (rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let scale = (screen_width / WIDTH as f32).min(screen_height / HEIGHT as f32);
        let camera = Camera2D {
            offset: Vector2::new(
                (screen_width - WIDTH as f32 * scale) * 0.5,
                (screen_height - HEIGHT as f32 * scale) * 0.5,
            ),
            target: Vector2::zero(),
            rotation: 0.0,
            zoom: scale,
        };
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(12, 13, 17, 255));
        let mut d = d.begin_mode2D(camera);
        d.clear_background(Color::new(24, 25, 30, 255));
        d.draw_text("Avatar artwork", 40, 22, 28, Color::RAYWHITE);
        d.draw_text(
            "Select an open or closed-eye slot, then browse or drop a replacement",
            40,
            60,
            17,
            Color::LIGHTGRAY,
        );
        for (state_index, state) in AvatarState::ALL.into_iter().enumerate() {
            for closed in [false, true] {
                let column = usize::from(closed);
                let bounds = Rectangle::new(
                    40.0 + column as f32 * 420.0,
                    110.0 + state_index as f32 * 126.0,
                    390.0,
                    106.0,
                );
                let name = image_named(&asset_dir, &stem(ArtworkSet::Global, state, closed))
                    .ok()
                    .flatten()
                    .and_then(|path| {
                        path.file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| "Using demo pose".into());
                button(
                    &mut d,
                    bounds,
                    &format!(
                        "{} · {}  |  {name}",
                        state.label(),
                        if closed { "closed eye" } else { "open" }
                    ),
                    selected == (state, closed),
                    mouse,
                );
            }
        }
        button(
            &mut d,
            Rectangle::new(400.0, 526.0, 220.0, 42.0),
            "Browse selected image",
            false,
            mouse,
        );
        button(
            &mut d,
            Rectangle::new(640.0, 526.0, 220.0, 42.0),
            "Remove selected image",
            false,
            mouse,
        );
        d.draw_text(
            "Changes reload in Audio Studio when this window closes.",
            40,
            600,
            17,
            Color::GRAY,
        );
        d.draw_text(&status, 40, 640, 17, Color::LIGHTGRAY);
        if browser.open {
            browser.draw(&mut d, mouse);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resized_windows_convert_mouse_hits_back_to_logical_coordinates() {
        let screen = Vector2::new(1200.0 * 0.5, 900.0 * 0.5);
        let logical = logical_point(screen, 1200.0, 900.0);
        assert!((logical.x - WIDTH as f32 * 0.5).abs() < 0.01);
        assert!((logical.y - HEIGHT as f32 * 0.5).abs() < 0.01);
        assert_eq!(
            logical_point(Vector2::new(450.0, 380.0), WIDTH as f32, HEIGHT as f32),
            Vector2::new(450.0, 380.0)
        );
    }

    #[test]
    fn frequency_header_does_not_overlap_voice_gain_controls() {
        let header_bottom = 54.0 + 18.0;
        let gain_label_top = CONTROL_TOPS[0] - 22.0;
        assert!(
            gain_label_top >= header_bottom + 8.0,
            "{gain_label_top} overlaps header ending at {header_bottom}"
        );
        assert!(CONTROL_TOPS[2] + 34.0 <= 286.0);
    }
}

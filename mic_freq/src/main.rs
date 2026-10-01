mod editor;

use editor::{Editor, EditorAction, PANEL_W};
use mic_freq::analysis::{self, Analyzer};
use mic_freq::audio::MicCapture;
use mic_freq::config::{self, ImageSettings, Layout, GENERATED_PREFIX, LAYOUT_FILE, MAX_HZ, MIN_HZ};
use raylib::prelude::*;
use std::path::{Path, PathBuf};

const FFT_SIZE: usize = 4096;
const LOUDNESS_RANGE_DB: f32 = 35.0;
const LEVEL_STEPS: u32 = 5;
const ATTACK_PER_SEC: f32 = 14.0;
const RELEASE_PER_SEC: f32 = 3.0;
const SPECTRUM_MAX_HZ: f32 = 4000.0;
const SPECTRUM_H: f32 = 140.0;

pub struct Asset {
    pub settings: ImageSettings,
    pub texture: Texture2D,
    pub level: f32,
}

pub struct Scene {
    pub dir: PathBuf,
    pub assets: Vec<Asset>,
    pub ignored: Vec<String>,
}

impl Scene {
    /// Load the saved layout (if any), merge it with the folder contents and load all textures.
    fn load(rl: &mut RaylibHandle, thread: &RaylibThread, dir: &Path) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let saved = Layout::load(&dir.join(LAYOUT_FILE)).unwrap_or_else(|e| {
            warnings.push(format!("could not read {LAYOUT_FILE}: {e}"));
            None
        });
        let layout = saved.unwrap_or_default().reconcile(&config::scan_image_files(dir));

        let mut assets = Vec::new();
        for settings in layout.images {
            match load_texture(rl, thread, dir, &settings.source) {
                Ok(texture) => assets.push(Asset { settings, texture, level: 0.0 }),
                Err(e) => warnings.push(e),
            }
        }
        (Self { dir: dir.to_path_buf(), assets, ignored: layout.ignored }, warnings)
    }

    fn save(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = self.dir.join(LAYOUT_FILE);
        let layout = Layout { images: self.assets.iter().map(|a| a.settings.clone()).collect(), ignored: self.ignored.clone() };
        layout.save(&path)?;
        Ok(path)
    }

    /// Copy a dropped image into the asset folder (if it isn't there already) and load it.
    fn import(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread, path: &Path) -> Result<(String, Texture2D), String> {
        if !config::is_image_file(path) {
            return Err(format!("unsupported file type: {}", path.display()));
        }
        let file_name = path.file_name().ok_or("invalid path")?.to_string_lossy().into_owned();
        let in_dir = path.parent().and_then(|p| p.canonicalize().ok()) == self.dir.canonicalize().ok();
        let source = if in_dir {
            file_name
        } else {
            std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
            let name = config::unique_file_name(&self.dir, &file_name);
            std::fs::copy(path, self.dir.join(&name)).map_err(|e| format!("copy failed: {e}"))?;
            name
        };
        let texture = load_texture(rl, thread, &self.dir, &source)?;
        self.ignored.retain(|s| s != &source);
        Ok((source, texture))
    }
}

fn load_texture(rl: &mut RaylibHandle, thread: &RaylibThread, dir: &Path, source: &str) -> Result<Texture2D, String> {
    let image = match source.strip_prefix(GENERATED_PREFIX) {
        Some(kind) => placeholder_image(kind).ok_or_else(|| format!("unknown placeholder '{kind}'"))?,
        None => {
            let path = dir.join(source);
            Image::load_image(&path.to_string_lossy()).map_err(|e| format!("skipping {}: {e}", path.display()))?
        }
    };
    rl.load_texture_from_image(thread, &image).map_err(|e| format!("texture for {source}: {e}"))
}

fn placeholder_image(kind: &str) -> Option<Image> {
    Some(match kind {
        "bass" => Image::gen_image_gradient_radial(256, 256, 0.0, Color::new(255, 80, 60, 255), Color::BLANK),
        "low-mid" => Image::gen_image_checked(256, 256, 8, 8, Color::ORANGE, Color::new(90, 40, 0, 255)),
        "mid" => Image::gen_image_cellular(256, 256, 32),
        "high-mid" => Image::gen_image_perlin_noise(256, 256, 0, 0, 4.0),
        "treble" => Image::gen_image_gradient_radial(256, 256, 0.3, Color::new(120, 200, 255, 255), Color::new(40, 0, 120, 0)),
        _ => return None,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let asset_dir = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("assets"));

    let mic = MicCapture::start(FFT_SIZE * 2)?;
    println!("Listening on '{}' @ {} Hz", mic.device_name, mic.sample_rate);
    let mut analyzer = Analyzer::new(FFT_SIZE, mic.sample_rate);
    analyzer.min_hz = MIN_HZ;
    analyzer.max_hz = MAX_HZ;

    let (mut rl, thread) = raylib::init().size(1280, 900).title("mic_freq").resizable().msaa_4x().build();
    rl.set_target_fps(60);
    rl.set_exit_key(None);
    rl.gui_set_style(GuiControl::DEFAULT, GuiDefaultProperty::TEXT_SIZE, 16);

    let (mut scene, warnings) = Scene::load(&mut rl, &thread, &asset_dir);
    let mut editor = Editor::default();
    for w in &warnings {
        eprintln!("{w}");
    }
    if let Some(w) = warnings.first() {
        editor.set_status(w.clone(), rl.get_time());
    }
    if scene.assets.iter().all(|a| a.settings.is_generated()) {
        println!("Using built-in placeholder images (add files to '{}' or drop them on the window)", asset_dir.display());
    }

    let mut samples = Vec::with_capacity(FFT_SIZE);
    let mut shown_hz: Option<f32> = None;
    let mut quit_prompt = false;

    loop {
        let now = rl.get_time();
        let dt = rl.get_frame_time();

        if rl.window_should_close() {
            if editor.dirty {
                quit_prompt = true;
            } else {
                break;
            }
        }

        let mut save_requested = false;
        if !editor.wants_keyboard() && !quit_prompt {
            let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
            if rl.is_key_pressed(KeyboardKey::KEY_TAB) {
                editor.open = !editor.open;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                if editor.open {
                    editor.open = false;
                } else if editor.dirty {
                    quit_prompt = true;
                } else {
                    break;
                }
            }
            if ctrl && rl.is_key_pressed(KeyboardKey::KEY_S) {
                save_requested = true;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_UP) {
                analyzer.gate_db = (analyzer.gate_db + 2.0).min(-6.0);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
                analyzer.gate_db = (analyzer.gate_db - 2.0).max(-90.0);
            }
        }

        if rl.is_file_dropped() {
            let dropped: Vec<String> = rl.load_dropped_files().paths().iter().map(|p| p.to_string()).collect();
            for p in dropped {
                match scene.import(&mut rl, &thread, Path::new(&p)) {
                    Ok((source, texture)) => {
                        let replace = editor.selected.filter(|&i| editor.drop_replaces && i < scene.assets.len());
                        if let Some(i) = replace {
                            let a = &mut scene.assets[i];
                            if !a.settings.is_generated() && a.settings.source != source {
                                scene.ignored.push(std::mem::take(&mut a.settings.source));
                            }
                            a.settings.source = source;
                            a.texture = texture;
                            editor.set_status(format!("Replaced image of '{}'", a.settings.name), now);
                        } else {
                            let mut s = ImageSettings::new(config::file_stem(&source), source);
                            s.center_hz = shown_hz
                                .or_else(|| scene.assets.last().map(|a| a.settings.center_hz * 2f32.sqrt()))
                                .unwrap_or(400.0)
                                .clamp(MIN_HZ, MAX_HZ);
                            editor.set_status(format!("Added '{}' at {:.0} Hz", s.name, s.center_hz), now);
                            scene.assets.push(Asset { settings: s, texture, level: 0.0 });
                            editor.select(Some(scene.assets.len() - 1));
                        }
                        editor.dirty = true;
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        editor.set_status(e, now);
                    }
                }
            }
        }

        mic.latest(FFT_SIZE, &mut samples);
        let result = analyzer.analyze(&samples);
        let loud = analysis::loudness(result.rms, analyzer.gate_db, LOUDNESS_RANGE_DB);

        shown_hz = match (result.dominant_hz, shown_hz) {
            (Some(f), Some(prev)) if (f / prev).log2().abs() < 0.08 => Some(prev + (f - prev) * 0.35),
            (Some(f), _) => Some(f),
            (None, _) => None,
        };

        for a in &mut scene.assets {
            let s = &a.settings;
            let target = match shown_hz {
                Some(f) if s.visible => analysis::band_affinity(f, s.center_hz, s.width_octaves) * (loud * s.gain).min(1.0),
                _ => 0.0,
            };
            let rate = if target > a.level { ATTACK_PER_SEC } else { RELEASE_PER_SEC };
            a.level += (target - a.level) * (1.0 - (-rate * dt).exp());
        }

        let (w, h) = (rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let stage_w = if editor.open { (w - PANEL_W).max(200.0) } else { w };
        let stage = Rectangle::new(0.0, 0.0, stage_w, h - SPECTRUM_H - 40.0);
        let placements = stage_layout(&scene.assets, stage, now as f32);

        if editor.open && !quit_prompt && rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let m = rl.get_mouse_position();
            if m.x < stage_w {
                if let Some(p) = placements.iter().rev().find(|p| p.rect.check_collision_point_rec(m)) {
                    editor.select(Some(p.index));
                }
            }
        }

        let mut action = save_requested.then_some(EditorAction::Save);
        let mut exit = false;
        {
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::new(18, 18, 24, 255));

            draw_header(&mut d, shown_hz, result.rms, analyzer.gate_db, stage_w, editor.open);
            let selected = editor.selected.filter(|_| editor.open);
            draw_assets(&mut d, &scene.assets, &placements, stage, selected);
            let spec = Rectangle::new(20.0, h - SPECTRUM_H - 20.0, stage_w - 40.0, SPECTRUM_H);
            draw_spectrum(&mut d, &result, &scene.assets, shown_hz, spec, selected);

            if editor.open {
                if quit_prompt {
                    d.gui_lock();
                }
                let panel = Rectangle::new(stage_w, 0.0, w - stage_w, h);
                if let Some(a) = editor.ui(&mut d, &mut scene, shown_hz, panel, now) {
                    action = Some(a);
                }
                d.gui_unlock();
            }

            if quit_prompt {
                d.draw_rectangle(0, 0, w as i32, h as i32, Color::BLACK.alpha(0.5));
                let r = Rectangle::new(w / 2.0 - 180.0, h / 2.0 - 60.0, 360.0, 120.0);
                match d.gui_message_box(r, "Unsaved changes", "Save image settings before quitting?", "Save;Discard;Cancel") {
                    1 => {
                        action = Some(EditorAction::Save);
                        exit = true;
                    }
                    2 => exit = true,
                    0 | 3 => quit_prompt = false,
                    _ => {}
                }
            }
        }

        match action {
            Some(EditorAction::Save) => match scene.save() {
                Ok(path) => {
                    editor.dirty = false;
                    editor.set_status(format!("Saved {}", path.display()), now);
                }
                Err(e) => {
                    eprintln!("save failed: {e}");
                    editor.set_status(format!("Save failed: {e}"), now);
                    exit = false;
                    quit_prompt = false;
                }
            },
            Some(EditorAction::Revert) => {
                let (fresh, warnings) = Scene::load(&mut rl, &thread, &asset_dir);
                scene = fresh;
                editor.dirty = false;
                editor.clamp_selection(scene.assets.len());
                editor.set_status(warnings.first().cloned().unwrap_or_else(|| "Reverted to saved settings".into()), now);
            }
            None => {}
        }
        if exit {
            break;
        }
    }
    Ok(())
}

struct Placement {
    index: usize,
    rect: Rectangle,
    rotation: f32,
}

/// Screen rectangle (unrotated) of every visible image for the current levels.
fn stage_layout(assets: &[Asset], area: Rectangle, time: f32) -> Vec<Placement> {
    let visible: Vec<usize> = (0..assets.len()).filter(|&i| assets[i].settings.visible).collect();
    if visible.is_empty() {
        return Vec::new();
    }
    let slot = area.width / visible.len() as f32;
    let bottom = area.y + area.height;
    let max_size = (slot * 0.85).min(area.height - 200.0).max(40.0);
    visible
        .into_iter()
        .enumerate()
        .map(|(k, index)| {
            let a = &assets[index];
            let s = &a.settings;
            let lvl = a.level.clamp(0.0, 1.0);
            let size = max_size * s.scale * (s.min_size + (1.0 - s.min_size) * lvl);
            let (tw, th) = (a.texture.width() as f32, a.texture.height() as f32);
            let aspect = tw / th.max(1.0);
            let (dw, dh) = if aspect >= 1.0 { (size, size / aspect) } else { (size * aspect, size) };
            let cx = area.x + slot * (k as f32 + 0.5);
            let base_y = bottom - 40.0 - lvl * s.lift_px;
            Placement {
                index,
                rect: Rectangle::new(cx - dw / 2.0, base_y - dh, dw, dh),
                rotation: (time * 6.0).sin() * s.wobble_deg * lvl,
            }
        })
        .collect()
}

fn draw_header(d: &mut RaylibDrawHandle, hz: Option<f32>, rms: f32, gate_db: f32, stage_w: f32, editor_open: bool) {
    let text = match hz {
        Some(f) => {
            let (note, cents) = analysis::note_name(f);
            format!("{f:7.1} Hz   {note} {cents:+.0}c")
        }
        None => "   --- Hz   (below gate)".to_string(),
    };
    d.draw_text(&text, 20, 16, 40, Color::RAYWHITE);
    d.draw_text(
        &format!("input {:6.1} dBFS   gate {gate_db:.0} dB (Up/Down)", analysis::amplitude_to_db(rms)),
        20,
        62,
        20,
        Color::GRAY,
    );
    if !editor_open {
        d.draw_text("[Tab] customize images", 20, 88, 18, Color::DARKGRAY);
    }
    d.draw_fps(stage_w as i32 - 90, 16);
}

fn draw_assets(d: &mut RaylibDrawHandle, assets: &[Asset], placements: &[Placement], area: Rectangle, selected: Option<usize>) {
    let bottom = area.y + area.height;
    let slot = if placements.is_empty() { area.width } else { area.width / placements.len() as f32 };
    for (k, p) in placements.iter().enumerate() {
        let a = &assets[p.index];
        let s = &a.settings;
        let lvl = a.level.clamp(0.0, 1.0);
        let step = analysis::discrete_level(lvl, LEVEL_STEPS);
        let (tw, th) = (a.texture.width() as f32, a.texture.height() as f32);
        let r = p.rect;
        let center = Rectangle::new(r.x + r.width / 2.0, r.y + r.height / 2.0, r.width, r.height);
        let opacity = s.min_opacity + (1.0 - s.min_opacity) * lvl;
        let tint = Color::new(s.tint[0], s.tint[1], s.tint[2], 255).alpha(opacity);
        d.draw_texture_pro(&a.texture, Rectangle::new(0.0, 0.0, tw, th), center, Vector2::new(r.width / 2.0, r.height / 2.0), p.rotation, tint);
        if selected == Some(p.index) {
            d.draw_rectangle_lines_ex(Rectangle::new(r.x - 4.0, r.y - 4.0, r.width + 8.0, r.height + 8.0), 2.0, Color::YELLOW);
        }

        let cx = area.x + slot * (k as f32 + 0.5);
        let label = format!("{}  {:.0} Hz  L{step}", s.name, s.center_hz);
        let lw = d.measure_text(&label, 18);
        let color = if selected == Some(p.index) { Color::YELLOW } else { Color::LIGHTGRAY };
        d.draw_text(&label, (cx - lw as f32 / 2.0) as i32, (bottom - 22.0) as i32, 18, color);
        for st in 0..LEVEL_STEPS {
            let col = if st < step {
                Color::color_from_hsv(120.0 - 120.0 * st as f32 / LEVEL_STEPS as f32, 0.8, 0.9)
            } else {
                Color::new(60, 60, 70, 255)
            };
            d.draw_rectangle((cx - 50.0 + st as f32 * 20.0) as i32, bottom as i32, 16, 8, col);
        }
    }
}

fn draw_spectrum(d: &mut RaylibDrawHandle, r: &analysis::Analysis, assets: &[Asset], hz: Option<f32>, area: Rectangle, selected: Option<usize>) {
    d.draw_rectangle_lines(area.x as i32, area.y as i32, area.width as i32, area.height as i32, Color::DARKGRAY);
    let x_of = |f: f32| area.x + area.width * ((f / 30.0).ln() / (SPECTRUM_MAX_HZ / 30.0).ln()).clamp(0.0, 1.0);

    if let Some(s) = selected.and_then(|i| assets.get(i)).map(|a| &a.settings) {
        let lo = x_of(s.center_hz / s.width_octaves.exp2());
        let hi = x_of(s.center_hz * s.width_octaves.exp2());
        d.draw_rectangle(lo as i32, area.y as i32, (hi - lo) as i32, area.height as i32, Color::YELLOW.alpha(0.12));
    }
    for (i, a) in assets.iter().enumerate().filter(|(_, a)| a.settings.visible) {
        let x = x_of(a.settings.center_hz) as i32;
        let col = if selected == Some(i) { Color::YELLOW.alpha(0.7) } else { Color::new(70, 70, 110, 255) };
        d.draw_line(x, area.y as i32, x, (area.y + area.height) as i32, col);
    }

    let bars = area.width as usize / 3;
    let mut prev_bin = 1usize;
    for b in 0..bars {
        let f_hi = 30.0 * (SPECTRUM_MAX_HZ / 30.0).powf((b + 1) as f32 / bars as f32);
        let bin_hi = ((f_hi / r.bin_hz) as usize).clamp(prev_bin + 1, r.magnitudes.len());
        let peak = r.magnitudes[prev_bin.min(bin_hi - 1)..bin_hi].iter().copied().fold(0.0, f32::max);
        prev_bin = bin_hi;
        let norm = ((analysis::amplitude_to_db(peak) + 80.0) / 80.0).clamp(0.0, 1.0);
        let bh = norm * area.height;
        let x = area.x + b as f32 * 3.0;
        d.draw_rectangle(x as i32, (area.y + area.height - bh) as i32, 2, bh as i32, Color::color_from_hsv(200.0 - 160.0 * norm, 0.7, 0.9));
    }

    if let Some(f) = hz {
        let x = x_of(f) as i32;
        d.draw_line(x, area.y as i32, x, (area.y + area.height) as i32, Color::YELLOW);
    }
}

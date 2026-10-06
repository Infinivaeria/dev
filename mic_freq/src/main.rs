use mic_freq::audio::MicCapture;
use mic_freq::voice::{self, AvatarState, VoiceState};
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
                .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(state.file_stem()))
            || !path.extension().is_some_and(|e| {
                IMAGE_EXTENSIONS.contains(&e.to_string_lossy().to_ascii_lowercase().as_str())
            })
        {
            continue;
        }
        if found.replace(path).is_some() {
            return Err(format!(
                "multiple {} images in {} (keep only one)",
                state.file_stem(),
                dir.display()
            )
            .into());
        }
    }
    Ok(found)
}

fn load_images(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    dir: &Path,
) -> Result<[Texture2D; 3], Box<dyn Error>> {
    let mut textures = Vec::new();
    for state in AvatarState::ALL {
        let image = if let Some(path) = image_file(dir, state)? {
            Image::load_image(&path.to_string_lossy())
                .map_err(|e| format!("could not load {}: {e}", path.display()))?
        } else {
            eprintln!(
                "No {} image in {}; using placeholder (add {}.png and press R to reload)",
                state.file_stem(),
                dir.display(),
                state.file_stem()
            );
            Image::gen_image_color(512, 512, state.placeholder_color())
        };
        let mut texture = rl.load_texture_from_image(thread, &image)?;
        texture.gen_texture_mipmaps();
        texture.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_TRILINEAR);
        textures.push(texture);
    }
    Ok(textures
        .try_into()
        .unwrap_or_else(|_| unreachable!("one texture per avatar state")))
}

fn assets_beside_executable(executable: &Path) -> Result<PathBuf, Box<dyn Error>> {
    Ok(executable
        .parent()
        .ok_or("executable has no parent directory")?
        .join("assets"))
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

fn main() -> Result<(), Box<dyn Error>> {
    let asset_dir = match std::env::args_os().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => assets_beside_executable(&std::env::current_exe()?)?,
    };
    let mic = MicCapture::start(WINDOW_SAMPLES * 2)?;
    println!(
        "Listening on '{}' @ {} Hz",
        mic.device_name, mic.sample_rate
    );

    let (mut rl, thread) = raylib::init()
        .size(900, 900)
        .title("mic_freq - voice avatar")
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_exit_key(None);
    rl.get_font_default()
        .texture()
        .set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
    let mut images = load_images(&mut rl, &thread, &asset_dir)?;
    let mut scene: Option<RenderTexture2D> = None;
    let mut voice = VoiceState::default();
    let mut samples = Vec::with_capacity(WINDOW_SAMPLES);
    let mut error: Option<String> = None;

    while !rl.window_should_close() && !rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
        if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            voice.gate_db = (voice.gate_db + 2.0).min(-26.0);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            voice.gate_db = (voice.gate_db - 2.0).max(-90.0);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            match load_images(&mut rl, &thread, &asset_dir) {
                Ok(new_images) => {
                    images = new_images;
                    error = None;
                }
                Err(e) => {
                    eprintln!("reload failed: {e}");
                    error = Some(format!("Reload failed: {e}"));
                }
            }
        }

        mic.latest(WINDOW_SAMPLES, &mut samples);
        let db = voice::amplitude_to_db(voice::rms(&samples));
        let state = voice.update(db, rl.get_frame_time());
        let (width, height) = (rl.get_screen_width().max(1), rl.get_screen_height().max(1));
        let (w, h) = (width as f32, height as f32);
        let ui = ui_scale(w, h);
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
        let texture = &images[state.index()];
        let (tw, th) = (texture.width() as f32, texture.height() as f32);
        let avatar = avatar_rect(w, h, tw, th);
        {
            let mut target = rl.begin_texture_mode(&thread, scene);
            let mut d = target.begin_mode2D(Camera2D {
                offset: Vector2::zero(),
                target: Vector2::zero(),
                rotation: 0.0,
                zoom: SUPERSAMPLE_SCALE as f32,
            });
            d.clear_background(Color::new(18, 18, 24, 255));
            d.draw_texture_pro(
                texture,
                Rectangle::new(0.0, 0.0, tw, th),
                avatar,
                Vector2::zero(),
                0.0,
                Color::WHITE,
            );
            d.draw_text(
                &format!(
                    "{}  |  input {db:.0} dBFS  |  gate {:.0} dB",
                    state.label(),
                    voice.gate_db
                ),
                (20.0 * ui).round() as i32,
                (18.0 * ui).round() as i32,
                (22.0 * ui).round().max(1.0) as i32,
                Color::RAYWHITE,
            );
            d.draw_text(
                "Up/Down: gate    R: reload images    Esc: quit",
                (20.0 * ui).round() as i32,
                (h - 34.0 * ui).round() as i32,
                (18.0 * ui).round().max(1.0) as i32,
                Color::LIGHTGRAY,
            );
            if let Some(message) = &error {
                d.draw_text(
                    message,
                    (20.0 * ui).round() as i32,
                    (52.0 * ui).round() as i32,
                    (18.0 * ui).round().max(1.0) as i32,
                    Color::RED,
                );
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

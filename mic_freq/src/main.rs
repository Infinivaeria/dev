use mic_freq::audio::MicCapture;
use mic_freq::voice::{self, AvatarState, VoiceState};
use raylib::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};

const WINDOW_SAMPLES: usize = 2048;
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
        textures.push(rl.load_texture_from_image(thread, &image)?);
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
        .msaa_4x()
        .build();
    rl.set_target_fps(60);
    rl.set_exit_key(None);
    let mut images = load_images(&mut rl, &thread, &asset_dir)?;
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
        let (w, h) = (rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let texture = &images[state.index()];
        let (tw, th) = (texture.width() as f32, texture.height() as f32);
        let scale = ((w - 60.0) / tw).min((h - 150.0) / th).max(0.01);
        let size = Vector2::new(tw * scale, th * scale);
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(18, 18, 24, 255));
        d.draw_texture_ex(
            texture,
            Vector2::new((w - size.x) / 2.0, (h - size.y) / 2.0),
            0.0,
            scale,
            Color::WHITE,
        );
        d.draw_text(
            &format!(
                "{}  |  input {db:.0} dBFS  |  gate {:.0} dB",
                state.label(),
                voice.gate_db
            ),
            20,
            18,
            22,
            Color::RAYWHITE,
        );
        d.draw_text(
            "Up/Down: gate    R: reload images    Esc: quit",
            20,
            h as i32 - 34,
            18,
            Color::LIGHTGRAY,
        );
        if let Some(message) = &error {
            d.draw_text(message, 20, 52, 18, Color::RED);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

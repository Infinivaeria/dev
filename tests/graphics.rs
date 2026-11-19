#[path = "../src/graphics.rs"]
mod graphics;

use graphics::*;
use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = Path::new("target").join(format!(
            "graphics-test-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn join(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn save_image(path: &Path, width: u32, height: u32, pixel: [u8; 4]) {
    RgbaImage::from_pixel(width, height, Rgba(pixel))
        .save_with_format(path, ImageFormat::Png)
        .unwrap();
}

#[test]
fn blends_visible_layers_with_opacity_and_offsets() {
    let dir = TestDir::new();
    let base = dir.join("base.png");
    let top = dir.join("top.png");
    save_image(&base, 2, 2, [255, 0, 0, 255]);
    save_image(&top, 1, 1, [0, 0, 255, 255]);
    let mut project = GraphicsProject::new(2, 2).unwrap();
    project.layers.push(GraphicsLayer::new(base, "base"));
    let mut top_layer = GraphicsLayer::new(top, "top");
    top_layer.x = 1;
    top_layer.y = 1;
    top_layer.opacity = 0.5;
    project.layers.push(top_layer);

    let output = blend_layers(&project).unwrap();
    assert_eq!(output.get_pixel(0, 0).0, [255, 0, 0, 255]);
    assert_eq!(output.get_pixel(1, 1).0, [127, 0, 128, 255]);

    let hidden = dir.join("hidden.png");
    save_image(&hidden, 2, 2, [0, 255, 0, 255]);
    let mut hidden_layer = GraphicsLayer::new(hidden, "hidden");
    hidden_layer.visible = false;
    project.layers.push(hidden_layer);
    assert_eq!(blend_layers(&project).unwrap(), output);
}

#[test]
fn applies_geometric_and_color_operations() {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(3, 2, |x, y| {
        Rgba([x as u8, y as u8, 20, 255])
    }));
    let cropped = apply_operations(
        image.clone(),
        &[
            ImageOperation::Crop {
                x: 1,
                y: 0,
                width: 2,
                height: 1,
            },
            ImageOperation::Rotate90,
            ImageOperation::FlipHorizontal,
            ImageOperation::Brightness { amount: 10 },
        ],
    )
    .unwrap();
    assert_eq!(cropped.dimensions(), (1, 2));
    assert_eq!(cropped.to_rgba8().get_pixel(0, 0).0[0], 11);

    let resized = apply_operation(
        &image,
        ImageOperation::Resize {
            width: 8,
            height: 5,
        },
    )
    .unwrap();
    assert_eq!(resized.dimensions(), (8, 5));
    assert!(apply_operation(&image, ImageOperation::GaussianBlur { sigma: f32::NAN }).is_err());
}

#[test]
fn saves_and_loads_project_json() {
    let dir = TestDir::new();
    let path = dir.join("project.json");
    let mut project = GraphicsProject::new(640, 480).unwrap();
    let mut layer = GraphicsLayer::new("images/face.png", "face");
    layer.opacity = 0.75;
    layer.x = -5;
    layer.y = 12;
    project.layers.push(layer);
    project.selected_image = Some("images/face.png".into());

    save_project(&project, &path).unwrap();
    let loaded = load_project(&path).unwrap();
    assert_eq!(loaded, project);
    assert!(std::fs::read_to_string(path)
        .unwrap()
        .contains("\"opacity\": 0.75"));
}

#[test]
fn animated_gif_round_trips_frames() {
    let dir = TestDir::new();
    let path = dir.join("animated.gif");
    let frames = vec![
        GifFrame {
            image: RgbaImage::from_pixel(3, 2, Rgba([255, 0, 0, 255])),
            delay_ms: 100,
        },
        GifFrame {
            image: RgbaImage::from_pixel(3, 2, Rgba([0, 0, 255, 255])),
            delay_ms: 200,
        },
    ];
    encode_animated_gif(&path, &frames, true).unwrap();
    let decoded = decode_animated_gif(path).unwrap();
    assert_eq!(decoded.len(), 2);
    assert_eq!(
        (decoded[0].image.width(), decoded[0].image.height()),
        (3, 2)
    );
    assert_eq!(decoded[0].delay_ms, 100);
    assert_eq!(decoded[1].delay_ms, 200);
    assert_eq!(decoded[1].image.get_pixel(0, 0).0[2], 255);
}

#[test]
fn writes_all_pngtuber_poses_and_blink_variants() {
    let dir = TestDir::new();
    let source = dir.join("source.png");
    save_image(&source, 8, 8, [220, 180, 150, 255]);
    let poses = PngTuberTransforms {
        idle: vec![],
        talking: vec![ImageOperation::Brightness { amount: 5 }],
        loud: vec![ImageOperation::Resize {
            width: 10,
            height: 10,
        }],
    };
    let eyes = [EyeBox {
        x: 2,
        y: 3,
        width: 3,
        height: 1,
    }];

    let written = write_pngtuber_poses(&source, dir.join("poses"), &poses, &eyes).unwrap();
    assert_eq!(written.len(), 6);
    for name in [
        "idle.png",
        "idle_blink.png",
        "talking.png",
        "talking_blink.png",
        "loud.png",
        "loud_blink.png",
    ] {
        assert!(dir.join(&format!("poses/{name}")).is_file());
    }
    let blink = image::open(dir.join("poses/idle_blink.png"))
        .unwrap()
        .to_rgba8();
    assert_eq!(blink.get_pixel(2, 3).0, [20, 20, 24, 255]);
    assert_eq!(
        image::open(dir.join("poses/loud.png"))
            .unwrap()
            .dimensions(),
        (10, 10)
    );
}

#[test]
fn rejects_unsafe_or_malformed_ffmpeg_arguments() {
    assert!(validate_ffmpeg_args(&[]).is_ok());
    assert!(
        validate_ffmpeg_args(&["-c:v".into(), "libx264".into(), "-crf".into(), "20".into()])
            .is_ok()
    );
    assert!(validate_ffmpeg_args(&["-i".into(), "other.mp4".into()]).is_err());
    assert!(validate_ffmpeg_args(&["-vf".into()]).is_err());
    assert!(validate_ffmpeg_args(&["-vf".into(), "-y".into()]).is_err());
}

#[test]
fn ffmpeg_helpers_work_when_ffmpeg_is_available() {
    if !ffmpeg_available() || !ffprobe_available() {
        return;
    }
    let dir = TestDir::new();
    for index in 1..=2 {
        save_image(
            &dir.join(&format!("frame_{index:04}.png")),
            16,
            16,
            [index as u8 * 80, 10, 20, 255],
        );
    }
    let video = dir.join("small.mp4");
    render_png_sequence(dir.join("frame_%04d.png"), &video, 2.0).unwrap();
    let metadata = probe_video(&video).unwrap();
    assert_eq!((metadata.width, metadata.height), (Some(16), Some(16)));
    assert!(metadata.duration_seconds.is_some());
    let frames = extract_representative_frames(&video, dir.join("extracted"), 1).unwrap();
    assert_eq!(frames.len(), 1);
}

#[cfg(feature = "native-video")]
#[test]
fn video_rs_probes_and_extracts_without_ffmpeg_commands() {
    use ndarray::Array3;
    use video_rs::encode::{Encoder, Settings};
    use video_rs::time::Time;

    let dir = TestDir::new();
    let video = dir.join("native.mp4");
    video_rs::init().unwrap();
    let settings = Settings::preset_h264_yuv420p(16, 16, false);
    let mut encoder = Encoder::new(video.as_path(), settings).unwrap();
    let step = Time::from_nth_of_a_second(2);
    let mut timestamp = Time::zero();
    for red in [20, 220] {
        let frame = Array3::from_shape_fn(
            (16, 16, 3),
            |(_, _, channel)| {
                if channel == 0 {
                    red
                } else {
                    30
                }
            },
        );
        encoder.encode(&frame, timestamp).unwrap();
        timestamp = timestamp.aligned_with(step).add();
    }
    encoder.finish().unwrap();
    let metadata = probe_video(&video).unwrap();
    assert_eq!((metadata.width, metadata.height), (Some(16), Some(16)));
    let frames = extract_representative_frames(&video, dir.join("native-frames"), 2).unwrap();
    assert_eq!(frames.len(), 2);
}

#[cfg(target_os = "linux")]
#[test]
fn graphics_studio_rotates_images_and_exports_pngtuber_poses() {
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    use std::process::{Command, Stdio};
    let dir = TestDir::new();
    let source = dir.join("avatar.png");
    save_image(&source, 12, 8, [220, 180, 150, 255]);
    let driver = dir.join("keys");
    let compiled = Command::new("cc")
        .args(["-Wall", "-Wextra", "-Werror"])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/x11_key_driver.c"
        ))
        .args(["-lX11", "-o"])
        .arg(&driver)
        .status()
        .unwrap();
    assert!(compiled.success());
    let mut child = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .args(["--graphics-studio", source.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let sent = Command::new(driver)
        .arg(child.id().to_string())
        .args([
            "wait:900",
            "click:270,580",
            "wait:200",
            "click:535,70",
            "wait:350",
            "key:Escape",
        ])
        .status()
        .unwrap();
    assert!(sent.success());
    let status = child.wait().unwrap();
    assert!(status.success());
    let edited = source.with_file_name("avatar-edited.png");
    assert_eq!(image::open(edited).unwrap().dimensions(), (8, 12));
    let poses = dir.join("pngtuber-export");
    for name in [
        "idle.png",
        "idle_blink.png",
        "talking.png",
        "talking_blink.png",
        "loud.png",
        "loud_blink.png",
    ] {
        assert!(poses.join(name).is_file());
    }
}

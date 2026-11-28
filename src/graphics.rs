use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
use image::imageops::FilterType;
use image::{AnimationDecoder, Delay, DynamicImage, Frame, ImageFormat, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub enum GraphicsError {
    Io(std::io::Error),
    Image(image::ImageError),
    Json(serde_json::Error),
    InvalidInput(String),
    Command {
        program: &'static str,
        stderr: String,
    },
    #[cfg(feature = "native-video")]
    Video(String),
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "graphics I/O error: {error}"),
            Self::Image(error) => write!(f, "image processing error: {error}"),
            Self::Json(error) => write!(f, "graphics project JSON error: {error}"),
            Self::InvalidInput(message) => write!(f, "invalid input: {message}"),
            Self::Command { program, stderr } => {
                write!(f, "{program} failed: {stderr}")
            }
            #[cfg(feature = "native-video")]
            Self::Video(message) => write!(f, "video-rs failed: {message}"),
        }
    }
}

impl std::error::Error for GraphicsError {}
impl From<std::io::Error> for GraphicsError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<image::ImageError> for GraphicsError {
    fn from(value: image::ImageError) -> Self {
        Self::Image(value)
    }
}
impl From<serde_json::Error> for GraphicsError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub type GraphicsResult<T> = Result<T, GraphicsError>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphicsLayer {
    pub path: PathBuf,
    pub name: String,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub x: i64,
    #[serde(default)]
    pub y: i64,
    #[serde(default = "default_visible")]
    pub visible: bool,
}

fn default_opacity() -> f32 {
    1.0
}
fn default_visible() -> bool {
    true
}

impl GraphicsLayer {
    pub fn new(path: impl Into<PathBuf>, name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            opacity: 1.0,
            x: 0,
            y: 0,
            visible: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphicsProject {
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub layers: Vec<GraphicsLayer>,
    #[serde(default)]
    pub selected_image: Option<PathBuf>,
}

impl GraphicsProject {
    pub fn new(width: u32, height: u32) -> GraphicsResult<Self> {
        validate_dimensions(width, height)?;
        Ok(Self {
            width,
            height,
            layers: Vec::new(),
            selected_image: None,
        })
    }

    pub fn load(path: impl AsRef<Path>) -> GraphicsResult<Self> {
        let project = serde_json::from_slice::<Self>(&fs::read(path)?)?;
        validate_dimensions(project.width, project.height)?;
        for layer in &project.layers {
            validate_opacity(layer.opacity)?;
        }
        Ok(project)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> GraphicsResult<()> {
        validate_dimensions(self.width, self.height)?;
        for layer in &self.layers {
            validate_opacity(layer.opacity)?;
        }
        fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

pub fn load_project(path: impl AsRef<Path>) -> GraphicsResult<GraphicsProject> {
    GraphicsProject::load(path)
}

pub fn save_project(project: &GraphicsProject, path: impl AsRef<Path>) -> GraphicsResult<()> {
    project.save(path)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ImageOperation {
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Resize {
        width: u32,
        height: u32,
    },
    Rotate90,
    FlipHorizontal,
    FlipVertical,
    Grayscale,
    Invert,
    Brightness {
        amount: i32,
    },
    Contrast {
        amount: f32,
    },
    GaussianBlur {
        sigma: f32,
    },
}

pub fn apply_operation(
    image: &DynamicImage,
    operation: ImageOperation,
) -> GraphicsResult<DynamicImage> {
    match operation {
        ImageOperation::Crop {
            x,
            y,
            width,
            height,
        } => {
            if x.checked_add(width)
                .is_none_or(|right| right > image.width())
                || y.checked_add(height)
                    .is_none_or(|bottom| bottom > image.height())
                || width == 0
                || height == 0
            {
                return Err(GraphicsError::InvalidInput(format!(
                    "crop ({x},{y}) {width}x{height} is outside image {}x{}",
                    image.width(),
                    image.height()
                )));
            }
            Ok(image.crop_imm(x, y, width, height))
        }
        ImageOperation::Resize { width, height } => {
            validate_dimensions(width, height)?;
            Ok(image.resize_exact(width, height, FilterType::Lanczos3))
        }
        ImageOperation::Rotate90 => Ok(image.rotate90()),
        ImageOperation::FlipHorizontal => Ok(image.fliph()),
        ImageOperation::FlipVertical => Ok(image.flipv()),
        ImageOperation::Grayscale => Ok(DynamicImage::ImageLuma8(image.to_luma8())),
        ImageOperation::Invert => {
            let mut result = image.clone();
            result.invert();
            Ok(result)
        }
        ImageOperation::Brightness { amount } => Ok(image.brighten(amount)),
        ImageOperation::Contrast { amount } => {
            if !amount.is_finite() {
                return Err(GraphicsError::InvalidInput(
                    "contrast must be finite".into(),
                ));
            }
            Ok(image.adjust_contrast(amount))
        }
        ImageOperation::GaussianBlur { sigma } => {
            if !sigma.is_finite() || sigma < 0.0 {
                return Err(GraphicsError::InvalidInput(
                    "blur sigma must be finite and non-negative".into(),
                ));
            }
            Ok(DynamicImage::ImageRgba8(
                imageproc::filter::gaussian_blur_f32(&image.to_rgba8(), sigma),
            ))
        }
    }
}

pub fn apply_operations(
    mut image: DynamicImage,
    operations: &[ImageOperation],
) -> GraphicsResult<DynamicImage> {
    for operation in operations {
        image = apply_operation(&image, *operation)?;
    }
    Ok(image)
}

pub fn read_image(path: impl AsRef<Path>) -> GraphicsResult<DynamicImage> {
    image::open(path).map_err(Into::into)
}

pub fn blend_layers(project: &GraphicsProject) -> GraphicsResult<RgbaImage> {
    validate_dimensions(project.width, project.height)?;
    let mut canvas = RgbaImage::new(project.width, project.height);
    for layer in &project.layers {
        if !layer.visible {
            continue;
        }
        validate_opacity(layer.opacity)?;
        let source = read_image(&layer.path)?.into_rgba8();
        blend_layer(&mut canvas, &source, layer.x, layer.y, layer.opacity)?;
    }
    Ok(canvas)
}

pub fn blend_layer(
    canvas: &mut RgbaImage,
    source: &RgbaImage,
    x: i64,
    y: i64,
    opacity: f32,
) -> GraphicsResult<()> {
    validate_opacity(opacity)?;
    for sy in 0..source.height() {
        let dy = y + i64::from(sy);
        if dy < 0 || dy >= i64::from(canvas.height()) {
            continue;
        }
        for sx in 0..source.width() {
            let dx = x + i64::from(sx);
            if dx < 0 || dx >= i64::from(canvas.width()) {
                continue;
            }
            let mut pixel = *source.get_pixel(sx, sy);
            pixel[3] = (f32::from(pixel[3]) * opacity).round().clamp(0.0, 255.0) as u8;
            alpha_over(canvas.get_pixel_mut(dx as u32, dy as u32), pixel);
        }
    }
    Ok(())
}

fn alpha_over(destination: &mut Rgba<u8>, source: Rgba<u8>) {
    let sa = u32::from(source[3]);
    let da = u32::from(destination[3]);
    let out_a = sa + ((da * (255 - sa) + 127) / 255);
    if out_a == 0 {
        *destination = Rgba([0, 0, 0, 0]);
        return;
    }
    let inverse = 255 - sa;
    for channel in 0..3 {
        let numerator =
            u32::from(source[channel]) * sa * 255 + u32::from(destination[channel]) * da * inverse;
        destination[channel] = (numerator / (out_a * 255)) as u8;
    }
    destination[3] = out_a as u8;
}

pub fn export_flattened_png(
    project: &GraphicsProject,
    output: impl AsRef<Path>,
) -> GraphicsResult<()> {
    let canvas = blend_layers(project)?;
    canvas.save_with_format(output, ImageFormat::Png)?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct GifFrame {
    pub image: RgbaImage,
    pub delay_ms: u32,
}

pub fn decode_animated_gif(path: impl AsRef<Path>) -> GraphicsResult<Vec<GifFrame>> {
    let decoder = GifDecoder::new(BufReader::new(File::open(path)?))?;
    let frames = decoder.into_frames().collect_frames()?;
    Ok(frames
        .into_iter()
        .map(|frame| {
            let (numerator, denominator) = frame.delay().numer_denom_ms();
            let delay_ms = numerator.checked_div(denominator.max(1)).unwrap_or(0);
            GifFrame {
                image: frame.into_buffer(),
                delay_ms,
            }
        })
        .collect())
}

pub fn encode_animated_gif(
    path: impl AsRef<Path>,
    frames: &[GifFrame],
    repeat: bool,
) -> GraphicsResult<()> {
    if frames.is_empty() {
        return Err(GraphicsError::InvalidInput(
            "a GIF requires at least one frame".into(),
        ));
    }
    let writer = BufWriter::new(File::create(path)?);
    let mut encoder = GifEncoder::new(writer);
    encoder.set_repeat(if repeat {
        Repeat::Infinite
    } else {
        Repeat::Finite(0)
    })?;
    let frames = frames.iter().map(|frame| {
        Frame::from_parts(
            frame.image.clone(),
            0,
            0,
            Delay::from_numer_denom_ms(frame.delay_ms, 1),
        )
    });
    encoder.encode_frames(frames)?;
    Ok(())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PngTuberTransforms {
    #[serde(default)]
    pub idle: Vec<ImageOperation>,
    #[serde(default)]
    pub talking: Vec<ImageOperation>,
    #[serde(default)]
    pub loud: Vec<ImageOperation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EyeBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn write_pngtuber_poses(
    source: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    transforms: &PngTuberTransforms,
    blink_boxes: &[EyeBox],
) -> GraphicsResult<Vec<PathBuf>> {
    let source = read_image(source)?;
    fs::create_dir_all(output_dir.as_ref())?;
    let poses = [
        ("idle", transforms.idle.as_slice()),
        ("talking", transforms.talking.as_slice()),
        ("loud", transforms.loud.as_slice()),
    ];
    let mut written = Vec::new();
    for (name, operations) in poses {
        let image = apply_operations(source.clone(), operations)?.into_rgba8();
        let path = output_dir.as_ref().join(format!("{name}.png"));
        image.save_with_format(&path, ImageFormat::Png)?;
        written.push(path);

        let mut blink = image;
        overlay_blink_bars(&mut blink, blink_boxes)?;
        let path = output_dir.as_ref().join(format!("{name}_blink.png"));
        blink.save_with_format(&path, ImageFormat::Png)?;
        written.push(path);
    }
    Ok(written)
}

pub fn overlay_blink_bars(image: &mut RgbaImage, boxes: &[EyeBox]) -> GraphicsResult<()> {
    for eye in boxes {
        if eye.width == 0
            || eye.height == 0
            || eye.x.saturating_add(eye.width) > image.width()
            || eye.y.saturating_add(eye.height) > image.height()
        {
            return Err(GraphicsError::InvalidInput(format!(
                "blink box ({},{}) {}x{} is outside pose {}x{}",
                eye.x,
                eye.y,
                eye.width,
                eye.height,
                image.width(),
                image.height()
            )));
        }
        for y in eye.y..eye.y + eye.height {
            for x in eye.x..eye.x + eye.width {
                *image.get_pixel_mut(x, y) = Rgba([20, 20, 24, 255]);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub duration_seconds: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub tags: HashMap<String, String>,
}

pub fn ffmpeg_available() -> bool {
    command_available("ffmpeg")
}

pub fn ffprobe_available() -> bool {
    command_available("ffprobe")
}

pub fn video_tools_available() -> bool {
    #[cfg(feature = "native-video")]
    {
        return video_rs::init().is_ok();
    }
    #[cfg(not(feature = "native-video"))]
    {
        ffmpeg_available() && ffprobe_available()
    }
}

fn command_available(program: &'static str) -> bool {
    video_command(program)
        .arg("-version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn video_command(program: &'static str) -> Command {
    let adjacent = std::env::current_exe().ok().and_then(|executable| {
        let parent = executable.parent()?.to_path_buf();
        [
            parent.join("ffmpeg/bin").join(program),
            parent.join("ffmpeg").join(program),
        ]
        .into_iter()
        .find(|path| path.is_file())
    });
    Command::new(adjacent.map_or_else(|| PathBuf::from(program), |path| path))
}

pub fn probe_video(path: impl AsRef<Path>) -> GraphicsResult<VideoMetadata> {
    let path = path.as_ref();
    ensure_input(path)?;
    #[cfg(feature = "native-video")]
    {
        if let Ok(metadata) = probe_video_native(path) {
            return Ok(metadata);
        }
    }
    if !ffprobe_available() {
        return Err(GraphicsError::InvalidInput(
            "video probing requires the native-video feature or ffprobe".into(),
        ));
    }
    let output = video_command("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,avg_frame_rate:stream_tags*:format=duration:format_tags*",
            "-of",
            "json",
        ])
        .arg(path)
        .output()?;
    if !output.status.success() {
        return Err(command_error("ffprobe", output.stderr));
    }
    let document: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let stream = document
        .get("streams")
        .and_then(|value| value.as_array())
        .and_then(|streams| streams.first());
    let duration_seconds = document.pointer("/format/duration").and_then(value_as_f64);
    let mut tags = HashMap::new();
    if let Some(format_tags) = document.pointer("/format/tags").and_then(|value| value.as_object()) {
        for (key, value) in format_tags {
            if let Some(value) = value.as_str() {
                tags.insert(key.to_ascii_lowercase(), value.trim().to_owned());
            }
        }
    }
    if let Some(stream_tags) = stream
        .and_then(|value| value.get("tags"))
        .and_then(|value| value.as_object())
    {
        for (key, value) in stream_tags {
            if let Some(value) = value.as_str() {
                tags.entry(key.to_ascii_lowercase())
                    .or_insert_with(|| value.trim().to_owned());
            }
        }
    }
    Ok(VideoMetadata {
        duration_seconds,
        width: stream
            .and_then(|s| s.get("width"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        height: stream
            .and_then(|s| s.get("height"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        fps: stream
            .and_then(|s| s.get("avg_frame_rate"))
            .and_then(|v| v.as_str())
            .and_then(parse_fraction),
        tags,
    })
}

pub fn extract_representative_frames(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    count: u32,
) -> GraphicsResult<Vec<PathBuf>> {
    ensure_input(input.as_ref())?;
    if count == 0 {
        return Err(GraphicsError::InvalidInput(
            "representative frame count must be greater than zero".into(),
        ));
    }
    #[cfg(feature = "native-video")]
    {
        if let Ok(paths) =
            extract_representative_frames_native(input.as_ref(), output_dir.as_ref(), count)
        {
            return Ok(paths);
        }
    }
    if !ffmpeg_available() || !ffprobe_available() {
        return Err(GraphicsError::InvalidInput(
            "frame extraction requires the native-video feature or ffmpeg/ffprobe".into(),
        ));
    }
    let metadata = probe_video(input.as_ref())?;
    let duration = metadata
        .duration_seconds
        .filter(|duration| duration.is_finite() && *duration > 0.0)
        .ok_or_else(|| GraphicsError::InvalidInput("video has no positive duration".into()))?;
    fs::create_dir_all(output_dir.as_ref())?;
    let pattern = output_dir.as_ref().join("frame_%06d.png");
    let rate = f64::from(count) / duration;
    let filter = format!("fps={rate:.12}");
    let output = video_command("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(input.as_ref())
        .args(["-vf", &filter, "-frames:v"])
        .arg(count.to_string())
        .arg(&pattern)
        .output()?;
    if !output.status.success() {
        return Err(command_error("ffmpeg", output.stderr));
    }
    let mut paths = (1..=count)
        .map(|index| output_dir.as_ref().join(format!("frame_{index:06}.png")))
        .collect::<Vec<_>>();
    paths.retain(|path| path.is_file());
    if paths.is_empty() {
        return Err(GraphicsError::InvalidInput(
            "ffmpeg produced no representative frames".into(),
        ));
    }
    Ok(paths)
}

pub fn render_png_sequence(
    input_pattern: impl AsRef<Path>,
    output: impl AsRef<Path>,
    fps: f64,
) -> GraphicsResult<()> {
    validate_rate(fps)?;
    validate_sequence_pattern(input_pattern.as_ref())?;
    ensure_output_parent(output.as_ref())?;
    let result = video_command("ffmpeg")
        .args(["-v", "error", "-y", "-framerate"])
        .arg(format_rate(fps))
        .arg("-i")
        .arg(input_pattern.as_ref())
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg(output.as_ref())
        .output()?;
    command_success("ffmpeg", result.status.success(), result.stderr)
}

pub fn validate_ffmpeg_args(args: &[String]) -> GraphicsResult<()> {
    const OPTIONS: &[&str] = &[
        "-c:v",
        "-c:a",
        "-b:v",
        "-b:a",
        "-crf",
        "-preset",
        "-vf",
        "-af",
        "-pix_fmt",
        "-r",
        "-ar",
        "-ac",
        "-movflags",
        "-profile:v",
        "-level",
    ];
    let mut index = 0;
    while index < args.len() {
        let option = &args[index];
        if !OPTIONS.contains(&option.as_str()) {
            return Err(GraphicsError::InvalidInput(format!(
                "unsupported or unsafe ffmpeg option: {option}"
            )));
        }
        let value = args.get(index + 1).ok_or_else(|| {
            GraphicsError::InvalidInput(format!("ffmpeg option {option} requires a value"))
        })?;
        if value.is_empty() || value.starts_with('-') || value.chars().any(char::is_control) {
            return Err(GraphicsError::InvalidInput(format!(
                "invalid value for ffmpeg option {option}"
            )));
        }
        index += 2;
    }
    Ok(())
}

pub fn transcode_video(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
    args: &[String],
) -> GraphicsResult<()> {
    ensure_input(input.as_ref())?;
    ensure_output_parent(output.as_ref())?;
    validate_ffmpeg_args(args)?;
    #[cfg(feature = "native-video")]
    {
        if native_transcode_supported(args) {
            return transcode_video_native(input.as_ref(), output.as_ref());
        }
    }
    if !ffmpeg_available() {
        return Err(GraphicsError::InvalidInput(
            "transcoding requires the native-video feature or ffmpeg".into(),
        ));
    }
    let result = video_command("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(input.as_ref())
        .args(args)
        .arg(output.as_ref())
        .output()?;
    command_success("ffmpeg", result.status.success(), result.stderr)
}

#[cfg(feature = "native-video")]
fn native_video_error(error: impl fmt::Display) -> GraphicsError {
    GraphicsError::Video(error.to_string())
}

#[cfg(feature = "native-video")]
fn probe_video_native(path: &Path) -> GraphicsResult<VideoMetadata> {
    video_rs::init().map_err(native_video_error)?;
    let decoder = video_rs::Decoder::new(path).map_err(native_video_error)?;
    let (width, height) = decoder.size();
    let duration = decoder.duration().ok().map(|value| value.as_secs_f64());
    Ok(VideoMetadata {
        duration_seconds: duration,
        width: Some(width),
        height: Some(height),
        fps: Some(f64::from(decoder.frame_rate())),
        tags: HashMap::new(),
    })
}

#[cfg(feature = "native-video")]
fn extract_representative_frames_native(
    input: &Path,
    output_dir: &Path,
    count: u32,
) -> GraphicsResult<Vec<PathBuf>> {
    video_rs::init().map_err(native_video_error)?;
    let metadata = probe_video_native(input)?;
    let duration = metadata
        .duration_seconds
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| GraphicsError::InvalidInput("video has no positive duration".into()))?;
    fs::create_dir_all(output_dir)?;
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let timestamp_ms = ((index as f64 + 0.5) * duration / count as f64 * 1000.0) as i64;
        let mut decoder = video_rs::Decoder::new(input).map_err(native_video_error)?;
        decoder.seek(timestamp_ms).map_err(native_video_error)?;
        let (_, frame) = decoder.decode().map_err(native_video_error)?;
        let image = frame_to_rgba(&frame)?;
        let output = output_dir.join(format!("frame_{:06}.png", index + 1));
        image.save_with_format(&output, ImageFormat::Png)?;
        paths.push(output);
    }
    Ok(paths)
}

#[cfg(feature = "native-video")]
fn frame_to_rgba(frame: &video_rs::Frame) -> GraphicsResult<RgbaImage> {
    let shape = frame.shape();
    if shape.len() != 3 || shape[2] < 3 {
        return Err(GraphicsError::InvalidInput(
            "decoded frame is not RGB".into(),
        ));
    }
    let (height, width) = (shape[0], shape[1]);
    let mut rgba = Vec::with_capacity(height * width * 4);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&[frame[[y, x, 0]], frame[[y, x, 1]], frame[[y, x, 2]], 255]);
        }
    }
    RgbaImage::from_raw(width as u32, height as u32, rgba)
        .ok_or_else(|| GraphicsError::InvalidInput("decoded frame shape is invalid".into()))
}

#[cfg(feature = "native-video")]
fn native_transcode_supported(args: &[String]) -> bool {
    args.is_empty()
        || args
            == [
                "-c:v", "libx264", "-crf", "23", "-pix_fmt", "yuv420p", "-c:a", "aac",
            ]
}

#[cfg(feature = "native-video")]
fn transcode_video_native(input: &Path, output: &Path) -> GraphicsResult<()> {
    video_rs::init().map_err(native_video_error)?;
    let mut decoder = video_rs::Decoder::new(input).map_err(native_video_error)?;
    let (width, height) = decoder.size();
    let settings =
        video_rs::encode::Settings::preset_h264_yuv420p(width as usize, height as usize, false);
    let mut encoder = video_rs::Encoder::new(output, settings).map_err(native_video_error)?;
    loop {
        match decoder.decode() {
            Ok((timestamp, frame)) => encoder
                .encode(&frame, timestamp)
                .map_err(native_video_error)?,
            Err(_) => break,
        }
    }
    encoder.finish().map_err(native_video_error)
}

fn validate_dimensions(width: u32, height: u32) -> GraphicsResult<()> {
    if width == 0 || height == 0 {
        return Err(GraphicsError::InvalidInput(
            "dimensions must be greater than zero".into(),
        ));
    }
    Ok(())
}

fn validate_opacity(opacity: f32) -> GraphicsResult<()> {
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err(GraphicsError::InvalidInput(format!(
            "layer opacity {opacity} must be finite and between 0 and 1"
        )));
    }
    Ok(())
}

fn validate_rate(fps: f64) -> GraphicsResult<()> {
    if !fps.is_finite() || fps <= 0.0 {
        return Err(GraphicsError::InvalidInput(
            "frame rate must be finite and greater than zero".into(),
        ));
    }
    Ok(())
}

fn validate_sequence_pattern(pattern: &Path) -> GraphicsResult<()> {
    let text = pattern.to_string_lossy();
    if !text.contains('%') || !text.contains('d') {
        return Err(GraphicsError::InvalidInput(
            "PNG sequence input must contain a printf frame pattern such as %06d".into(),
        ));
    }
    Ok(())
}

fn ensure_input(path: &Path) -> GraphicsResult<()> {
    if !path.is_file() {
        return Err(GraphicsError::InvalidInput(format!(
            "input file does not exist: {}",
            path.display()
        )));
    }
    Ok(())
}

fn ensure_output_parent(path: &Path) -> GraphicsResult<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

fn parse_fraction(value: &str) -> Option<f64> {
    let (numerator, denominator) = value.split_once('/')?;
    let denominator = denominator.parse::<f64>().ok()?;
    (denominator != 0.0).then(|| numerator.parse::<f64>().ok().map(|n| n / denominator))?
}

fn format_rate(rate: f64) -> String {
    format!("{rate:.8}")
}

fn command_error(program: &'static str, stderr: Vec<u8>) -> GraphicsError {
    GraphicsError::Command {
        program,
        stderr: String::from_utf8_lossy(&stderr).trim().to_owned(),
    }
}

fn command_success(program: &'static str, success: bool, stderr: Vec<u8>) -> GraphicsResult<()> {
    if success {
        Ok(())
    } else {
        Err(command_error(program, stderr))
    }
}

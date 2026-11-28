//! Scalable raylib front end for the graphics engine.

use crate::graphics::{
    self, decode_animated_gif, read_image, video_tools_available, GraphicsProject, ImageOperation,
    PngTuberTransforms, VideoMetadata,
};
use crate::imaging::upload_rgba;
use image::{DynamicImage, ImageFormat};
use raylib::prelude::*;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

pub const LOGICAL_WIDTH: f32 = 1100.0;
pub const LOGICAL_HEIGHT: f32 = 760.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiGeometry {
    pub scale: f32,
    pub offset: Vector2,
}

impl UiGeometry {
    pub fn from_screen(width: f32, height: f32) -> Self {
        let scale = (width.max(1.0) / LOGICAL_WIDTH).min(height.max(1.0) / LOGICAL_HEIGHT);
        Self {
            scale,
            offset: Vector2::new(
                (width - LOGICAL_WIDTH * scale) * 0.5,
                (height - LOGICAL_HEIGHT * scale) * 0.5,
            ),
        }
    }

    pub fn to_logical(self, point: Vector2) -> Vector2 {
        (point - self.offset) * (1.0 / self.scale.max(f32::EPSILON))
    }
}

pub fn ui_geometry(width: f32, height: f32) -> UiGeometry {
    UiGeometry::from_screen(width, height)
}

pub fn hit_cell(logical_point: Vector2, bounds: Rectangle) -> bool {
    bounds.check_collision_point_rec(logical_point)
}

#[derive(Clone)]
struct Snapshot {
    image: DynamicImage,
    path: PathBuf,
}

struct UiState {
    project: GraphicsProject,
    project_path: Option<PathBuf>,
    selected: Option<usize>,
    layer_scroll: usize,
    image: Option<DynamicImage>,
    texture: Option<Texture2D>,
    dirty_texture: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    gif_frames: Vec<graphics::GifFrame>,
    gif_index: usize,
    gif_playing: bool,
    gif_clock_ms: f32,
    video_path: Option<PathBuf>,
    video_metadata: Option<VideoMetadata>,
    status: String,
    is_error: bool,
    help_open: bool,
    close: bool,
}

impl UiState {
    fn new(path: Option<PathBuf>) -> Self {
        let mut state = Self {
            project: GraphicsProject::new(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32)
                .expect("valid logical project"),
            project_path: None,
            selected: None,
            layer_scroll: 0,
            image: None,
            texture: None,
            dirty_texture: false,
            undo: Vec::new(),
            redo: Vec::new(),
            gif_frames: Vec::new(),
            gif_index: 0,
            gif_playing: false,
            gif_clock_ms: 0.0,
            video_path: None,
            video_metadata: None,
            status: "Ready. Drop an image, animated GIF, video, or graphics project JSON."
                .to_owned(),
            is_error: false,
            help_open: false,
            close: false,
        };
        if let Some(path) = path {
            state.open_drop(path);
        }
        state
    }

    fn message(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.is_error = false;
    }

    fn error(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.is_error = true;
    }

    fn selected_path(&self) -> Option<PathBuf> {
        self.selected
            .and_then(|index| self.project.layers.get(index))
            .map(|layer| layer.path.clone())
    }

    fn no_selection(&mut self) -> bool {
        if self.image.is_none() || self.selected.is_none() {
            self.error("No operation performed: select an image layer first");
            true
        } else {
            false
        }
    }

    fn open_drop(&mut self, path: PathBuf) {
        let extension = extension(&path);
        if extension == "json" {
            match GraphicsProject::load(&path) {
                Ok(project) => {
                    self.project = project;
                    self.project_path = Some(path.clone());
                    self.selected = self
                        .project
                        .selected_image
                        .as_ref()
                        .and_then(|selected| {
                            self.project.layers.iter().position(|l| &l.path == selected)
                        })
                        .or((!self.project.layers.is_empty()).then_some(0));
                    if self.load_selected() {
                        self.message(format!("Loaded project {}", path.display()));
                    }
                }
                Err(error) => self.error(format!("Project load failed: {error}")),
            }
        } else if ["mp4", "mov", "mkv", "webm", "avi"].contains(&extension.as_str()) {
            self.video_path = Some(path.clone());
            self.video_metadata = if video_tools_available() {
                match graphics::probe_video(&path) {
                    Ok(metadata) => {
                        self.message(format!("Loaded video {}", path.display()));
                        Some(metadata)
                    }
                    Err(error) => {
                        self.error(format!("Video metadata failed: {error}"));
                        None
                    }
                }
            } else {
                self.error("Loaded video, but no video-rs or ffprobe backend is available");
                None
            };
        } else if extension == "gif" {
            match read_image(&path) {
                Ok(image) => {
                    self.add_image(path.clone(), image);
                    match decode_animated_gif(path) {
                        Ok(frames) if !frames.is_empty() => {
                            self.gif_frames = frames;
                            self.gif_index = 0;
                            self.gif_clock_ms = 0.0;
                            self.message(format!(
                                "Loaded animated GIF ({} frames)",
                                self.gif_frames.len()
                            ));
                        }
                        Ok(_) => self.message("Loaded GIF (it contains no frames)"),
                        Err(error) => self.error(format!("GIF frame info failed: {error}")),
                    }
                }
                Err(error) => self.error(format!("Image load failed: {error}")),
            }
        } else {
            match read_image(&path) {
                Ok(image) => self.add_image(path, image),
                Err(error) => self.error(format!("Unsupported drop or image load failed: {error}")),
            }
        }
    }

    fn add_image(&mut self, path: PathBuf, image: DynamicImage) {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "image".into());
        self.project
            .layers
            .push(graphics::GraphicsLayer::new(path.clone(), name));
        self.selected = Some(self.project.layers.len() - 1);
        self.layer_scroll = self.project.layers.len().saturating_sub(8);
        self.project.selected_image = Some(path.clone());
        self.image = Some(image);
        self.texture = None;
        self.dirty_texture = true;
        self.gif_frames.clear();
        self.undo.clear();
        self.redo.clear();
        self.message(format!("Added image {}", path.display()));
    }

    fn load_selected(&mut self) -> bool {
        self.undo.clear();
        self.redo.clear();
        self.gif_frames.clear();
        self.gif_index = 0;
        self.gif_playing = false;
        self.gif_clock_ms = 0.0;
        self.texture = None;
        self.image = None;
        let Some(path) = self.selected_path() else {
            return true;
        };
        self.image = match read_image(&path) {
            Ok(image) => {
                self.project.selected_image = Some(path.clone());
                if extension(&path) == "gif" {
                    match decode_animated_gif(&path) {
                        Ok(frames) => self.gif_frames = frames,
                        Err(error) => {
                            self.error(format!("GIF frame info failed: {error}"));
                            return false;
                        }
                    }
                }
                self.dirty_texture = true;
                Some(image)
            }
            Err(error) => {
                self.error(format!("Could not load selected layer: {error}"));
                None
            }
        };
        self.image.is_some()
    }

    fn apply(&mut self, operation: ImageOperation, label: &str) {
        if self.no_selection() {
            return;
        }
        let Some(path) = self.selected_path() else {
            self.error("No operation performed: selected layer has no path");
            return;
        };
        let Some(image) = &self.image else {
            return;
        };
        match graphics::apply_operation(image, operation) {
            Ok(result) => {
                self.undo.push(Snapshot {
                    image: image.clone(),
                    path: path.clone(),
                });
                self.redo.clear();
                let output = edited_path(&path);
                if let Err(error) = save_image_atomic(&result, &output) {
                    self.undo.pop();
                    self.error(format!("{label} failed: {error}"));
                    return;
                }
                self.image = Some(result);
                self.dirty_texture = true;
                self.texture = None;
                if let Some(layer) = self.selected.and_then(|i| self.project.layers.get_mut(i)) {
                    layer.path = output.clone();
                }
                self.project.selected_image = Some(output.clone());
                self.gif_frames.clear();
                self.message(format!("{label} saved to {}", output.display()));
            }
            Err(error) => self.error(format!("{label} failed: {error}")),
        }
    }

    fn undo_action(&mut self) {
        let Some(snapshot) = self.undo.pop() else {
            self.error("Undo: nothing to undo");
            return;
        };
        let current = self.current_snapshot();
        if self.restore(snapshot, "Undid edit").is_none() {
            return;
        }
        if let Some(current) = current {
            self.redo.push(current);
        }
    }

    fn redo_action(&mut self) {
        let Some(snapshot) = self.redo.pop() else {
            self.error("Redo: nothing to redo");
            return;
        };
        let current = self.current_snapshot();
        if self.restore(snapshot, "Redid edit").is_none() {
            return;
        }
        if let Some(current) = current {
            self.undo.push(current);
        }
    }

    fn current_snapshot(&self) -> Option<Snapshot> {
        Some(Snapshot {
            image: self.image.clone()?,
            path: self.selected_path()?,
        })
    }

    fn restore(&mut self, snapshot: Snapshot, label: &str) -> Option<()> {
        self.image = Some(snapshot.image);
        if let Some(layer) = self.selected.and_then(|i| self.project.layers.get_mut(i)) {
            layer.path = snapshot.path.clone();
        }
        self.project.selected_image = Some(snapshot.path);
        self.texture = None;
        self.dirty_texture = true;
        self.message(label);
        Some(())
    }

    fn save_project(&mut self) {
        let path = match self.project_path.clone().or_else(|| {
            self.selected_path()
                .or_else(|| Some(PathBuf::from("graphics-project")))
                .map(|path| path.with_extension("json"))
        }) {
            Some(path) => path,
            None => {
                self.error("Save failed: choose an image or project path");
                return;
            }
        };
        match self.project.save(&path) {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.message(format!("Saved project {}", path.display()));
            }
            Err(error) => self.error(format!("Save failed: {error}")),
        }
    }

    fn export_png(&mut self) {
        let output = self
            .project_path
            .as_ref()
            .map(|path| path.with_extension("png"))
            .or_else(|| self.selected_path().map(|path| edited_path(&path)));
        let Some(output) = output else {
            self.error("Export failed: add an image first");
            return;
        };
        match graphics::export_flattened_png(&self.project, &output) {
            Ok(()) => self.message(format!("Exported {}", output.display())),
            Err(error) => self.error(format!("Export failed: {error}")),
        }
    }

    fn pngtuber(&mut self) {
        let Some(path) = self.selected_path() else {
            self.error("PNGTuber export failed: select an image");
            return;
        };
        let Some(image) = self.image.as_ref() else {
            self.error("PNGTuber export failed: selected image is unavailable");
            return;
        };
        let output = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("pngtuber-export");
        let width = image.width();
        let height = image.height();
        let eye_width = (width as f32 * 0.20).max(1.0) as u32;
        let eye_height = (height as f32 * 0.10).max(1.0) as u32;
        let boxes = [35, 55].map(|percent| graphics::EyeBox {
            x: width.saturating_mul(percent) / 100,
            y: height.saturating_mul(45) / 100,
            width: eye_width.min(width),
            height: eye_height.min(height.saturating_sub(height * 45 / 100)),
        });
        match graphics::write_pngtuber_poses(&path, &output, &PngTuberTransforms::default(), &boxes)
        {
            Ok(files) => self.message(format!(
                "Exported {} PNGTuber poses to {}",
                files.len(),
                output.display()
            )),
            Err(error) => self.error(format!("PNGTuber export failed: {error}")),
        }
    }

    fn extract_frames(&mut self) {
        let Some(input) = self.video_path.clone() else {
            self.error("Extract Frames failed: drop a video first");
            return;
        };
        if !video_tools_available() {
            self.error("Extract Frames failed: no video backend is available");
            return;
        }
        let output = input.with_file_name(
            input
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
                + "-frames",
        );
        match graphics::extract_representative_frames(&input, &output, 12) {
            Ok(paths) => self.message(format!(
                "Extracted {} frames to {}",
                paths.len(),
                output.display()
            )),
            Err(error) => self.error(format!("Extract Frames failed: {error}")),
        }
    }

    fn transcode(&mut self) {
        let Some(input) = self.video_path.clone() else {
            self.error("Transcode failed: drop a video first");
            return;
        };
        if !video_tools_available() {
            self.error("Transcode failed: no video backend is available");
            return;
        }
        let output = sibling_named(&input, "-transcoded.mp4");
        match graphics::transcode_video(
            &input,
            &output,
            &[
                "-c:v".into(),
                "libx264".into(),
                "-crf".into(),
                "23".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-c:a".into(),
                "aac".into(),
            ],
        ) {
            Ok(()) => self.message(format!("Transcoded {}", output.display())),
            Err(error) => self.error(format!("Transcode failed: {error}")),
        }
    }

    fn move_layer(&mut self, direction: isize) {
        let Some(index) = self.selected else {
            self.error("No layer selected");
            return;
        };
        let target = index as isize + direction;
        if target < 0 || target as usize >= self.project.layers.len() {
            self.error("Layer cannot move further");
            return;
        }
        self.project.layers.swap(index, target as usize);
        self.selected = Some(target as usize);
        self.message("Moved layer");
    }

    fn remove_layer(&mut self) {
        let Some(index) = self.selected else {
            self.error("No layer selected");
            return;
        };
        self.project.layers.remove(index);
        self.layer_scroll = self
            .layer_scroll
            .min(self.project.layers.len().saturating_sub(8));
        self.selected = index
            .checked_sub(1)
            .or_else(|| (!self.project.layers.is_empty()).then_some(0));
        if self.load_selected() {
            self.message("Removed layer");
        }
    }

    fn input(&mut self, rl: &mut RaylibHandle) {
        let dismiss_key = rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) && self.help_open;
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            if self.help_open {
                self.help_open = false;
            } else {
                self.close = true;
            }
        }
        if dismiss_key || self.close {
            return;
        }
        let geometry = ui_geometry(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let mouse = geometry.to_logical(rl.get_mouse_position());
        let help_clicked = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
            && (hit_cell(mouse, rect(1000.0, 52.0, 82.0, 34.0))
                || (self.help_open && hit_cell(mouse, rect(894.0, 548.0, 78.0, 30.0))));
        if help_clicked {
            self.help_open = !self.help_open;
        }
        if self.help_open || help_clicked {
            return;
        }
        if rl.is_file_dropped() {
            let files: Vec<PathBuf> = rl
                .load_dropped_files()
                .paths()
                .iter()
                .map(PathBuf::from)
                .collect();
            if files.is_empty() {
                self.error("Drop failed: no files were supplied");
            }
            for path in files {
                self.open_drop(path);
            }
        }
        if !self.gif_frames.is_empty() && self.gif_playing {
            self.gif_clock_ms += rl.get_frame_time() * 1000.0;
            let delay = self.gif_frames[self.gif_index].delay_ms.max(1) as f32;
            if self.gif_clock_ms >= delay {
                self.gif_clock_ms %= delay;
                self.gif_index = (self.gif_index + 1) % self.gif_frames.len();
                self.texture = None;
                self.dirty_texture = true;
            }
        }

        let click = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let hit = |bounds| click && hit_cell(mouse, bounds);

        if hit(rect(12.0, 52.0, 120.0, 34.0)) {
            self.message("Drop a new image anywhere in the window");
        }
        if hit(rect(140.0, 52.0, 100.0, 34.0)) {
            self.save_project();
        }
        if hit(rect(248.0, 52.0, 100.0, 34.0)) {
            self.message("Drop a project .json file to load it");
        }
        if hit(rect(356.0, 52.0, 112.0, 34.0)) {
            self.export_png();
        }
        if hit(rect(476.0, 52.0, 120.0, 34.0)) {
            self.pngtuber();
        }
        if hit(rect(604.0, 52.0, 82.0, 34.0)) {
            self.undo_action();
        }
        if hit(rect(694.0, 52.0, 82.0, 34.0)) {
            self.redo_action();
        }

        if hit_cell(mouse, rect(772.0, 104.0, 316.0, 508.0)) {
            let maximum = self.project.layers.len().saturating_sub(8);
            self.layer_scroll = (self.layer_scroll as isize - rl.get_mouse_wheel_move() as isize)
                .clamp(0, maximum as isize) as usize;
        }
        for display_index in 0..self
            .project
            .layers
            .len()
            .saturating_sub(self.layer_scroll)
            .min(8)
        {
            let index = self.layer_scroll + display_index;
            let y = 132.0 + display_index as f32 * 60.0;
            if hit(rect(782.0, y, 250.0, 52.0)) {
                let name = self.project.layers[index].name.clone();
                self.selected = Some(index);
                if self.load_selected() {
                    self.message(format!("Selected {name}"));
                }
                break;
            }
            if hit(rect(1038.0, y + 5.0, 28.0, 20.0)) {
                self.selected = Some(index);
                self.project.layers[index].visible = !self.project.layers[index].visible;
                break;
            }
            if hit(rect(1038.0, y + 28.0, 28.0, 18.0)) {
                self.selected = Some(index);
                let step = ((self.project.layers[index].opacity * 10.0).round() as u32 + 1) % 11;
                self.project.layers[index].opacity = step as f32 / 10.0;
                break;
            }
        }
        if hit(rect(782.0, 620.0, 86.0, 28.0)) {
            self.move_layer(-1);
        }
        if hit(rect(874.0, 620.0, 86.0, 28.0)) {
            self.move_layer(1);
        }
        if hit(rect(966.0, 620.0, 100.0, 28.0)) {
            self.remove_layer();
        }

        let operations = [
            ("Crop 80%", rect(12.0, 566.0, 104.0, 30.0)),
            ("Resize half", rect(122.0, 566.0, 104.0, 30.0)),
            ("Rotate", rect(232.0, 566.0, 80.0, 30.0)),
            ("Flip H", rect(318.0, 566.0, 72.0, 30.0)),
            ("Flip V", rect(396.0, 566.0, 72.0, 30.0)),
            ("Gray", rect(474.0, 566.0, 66.0, 30.0)),
            ("Bright+", rect(546.0, 566.0, 80.0, 30.0)),
            ("Contrast+", rect(632.0, 566.0, 98.0, 30.0)),
            ("Blur", rect(736.0, 566.0, 66.0, 30.0)),
        ];
        for (label, bounds) in operations {
            if !hit(bounds) || self.no_selection() {
                continue;
            }
            let (width, height) = (
                self.image.as_ref().unwrap().width(),
                self.image.as_ref().unwrap().height(),
            );
            let operation = match label {
                "Crop 80%" => ImageOperation::Crop {
                    x: width.saturating_sub(width * 4 / 5) / 2,
                    y: height.saturating_sub(height * 4 / 5) / 2,
                    width: (width * 4 / 5).max(1),
                    height: (height * 4 / 5).max(1),
                },
                "Resize half" => ImageOperation::Resize {
                    width: (width / 2).max(1),
                    height: (height / 2).max(1),
                },
                "Rotate" => ImageOperation::Rotate90,
                "Flip H" => ImageOperation::FlipHorizontal,
                "Flip V" => ImageOperation::FlipVertical,
                "Gray" => ImageOperation::Grayscale,
                "Bright+" => ImageOperation::Brightness { amount: 20 },
                "Contrast+" => ImageOperation::Contrast { amount: 20.0 },
                "Blur" => ImageOperation::GaussianBlur { sigma: 2.0 },
                _ => unreachable!(),
            };
            self.apply(operation, label);
        }

        if !self.gif_frames.is_empty() {
            if hit(rect(12.0, 652.0, 78.0, 26.0)) && self.gif_frames.len() > 1 {
                self.gif_index =
                    (self.gif_index + self.gif_frames.len() - 1) % self.gif_frames.len();
                self.texture = None;
                self.dirty_texture = true;
            }
            if hit(rect(96.0, 652.0, 78.0, 26.0)) {
                self.gif_playing = !self.gif_playing;
                self.message(if self.gif_playing {
                    "GIF playing"
                } else {
                    "GIF paused"
                });
            }
            if hit(rect(180.0, 652.0, 78.0, 26.0)) && self.gif_frames.len() > 1 {
                self.gif_index = (self.gif_index + 1) % self.gif_frames.len();
                self.texture = None;
                self.dirty_texture = true;
            }
        }
        if hit(rect(780.0, 670.0, 132.0, 26.0)) {
            self.extract_frames();
        }
        if hit(rect(918.0, 670.0, 148.0, 26.0)) {
            self.transcode();
        }
    }

    fn pump_texture(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        if !self.dirty_texture {
            return;
        }
        let source = self
            .gif_frames
            .get(self.gif_index)
            .map(|frame| frame.image.clone())
            .or_else(|| self.image.as_ref().map(|image| image.to_rgba8()));
        if let Some(source) = source {
            match upload_rgba(rl, thread, &source) {
                Ok(texture) => self.texture = Some(texture),
                Err(error) => self.error(format!("Preview upload failed: {error}")),
            }
        }
        self.dirty_texture = false;
    }

    fn draw(&self, d: &mut impl RaylibDraw) {
        d.clear_background(Color::new(22, 25, 32, 255));
        d.draw_text("Graphics Studio", 12, 12, 28, Color::WHITE);
        button(d, rect(12.0, 52.0, 120.0, 34.0), "Add New", false);
        button(d, rect(140.0, 52.0, 100.0, 34.0), "Save", false);
        button(d, rect(248.0, 52.0, 100.0, 34.0), "Load (drop)", false);
        button(d, rect(356.0, 52.0, 112.0, 34.0), "Export PNG", false);
        button(d, rect(476.0, 52.0, 120.0, 34.0), "PNGTuber", false);
        button(
            d,
            rect(604.0, 52.0, 82.0, 34.0),
            "Undo",
            self.undo.is_empty(),
        );
        button(
            d,
            rect(694.0, 52.0, 82.0, 34.0),
            "Redo",
            self.redo.is_empty(),
        );
        button(d, rect(1000.0, 52.0, 82.0, 34.0), "Help", self.help_open);

        panel(d, rect(12.0, 104.0, 752.0, 446.0), "Selected layer preview");
        draw_checkerboard(d, rect(28.0, 136.0, 720.0, 398.0));
        if let Some(texture) = &self.texture {
            let bounds = rect(28.0, 136.0, 720.0, 398.0);
            let scale = (bounds.width / texture.width() as f32)
                .min(bounds.height / texture.height() as f32);
            let width = texture.width() as f32 * scale;
            let height = texture.height() as f32 * scale;
            let dest = rect(
                bounds.x + (bounds.width - width) / 2.0,
                bounds.y + (bounds.height - height) / 2.0,
                width,
                height,
            );
            d.draw_texture_pro(
                texture,
                rect(0.0, 0.0, texture.width() as f32, texture.height() as f32),
                dest,
                Vector2::zero(),
                0.0,
                Color::WHITE,
            );
        } else {
            centered(
                d,
                rect(28.0, 136.0, 720.0, 398.0),
                "Drop an image to begin",
                22,
            );
        }

        panel(d, rect(772.0, 104.0, 316.0, 552.0), "Layers");
        for display_index in 0..self
            .project
            .layers
            .len()
            .saturating_sub(self.layer_scroll)
            .min(8)
        {
            let index = self.layer_scroll + display_index;
            let layer = &self.project.layers[index];
            let y = 132.0 + display_index as f32 * 60.0;
            let bounds = rect(782.0, y, 282.0, 52.0);
            d.draw_rectangle_rec(
                bounds,
                if self.selected == Some(index) {
                    Color::new(48, 84, 120, 255)
                } else {
                    Color::new(38, 43, 53, 255)
                },
            );
            d.draw_text(
                &truncate(&layer.name, 29),
                790,
                y as i32 + 7,
                16,
                Color::WHITE,
            );
            d.draw_text(
                &format!(
                    "opacity {:.0}%  {}",
                    layer.opacity * 100.0,
                    layer.path.display()
                ),
                790,
                y as i32 + 29,
                12,
                Color::LIGHTGRAY,
            );
            button(
                d,
                rect(1038.0, y + 5.0, 28.0, 20.0),
                if layer.visible { "V" } else { "H" },
                false,
            );
            button(d, rect(1038.0, y + 28.0, 28.0, 18.0), "O", false);
        }
        button(d, rect(782.0, 620.0, 86.0, 28.0), "Up", false);
        button(d, rect(874.0, 620.0, 86.0, 28.0), "Down", false);
        button(d, rect(966.0, 620.0, 100.0, 28.0), "Remove", false);

        for (label, bounds) in [
            ("Crop 80%", rect(12.0, 566.0, 104.0, 30.0)),
            ("Resize half", rect(122.0, 566.0, 104.0, 30.0)),
            ("Rotate", rect(232.0, 566.0, 80.0, 30.0)),
            ("Flip H", rect(318.0, 566.0, 72.0, 30.0)),
            ("Flip V", rect(396.0, 566.0, 72.0, 30.0)),
            ("Gray", rect(474.0, 566.0, 66.0, 30.0)),
            ("Bright+", rect(546.0, 566.0, 80.0, 30.0)),
            ("Contrast+", rect(632.0, 566.0, 98.0, 30.0)),
            ("Blur", rect(736.0, 566.0, 66.0, 30.0)),
        ] {
            button(d, bounds, label, false);
        }

        panel(d, rect(12.0, 608.0, 746.0, 82.0), "Animated GIF");
        if self.gif_frames.is_empty() {
            d.draw_text("No animated GIF selected.", 22, 638, 15, Color::LIGHTGRAY);
        } else {
            let frame = &self.gif_frames[self.gif_index];
            d.draw_text(
                &format!(
                    "Frame {}/{}  delay {} ms",
                    self.gif_index + 1,
                    self.gif_frames.len(),
                    frame.delay_ms
                ),
                22,
                634,
                15,
                Color::WHITE,
            );
            button(d, rect(12.0, 652.0, 78.0, 26.0), "Prev", false);
            button(
                d,
                rect(96.0, 652.0, 78.0, 26.0),
                if self.gif_playing { "Pause" } else { "Play" },
                self.gif_playing,
            );
            button(d, rect(180.0, 652.0, 78.0, 26.0), "Next", false);
        }

        let metadata = self.video_metadata.as_ref();
        let mut metadata_lines = Vec::new();
        if let Some(metadata) = metadata {
            metadata_lines.push(format!(
                "{}x{}  {:.2}s  {:.2} fps",
                metadata.width.unwrap_or(0),
                metadata.height.unwrap_or(0),
                metadata.duration_seconds.unwrap_or(0.0),
                metadata.fps.unwrap_or(0.0)
            ));
            let mut tags = metadata.tags.iter().collect::<Vec<_>>();
            tags.sort_by(|(left, _), (right, _)| left.cmp(right));
            metadata_lines.extend(
                tags.into_iter()
                    .filter(|(_, value)| !value.trim().is_empty())
                    .map(|(name, value)| format!("{name}: {value}")),
            );
        } else if let Some(path) = &self.video_path {
            metadata_lines.push(format!("Video: {}", path.display()));
        } else {
            metadata_lines.push("Video: drop MP4/MOV/MKV/WEBM/AVI".into());
        }
        let mut line_index = 0;
        for metadata_line in metadata_lines {
            for line in crate::ui::wrap_full(&metadata_line, 12.0, 286.0) {
                d.draw_text(&line, 780, 612 + line_index as i32 * 14, 12, Color::LIGHTGRAY);
                line_index += 1;
            }
        }
        button(d, rect(780.0, 670.0, 132.0, 26.0), "Extract Frames", false);
        button(d, rect(918.0, 670.0, 148.0, 26.0), "Transcode MP4", false);

        let status_color = if self.is_error {
            Color::new(105, 36, 42, 255)
        } else {
            Color::new(35, 54, 46, 255)
        };
        d.draw_rectangle_rec(rect(12.0, 706.0, 1076.0, 42.0), status_color);
        d.draw_text(
            &truncate(&self.status, 145),
            22,
            718,
            16,
            if self.is_error {
                Color::new(255, 205, 205, 255)
            } else {
                Color::WHITE
            },
        );
        if self.help_open {
            draw_help(d);
        }
    }
}

pub fn run(path: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let (mut rl, thread) = raylib::init()
        .size(LOGICAL_WIDTH as i32, LOGICAL_HEIGHT as i32)
        .title("Selenite Graphics Studio")
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_window_min_size(800, 600);
    rl.set_exit_key(None);
    let mut state = UiState::new(path);
    while !rl.window_should_close() && !state.close {
        state.input(&mut rl);
        state.pump_texture(&mut rl, &thread);
        let geometry = ui_geometry(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let camera = Camera2D {
            offset: geometry.offset,
            target: Vector2::zero(),
            rotation: 0.0,
            zoom: geometry.scale,
        };
        {
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::BLACK);
            let mut d = d.begin_mode2D(camera);
            state.draw(&mut d);
        }
    }
    Ok(())
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
    Rectangle::new(x, y, width, height)
}

fn panel(d: &mut impl RaylibDraw, bounds: Rectangle, title: &str) {
    d.draw_rectangle_rec(bounds, Color::new(30, 35, 44, 255));
    d.draw_rectangle_lines_ex(bounds, 1.0, Color::new(66, 75, 91, 255));
    d.draw_text(
        title,
        bounds.x as i32 + 10,
        bounds.y as i32 + 8,
        16,
        Color::LIGHTGRAY,
    );
}

fn button(d: &mut impl RaylibDraw, bounds: Rectangle, label: &str, active: bool) {
    d.draw_rectangle_rec(
        bounds,
        if active {
            Color::new(43, 102, 91, 255)
        } else {
            Color::new(52, 66, 86, 255)
        },
    );
    d.draw_rectangle_lines_ex(bounds, 1.0, Color::new(92, 111, 139, 255));
    let font = 14;
    let width = label.chars().count() as f32 * font as f32 * 0.58;
    d.draw_text(
        label,
        (bounds.x + (bounds.width - width) / 2.0) as i32,
        (bounds.y + (bounds.height - font as f32) / 2.0) as i32,
        font,
        Color::WHITE,
    );
}

fn draw_help(d: &mut impl RaylibDraw) {
    d.draw_rectangle_rec(
        rect(0.0, 0.0, LOGICAL_WIDTH, LOGICAL_HEIGHT),
        Color::new(0, 0, 0, 190),
    );
    let bounds = rect(110.0, 166.0, 880.0, 428.0);
    panel(d, bounds, "Graphics Studio Help");
    for (index, line) in [
        "Drop an image to create a layer. Drop a graphics project .json file to reopen a project.",
        "Use Add New to start a blank layer, then Save to store the layered project.",
        "Export PNG saves the composited artwork. PNGTuber opens the pose/artwork workflow.",
        "Select a layer in the right-hand Layers list. V/H toggles visibility; O cycles opacity.",
        "Use Up and Down to reorder layers, Remove to delete one, and Undo/Redo to revert edits.",
        "Image operations crop, resize, rotate, flip, grayscale, brighten, increase contrast and blur.",
        "Drop animated GIFs for frame preview/export. Drop supported video files to extract frames or transcode.",
        "Press Esc to close this help panel or close the window. Errors and progress appear in the status bar.",
    ]
    .iter()
    .enumerate()
    {
        d.draw_text(
            line,
            bounds.x as i32 + 22,
            bounds.y as i32 + 48 + index as i32 * 38,
            15,
            Color::new(224, 231, 242, 255),
        );
    }
    button(d, rect(894.0, 548.0, 78.0, 30.0), "Close", false);
}

fn centered(d: &mut impl RaylibDraw, bounds: Rectangle, label: &str, size: i32) {
    let width = label.chars().count() as f32 * size as f32 * 0.58;
    d.draw_text(
        label,
        (bounds.x + (bounds.width - width) / 2.0) as i32,
        (bounds.y + bounds.height / 2.0 - size as f32 / 2.0) as i32,
        size,
        Color::GRAY,
    );
}

fn draw_checkerboard(d: &mut impl RaylibDraw, bounds: Rectangle) {
    d.draw_rectangle_rec(bounds, Color::new(18, 20, 26, 255));
    for y in 0..((bounds.height / 20.0).ceil() as i32) {
        for x in 0..((bounds.width / 20.0).ceil() as i32) {
            if (x + y) % 2 == 0 {
                d.draw_rectangle(
                    (bounds.x + x as f32 * 20.0) as i32,
                    (bounds.y + y as f32 * 20.0) as i32,
                    20.min((bounds.x + bounds.width - bounds.x - x as f32 * 20.0) as i32),
                    20.min((bounds.y + bounds.height - bounds.y - y as f32 * 20.0) as i32),
                    Color::new(28, 31, 39, 255),
                );
            }
        }
    }
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

fn edited_path(path: &Path) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let stem = stem.strip_suffix("-edited").unwrap_or(&stem);
    path.with_file_name(format!("{stem}-edited.png"))
}

fn sibling_named(path: &Path, suffix: &str) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!("{stem}{suffix}"))
}

fn save_image_atomic(image: &DynamicImage, path: &Path) -> Result<(), Box<dyn Error>> {
    let temporary = path.with_extension("png.part");
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    image.save_with_format(&temporary, ImageFormat::Png)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn truncate(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        text.to_owned()
    } else {
        text.chars()
            .take(width.saturating_sub(1))
            .chain(std::iter::once('…'))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_scales_and_centers_hit_cells_exactly() {
        let geometry = ui_geometry(2200.0, 760.0);
        assert_eq!(geometry.scale, 1.0);
        assert_eq!(geometry.offset, Vector2::new(550.0, 0.0));
        let logical = geometry.to_logical(Vector2::new(562.0, 52.0));
        assert!(hit_cell(logical, rect(12.0, 52.0, 120.0, 34.0)));
        assert!(!hit_cell(
            Vector2::new(131.9, 51.9),
            rect(12.0, 52.0, 120.0, 34.0)
        ));
    }

    #[test]
    fn geometry_handles_small_and_exact_logical_sizes() {
        let geometry = ui_geometry(800.0, 600.0);
        assert_eq!(geometry.scale, 800.0 / LOGICAL_WIDTH);
        assert_eq!(geometry.to_logical(geometry.offset), Vector2::zero());
        let exact = ui_geometry(LOGICAL_WIDTH, LOGICAL_HEIGHT);
        assert_eq!(exact.offset, Vector2::zero());
        assert_eq!(exact.scale, 1.0);
    }

    #[test]
    fn operations_without_selection_report_no_operation() {
        let mut state = UiState::new(None);
        state.apply(ImageOperation::Grayscale, "Gray");
        assert!(state.is_error);
        assert!(state.status.contains("No operation performed"));
        assert!(state.undo.is_empty());
        assert!(state.image.is_none());
    }
}

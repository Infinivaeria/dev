//! Image decoding without size limits, background thumbnail loading for the
//! grid/inventory, and tiled textures so the viewer can show images larger
//! than the GPU's maximum texture size at full resolution.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

use image::{ImageReader, RgbaImage};
use raylib::prelude::*;

/// Longest edge of grid/inventory thumbnails.
pub const THUMBNAIL_MAX: u32 = 1024;
/// Largest texture edge uploaded at once; bigger images are split into tiles.
pub const TILE_SIZE: u32 = 4096;
const THUMBNAIL_WORKERS: usize = 2;
const UPLOADS_PER_FRAME: usize = 4;

/// Reads only the header to report `(width, height)`.
pub fn dimensions(path: &Path) -> Result<(u32, u32), String> {
    let mut reader = ImageReader::open(path)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|error| format!("could not open {}: {error}", path.display()))?;
    reader.no_limits();
    reader
        .into_dimensions()
        .map_err(|error| format!("could not read {}: {error}", path.display()))
}

/// Decodes any supported image to RGBA8 with the decoder's default memory
/// and dimension limits removed, so arbitrarily large images load.
pub fn decode_full(path: &Path) -> Result<RgbaImage, String> {
    let mut reader = ImageReader::open(path)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|error| format!("could not open {}: {error}", path.display()))?;
    reader.no_limits();
    let image = reader
        .decode()
        .map_err(|error| format!("failed to decode image {}: {error}", path.display()))?;
    if image.width() == 0 || image.height() == 0 {
        return Err(format!("image {} has zero dimensions", path.display()));
    }
    Ok(image.into_rgba8())
}

/// Decodes and downsizes to at most `max` pixels on the longest edge.
pub fn decode_thumbnail(path: &Path, max: u32) -> Result<RgbaImage, String> {
    let full = decode_full(path)?;
    let (width, height) = full.dimensions();
    if width <= max && height <= max {
        return Ok(full);
    }
    let image = image::DynamicImage::ImageRgba8(full);
    Ok(image.thumbnail(max, max).into_rgba8())
}

/// Uploads a sub-rectangle of `source` as a mipmapped, trilinear-filtered
/// texture.
pub fn upload_region(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    source: &RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Result<Texture2D, String> {
    if width == 0 || height == 0 {
        return Err("cannot upload an empty image region".to_owned());
    }
    // `gen_image_color` allocates an R8G8B8A8 buffer with raylib's own
    // allocator, so filling it in place keeps `UnloadImage` on drop valid.
    let image = Image::gen_image_color(width as i32, height as i32, Color::BLANK);
    let row_bytes = width as usize * 4;
    let stride = source.width() as usize * 4;
    let raw = source.as_raw();
    unsafe {
        let dest =
            std::slice::from_raw_parts_mut(image.data() as *mut u8, row_bytes * height as usize);
        for row in 0..height as usize {
            let start = (y as usize + row) * stride + x as usize * 4;
            dest[row * row_bytes..(row + 1) * row_bytes]
                .copy_from_slice(&raw[start..start + row_bytes]);
        }
    }
    let mut texture = rl
        .load_texture_from_image(thread, &image)
        .map_err(|error| format!("failed to upload texture: {error}"))?;
    texture.gen_texture_mipmaps();
    texture.set_texture_filter(thread, TextureFilter::TEXTURE_FILTER_TRILINEAR);
    Ok(texture)
}

pub fn upload_rgba(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    source: &RgbaImage,
) -> Result<Texture2D, String> {
    upload_region(rl, thread, source, 0, 0, source.width(), source.height())
}

type ThumbResult = (PathBuf, Result<RgbaImage, String>);

/// Thumbnails decoded on worker threads and uploaded a few per frame, so
/// huge images never stall the UI. Failures are remembered (not retried
/// every frame).
pub struct ThumbnailCache {
    textures: HashMap<PathBuf, Texture2D>,
    pending: HashSet<PathBuf>,
    failed: HashMap<PathBuf, String>,
    requests: Sender<PathBuf>,
    results: Receiver<ThumbResult>,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        let (requests, request_rx) = mpsc::channel::<PathBuf>();
        let (result_tx, results) = mpsc::channel::<ThumbResult>();
        let request_rx = Arc::new(Mutex::new(request_rx));
        for _ in 0..THUMBNAIL_WORKERS {
            let request_rx = Arc::clone(&request_rx);
            let result_tx = result_tx.clone();
            thread::spawn(move || loop {
                let next = request_rx
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .recv();
                let Ok(path) = next else { break };
                let decoded = decode_thumbnail(&path, THUMBNAIL_MAX);
                if result_tx.send((path, decoded)).is_err() {
                    break;
                }
            });
        }
        Self {
            textures: HashMap::new(),
            pending: HashSet::new(),
            failed: HashMap::new(),
            requests,
            results,
        }
    }

    /// Queues `path` for decoding unless it's loaded, loading or failed.
    pub fn request(&mut self, path: &Path) {
        if self.textures.contains_key(path)
            || self.pending.contains(path)
            || self.failed.contains_key(path)
        {
            return;
        }
        if self.requests.send(path.to_path_buf()).is_ok() {
            self.pending.insert(path.to_path_buf());
        }
    }

    pub fn get(&self, path: &Path) -> Option<&Texture2D> {
        self.textures.get(path)
    }

    pub fn is_pending(&self, path: &Path) -> bool {
        self.pending.contains(path)
    }

    pub fn failure(&self, path: &Path) -> Option<&str> {
        self.failed.get(path).map(String::as_str)
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Uploads finished decodes to the GPU (a few per call).
    pub fn pump(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        for _ in 0..UPLOADS_PER_FRAME {
            let Ok((path, decoded)) = self.results.try_recv() else {
                break;
            };
            self.pending.remove(&path);
            match decoded.and_then(|image| upload_rgba(rl, thread, &image)) {
                Ok(texture) => {
                    self.textures.insert(path, texture);
                }
                Err(error) => {
                    self.failed.insert(path, error);
                }
            }
        }
    }

    /// Drops loaded textures and failures (e.g. after switching profiles);
    /// in-flight decodes still land when they finish.
    pub fn clear(&mut self) {
        self.textures.clear();
        self.failed.clear();
    }
}

/// A full-resolution image split into GPU-sized tiles.
pub struct TiledImage {
    pub width: u32,
    pub height: u32,
    tiles: Vec<(Rectangle, Texture2D)>,
}

impl TiledImage {
    pub fn upload(
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        image: &RgbaImage,
    ) -> Result<Self, String> {
        let (width, height) = image.dimensions();
        let mut tiles = Vec::new();
        for (x, y, w, h) in tile_layout(width, height, TILE_SIZE) {
            let texture = upload_region(rl, thread, image, x, y, w, h)?;
            tiles.push((
                Rectangle::new(x as f32, y as f32, w as f32, h as f32),
                texture,
            ));
        }
        Ok(Self {
            width,
            height,
            tiles,
        })
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    /// Draws with the image's top-left at `origin`, scaled by `zoom`,
    /// skipping tiles outside `clip`.
    pub fn draw(&self, d: &mut RaylibDrawHandle<'_>, origin: Vector2, zoom: f32, clip: Rectangle) {
        for (area, texture) in &self.tiles {
            let dest = Rectangle::new(
                origin.x + area.x * zoom,
                origin.y + area.y * zoom,
                area.width * zoom,
                area.height * zoom,
            );
            if dest.x > clip.x + clip.width
                || dest.y > clip.y + clip.height
                || dest.x + dest.width < clip.x
                || dest.y + dest.height < clip.y
            {
                continue;
            }
            let source = Rectangle::new(0.0, 0.0, area.width, area.height);
            d.draw_texture_pro(texture, source, dest, Vector2::zero(), 0.0, Color::WHITE);
        }
    }
}

/// `(x, y, width, height)` tiles covering a `width`×`height` image.
pub fn tile_layout(width: u32, height: u32, tile: u32) -> Vec<(u32, u32, u32, u32)> {
    let mut tiles = Vec::new();
    let mut y = 0;
    while y < height {
        let h = tile.min(height - y);
        let mut x = 0;
        while x < width {
            let w = tile.min(width - x);
            tiles.push((x, y, w, h));
            x += w;
        }
        y += h;
    }
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_cover_image_exactly() {
        assert_eq!(tile_layout(100, 50, 4096), vec![(0, 0, 100, 50)]);
        let tiles = tile_layout(9000, 5000, 4096);
        assert_eq!(tiles.len(), 6);
        assert_eq!(tiles[2], (8192, 0, 808, 4096));
        assert_eq!(tiles[5], (8192, 4096, 808, 904));
        let area: u64 = tiles.iter().map(|t| t.2 as u64 * t.3 as u64).sum();
        assert_eq!(area, 9000 * 5000);
        assert!(tile_layout(0, 10, 64).is_empty());
    }

    #[test]
    fn decodes_large_images_and_downsizes_thumbnails() {
        let dir = std::env::temp_dir().join(format!(
            "selenite-imaging-{}-{}",
            std::process::id(),
            crate::timestamp()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // Wrong extension on purpose: the format is sniffed from content.
        let path = dir.join("wide.dat");
        let image = RgbaImage::from_pixel(5000, 300, image::Rgba([10, 20, 30, 255]));
        image
            .save_with_format(&path, image::ImageFormat::Png)
            .unwrap();

        assert_eq!(dimensions(&path).unwrap(), (5000, 300));
        assert_eq!(decode_full(&path).unwrap().dimensions(), (5000, 300));
        let thumb = decode_thumbnail(&path, 1024).unwrap();
        assert_eq!(thumb.width(), 1024);
        assert!(thumb.height() <= 62 && thumb.height() >= 60);
        assert!(decode_full(&dir.join("missing.png")).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}

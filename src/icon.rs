use std::{path::{Path, PathBuf}, ffi::OsString};

use raylib::prelude::*;

use crate::{desktop, settings::Settings, ui};

struct Geometry {
    bounds: Rectangle,
    field: Rectangle,
    list: Rectangle,
    preview: Rectangle,
    buttons: [Rectangle; 5],
    message: Rectangle,
    row: f32,
    font: f32,
}

impl Geometry {
    fn new(width: i32, height: i32) -> Self {
        let bounds = Rectangle::new(0.0, 0.0, (width as f32 - 24.0).clamp(1.0, 680.0), (height as f32 - 24.0).clamp(1.0, 480.0));
        let bounds = Rectangle::new((width as f32 - bounds.width) / 2.0, (height as f32 - bounds.height) / 2.0, bounds.width, bounds.height);
        let row = (bounds.height / 10.0).clamp(16.0, 32.0);
        let inner = bounds.width - 16.0;
        let field = Rectangle::new(bounds.x + 8.0, bounds.y + row, inner, row);
        let button_y = bounds.y + bounds.height - row * 2.0 - 4.0;
        let list = Rectangle::new(field.x, field.y + row + 4.0, inner * 0.6 - 4.0, (button_y - field.y - row - 8.0).max(1.0));
        let size = (inner * 0.4 - 8.0).min(list.height).max(1.0);
        let preview = Rectangle::new(list.x + list.width + 12.0, list.y, size, size);
        Self {
            bounds, field, list, preview,
            buttons: std::array::from_fn(|index| Rectangle::new(field.x + index as f32 * inner / 5.0, button_y, inner / 5.0 - 2.0, row)),
            message: Rectangle::new(field.x, button_y + row + 2.0, inner, row),
            row, font: (row * 0.46).max(8.0),
        }
    }
}

pub fn apply_saved(rl: &mut RaylibHandle) {
    let settings = Settings::load(&Settings::path_in(&crate::profiles::default_base_dir()));
    if let Some(path) = settings.application_icon {
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(image) = Image::load_image_from_mem(".png", &bytes) { rl.set_window_icon(&image); }
        }
    }
}

pub struct Picker {
    path: String,
    directory: PathBuf,
    entries: Vec<PathBuf>,
    scroll: usize,
    selected_text: bool,
    editing: bool,
    bytes: Option<Vec<u8>>,
    preview: Option<Texture2D>,
    message: String,
    pub closed: bool,
}

impl Picker {
    pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        let directory = if home.join("Pictures").is_dir() { home.join("Pictures") } else { home };
        let mut picker = Self {
            path: directory.to_string_lossy().into_owned(), directory,
            entries: Vec::new(), scroll: 0, selected_text: true, editing: true,
            bytes: None, preview: None, message: String::new(), closed: false,
        };
        picker.refresh();
        let settings = Settings::load(&Settings::path_in(&crate::profiles::default_base_dir()));
        if let Some(path) = settings.application_icon { picker.load(rl, thread, &path); }
        picker
    }

    fn refresh(&mut self) {
        match std::fs::read_dir(&self.directory) {
            Ok(entries) => {
                self.entries = entries.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.is_dir() || matches!(path.extension().and_then(|value| value.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("png" | "jpg" | "jpeg" | "bmp" | "ico" | "webp" | "gif" | "tga"))).take(2000).collect();
                self.entries.sort_by_key(|path| (!path.is_dir(), path.file_name().unwrap_or_default().to_string_lossy().to_lowercase()));
                self.scroll = 0;
            }
            Err(error) => { self.entries.clear(); self.message = error.to_string(); }
        }
    }

    fn load(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread, path: &Path) {
        let result = desktop::icon_bytes(path).and_then(|bytes| {
            let image = Image::load_image_from_mem(".png", &bytes).map_err(|error| error.to_string())?;
            let preview = rl.load_texture_from_image(thread, &image).map_err(|error| error.to_string())?;
            Ok((bytes, preview))
        });
        match result {
            Ok((bytes, preview)) => { self.bytes = Some(bytes); self.preview = Some(preview); self.message.clear(); }
            Err(error) => { self.bytes = None; self.preview = None; self.message = error; }
        }
    }

    fn choose(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread, path: PathBuf) {
        self.path = path.to_string_lossy().into_owned();
        self.selected_text = true;
        self.editing = false;
        if path.is_dir() { self.directory = path; self.refresh(); }
        else { self.load(rl, thread, &path); }
    }

    pub fn update(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        let geometry = Geometry::new(rl.get_screen_width(), rl.get_screen_height());
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) { self.closed = true; return; }
        if rl.is_file_dropped() {
            let files = rl.load_dropped_files();
            let paths = files.paths();
            if paths.len() == 1 { self.choose(rl, thread, PathBuf::from(paths[0])); }
            else { self.message = "Choose one image or folder".into(); }
        }
        let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        let previous_path = self.path.clone();
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_A) { self.selected_text = true; self.editing = true; }
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_V) {
            if let Ok(text) = rl.get_clipboard_text() {
                if self.selected_text { self.path.clear(); }
                self.path.push_str(text.trim()); self.selected_text = false; self.editing = true;
            }
        }
        while let Some(character) = rl.get_char_pressed() {
            if self.editing && !ctrl && !character.is_control() && self.path.len() < 4096 {
                if self.selected_text { self.path.clear(); }
                self.path.push(character); self.selected_text = false;
            }
        }
        if self.editing && rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
            if self.selected_text { self.path.clear(); self.selected_text = false; } else { self.path.pop(); }
        }
        if self.path != previous_path { self.bytes = None; self.preview = None; self.message.clear(); }
        let mouse = rl.get_mouse_position();
        if geometry.list.check_collision_point_rec(mouse) {
            self.scroll = (self.scroll as isize - (rl.get_mouse_wheel_move() * 3.0) as isize).clamp(0, self.entries.len().saturating_sub(1) as isize) as usize;
        }
        let mut action = if rl.is_key_pressed(KeyboardKey::KEY_ENTER) { Some(if self.editing { 0 } else { 2 }) } else { None };
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if geometry.field.check_collision_point_rec(mouse) { self.editing = true; }
            if geometry.list.check_collision_point_rec(mouse) {
                let index = self.scroll + ((mouse.y - geometry.list.y) / geometry.row).floor() as usize;
                if let Some(path) = self.entries.get(index).cloned() { self.choose(rl, thread, path); }
            }
            action = geometry.buttons.iter().position(|button| button.check_collision_point_rec(mouse));
        }
        match action {
            Some(0) => {
                let path = if let Some(rest) = self.path.strip_prefix("~/") { PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| OsString::from("."))).join(rest) } else { PathBuf::from(&self.path) };
                self.choose(rl, thread, path);
            }
            Some(1) => { if let Some(parent) = self.directory.parent().map(Path::to_owned) { self.choose(rl, thread, parent); } }
            Some(2) if self.bytes.is_some() => {
                match desktop::change_icon(self.bytes.as_deref()) {
                    Ok(_) => {
                        if let Ok(image) = Image::load_image_from_mem(".png", self.bytes.as_ref().unwrap()) { rl.set_window_icon(&image); }
                        self.message = "Icon applied to shortcuts and this window".into();
                    }
                    Err(error) => self.message = error,
                }
            }
            Some(3) => match desktop::change_icon(None) {
                Ok(_) => { rl.set_window_icons(&mut []); self.bytes = None; self.preview = None; self.message = "Default icon restored".into(); }
                Err(error) => self.message = error,
            },
            Some(4) => self.closed = true,
            _ => {},
        }
    }

    pub fn draw<D: RaylibDraw>(&self, d: &mut D, width: i32, height: i32) {
        let geometry = Geometry::new(width, height);
        d.draw_rectangle(0, 0, width, height, Color::new(0, 0, 0, 170));
        d.draw_rectangle_rec(geometry.bounds, Color::new(28, 29, 35, 255));
        ui::draw_text_in(d, "Application Icon", Rectangle::new(geometry.field.x, geometry.bounds.y, geometry.field.width, geometry.row), 0.0, geometry.font, Color::RAYWHITE);
        d.draw_rectangle_rec(geometry.field, Color::new(42, 44, 54, 255));
        let path = ui::fit(&self.path, geometry.font, geometry.field.width - 16.0);
        ui::draw_text_in(d, &path, geometry.field, 8.0, geometry.font, Color::RAYWHITE);
        let visible = (geometry.list.height / geometry.row).floor() as usize;
        for (slot, path) in self.entries.iter().skip(self.scroll).take(visible).enumerate() {
            let rect = Rectangle::new(geometry.list.x, geometry.list.y + slot as f32 * geometry.row, geometry.list.width, geometry.row - 1.0);
            d.draw_rectangle_rec(rect, Color::new(36, 39, 47, 255));
            let name = format!("{}{}", path.file_name().unwrap_or_default().to_string_lossy(), if path.is_dir() { "/" } else { "" });
            ui::draw_text_in(d, &ui::fit(&name, geometry.font, rect.width - 12.0), rect, 6.0, geometry.font, Color::RAYWHITE);
        }
        let tile = (geometry.preview.width / 8.0).max(1.0);
        for row in 0..8 { for column in 0..8 {
            d.draw_rectangle_rec(Rectangle::new(geometry.preview.x + column as f32 * tile, geometry.preview.y + row as f32 * tile, tile, tile), if (row + column) % 2 == 0 { Color::new(65, 68, 75, 255) } else { Color::new(100, 103, 110, 255) });
        } }
        if let Some(texture) = &self.preview { d.draw_texture_pro(texture, Rectangle::new(0.0, 0.0, 256.0, 256.0), geometry.preview, Vector2::zero(), 0.0, Color::WHITE); }
        for (index, rect) in geometry.buttons.iter().enumerate() {
            let disabled = index == 2 && self.bytes.is_none();
            d.draw_rectangle_rec(*rect, if disabled { Color::new(36, 38, 44, 255) } else { Color::new(48, 65, 85, 255) });
            ui::draw_text_centered(d, &ui::fit(["Open", "Up", "Apply", "Reset", "Close"][index], geometry.font, rect.width - 6.0), *rect, geometry.font, if disabled { Color::GRAY } else { Color::RAYWHITE });
        }
        ui::draw_text_in(d, &ui::fit(&self.message, geometry.font * 0.85, geometry.message.width), geometry.message, 0.0, geometry.font * 0.85, Color::GOLD);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_picker_controls_fit_compact_and_large_windows() {
        for (width, height) in [(240, 140), (420, 300), (1100, 760), (3840, 2160)] {
            let geometry = Geometry::new(width, height);
            for rect in [geometry.bounds, geometry.field, geometry.list, geometry.preview, geometry.message].into_iter().chain(geometry.buttons) {
                assert!(rect.x >= 0.0 && rect.x + rect.width <= width as f32 + 0.01);
                assert!(rect.y >= 0.0 && rect.y + rect.height <= height as f32 + 0.01);
            }
            assert!(!geometry.list.check_collision_recs(geometry.preview));
            for button in geometry.buttons { assert!(!button.check_collision_recs(geometry.list)); }
        }
    }
}
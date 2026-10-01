use std::path::PathBuf;

use raylib::prelude::*;

use crate::{
    imaging::ThumbnailCache, load_existing_root, open_with_system, persistence::CellKind,
    point_in_rect, spawn_mode, ui, Mode, BUTTON_BG, BUTTON_HOVER_BG, BUTTON_TEXT, TOOLBAR_BG,
    VIEWER_BACKGROUND, VIEWER_TEXT, WHITE, WINDOW_HEIGHT, WINDOW_WIDTH,
};

/// Window-scaled layout metrics for the inventory list.
struct Metrics {
    scale: f32,
    toolbar_height: f32,
    row_height: f32,
    thumb_size: f32,
    padding: f32,
    refresh_button: Rectangle,
    button_text: f32,
}

impl Metrics {
    fn new(screen_width: i32, screen_height: i32) -> Self {
        let scale = ui::scale(screen_width, screen_height);
        let button_text = 20.0 * scale;
        let margin = 8.0 * scale;
        let button_height = button_text + 14.0 * scale;
        let button_width = ui::measure("Refresh", button_text) + 24.0 * scale;
        Self {
            scale,
            toolbar_height: button_height + margin * 2.0,
            row_height: 72.0 * scale,
            thumb_size: 54.0 * scale,
            padding: 12.0 * scale,
            refresh_button: Rectangle::new(margin, margin, button_width, button_height),
            button_text,
        }
    }
}

#[derive(Clone)]
struct InventoryItem {
    location: String,
    col: i64,
    row: i64,
    kind: CellKind,
    file_path: Option<PathBuf>,
}

pub struct InventoryApp {
    json_path: PathBuf,
    items: Vec<InventoryItem>,
    scroll_offset: f32,
    selected_index: Option<usize>,
    textures: ThumbnailCache,
}

impl InventoryApp {
    pub fn new(json_path: PathBuf) -> Result<Self, String> {
        let mut app = Self {
            json_path: json_path.clone(),
            items: Vec::new(),
            scroll_offset: 0.0,
            selected_index: None,
            textures: ThumbnailCache::new(),
        };
        app.reload()?;
        Ok(app)
    }

    pub fn reload(&mut self) -> Result<(), String> {
        let root = load_existing_root(&self.json_path)?;
        let locked = root.lock().unwrap_or_else(|e| e.into_inner());
        let raw_items = locked.collect_all_items("");
        self.items = raw_items
            .into_iter()
            .map(|(location, col, row, kind, file_path)| InventoryItem {
                location,
                col,
                row,
                kind,
                file_path,
            })
            .collect();
        self.selected_index = None;
        Ok(())
    }

    pub fn run(mut self) -> Result<(), String> {
        let (mut rl, thread) = raylib::init()
            .size(WINDOW_WIDTH, WINDOW_HEIGHT)
            .title("Selenite Inventory - List View")
            .resizable()
            .build();
        rl.set_target_fps(60);
        ui::load_font(&thread);

        while !rl.window_should_close() {
            self.update(&mut rl)?;
            self.preload_visible_textures(&rl);
            self.textures.pump(&mut rl, &thread);
            self.draw(&mut rl, &thread);
        }
        // Free GPU textures before the window and GL context close.
        self.textures.clear();
        drop(rl);
        Ok(())
    }

    fn update(&mut self, rl: &mut RaylibHandle) -> Result<(), String> {
        ui::handle_scale_keys(rl);
        let m = Metrics::new(rl.get_screen_width(), rl.get_screen_height());
        let screen_height = rl.get_screen_height() as f32;
        let content_height = self.items.len() as f32 * m.row_height;
        let visible_height = screen_height - m.toolbar_height;
        let max_scroll = (content_height - visible_height).max(0.0);

        // Scroll handling
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            self.scroll_offset = self.scroll_offset - wheel * 48.0 * m.scale;
        }
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);

        // Keyboard navigation
        if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            if let Some(idx) = self.selected_index {
                if idx > 0 {
                    self.selected_index = Some(idx - 1);
                }
            } else if !self.items.is_empty() {
                self.selected_index = Some(0);
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            if let Some(idx) = self.selected_index {
                if idx + 1 < self.items.len() {
                    self.selected_index = Some(idx + 1);
                }
            } else if !self.items.is_empty() {
                self.selected_index = Some(0);
            }
        }

        if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
            if let Some(idx) = self.selected_index {
                if let Some(item) = self.items.get(idx) {
                    Self::open_inventory_item(item)?;
                }
            }
        }

        // Mouse click handling
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let mouse = rl.get_mouse_position();
            // Refresh button in toolbar
            if point_in_rect(mouse, m.refresh_button) {
                let _ = self.reload();
                return Ok(());
            }

            if mouse.y >= m.toolbar_height {
                let relative_y = mouse.y - m.toolbar_height + self.scroll_offset;
                let clicked_idx = (relative_y / m.row_height) as usize;
                if clicked_idx < self.items.len() {
                    if self.selected_index == Some(clicked_idx) {
                        Self::open_inventory_item(&self.items[clicked_idx])?;
                    } else {
                        self.selected_index = Some(clicked_idx);
                    }
                }
            }
        }

        Ok(())
    }

    fn preload_visible_textures(&mut self, rl: &RaylibHandle) {
        let m = Metrics::new(rl.get_screen_width(), rl.get_screen_height());
        let screen_height = rl.get_screen_height() as f32;
        let start_idx = (self.scroll_offset / m.row_height).floor() as usize;
        let visible_count = ((screen_height - m.toolbar_height) / m.row_height).ceil() as usize + 2;
        let end_idx = (start_idx + visible_count).min(self.items.len());

        for i in start_idx..end_idx {
            if self.items[i].kind == CellKind::Image {
                if let Some(ref path) = self.items[i].file_path {
                    self.textures.request(path);
                }
            }
        }
    }

    fn draw(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();
        let mouse = rl.get_mouse_position();
        let m = Metrics::new(screen_width, screen_height);

        let mut d = rl.begin_drawing(thread);
        d.clear_background(VIEWER_BACKGROUND);

        let start_idx = (self.scroll_offset / m.row_height).floor() as usize;
        let visible_count =
            ((screen_height as f32 - m.toolbar_height) / m.row_height).ceil() as usize + 2;
        let end_idx = (start_idx + visible_count).min(self.items.len());

        for i in start_idx..end_idx {
            let item = &self.items[i];
            let item_y = m.toolbar_height + (i as f32 * m.row_height) - self.scroll_offset;
            let row_rect = Rectangle::new(0.0, item_y, screen_width as f32, m.row_height);

            let is_selected = self.selected_index == Some(i);
            let is_hovered = point_in_rect(mouse, row_rect);

            if is_selected {
                d.draw_rectangle_rec(row_rect, Color::new(50, 70, 110, 255));
            } else if is_hovered {
                d.draw_rectangle_rec(row_rect, Color::new(35, 35, 45, 255));
            }

            d.draw_rectangle_lines_ex(row_rect, 1.0, Color::new(50, 50, 60, 255));

            let thumb_rect = Rectangle::new(
                m.padding,
                item_y + (m.row_height - m.thumb_size) / 2.0,
                m.thumb_size,
                m.thumb_size,
            );

            match item.kind {
                CellKind::Image => {
                    if let Some(ref path) = item.file_path {
                        if let Some(tex) = self.textures.get(path) {
                            let scale = (thumb_rect.width / tex.width as f32)
                                .min(thumb_rect.height / tex.height as f32);
                            let dw = tex.width as f32 * scale;
                            let dh = tex.height as f32 * scale;
                            let dest = Rectangle::new(
                                thumb_rect.x + (thumb_rect.width - dw) / 2.0,
                                thumb_rect.y + (thumb_rect.height - dh) / 2.0,
                                dw,
                                dh,
                            );
                            let src = Rectangle::new(0.0, 0.0, tex.width as f32, tex.height as f32);
                            d.draw_texture_pro(tex, src, dest, Vector2::new(0.0, 0.0), 0.0, WHITE);
                        } else {
                            d.draw_rectangle_lines_ex(
                                thumb_rect,
                                1.0,
                                Color::new(100, 100, 120, 255),
                            );
                        }
                    }
                }
                CellKind::Grid => {
                    let inset_px = m.thumb_size * 0.16;
                    d.draw_rectangle_lines_ex(thumb_rect, 2.0, Color::new(120, 200, 160, 255));
                    let inset = Rectangle::new(
                        thumb_rect.x + inset_px,
                        thumb_rect.y + inset_px,
                        thumb_rect.width - inset_px * 2.0,
                        thumb_rect.height - inset_px * 2.0,
                    );
                    d.draw_rectangle_lines_ex(inset, 1.5, Color::new(120, 200, 160, 255));
                }
                kind => {
                    let color = match kind {
                        CellKind::Audio => Color::new(145, 85, 190, 255),
                        CellKind::Video => Color::new(200, 90, 80, 255),
                        CellKind::RubyScript => Color::new(190, 55, 65, 255),
                        CellKind::File => Color::new(90, 105, 120, 255),
                        CellKind::Image | CellKind::Grid => unreachable!(),
                    };
                    d.draw_rectangle_rec(thumb_rect, color);
                    let size = 14.0 * m.scale;
                    let label = ui::fit(kind.label(), size, thumb_rect.width - 4.0 * m.scale);
                    ui::draw_text_centered(&mut d, &label, thumb_rect, size, WHITE);
                }
            }

            // Item details text
            let text_x = thumb_rect.x + thumb_rect.width + 16.0 * m.scale;
            let title = match item.kind {
                CellKind::Grid => "Sub-Grid Container".to_string(),
                _ => item
                    .file_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or(item.kind.label())
                    .to_string(),
            };

            let hint_size = 15.0 * m.scale;
            let open_hint = "[Click/Enter to Open]";
            let hint_w = if item.kind != CellKind::Grid {
                ui::measure(open_hint, hint_size) + 20.0 * m.scale
            } else {
                0.0
            };
            let text_width = (screen_width as f32 - text_x - hint_w - m.padding).max(0.0);

            let title_size = 20.0 * m.scale;
            let title = ui::fit(&title, title_size, text_width);
            ui::draw_text(
                &mut d,
                &title,
                text_x,
                item_y + m.row_height * 0.16,
                title_size,
                BUTTON_TEXT,
            );

            let sub_size = 15.0 * m.scale;
            let subtext = format!(
                "Hierarchy: {} | Position: ({}, {})",
                item.location, item.col, item.row
            );
            let subtext = ui::fit(&subtext, sub_size, text_width);
            ui::draw_text(
                &mut d,
                &subtext,
                text_x,
                item_y + m.row_height * 0.56,
                sub_size,
                Color::new(170, 170, 185, 255),
            );

            if item.kind != CellKind::Grid {
                ui::draw_text(
                    &mut d,
                    open_hint,
                    screen_width as f32 - hint_w,
                    item_y + (m.row_height - hint_size) / 2.0,
                    hint_size,
                    Color::new(100, 160, 240, 255),
                );
            }
        }

        // Draw Toolbar header
        d.draw_rectangle(
            0,
            0,
            screen_width,
            m.toolbar_height.ceil() as i32,
            TOOLBAR_BG,
        );
        let ref_hovered = point_in_rect(mouse, m.refresh_button);
        d.draw_rectangle_rec(
            m.refresh_button,
            if ref_hovered {
                BUTTON_HOVER_BG
            } else {
                BUTTON_BG
            },
        );
        ui::draw_text_centered(
            &mut d,
            "Refresh",
            m.refresh_button,
            m.button_text,
            BUTTON_TEXT,
        );

        let count_size = 18.0 * m.scale;
        let count_x = m.refresh_button.x + m.refresh_button.width + 16.0 * m.scale;
        let count_str = ui::fit(
            &format!("Total Attached Items: {}", self.items.len()),
            count_size,
            screen_width as f32 - count_x - m.padding,
        );
        ui::draw_text(
            &mut d,
            &count_str,
            count_x,
            (m.toolbar_height - count_size) / 2.0,
            count_size,
            VIEWER_TEXT,
        );

        // Scrollbar
        let content_height = self.items.len() as f32 * m.row_height;
        let visible_height = screen_height as f32 - m.toolbar_height;
        if content_height > visible_height {
            let bar_width = 8.0 * m.scale;
            let bar_height = (visible_height * (visible_height / content_height)).max(20.0);
            let bar_y = m.toolbar_height + (self.scroll_offset / content_height) * visible_height;
            let bar_rect = Rectangle::new(
                screen_width as f32 - bar_width - 2.0,
                bar_y,
                bar_width,
                bar_height,
            );
            d.draw_rectangle_rec(bar_rect, Color::new(100, 100, 120, 180));
        }
    }

    fn open_inventory_item(item: &InventoryItem) -> Result<(), String> {
        let Some(path) = &item.file_path else {
            return Ok(());
        };
        if item.kind == CellKind::Image {
            spawn_mode(Mode::ImageViewer(path.clone()), None)
        } else {
            open_with_system(path)
        }
    }
}

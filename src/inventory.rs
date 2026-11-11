use std::path::PathBuf;

use raylib::prelude::*;

use crate::{
    imaging::ThumbnailCache, load_existing_root, open_with_system, persistence::CellKind,
    point_in_rect, spawn_mode, ui, Mode, BUTTON_BG, BUTTON_HOVER_BG, BUTTON_TEXT, TOOLBAR_BG,
    VIEWER_BACKGROUND, VIEWER_TEXT, WHITE, WINDOW_HEIGHT, WINDOW_WIDTH,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRow {
    pub cell: (i64, i64),
    pub index: usize,
    pub name: String,
}

pub fn grid_assets(grid: &crate::persistence::SavedGrid) -> Vec<AssetRow> {
    let mut rows = grid
        .stacks()
        .flat_map(|(cell, items)| {
            items.iter().enumerate().map(move |(index, item)| AssetRow {
                cell,
                index,
                name: format!(
                    "{} ({},{}, item {})",
                    item.label(),
                    cell.0,
                    cell.1,
                    index + 1
                ),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|asset| (asset.cell.1, asset.cell.0, asset.index));
    rows
}

pub struct AssetPanel {
    pub controls: crate::stack_panel::Panel,
}

impl AssetPanel {
    pub fn new() -> Self {
        Self {
            controls: crate::stack_panel::Panel {
                cell: (0, 0),
                scroll: 0,
                pending_remove: None,
                query: String::new(),
                focused: Some(0),
            },
        }
    }

    pub fn layout(
        &self,
        rows: &[AssetRow],
        width: i32,
        height: i32,
        title: &str,
    ) -> crate::stack_panel::Layout {
        let names = rows
            .iter()
            .map(|asset| asset.name.clone())
            .collect::<Vec<_>>();
        let mut layout = self.controls.layout(&names, width, height);
        layout.set_title(format!("Asset Inventory - {title} - {} items", rows.len()));
        layout
    }
}

/// Window-scaled layout metrics for the inventory list.
struct Metrics {
    scale: f32,
    toolbar_height: f32,
    row_height: f32,
    thumb_size: f32,
    padding: f32,
    refresh_button: Rectangle,
    options: [Rectangle; 3],
    button_text: f32,
    search: Rectangle,
    selectors: [Rectangle; 2],
    actions: [Rectangle; 3],
}

impl Metrics {
    fn new(screen_width: i32, screen_height: i32) -> Self {
        let scale = ui::scale(screen_width, screen_height).min(screen_height as f32 / 340.0);
        let button_text = 20.0 * scale;
        let margin = 8.0 * scale;
        let button_height = button_text + 14.0 * scale;
        let button_width = (screen_width as f32 - margin * 5.0) / 4.0;
        Self {
            scale,
            toolbar_height: button_height * 3.0 + margin * 4.0,
            row_height: 72.0 * scale,
            thumb_size: 54.0 * scale,
            padding: 12.0 * scale,
            refresh_button: Rectangle::new(margin, margin, button_width, button_height),
            options: std::array::from_fn(|index| Rectangle::new(margin + (index + 1) as f32 * (button_width + margin), margin, button_width, button_height)),
            button_text,
            search: Rectangle::new(margin, button_height + margin * 2.0, button_width * 2.0 + margin, button_height),
            selectors: std::array::from_fn(|index| Rectangle::new(margin + (index + 2) as f32 * (button_width + margin), button_height + margin * 2.0, button_width, button_height)),
            actions: std::array::from_fn(|index| Rectangle::new(margin + (index + 1) as f32 * (button_width + margin), button_height * 2.0 + margin * 3.0, button_width, button_height)),
        }
    }

    fn menu_rows(&self, index: usize, count: usize, height: i32) -> Vec<Rectangle> {
        let anchor = self.selectors[index];
        let top = anchor.y + anchor.height;
        let row_height = anchor.height.min((height as f32 - top - self.padding).max(1.0) / count as f32);
        (0..count).map(|row| Rectangle::new(anchor.x, top + row as f32 * row_height, anchor.width, (row_height - 1.0).max(1.0))).collect()
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum SortOrder {
    #[default]
    Name,
    Type,
    Location,
}

const ASSET_TYPES: [Option<CellKind>; 7] = [None, Some(CellKind::Image), Some(CellKind::Audio), Some(CellKind::Video), Some(CellKind::RubyScript), Some(CellKind::File), Some(CellKind::Grid)];
const SORT_ORDERS: [SortOrder; 3] = [SortOrder::Name, SortOrder::Type, SortOrder::Location];

impl SortOrder {
    fn label(self) -> &'static str {
        match self { Self::Name => "Name", Self::Type => "Type", Self::Location => "Location" }
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
    all_items: Vec<InventoryItem>,
    query: String,
    search_focused: bool,
    kind: Option<CellKind>,
    sort: SortOrder,
    dropdown: Option<usize>,
    message: String,
    scroll_offset: f32,
    selected_index: Option<usize>,
    textures: ThumbnailCache,
    missing_only: bool,
    scale_menu: ui::ScaleMenu,
    suite_menu: crate::suite::Menu,
    quit: bool,
}

impl InventoryApp {
    pub fn new(json_path: PathBuf) -> Result<Self, String> {
        let mut app = Self {
            json_path: json_path.clone(),
            items: Vec::new(),
            all_items: Vec::new(),
            query: String::new(),
            search_focused: false,
            kind: None,
            sort: SortOrder::Name,
            dropdown: None,
            message: String::new(),
            scroll_offset: 0.0,
            selected_index: None,
            textures: ThumbnailCache::new(),
            missing_only: false,
            scale_menu: ui::ScaleMenu::default(),
            suite_menu: crate::suite::Menu::default(),
            quit: false,
        };
        app.reload()?;
        Ok(app)
    }

    pub fn reload(&mut self) -> Result<(), String> {
        let root = load_existing_root(&self.json_path)?;
        let locked = root.lock().unwrap_or_else(|e| e.into_inner());
        let raw_items = locked.collect_all_items("");
        self.all_items = raw_items
            .into_iter()
            .map(|(location, col, row, kind, file_path)| InventoryItem {
                location,
                col,
                row,
                kind,
                file_path,
            })
            .collect();
        self.apply_filters();
        Ok(())
    }

    fn apply_filters(&mut self) {
        let query = self.query.to_lowercase();
        self.items = self.all_items.iter().filter(|item| {
            (!self.missing_only || item.file_path.as_ref().is_some_and(|path| !path.exists()))
                && self.kind.is_none_or(|kind| kind == item.kind)
                && (query.is_empty() || item.location.to_lowercase().contains(&query)
                    || item.file_path.as_ref().is_some_and(|path| path.to_string_lossy().to_lowercase().contains(&query)))
        }).cloned().collect();
        self.items.sort_by_cached_key(|item| {
            let name = item.file_path.as_ref().and_then(|path| path.file_name()).map(|name| name.to_string_lossy().to_lowercase()).unwrap_or_default();
            let location = item.location.clone();
            match self.sort {
                SortOrder::Name => (name, location, item.row, item.col, String::new()),
                SortOrder::Type => (item.kind.label().to_owned(), name, item.row, item.col, location),
                SortOrder::Location => (location, String::new(), item.row, item.col, name),
            }
        });
        self.selected_index = None;
        self.scroll_offset = 0.0;
        self.message.clear();
    }

    fn selected_path(&self) -> Option<&PathBuf> {
        self.selected_index.and_then(|index| self.items.get(index)).and_then(|item| item.file_path.as_ref())
    }

    fn copy_path(&mut self, rl: &mut RaylibHandle) {
        let result = self.selected_path().ok_or_else(|| "Select an asset first".to_owned())
            .and_then(|path| rl.set_clipboard_text(&path.to_string_lossy()).map_err(|error| error.to_string()));
        self.message = result.map_or_else(|error| error, |_| "Path copied".to_owned());
    }

    fn open_folder(&mut self) {
        let result = self.selected_path().and_then(|path| path.parent()).filter(|path| path.is_dir())
            .ok_or_else(|| "Asset folder is not available".to_owned()).and_then(open_with_system);
        self.message = result.map_or_else(|error| error, |_| "Opened asset folder".to_owned());
    }

    pub fn run(mut self) -> Result<(), String> {
        let (mut rl, thread) = raylib::init()
            .size(WINDOW_WIDTH, WINDOW_HEIGHT)
            .title("Selenite Inventory - List View")
            .resizable()
            .build();
        rl.set_target_fps(60);
        rl.set_exit_key(None);
        rl.set_window_min_size(480, 320);
        ui::fit_window_to_display(&mut rl, WINDOW_WIDTH, WINDOW_HEIGHT);
        ui::set_user_scale(crate::settings::saved_ui_scale());
        ui::limit_to_toolbar();
        ui::load_font(&thread);
        let mut renderer = crate::rendering::Renderer::default();

        let result = (|| {
            while !rl.window_should_close() && !self.quit {
                self.update(&mut rl, &thread)?;
                self.preload_visible_textures(&rl);
                self.textures.pump(&mut rl, &thread);
                self.draw(&mut rl, &thread, &mut renderer)?;
            }
            Ok(())
        })();
        // Free GPU textures before the window and GL context close.
        self.textures.clear();
        self.suite_menu.clear_resources();
        drop(renderer);
        drop(rl);
        result
    }

    fn update(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) -> Result<(), String> {
        let suite_source = self.selected_path().cloned();
        if self.suite_menu.update(rl, thread, suite_source.as_deref(), Some(&self.json_path)) {
            self.scale_menu.open = false;
            return Ok(());
        }
        if self.scale_menu.update(rl) { return Ok(()); }
        if let Some(scale) = ui::handle_scale_keys(rl) {
            let _ = crate::settings::save_ui_scale(scale);
        }
        let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        let m = Metrics::new(rl.get_screen_width(), rl.get_screen_height());
        if self.dropdown.is_some() && rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) { self.dropdown = None; return Ok(()); }
        if self.search_focused && rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) { self.search_focused = false; return Ok(()); }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) { self.quit = true; return Ok(()); }
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_F) { self.search_focused = true; self.dropdown = None; }
        if self.search_focused {
            let previous = self.query.clone();
            if ctrl && rl.is_key_pressed(KeyboardKey::KEY_A) { self.query.clear(); }
            if ctrl && rl.is_key_pressed(KeyboardKey::KEY_V) {
                if let Ok(text) = rl.get_clipboard_text() { self.query.push_str(text.trim()); }
            }
            while let Some(character) = rl.get_char_pressed() {
                if !ctrl && !character.is_control() && self.query.len() < 1024 { self.query.push(character); }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) { self.query.pop(); }
            if self.query != previous { self.apply_filters(); }
            if rl.is_key_pressed(KeyboardKey::KEY_ENTER) || rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
                self.search_focused = false;
                self.selected_index = (!self.items.is_empty()).then_some(0);
                return Ok(());
            }
        }
        if let Some(index) = self.dropdown {
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                let count = if index == 0 { ASSET_TYPES.len() } else { SORT_ORDERS.len() };
                let rows = m.menu_rows(index, count, rl.get_screen_height());
                if let Some(choice) = rows.iter().position(|row| row.check_collision_point_rec(rl.get_mouse_position())) {
                    if index == 0 { self.kind = ASSET_TYPES[choice]; } else { self.sort = SORT_ORDERS[choice]; }
                    self.apply_filters();
                }
                self.dropdown = None;
            }
            return Ok(());
        }
        if !self.search_focused && rl.is_key_pressed(KeyboardKey::KEY_M) {
            self.missing_only = !self.missing_only;
            self.scroll_offset = 0.0;
            self.reload()?;
        }
        if !self.search_focused && ctrl && rl.is_key_pressed(KeyboardKey::KEY_C) { self.copy_path(rl); }
        if !self.search_focused && ctrl && rl.is_key_pressed(KeyboardKey::KEY_O) { self.open_folder(); }
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
        if !self.search_focused && rl.is_key_pressed(KeyboardKey::KEY_UP) {
            if let Some(idx) = self.selected_index {
                if idx > 0 {
                    self.selected_index = Some(idx - 1);
                }
            } else if !self.items.is_empty() {
                self.selected_index = Some(0);
            }
        }
        if !self.search_focused && rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            if let Some(idx) = self.selected_index {
                if idx + 1 < self.items.len() {
                    self.selected_index = Some(idx + 1);
                }
            } else if !self.items.is_empty() {
                self.selected_index = Some(0);
            }
        }
        if !self.search_focused && (rl.is_key_pressed(KeyboardKey::KEY_UP) || rl.is_key_pressed(KeyboardKey::KEY_DOWN)) {
            if let Some(index) = self.selected_index {
                let top = index as f32 * m.row_height;
                if top < self.scroll_offset { self.scroll_offset = top; }
                else if top + m.row_height > self.scroll_offset + visible_height { self.scroll_offset = (top + m.row_height - visible_height).clamp(0.0, max_scroll); }
            }
        }

        if !self.search_focused && rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
            if let Some(idx) = self.selected_index {
                if let Some(item) = self.items.get(idx) {
                    Self::open_inventory_item(item)?;
                }
            }
        }

        // Mouse click handling
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let mouse = rl.get_mouse_position();
            self.search_focused = point_in_rect(mouse, m.search);
            if self.search_focused { return Ok(()); }
            if let Some(index) = m.selectors.iter().position(|rect| rect.check_collision_point_rec(mouse)) {
                self.dropdown = Some(index);
                return Ok(());
            }
            if self.selected_path().is_some() {
                if m.actions[0].check_collision_point_rec(mouse) { self.copy_path(rl); return Ok(()); }
                if m.actions[1].check_collision_point_rec(mouse) { self.open_folder(); return Ok(()); }
                if m.actions[2].check_collision_point_rec(mouse) {
                    self.message = crate::desktop::for_path(self.selected_path().unwrap()).unwrap_or_else(|error| error);
                    return Ok(());
                }
            }
            // Refresh button in toolbar
            if point_in_rect(mouse, m.refresh_button) {
                let _ = self.reload();
                return Ok(());
            }
            if point_in_rect(mouse, m.options[0]) {
                self.missing_only = !self.missing_only;
                self.scroll_offset = 0.0;
                self.reload()?;
                return Ok(());
            }
            if point_in_rect(mouse, m.options[1]) { self.scale_menu.open = true; return Ok(()); }
            if point_in_rect(mouse, m.options[2]) { self.suite_menu.open = true; return Ok(()); }

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

    fn draw(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        renderer: &mut crate::rendering::Renderer,
    ) -> Result<(), String> {
        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();
        let mouse = rl.get_mouse_position();
        let m = Metrics::new(screen_width, screen_height);

        let mut d = renderer.begin(rl, thread)?;
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
            &ui::fit("Refresh", m.button_text, m.refresh_button.width - 12.0),
            m.refresh_button,
            m.button_text,
            BUTTON_TEXT,
        );
        for (index, rect) in m.options.iter().enumerate() {
            d.draw_rectangle_rec(*rect, if index == 0 && self.missing_only || point_in_rect(mouse, *rect) { BUTTON_HOVER_BG } else { BUTTON_BG });
            ui::draw_text_centered(&mut d, &ui::fit(["Missing", "Scale", "Suite"][index], m.button_text, rect.width - 12.0), *rect, m.button_text, BUTTON_TEXT);
        }
        d.draw_rectangle_rec(m.search, Color::new(32, 34, 41, 255));
        d.draw_rectangle_lines_ex(m.search, 1.0, if self.search_focused { Color::SKYBLUE } else { Color::new(75, 78, 90, 255) });
        let query = if self.query.is_empty() { "Search" } else { &self.query };
        ui::draw_text_in(&mut d, &ui::fit(query, m.button_text * 0.85, m.search.width - 20.0), m.search, 10.0, m.button_text * 0.85, BUTTON_TEXT);
        for (index, rect) in m.selectors.iter().enumerate() {
            d.draw_rectangle_rec(*rect, if self.dropdown == Some(index) || rect.check_collision_point_rec(mouse) { BUTTON_HOVER_BG } else { BUTTON_BG });
            let label = if index == 0 { format!("Type: {}", self.kind.map_or("All", CellKind::label)) } else { format!("Sort: {}", self.sort.label()) };
            ui::draw_text_centered(&mut d, &ui::fit(&label, m.button_text * 0.8, rect.width - 12.0), *rect, m.button_text * 0.8, BUTTON_TEXT);
        }
        for (rect, label) in m.actions.iter().zip(["Copy Path", "Folder", "Shortcut"]) {
            let enabled = self.selected_path().is_some();
            d.draw_rectangle_rec(*rect, if !enabled { Color::new(40, 42, 48, 255) } else if rect.check_collision_point_rec(mouse) { BUTTON_HOVER_BG } else { BUTTON_BG });
            ui::draw_text_centered(&mut d, &ui::fit(label, m.button_text * 0.8, rect.width - 12.0), *rect, m.button_text * 0.8, if enabled { BUTTON_TEXT } else { Color::GRAY });
        }

        let count_size = 18.0 * m.scale;
        let count_x = m.padding;
        let count = if self.message.is_empty() { format!("{}: {} / {}", if self.missing_only { "Missing" } else { "Assets" }, self.items.len(), self.all_items.len()) } else { self.message.clone() };
        let count_str = ui::fit(
            &count,
            count_size,
            m.actions[0].x - count_x - m.padding,
        );
        ui::draw_text(
            &mut d,
            &count_str,
            count_x,
            m.actions[0].y + (m.actions[0].height - count_size) / 2.0,
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
        if let Some(index) = self.dropdown {
            let count = if index == 0 { ASSET_TYPES.len() } else { SORT_ORDERS.len() };
            for (choice, rect) in m.menu_rows(index, count, screen_height).iter().enumerate() {
                let selected = if index == 0 { self.kind == ASSET_TYPES[choice] } else { self.sort == SORT_ORDERS[choice] };
                d.draw_rectangle_rec(*rect, if selected || rect.check_collision_point_rec(mouse) { BUTTON_HOVER_BG } else { Color::new(34, 37, 46, 255) });
                let label = if index == 0 { ASSET_TYPES[choice].map_or("All", CellKind::label) } else { SORT_ORDERS[choice].label() };
                let font = (m.button_text * 0.8).min(rect.height * 0.65).max(8.0);
                ui::draw_text_in(&mut d, &ui::fit(label, font, rect.width - 16.0), *rect, 8.0, font, BUTTON_TEXT);
            }
        }
        self.scale_menu.draw(&mut d, screen_width, screen_height);
        self.suite_menu.draw(&mut d, screen_width, screen_height);
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::SavedGrid;

    #[test]
    fn asset_search_type_and_sort_filters_compose_without_reloading() {
        let directory = std::env::temp_dir().join(format!("selenite-asset-filters-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let alpha = directory.join("Alpha.rb");
        let zeta = directory.join("zeta.txt");
        std::fs::write(&alpha, "puts 1").unwrap();
        std::fs::write(&zeta, "text").unwrap();
        let missing = directory.join("missing.wav");
        let mut grid = SavedGrid::new();
        grid.push_file(0, 0, zeta.clone());
        grid.push_file(1, 0, alpha.clone());
        grid.push_file(2, 0, missing.clone());
        let path = directory.join("grid.json");
        grid.save(&path).unwrap();
        let mut app = InventoryApp::new(path.clone()).unwrap();
        assert_eq!(app.items[0].file_path.as_ref(), Some(&alpha));
        std::fs::remove_file(&path).unwrap();
        app.query = "ALPHA".into();
        app.apply_filters();
        assert_eq!(app.items.len(), 1);
        assert_eq!(app.items[0].file_path.as_ref(), Some(&alpha));
        app.kind = Some(CellKind::Audio);
        app.apply_filters();
        assert!(app.items.is_empty());
        app.query.clear();
        app.missing_only = true;
        app.apply_filters();
        assert_eq!(app.items.len(), 1);
        assert_eq!(app.items[0].file_path.as_ref(), Some(&missing));
        app.kind = None;
        app.missing_only = false;
        app.sort = SortOrder::Location;
        app.apply_filters();
        assert_eq!(app.items[0].file_path.as_ref(), Some(&zeta));
        app.sort = SortOrder::Type;
        app.apply_filters();
        assert!(app.items.windows(2).all(|pair| pair[0].kind.label() <= pair[1].kind.label()));
        assert_eq!(app.all_items.len(), 3);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn asset_manager_controls_and_dropdowns_fit_scaled_windows() {
        for zoom in [0.5, 0.75, 1.0, 1.25, 2.0, 3.0] {
            ui::set_user_scale(zoom);
            for (width, height) in [(480, 320), (960, 720), (1100, 760), (3840, 2160)] {
                let metrics = Metrics::new(width, height);
                assert!(metrics.toolbar_height < height as f32);
                let controls: Vec<_> = std::iter::once(metrics.refresh_button).chain(metrics.options).chain(std::iter::once(metrics.search)).chain(metrics.selectors).chain(metrics.actions).collect();
                for (index, rect) in controls.iter().enumerate() {
                    assert!(rect.x >= 0.0 && rect.x + rect.width <= width as f32 + 0.01);
                    assert!(rect.y >= 0.0 && rect.y + rect.height <= metrics.toolbar_height);
                    for other in &controls[index + 1..] { assert!(!rect.check_collision_recs(*other)); }
                }
                for (index, count) in [(0, ASSET_TYPES.len()), (1, SORT_ORDERS.len())] {
                    for rect in metrics.menu_rows(index, count, height) {
                        assert!(rect.y >= 0.0 && rect.y + rect.height <= height as f32);
                    }
                }
            }
        }
        ui::set_user_scale(1.0);
    }

    #[test]
    fn missing_file_filter_keeps_only_broken_links() {
        let directory = std::env::temp_dir().join(format!("selenite-missing-assets-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let existing = directory.join("existing.txt");
        let missing = directory.join("missing.txt");
        std::fs::write(&existing, "present").unwrap();
        let mut grid = SavedGrid::new();
        grid.push_file(0, 0, existing);
        grid.push_file(1, 0, missing.clone());
        grid.push_grid(2, 0);
        let path = directory.join("grid.json");
        grid.save(&path).unwrap();
        let mut app = InventoryApp::new(path).unwrap();
        assert_eq!(app.items.len(), 3);
        app.missing_only = true;
        app.reload().unwrap();
        assert_eq!(app.items.len(), 1);
        assert_eq!(app.items[0].file_path.as_ref(), Some(&missing));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn asset_inventory_lists_each_stack_item_only_in_its_own_grid() {
        let mut grid = SavedGrid::new();
        grid.push_files(4, 2, ["song.mp3", "cover.png", "notes.txt"]);
        let child = grid.push_grid(0, 0);
        child
            .lock()
            .unwrap()
            .push_files(0, 0, ["child-one.txt", "child-two.txt"]);
        let assets = grid_assets(&grid);
        assert_eq!(assets.len(), 4);
        assert_eq!(assets[1].cell, (4, 2));
        assert_eq!(assets[1].index, 0);
        assert_eq!(assets[3].index, 2);
        assert!(!assets.iter().any(|asset| asset.name.contains("child-one")));
        assert_eq!(grid_assets(&child.lock().unwrap()).len(), 2);
        let panel = AssetPanel::new();
        let layout = panel.layout(&assets, 640, 480, "root");
        assert!(layout
            .buttons
            .iter()
            .any(|(_, _, _, action)| *action == crate::stack_panel::Action::Remove(3)));
    }
}

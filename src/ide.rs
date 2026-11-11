use std::{
    collections::VecDeque,
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};

use crate::{
    editor::{draw_mono, measure_mono},
    ui,
};
use raylib::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
enum Tab {
    Output,
    Console,
    #[default]
    Terminal,
    Help,
}

const HELP: &[(&str, &str)] = &[
    ("IDE Guide", include_str!("../docs/ide.md")),
    ("First Game", include_str!("../docs/game-tutorial.md")),
    ("Game API", include_str!("../docs/game.md")),
    ("Selenite API", include_str!("../docs/manual.md")),
    ("Native Raylib", include_str!("../docs/raylib-native.md")),
    (
        "All Raylib Functions",
        include_str!("../docs/raylib-api.md"),
    ),
    ("Storage", include_str!("../docs/game-storage.md")),
    ("Networking", include_str!("../docs/networking.md")),
    ("Plugins", include_str!("../docs/plugins.md")),
    ("Projects", include_str!("../docs/projects.md")),
];

struct HelpCache {
    width: i32,
    size: i32,
    topic: usize,
    query: String,
    layout: crate::markdown::Layout,
}

struct QuickOpen {
    query: String,
    files: Vec<(std::path::PathBuf, String)>,
    selected: usize,
}

impl QuickOpen {
    fn matches(&self) -> Vec<&std::path::PathBuf> {
        let query = self.query.to_lowercase();
        self.files.iter().filter(|(_, name)| name.contains(&query)).map(|(path, _)| path).collect()
    }

    fn bounds(width: i32, height: i32) -> Rectangle {
        let width_menu = (width as f32 - 24.0).clamp(1.0, 720.0);
        let height_menu = (height as f32 - 24.0).clamp(1.0, 500.0);
        Rectangle::new((width as f32 - width_menu) / 2.0, (height as f32 - height_menu) / 2.0, width_menu, height_menu)
    }
}

const DOCK_BUTTONS: usize = 11;

#[derive(Clone, Copy)]
struct Metrics {
    unit: f32,
    font: f32,
    help_font: f32,
    mono_font: f32,
    padding: f32,
    row_height: f32,
    bar_height: f32,
    columns: usize,
}

struct HelpGeometry {
    previous: Rectangle,
    next: Rectangle,
    topic: Rectangle,
    search: Option<Rectangle>,
    content: Rectangle,
}

impl Metrics {
    fn new(width: i32, height: i32) -> Self {
        Self::at_scale(width, ui::scale(width, height))
    }

    fn at_scale(width: i32, scale: f32) -> Self {
        let unit = scale / (1.25 * 760.0 / 720.0);
        let columns = ((width.max(1) as f32 + 0.5) / (100.0 * unit))
            .floor()
            .clamp(1.0, DOCK_BUTTONS as f32) as usize;
        Self {
            unit,
            font: (12.0 * scale).round().max(8.0),
            help_font: (13.0 * scale).round().max(10.0),
            mono_font: (14.0 * scale).round().max(8.0),
            padding: (8.0 * unit).round().max(4.0),
            row_height: (24.0 * unit).round().max(18.0),
            bar_height: (32.0 * unit).round().max(22.0),
            columns,
        }
    }

    fn toolbar_height(self) -> f32 {
        DOCK_BUTTONS.div_ceil(self.columns) as f32 * self.bar_height
    }

    fn editor_height(self, layout: &Layout, height: i32) -> i32 {
        let toolbar = self.toolbar_height();
        let minimum = (140.0 * self.unit).round().max(90.0);
        let available = (height as f32 - toolbar - minimum).max(0.0);
        let panel = if layout.visible {
            (height as f32 * layout.panel_fraction)
                .round()
                .min(available)
        } else {
            0.0
        };
        (height as f32 - toolbar - panel).max(1.0).round() as i32
    }

    fn button(self, index: usize, width: i32, top: f32) -> Rectangle {
        let button_width = width as f32 / self.columns as f32;
        Rectangle::new(
            (index % self.columns) as f32 * button_width,
            top + (index / self.columns) as f32 * self.bar_height,
            button_width,
            self.bar_height,
        )
    }

    fn menu_height(self) -> f32 {
        (14.0 * self.unit * (1.25 * 760.0 / 720.0)).round().max(8.0) * 2.0
    }

    fn dock_top(self, layout: &Layout, height: i32) -> f32 {
        if layout.dock_top { self.menu_height() } else { self.editor_height(layout, height) as f32 }
    }

    fn editor_offset(self, layout: &Layout, height: i32) -> f32 {
        if layout.dock_top { height as f32 - self.editor_height(layout, height) as f32 } else { 0.0 }
    }

    fn body(self, layout: &Layout, width: i32, height: i32) -> Rectangle {
        let top = self.dock_top(layout, height) + self.toolbar_height();
        let panel = height as f32 - self.editor_height(layout, height) as f32 - self.toolbar_height();
        Rectangle::new(0.0, top, width as f32, panel.max(1.0))
    }

    fn help(self, body: Rectangle, search: bool) -> HelpGeometry {
        let nav_width = (40.0 * self.unit).round().max(22.0);
        let stacked =
            search && body.width < nav_width * 2.0 + 400.0 * self.unit + self.padding * 2.0;
        let topic_width = if search && !stacked {
            (240.0 * self.unit).min(body.width - nav_width * 2.0)
        } else {
            body.width - nav_width * 2.0
        };
        let header_height = self.bar_height * if stacked { 2.0 } else { 1.0 };
        let topic = Rectangle::new(
            body.x + nav_width * 2.0,
            body.y,
            topic_width,
            self.bar_height,
        );
        let search = search.then(|| {
            if stacked {
                Rectangle::new(
                    body.x + self.padding,
                    body.y + self.bar_height,
                    body.width - self.padding * 2.0,
                    self.bar_height,
                )
            } else {
                let left = topic.x + topic.width + self.padding;
                Rectangle::new(
                    left,
                    body.y,
                    (body.x + body.width - left - self.padding).max(1.0),
                    self.bar_height,
                )
            }
        });
        HelpGeometry {
            previous: Rectangle::new(body.x, body.y, nav_width, self.bar_height),
            next: Rectangle::new(body.x + nav_width, body.y, nav_width, self.bar_height),
            topic,
            search,
            content: Rectangle::new(
                body.x,
                body.y + header_height,
                body.width,
                (body.height - header_height).max(1.0),
            ),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Layout {
    panel_fraction: f32,
    dock_top: bool,
    visible: bool,
    tab: Tab,
    application: bool,
    window_size: (i32, i32),
    sidebar_fraction: f32,
    sidebar_visible: bool,
    workspaces: Vec<std::path::PathBuf>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            panel_fraction: 0.34,
            dock_top: false,
            visible: true,
            tab: Tab::Terminal,
            application: false,
            window_size: (1100, 760),
            sidebar_fraction: 0.23,
            sidebar_visible: true,
            workspaces: Vec::new(),
        }
    }
}

impl Layout {
    fn sanitize(mut self) -> Self {
        self.panel_fraction = if self.panel_fraction.is_finite() {
            self.panel_fraction.clamp(0.18, 0.7)
        } else {
            0.34
        };
        self.window_size.0 = self.window_size.0.clamp(420, 16384);
        self.window_size.1 = self.window_size.1.clamp(300, 16384);
        self.sidebar_fraction = if self.sidebar_fraction.is_finite() {
            self.sidebar_fraction.clamp(0.16, 0.45)
        } else {
            0.23
        };
        self.workspaces.truncate(30);
        self
    }
}

pub(crate) struct Workspace {
    pub session: Session,
    layout: Layout,
    directory: std::path::PathBuf,
    width: i32,
    height: i32,
    resizing: bool,
    sidebar_resizing: bool,
    dock_drag: Option<Vector2>,
    dock_moving: bool,
    workspace_tab: bool,
    browse_dir: std::path::PathBuf,
    entries: Vec<std::path::PathBuf>,
    sidebar_scroll: usize,
    help_topic: usize,
    help_scroll: f32,
    help_query: String,
    help_cache: Option<HelpCache>,
    help_topics_open: bool,
    help_capture: bool,
    quick_open: Option<QuickOpen>,
    requested_file: Option<std::path::PathBuf>,
    #[cfg(unix)]
    terminal: Option<crate::terminal::EmbeddedTerminal>,
}

impl Workspace {
    pub fn new(path: &Path, grid: Option<&Path>) -> Result<Self, String> {
        let layout = std::fs::read_to_string(Self::layout_path())
            .ok()
            .and_then(|text| serde_json::from_str::<Layout>(&text).ok())
            .unwrap_or_default()
            .sanitize();
        let mut session = Session::new(path, grid)?;
        if session.project.is_none() {
            session.application = layout.application;
        }
        let absolute = std::path::absolute(path).map_err(|error| error.to_string())?;
        let directory = session
            .project
            .as_ref()
            .map(|project| project.root.clone())
            .unwrap_or_else(|| absolute.parent().unwrap().to_owned());
        let mut workspace = Self {
            session,
            layout,
            directory: directory.clone(),
            width: 1100,
            height: 760,
            resizing: false,
            sidebar_resizing: false,
            dock_drag: None,
            dock_moving: false,
            workspace_tab: false,
            browse_dir: directory.clone(),
            entries: Vec::new(),
            sidebar_scroll: 0,
            help_topic: 0,
            help_scroll: 0.0,
            help_query: String::new(),
            help_cache: None,
            help_topics_open: false,
            help_capture: false,
            quick_open: None,
            requested_file: None,
            #[cfg(unix)]
            terminal: None,
        };
        workspace.set_workspace(&directory)?;
        Ok(workspace)
    }
    fn layout_path() -> std::path::PathBuf {
        crate::profiles::default_base_dir().join("ide.json")
    }
    pub fn window_size(&self) -> (i32, i32) {
        self.layout.window_size
    }
    pub fn editor_height(&self, height: i32) -> i32 {
        self.metrics().editor_height(&self.layout, height)
    }
    pub fn editor_offset(&self) -> f32 {
        self.metrics().editor_offset(&self.layout, self.height)
    }
    fn metrics(&self) -> Metrics {
        Metrics::new(self.width, self.height)
    }
    pub fn resize(&mut self, width: i32, height: i32) {
        self.width = width.max(1);
        self.height = height.max(1);
    }
    pub fn dragging(&self) -> bool {
        self.resizing || self.sidebar_resizing || self.dock_moving
    }
    pub fn captures_input(&self) -> bool {
        self.help_capture
    }
    pub fn take_scale_request(&mut self) -> bool {
        #[cfg(unix)]
        if let Some(terminal) = &mut self.terminal { return terminal.take_scale_request(); }
        false
    }
    pub fn take_file_request(&mut self) -> Option<std::path::PathBuf> {
        self.requested_file.take()
    }
    pub fn sidebar_width(&self, width: i32) -> f32 {
        let unit = self.metrics().unit;
        if self.layout.sidebar_visible {
            (width as f32 * self.layout.sidebar_fraction).clamp(
                140.0 * unit,
                (width as f32 - 260.0 * unit).max(140.0 * unit),
            )
        } else {
            0.0
        }
    }
    pub fn set_workspace(&mut self, directory: &Path) -> Result<(), String> {
        let directory = directory
            .canonicalize()
            .map_err(|error| error.to_string())?;
        if !directory.is_dir() {
            return Err("Workspace must be a directory".to_owned());
        }
        self.directory = directory.clone();
        if directory.join(crate::project::MANIFEST).is_file() {
            if let Err(error) = crate::settings::remember_project(&directory) { self.session.print(&error); }
        }
        self.browse_dir = directory.clone();
        self.layout.workspaces.retain(|entry| entry != &directory);
        self.layout.workspaces.insert(0, directory);
        self.layout.workspaces.truncate(30);
        #[cfg(unix)]
        {
            self.terminal = None;
        }
        self.refresh_files();
        Ok(())
    }
    fn refresh_files(&mut self) {
        match std::fs::read_dir(&self.browse_dir) {
            Ok(entries) => {
                self.entries = entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .take(5000)
                    .collect();
                self.entries.sort_by_key(|path| {
                    (
                        !path.is_dir(),
                        path.file_name().unwrap_or_default().to_ascii_lowercase(),
                    )
                });
                self.sidebar_scroll = 0;
            }
            Err(error) => {
                self.entries.clear();
                self.session.print(&error.to_string());
            }
        }
    }
    pub fn sidebar_input(
        &mut self,
        rl: &mut RaylibHandle,
        top: f32,
        bottom: f32,
    ) -> (bool, Option<std::path::PathBuf>) {
        let metrics = self.metrics();
        let grab = (4.0 * metrics.unit).max(4.0);
        let width = self.sidebar_width(rl.get_screen_width());
        if width == 0.0 {
            return (false, None);
        }
        let mouse = rl.get_mouse_position();
        let inside = mouse.x < width + grab && mouse.y >= top && mouse.y < bottom;
        let click = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        if click && inside && (mouse.x - width).abs() <= grab {
            self.sidebar_resizing = true;
        }
        if self.sidebar_resizing {
            self.layout.sidebar_fraction =
                (mouse.x / rl.get_screen_width() as f32).clamp(0.16, 0.45);
            if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
                self.sidebar_resizing = false;
            }
            return (true, None);
        }
        if !inside {
            return (false, None);
        }
        self.session.focused = false;
        let row_height = metrics.row_height;
        let count = if self.workspace_tab {
            self.layout.workspaces.len()
        } else {
            self.entries.len()
        };
        self.sidebar_scroll = (self.sidebar_scroll as isize
            - (rl.get_mouse_wheel_move() * 3.0) as isize)
            .clamp(0, count.saturating_sub(1) as isize) as usize;
        if click {
            if mouse.y < top + row_height {
                self.workspace_tab = mouse.x > width / 2.0;
                self.sidebar_scroll = 0;
            } else if mouse.y < top + row_height * 2.0 {
                if mouse.x < width / 2.0 {
                    if let Some(parent) = self.browse_dir.parent() {
                        self.browse_dir = parent.to_owned();
                    }
                }
                self.refresh_files();
            } else {
                let index = ((mouse.y - top - row_height * 3.0) / row_height).floor() as isize;
                if index >= 0 {
                    let index = index as usize + self.sidebar_scroll;
                    let path = if self.workspace_tab {
                        self.layout.workspaces.get(index)
                    } else {
                        self.entries.get(index)
                    }
                    .cloned();
                    if let Some(path) = path {
                        if self.workspace_tab {
                            if let Err(error) = self.set_workspace(&path) {
                                self.session.print(&error);
                            } else if path.join(crate::project::MANIFEST).is_file() {
                                match crate::project::Project::load(&path) {
                                    Ok(project) => return (true, Some(project.entry())),
                                    Err(error) => self.session.print(&error),
                                }
                            }
                        } else if path.is_dir() {
                            self.browse_dir = path;
                            self.refresh_files();
                        } else {
                            return (true, Some(path));
                        }
                    }
                }
            }
        }
        (click || rl.get_mouse_wheel_move() != 0.0, None)
    }
    pub fn draw_sidebar<D: RaylibDraw>(&self, d: &mut D, top: f32, bottom: f32, active: &Path) {
        let metrics = self.metrics();
        let width = self.sidebar_width(self.width);
        if width == 0.0 {
            return;
        }
        let rect = Rectangle::new(0.0, top, width, (bottom - top).max(1.0));
        d.draw_rectangle_rec(rect, Color::new(24, 25, 30, 255));
        let (x, y, clip_width, height) = crate::rendering::scissor_rect(rect);
        let mut clip = d.begin_scissor_mode(x, y, clip_width, height);
        let row_height = metrics.row_height;
        let font = metrics.font;
        let padding = metrics.padding;
        let text_top = (row_height - font) / 2.0;
        for (index, label) in ["Files", "Workspaces"].iter().enumerate() {
            if self.workspace_tab == (index == 1) {
                clip.draw_rectangle(
                    (index as f32 * width / 2.0) as i32,
                    top as i32,
                    (width / 2.0) as i32,
                    row_height as i32,
                    Color::new(50, 64, 75, 255),
                );
            }
            ui::draw_text(
                &mut clip,
                &ui::fit(label, font, width / 2.0 - padding * 2.0),
                index as f32 * width / 2.0 + padding,
                top + text_top,
                font,
                Color::RAYWHITE,
            );
        }
        ui::draw_text(
            &mut clip,
            &ui::fit("Up", font, width / 2.0 - padding * 2.0),
            padding,
            top + row_height + text_top,
            font,
            Color::LIGHTGRAY,
        );
        ui::draw_text(
            &mut clip,
            &ui::fit("Refresh", font, width / 2.0 - padding * 2.0),
            width / 2.0 + padding,
            top + row_height + text_top,
            font,
            Color::LIGHTGRAY,
        );
        let location = ui::fit(
            &self.browse_dir.to_string_lossy(),
            font,
            width - padding * 2.0,
        );
        ui::draw_text(
            &mut clip,
            &location,
            padding,
            top + row_height * 2.0 + text_top,
            font,
            Color::GRAY,
        );
        let entries = if self.workspace_tab {
            &self.layout.workspaces
        } else {
            &self.entries
        };
        for (row, path) in entries.iter().skip(self.sidebar_scroll).enumerate() {
            let y = top + row_height * (row as f32 + 3.0);
            if y >= bottom {
                break;
            }
            if path == active {
                clip.draw_rectangle(
                    0,
                    y as i32,
                    width as i32,
                    row_height as i32,
                    Color::new(44, 58, 68, 255),
                );
            }
            let name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy();
            let label = if self.workspace_tab {
                path.display().to_string()
            } else {
                format!("{} {name}", if path.is_dir() { "+" } else { " " })
            };
            ui::draw_text(
                &mut clip,
                &ui::fit(&label, font, width - padding * 2.0),
                padding,
                y + text_top,
                font,
                if path.is_dir() {
                    Color::SKYBLUE
                } else {
                    Color::RAYWHITE
                },
            );
        }
        clip.draw_rectangle(
            width as i32 - 2,
            top as i32,
            2,
            (bottom - top) as i32,
            Color::GRAY,
        );
    }
    pub fn save_layout(&mut self, width: i32, height: i32) -> Result<(), String> {
        self.layout.window_size = (width, height);
        self.layout.application = self.session.application;
        let path = Self::layout_path();
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|error| error.to_string())?;
        let text = serde_json::to_string_pretty(&self.layout).map_err(|error| error.to_string())?;
        std::fs::write(path.with_extension("json.tmp"), text).map_err(|error| error.to_string())?;
        std::fs::rename(path.with_extension("json.tmp"), path).map_err(|error| error.to_string())
    }
    fn buttons(&self) -> Vec<(&'static str, Rectangle)> {
        let metrics = self.metrics();
        let labels = [
            "Output",
            "Ruby",
            "Terminal",
            "Run",
            "Stop",
            if self.session.application {
                "App"
            } else {
                "Game"
            },
            "Clear",
            if self.layout.visible { "Hide" } else { "Show" },
            "Reset",
            "Files",
            "Help",
        ];
        labels
            .into_iter()
            .enumerate()
            .map(|(index, label)| {
                (
                    label,
                    metrics.button(index, self.width, metrics.dock_top(&self.layout, self.height)),
                )
            })
            .collect()
    }
    fn body(&self) -> Rectangle {
        self.metrics().body(&self.layout, self.width, self.height)
    }
    pub fn update(&mut self, rl: &mut RaylibHandle, now: f64) -> bool {
        self.help_capture = false;
        self.resize(rl.get_screen_width(), rl.get_screen_height());
        self.session.poll();
        let mouse = rl.get_mouse_position();
        let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        let shift = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_P) {
            match crate::project::source_files(&self.directory) {
                Ok(files) => self.quick_open = Some(QuickOpen {
                    query: String::new(),
                    files: files.into_iter().map(|path| {
                        let name = path.strip_prefix(&self.directory).unwrap_or(&path).to_string_lossy().to_lowercase();
                        (path, name)
                    }).collect(),
                    selected: 0,
                }),
                Err(error) => self.session.print(&error),
            }
        }
        if self.quick_open.is_some() {
            self.help_capture = true;
            if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) { self.quick_open = None; return false; }
            let bounds = QuickOpen::bounds(self.width, self.height);
            let row = self.metrics().bar_height.min(bounds.height / 3.0);
            let picker = self.quick_open.as_mut().unwrap();
            let previous = picker.query.clone();
            if ctrl && rl.is_key_pressed(KeyboardKey::KEY_A) { picker.query.clear(); }
            if ctrl && rl.is_key_pressed(KeyboardKey::KEY_V) {
                if let Ok(text) = rl.get_clipboard_text() { picker.query.push_str(text.trim()); }
            }
            while let Some(character) = rl.get_char_pressed() {
                if !ctrl && !character.is_control() { picker.query.push(character); }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) { picker.query.pop(); }
            if picker.query != previous { picker.selected = 0; }
            let count = picker.matches().len();
            if count > 0 {
                if rl.is_key_pressed(KeyboardKey::KEY_DOWN) { picker.selected = (picker.selected + 1) % count; }
                if rl.is_key_pressed(KeyboardKey::KEY_UP) { picker.selected = (picker.selected + count - 1) % count; }
            }
            let mut selected = rl.is_key_pressed(KeyboardKey::KEY_ENTER).then_some(picker.selected);
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                let mouse = rl.get_mouse_position();
                if bounds.check_collision_point_rec(mouse) && mouse.y >= bounds.y + row {
                    let visible = ((bounds.height - row) / row).floor().max(1.0) as usize;
                    let first = picker.selected.saturating_sub(visible - 1);
                    selected = Some(first + ((mouse.y - bounds.y - row) / row).floor() as usize);
                }
            }
            if let Some(path) = selected.and_then(|index| picker.matches().get(index).map(|path| (*path).clone())) {
                self.requested_file = Some(path);
                self.quick_open = None;
                self.session.focused = false;
            }
            return false;
        }
        let file_shortcut = ctrl
            && [
                KeyboardKey::KEY_N,
                KeyboardKey::KEY_O,
                KeyboardKey::KEY_S,
                KeyboardKey::KEY_Q,
            ]
            .iter()
            .any(|key| rl.is_key_pressed(*key));
        let mut run = rl.is_key_pressed(KeyboardKey::KEY_F5) && !shift;
        if rl.is_key_pressed(KeyboardKey::KEY_F5) && shift {
            self.session.stop();
        }
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_GRAVE) {
            self.layout.visible = !self.layout.visible;
            self.session.focused = self.layout.visible;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F6) {
            self.layout.tab = if self.layout.tab == Tab::Console {
                Tab::Terminal
            } else {
                Tab::Console
            };
            self.layout.visible = true;
            self.session.focused = true;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_F1) {
            self.layout.tab = Tab::Help;
            self.layout.visible = true;
            self.session.focused = true;
        }
        let edge = if self.layout.dock_top {
            self.metrics().menu_height() + self.editor_offset()
        } else { self.editor_height(self.height) as f32 };
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            self.session.focused = self.body().check_collision_point_rec(mouse)
                || self.buttons().iter().any(|(_, rect)| rect.check_collision_point_rec(mouse));
            if self.layout.visible && (mouse.y - edge).abs() <= 4.0 {
                self.resizing = true;
            } else if let Some(index) = self
                .buttons()
                .iter()
                .position(|(_, rect)| rect.check_collision_point_rec(mouse))
            {
                match index {
                    0..=2 => {
                        self.layout.tab = [Tab::Output, Tab::Console, Tab::Terminal][index];
                        self.layout.visible = true;
                        self.dock_drag = Some(mouse);
                    }
                    3 => run = true,
                    4 => self.session.stop(),
                    5 => self.session.application = !self.session.application,
                    6 => {
                        self.session.lines.clear();
                        self.session.scroll = 0;
                        #[cfg(unix)]
                        if self.layout.tab == Tab::Terminal {
                            if let Some(terminal) = &mut self.terminal { terminal.clear(); }
                        }
                    }
                    7 => {
                        self.layout.visible = !self.layout.visible;
                        self.session.focused = false;
                    }
                    8 => {
                        self.layout = Layout::default();
                        self.session.application = false;
                    }
                    9 => self.layout.sidebar_visible = !self.layout.sidebar_visible,
                    10 => {
                        self.layout.tab = Tab::Help;
                        self.layout.visible = true;
                        self.dock_drag = Some(mouse);
                    }
                    _ => {}
                }
            }
        }
        if let Some(origin) = self.dock_drag {
            if (mouse.x - origin.x).abs() + (mouse.y - origin.y).abs() > 8.0 * self.metrics().unit {
                self.dock_moving = true;
            }
            if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
                if self.dock_moving { self.layout.dock_top = mouse.y < self.height as f32 / 2.0; }
                self.dock_drag = None;
                self.dock_moving = false;
            }
        }
        if self.resizing {
            let extent = if self.layout.dock_top {
                mouse.y - self.metrics().menu_height()
            } else { self.height as f32 - mouse.y };
            self.layout.panel_fraction =
                ((extent - self.metrics().toolbar_height())
                    / self.height as f32)
                    .clamp(0.18, 0.7);
            if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
                self.resizing = false;
            }
        }
        if !self.layout.visible {
            self.session.focused = false;
            return run;
        }
        let body = self.body();
        if self.layout.tab == Tab::Terminal {
            #[cfg(unix)]
            {
                if self.terminal.is_none() {
                    match crate::terminal::EmbeddedTerminal::new(
                        &self.directory,
                        &self.session.grid_path,
                    ) {
                        Ok(terminal) => self.terminal = Some(terminal),
                        Err(error) => {
                            self.session.print(&error);
                            self.layout.tab = Tab::Output;
                        }
                    }
                }
                if let Some(terminal) = &mut self.terminal {
                    terminal.update(
                        rl,
                        body,
                        self.session.focused && !self.resizing && !self.dock_moving && !run && !file_shortcut,
                        now,
                    );
                }
            }
            #[cfg(not(unix))]
            {
                self.session
                    .print("Embedded terminals currently require Unix");
                self.layout.tab = Tab::Output;
            }
        } else if self.layout.tab == Tab::Help {
            let picker_was_open = self.help_topics_open;
            self.update_help(rl, body);
            self.help_capture = picker_was_open || self.help_topics_open;
            if self.help_capture {
                self.session.focused = true;
            }
        } else {
            if body.check_collision_point_rec(mouse) {
                self.session.scroll = (self.session.scroll as isize
                    + (rl.get_mouse_wheel_move() * 3.0) as isize)
                    .clamp(0, self.session.lines.len().saturating_sub(1) as isize)
                    as usize;
            }
            if self.session.focused && self.layout.tab == Tab::Console && !run && !file_shortcut {
                if ctrl && rl.is_key_pressed(KeyboardKey::KEY_V) {
                    if let Ok(text) = rl.get_clipboard_text() {
                        self.session
                            .input
                            .push_str(&text.replace(['\n', '\r'], " "));
                    }
                }
                while let Some(character) = rl.get_char_pressed() {
                    if !ctrl && !character.is_control() {
                        self.session.input.push(character);
                    }
                }
                if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
                    self.session.input.pop();
                }
                if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                    self.session.evaluate();
                }
            }
        }
        run
    }
    pub fn draw_picker<D: RaylibDraw>(&self, d: &mut D) {
        let Some(picker) = &self.quick_open else { return; };
        let bounds = QuickOpen::bounds(self.width, self.height);
        let metrics = self.metrics();
        let row = metrics.bar_height.min(bounds.height / 3.0);
        let font = metrics.font.min(row * 0.6);
        d.draw_rectangle(0, 0, self.width, self.height, Color::new(0, 0, 0, 170));
        d.draw_rectangle_rec(bounds, Color::new(28, 29, 35, 255));
        let header = Rectangle::new(bounds.x, bounds.y, bounds.width, row);
        let query = format!("Quick Open: {}", picker.query);
        ui::draw_text_in(d, &ui::fit(&query, font, bounds.width - 24.0), header, 12.0, font, Color::RAYWHITE);
        let matches = picker.matches();
        let visible = ((bounds.height - row) / row).floor().max(1.0) as usize;
        let first = picker.selected.saturating_sub(visible - 1);
        for (index, path) in matches.iter().enumerate().skip(first).take(visible) {
            let rect = Rectangle::new(bounds.x, bounds.y + (index - first + 1) as f32 * row, bounds.width, row);
            if index == picker.selected { d.draw_rectangle_rec(rect, Color::new(50, 64, 75, 255)); }
            let label = path.strip_prefix(&self.directory).unwrap_or(path).to_string_lossy();
            ui::draw_text_in(d, &ui::fit(&label, font, rect.width - 24.0), rect, 12.0, font, Color::RAYWHITE);
        }
        if matches.is_empty() {
            ui::draw_text(d, "No matching files", bounds.x + 12.0, bounds.y + row + 8.0, font, Color::GRAY);
        }
    }

    pub fn draw<D: RaylibDraw>(&self, d: &mut D, now: f64) {
        let metrics = self.metrics();
        let body = self.body();
        d.draw_rectangle(
            0,
            metrics.dock_top(&self.layout, self.height) as i32,
            self.width,
            metrics.toolbar_height() as i32,
            Color::new(18, 18, 24, 255),
        );
        for (index, (label, rect)) in self.buttons().iter().enumerate() {
            let selected = (index < 3
                && self.layout.tab == [Tab::Output, Tab::Console, Tab::Terminal][index])
                || (index == 10 && self.layout.tab == Tab::Help);
            if selected {
                d.draw_rectangle_rec(*rect, Color::new(50, 64, 75, 255));
            }
            let label = ui::fit(label, metrics.font, rect.width - metrics.padding);
            ui::draw_text(
                d,
                &label,
                rect.x + (rect.width - ui::measure(&label, metrics.font)) / 2.0,
                rect.y + (rect.height - metrics.font) / 2.0,
                metrics.font,
                Color::RAYWHITE,
            );
        }
        d.draw_rectangle(
            0,
            if self.layout.dock_top { (metrics.menu_height() + self.editor_offset()) as i32 - 2 } else { self.editor_height(self.height) },
            self.width,
            2,
            if self.resizing {
                Color::GOLD
            } else {
                Color::GRAY
            },
        );
        if !self.layout.visible {
            return;
        }
        #[cfg(unix)]
        if self.layout.tab == Tab::Terminal {
            if let Some(terminal) = &self.terminal {
                terminal.draw(d, body, now);
                return;
            }
        }
        if self.layout.tab == Tab::Help {
            self.draw_help(d, body);
            return;
        }
        d.draw_rectangle_rec(body, Color::new(20, 21, 26, 255));
        let (x, y, width, height) = crate::rendering::scissor_rect(body);
        let mut clip = d.begin_scissor_mode(x, y, width, height);
        let font = metrics.mono_font;
        let line_height = font * 1.4;
        let prompt_height = if self.layout.tab == Tab::Console {
            line_height + metrics.padding
        } else {
            0.0
        };
        let count =
            ((body.height - prompt_height - metrics.padding) / line_height).max(0.0) as usize;
        let end = self.session.lines.len().saturating_sub(self.session.scroll);
        let start = end.saturating_sub(count);
        for (row, line) in self.session.lines.range(start..end).enumerate() {
            draw_mono(
                &mut clip,
                line,
                metrics.padding,
                body.y + metrics.padding / 2.0 + row as f32 * line_height,
                font,
                Color::RAYWHITE,
            );
        }
        if self.layout.tab == Tab::Console {
            let prompt = format!("> {}", self.session.input);
            let mut first = 0;
            while measure_mono(&prompt[first..], font)
                > (body.width - metrics.padding * 2.0).max(1.0)
                && first < prompt.len()
            {
                first += prompt[first..].chars().next().unwrap().len_utf8();
            }
            draw_mono(
                &mut clip,
                &prompt[first..],
                metrics.padding,
                body.y + body.height - prompt_height,
                font,
                Color::new(140, 230, 170, 255),
            );
        }
    }

    fn update_help(&mut self, rl: &mut RaylibHandle, body: Rectangle) {
        let metrics = self.metrics();
        let controls = metrics.help(body, self.help_topic == 5);
        let mouse = rl.get_mouse_position();
        if self.help_topics_open && rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let rect = self.help_topics_rect(body);
            if rect.check_collision_point_rec(mouse) {
                self.help_topic =
                    (((mouse.y - rect.y) / metrics.row_height) as usize).min(HELP.len() - 1);
                self.help_scroll = 0.0;
            }
            self.help_topics_open = false;
        } else if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if controls.previous.check_collision_point_rec(mouse) {
                self.help_topic = (self.help_topic + HELP.len() - 1) % HELP.len();
                self.help_scroll = 0.0;
            } else if controls.next.check_collision_point_rec(mouse) {
                self.help_topic = (self.help_topic + 1) % HELP.len();
                self.help_scroll = 0.0;
            } else if controls.topic.check_collision_point_rec(mouse) {
                self.help_topics_open = true;
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.help_topics_open = false;
        }
        if self.session.focused && self.help_topic == 5 {
            while let Some(character) = rl.get_char_pressed() {
                if !character.is_control() {
                    self.help_query.push(character);
                    self.help_scroll = 0.0;
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
                self.help_query.pop();
                self.help_scroll = 0.0;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                self.help_query.clear();
            }
        }
        let size = metrics.help_font;
        let content_width = (body.width - metrics.padding * 3.0).floor().max(80.0);
        let stale = self.help_cache.as_ref().is_none_or(|cache| {
            cache.width != content_width as i32
                || cache.size != (size * 10.0) as i32
                || cache.topic != self.help_topic
                || cache.query != self.help_query
        });
        if stale {
            let text = if self.help_topic == 5 && !self.help_query.is_empty() {
                let query = self.help_query.to_ascii_lowercase();
                HELP[5]
                    .1
                    .split("\n## ")
                    .filter(|section| section.to_ascii_lowercase().contains(&query))
                    .map(|section| format!("## {section}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                HELP[self.help_topic].1.to_owned()
            };
            let metrics = crate::markdown::Metrics {
                text: &|text, size| ui::measure(text, size),
                mono: &|text, size| measure_mono(text, size),
            };
            let layout = crate::markdown::layout(
                &crate::markdown::parse(&text),
                content_width,
                size,
                &metrics,
            );
            self.help_cache = Some(HelpCache {
                width: content_width as i32,
                size: (size * 10.0) as i32,
                topic: self.help_topic,
                query: self.help_query.clone(),
                layout,
            });
        }
        if body.check_collision_point_rec(mouse) {
            self.help_scroll -= rl.get_mouse_wheel_move() * size * 3.0;
        }
        let max = self.help_cache.as_ref().map_or(0.0, |cache| {
            let content = metrics.help(body, self.help_topic == 5).content;
            (cache.layout.height - content.height + metrics.padding * 2.0).max(0.0)
        });
        self.help_scroll = self.help_scroll.clamp(0.0, max);
    }

    fn draw_help<D: RaylibDraw>(&self, d: &mut D, body: Rectangle) {
        use crate::markdown::{Item, RectKind};
        let metrics = self.metrics();
        let controls = metrics.help(body, self.help_topic == 5);
        d.draw_rectangle_rec(body, Color::new(25, 27, 31, 255));
        let (x, y, width, height) = crate::rendering::scissor_rect(body);
        let mut clip = d.begin_scissor_mode(x, y, width, height);
        let title = format!("{} v", HELP[self.help_topic].0);
        for (text, rect) in [
            ("<", controls.previous),
            (">", controls.next),
            (title.as_str(), controls.topic),
        ] {
            let text = ui::fit(
                text,
                metrics.font,
                (rect.width - metrics.padding * 2.0).max(1.0),
            );
            ui::draw_text(
                &mut clip,
                &text,
                rect.x + metrics.padding,
                rect.y + (rect.height - metrics.font) / 2.0,
                metrics.font,
                Color::SKYBLUE,
            );
        }
        if let Some(rect) = controls.search {
            let field = Rectangle::new(
                rect.x,
                rect.y + metrics.padding / 2.0,
                rect.width,
                rect.height - metrics.padding,
            );
            clip.draw_rectangle_rec(field, Color::new(34, 36, 44, 255));
            let text = ui::fit(
                &format!("Search: {}", self.help_query),
                metrics.font,
                (field.width - metrics.padding * 2.0).max(1.0),
            );
            ui::draw_text(
                &mut clip,
                &text,
                field.x + metrics.padding,
                rect.y + (rect.height - metrics.font) / 2.0,
                metrics.font,
                Color::RAYWHITE,
            );
        }
        let Some(cache) = &self.help_cache else {
            return;
        };
        let (x, y, width, height) = crate::rendering::scissor_rect(controls.content);
        let mut content = clip.begin_scissor_mode(x, y, width, height);
        for item in &cache.layout.items {
            match item {
                Item::Text { y, size, runs } => {
                    let top = controls.content.y + metrics.padding + y - self.help_scroll;
                    if top + size * 1.5 < controls.content.y || top > body.y + body.height {
                        continue;
                    }
                    for run in runs {
                        let color = if run.token == crate::syntax::Token::Plain {
                            Color::RAYWHITE
                        } else {
                            crate::editor::token_color(run.token)
                        };
                        if run.style.code {
                            draw_mono(
                                &mut content,
                                &run.text,
                                metrics.padding * 1.5 + run.x,
                                top,
                                size * 0.92,
                                color,
                            );
                        } else {
                            ui::draw_text(
                                &mut content,
                                &run.text,
                                metrics.padding * 1.5 + run.x,
                                top,
                                *size,
                                color,
                            );
                        }
                    }
                }
                Item::Rect {
                    x,
                    y,
                    width,
                    height,
                    kind,
                } => {
                    let top = controls.content.y + metrics.padding + y - self.help_scroll;
                    if top + height < controls.content.y || top > body.y + body.height {
                        continue;
                    }
                    let color = if *kind == RectKind::CodeBackground {
                        Color::new(17, 18, 22, 255)
                    } else {
                        Color::new(65, 70, 80, 255)
                    };
                    content.draw_rectangle(
                        (metrics.padding * 1.5 + x) as i32,
                        top as i32,
                        *width as i32,
                        *height as i32,
                        color,
                    );
                }
            }
        }
        drop(content);
        drop(clip);
        if self.help_topics_open {
            let rect = self.help_topics_rect(body);
            d.draw_rectangle_rec(rect, Color::new(40, 43, 50, 255));
            d.draw_rectangle_lines_ex(rect, 1.0, Color::GRAY);
            for (row, (title, _)) in HELP.iter().enumerate() {
                ui::draw_text(
                    d,
                    &ui::fit(title, metrics.font, rect.width - metrics.padding * 2.0),
                    rect.x + metrics.padding,
                    rect.y
                        + row as f32 * metrics.row_height
                        + (metrics.row_height - metrics.font) / 2.0,
                    metrics.font,
                    Color::RAYWHITE,
                );
            }
        }
    }

    fn help_topics_rect(&self, body: Rectangle) -> Rectangle {
        let metrics = self.metrics();
        let height = HELP.len() as f32 * metrics.row_height;
        let left = metrics.help(body, self.help_topic == 5).topic.x;
        Rectangle::new(
            left,
            (body.y + metrics.bar_height)
                .min(self.height as f32 - height - metrics.padding)
                .max(0.0),
            (body.width - left - metrics.padding).min(280.0 * metrics.unit),
            height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_dock_moves_without_covering_the_editor_or_menu() {
        for (width, height, scale) in [(420, 300, 0.9375), (1100, 760, 1.3194444), (2200, 1520, 2.6388888)] {
            let metrics = Metrics::at_scale(width, scale);
            for dock_top in [false, true] {
                for visible in [false, true] {
                    let layout = Layout { dock_top, visible, ..Layout::default() };
                    let offset = metrics.editor_offset(&layout, height);
                    let body = metrics.body(&layout, width, height);
                    let toolbar = metrics.dock_top(&layout, height);
                    assert!(toolbar >= metrics.menu_height());
                    assert!(body.y + body.height <= height as f32 + 1.0);
                    if dock_top {
                        assert_eq!(metrics.editor_height(&layout, height) as f32 + offset, height as f32);
                        if visible { assert!((body.y + body.height - metrics.menu_height() - offset).abs() < 1.0); }
                    } else { assert_eq!(offset, 0.0); }
                    let restored: Layout = serde_json::from_str(&serde_json::to_string(&layout).unwrap()).unwrap();
                    assert_eq!(restored.dock_top, dock_top);
                }
            }
        }
    }

    #[test]
    fn window_metrics_scale_and_wrap_without_overlapping_panes() {
        let layout = Layout::default();
        for (width, height, scale) in [
            (420, 300, 0.9375),
            (1100, 760, 1.25 * 760.0 / 720.0),
            (3840, 2160, 3.75),
        ] {
            let metrics = Metrics::at_scale(width, scale);
            let top = metrics.editor_height(&layout, height) as f32;
            assert!(top >= 90.0);
            assert!(top + metrics.toolbar_height() < height as f32);
            let buttons: Vec<_> = (0..DOCK_BUTTONS)
                .map(|index| metrics.button(index, width, top))
                .collect();
            for (index, button) in buttons.iter().enumerate() {
                assert!(button.x >= 0.0 && button.x + button.width <= width as f32 + 0.1);
                assert!(
                    button.y >= top
                        && button.y + button.height <= top + metrics.toolbar_height() + 0.1
                );
                for other in &buttons[index + 1..] {
                    assert!(!button.check_collision_recs(*other));
                }
            }
            if width == 420 {
                assert!(metrics.columns < DOCK_BUTTONS);
            }
        }
        let small = Metrics::at_scale(1100, 1.25);
        let large = Metrics::at_scale(3840, 3.75);
        assert!(large.font > small.font);
        assert!(large.row_height > small.row_height);
        assert!(large.bar_height > small.bar_height);
    }

    #[test]
    fn zoomed_window_layout_keeps_help_and_controls_inside_the_window() {
        ui::limit_to_toolbar();
        for zoom in [0.5, 1.0, 1.5, 3.0] {
            ui::set_user_scale(zoom);
            for (width, height) in [
                (420, 300),
                (420, 900),
                (1100, 760),
                (2200, 1520),
                (3840, 2160),
                (6144, 3456),
            ] {
                let metrics = Metrics::new(width, height);
                for fraction in [0.18, 0.34, 0.7] {
                    let layout = Layout {
                        panel_fraction: fraction,
                        ..Layout::default()
                    };
                    let top =
                        metrics.editor_height(&layout, height) as f32 + metrics.toolbar_height();
                    let body = Rectangle::new(0.0, top, width as f32, height as f32 - top);
                    let help = metrics.help(body, true);
                    assert!(body.height > 0.0);
                    assert!(
                        help.content.y < height as f32,
                        "Help outside {width}x{height} at zoom {zoom}"
                    );
                    for rect in [
                        help.previous,
                        help.next,
                        help.topic,
                        help.search.unwrap(),
                        help.content,
                    ] {
                        assert!(rect.x >= 0.0 && rect.x + rect.width <= width as f32 + 0.1);
                        assert!(rect.y >= top && rect.y + rect.height <= height as f32 + 0.1);
                    }
                    assert!(!help.topic.check_collision_recs(help.search.unwrap()));
                }
            }
        }
        ui::set_user_scale(1.0);
    }

    #[test]
    fn layout_defaults_roundtrip_and_bounds() {
        let layout: Layout = serde_json::from_str("{}").unwrap();
        let metrics = Metrics::at_scale(1100, 1.25 * 760.0 / 720.0);
        assert_eq!(metrics.editor_height(&layout, 760), 470);
        let fixed = Layout {
            panel_fraction: f32::NAN,
            window_size: (0, i32::MAX),
            ..layout
        }
        .sanitize();
        assert_eq!(fixed.panel_fraction, 0.34);
        assert_eq!(fixed.window_size, (420, 16384));
        let restored: Layout =
            serde_json::from_str(&serde_json::to_string(&fixed).unwrap()).unwrap();
        assert!(Metrics::at_scale(420, 0.9375).editor_height(&restored, 300) >= 90);
    }
}

pub(crate) struct Session {
    pub lines: VecDeque<String>,
    pub input: String,
    pub focused: bool,
    pub application: bool,
    pub scroll: usize,
    child: Option<Child>,
    receiver: Receiver<String>,
    sender: SyncSender<String>,
    engine: crate::scripting::ScriptEngine,
    grid_path: std::path::PathBuf,
    shared_grid: bool,
    project: Option<crate::project::Project>,
}

impl Session {
    pub fn new(path: &Path, grid: Option<&Path>) -> Result<Self, String> {
        let project = crate::project::Project::discover(path)?;
        let shared_grid = grid.is_some();
        let grid_path = grid
            .map(Path::to_path_buf)
            .or_else(|| project.as_ref().map(crate::project::Project::grid))
            .unwrap_or_else(|| path.with_extension("save.json"));
        let grid_path = std::path::absolute(grid_path).map_err(|error| error.to_string())?;
        let (engine, _, _) = crate::script_engine_for(crate::Target::Grid(grid_path.clone()))?;
        let (sender, receiver) = mpsc::sync_channel(256);
        Ok(Self {
            lines: VecDeque::new(),
            input: String::new(),
            focused: false,
            application: project
                .as_ref()
                .is_some_and(|project| project.manifest.kind == crate::project::Kind::App),
            scroll: 0,
            child: None,
            receiver,
            sender,
            engine,
            grid_path,
            shared_grid,
            project,
        })
    }

    pub fn print(&mut self, text: &str) {
        for line in text.lines() {
            self.lines.push_back(line.to_owned());
            if self.lines.len() > 2000 {
                self.lines.pop_front();
            }
        }
        self.scroll = 0;
    }

    pub fn evaluate(&mut self) {
        let source = std::mem::take(&mut self.input);
        self.print(&format!("> {source}"));
        let (output, value) = self.engine.eval_captured(&source);
        self.print(&output);
        self.print(&value);
        for request in self.engine.drain_requests() {
            self.print(&format!(
                "Organizer action unavailable in IDE console: {request:?}"
            ));
        }
    }

    pub fn run(&mut self, path: &Path) -> Result<(), String> {
        self.poll();
        if self.child.is_some() {
            return Err("A program is already running; stop it first".to_owned());
        }
        let path = self
            .project
            .as_ref()
            .map(crate::project::Project::entry)
            .unwrap_or_else(|| path.to_owned());
        let path = path.canonicalize().map_err(|error| error.to_string())?;
        let directory = self
            .project
            .as_ref()
            .map(|project| project.root.as_path())
            .unwrap_or_else(|| path.parent().unwrap_or(Path::new(".")));
        let mut command = Command::new(std::env::current_exe().map_err(|error| error.to_string())?);
        command
            .arg(if self.application {
                "--script"
            } else {
                "--game"
            })
            .arg(&path)
            .arg("--grid")
            .arg(&self.grid_path)
            .current_dir(directory)
            .env("SELENITE_IDE_CHILD", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        let mut child = command.spawn().map_err(|error| error.to_string())?;
        for stream in [
            Box::new(child.stdout.take().unwrap()) as Box<dyn std::io::Read + Send>,
            Box::new(child.stderr.take().unwrap()) as Box<dyn std::io::Read + Send>,
        ] {
            let sender = self.sender.clone();
            thread::spawn(move || {
                for line in BufReader::new(stream).lines() {
                    match line {
                        Ok(line) => {
                            if sender.send(line).is_err() {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
            });
        }
        self.child = Some(child);
        self.print(&format!("Running {}", path.display()));
        Ok(())
    }

    pub fn set_path(&mut self, path: &Path) -> Result<(), String> {
        let project = crate::project::Project::discover(path)?;
        if project.as_ref().map(|project| &project.root)
            != self.project.as_ref().map(|project| &project.root)
        {
            if let Some(project) = &project {
                self.application = project.manifest.kind == crate::project::Kind::App;
            }
        }
        self.project = project;
        if self.shared_grid {
            return Ok(());
        }
        let grid_path = std::path::absolute(
            self.project
                .as_ref()
                .map(crate::project::Project::grid)
                .unwrap_or_else(|| path.with_extension("save.json")),
        )
        .map_err(|error| error.to_string())?;
        if grid_path == self.grid_path {
            return Ok(());
        }
        let grid = crate::load_or_new_root(&grid_path)?;
        self.engine
            .set_context(crate::scripting::ScriptContext::new(
                std::sync::Arc::clone(&grid),
                grid,
                grid_path.clone(),
            ));
        self.grid_path = grid_path;
        Ok(())
    }

    pub fn poll(&mut self) {
        for _ in 0..256 {
            match self.receiver.try_recv() {
                Ok(line) => self.print(&line),
                Err(_) => break,
            }
        }
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.child = None;
                    self.print(&format!("Process exited: {status}"));
                    if let Err(error) = self.reload_grid() {
                        self.print(&error);
                    }
                }
                Ok(None) => {}
                Err(error) => self.print(&error.to_string()),
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            self.print("Process stopped");
            if let Err(error) = self.reload_grid() {
                self.print(&error);
            }
        }
    }

    fn reload_grid(&mut self) -> Result<(), String> {
        let grid = crate::load_or_new_root(&self.grid_path)?;
        self.engine
            .set_context(crate::scripting::ScriptContext::new(
                std::sync::Arc::clone(&grid),
                grid,
                self.grid_path.clone(),
            ));
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

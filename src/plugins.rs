//! Ruby plugins: discovery of `*.rb` files in the global and per-profile
//! plugin folders, the manifest types the Ruby side reports back, bundled
//! example plugins, and the Plugins panel UI.
//!
//! Loading and invoking happen in `scripting.rs` (only with the
//! `scripting` feature); this module is always compiled so the panel can
//! explain how to enable plugins in builds without Ruby.

use std::{
    fs,
    path::{Path, PathBuf},
};

use raylib::prelude::*;

use crate::ui;

/// Example plugins bundled into the binary; "Install examples" copies them
/// into the global plugins folder.
pub const EXAMPLES: &[(&str, &str)] = &[
    ("hello.rb", include_str!("../examples/plugins/hello.rb")),
    (
        "file_inspector.rb",
        include_str!("../examples/plugins/file_inspector.rb"),
    ),
    (
        "now_playing.rb",
        include_str!("../examples/plugins/now_playing.rb"),
    ),
    (
        "grid_tools.rb",
        include_str!("../examples/plugins/grid_tools.rb"),
    ),
    ("clock.rb", include_str!("../examples/plugins/clock.rb")),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Button,
    Menu,
    Command,
    Timer,
    Hook,
}

impl ItemKind {
    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "button" => Self::Button,
            "menu" => Self::Menu,
            "command" => Self::Command,
            "timer" => Self::Timer,
            "hook" => Self::Hook,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PluginItem {
    pub id: i64,
    pub kind: ItemKind,
    pub label: String,
    /// Shortcut for buttons, e.g. `F6`.
    pub key: Option<String>,
    /// Cell kinds a menu item applies to (`image`, `audio`, `grid`,
    /// `empty`, ...); empty = every cell.
    pub kinds: Vec<String>,
    /// Seconds between runs for timers.
    pub interval: f64,
}

impl PluginItem {
    /// Whether a context-menu item applies to a cell of `kind`
    /// (`None` = empty cell).
    pub fn applies_to(&self, kind: Option<&str>) -> bool {
        self.kinds.is_empty()
            || self.kinds.iter().any(|wanted| {
                wanted == kind.unwrap_or("empty")
                    || (wanted == "file" && kind.is_some_and(|kind| kind != "grid"))
            })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PluginInfo {
    pub file: PathBuf,
    pub name: String,
    pub description: String,
    pub version: String,
    pub enabled: bool,
    pub error: Option<String>,
    pub items: Vec<PluginItem>,
}

impl PluginInfo {
    /// Placeholder for a file that is disabled or produced no plugin.
    pub fn for_file(file: &Path, enabled: bool, error: Option<String>) -> Self {
        Self {
            file: file.to_path_buf(),
            name: file_name(file),
            description: String::new(),
            version: String::new(),
            enabled,
            error,
            items: Vec::new(),
        }
    }

    pub fn file_name(&self) -> String {
        file_name(&self.file)
    }
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("plugin.rb")
        .to_owned()
}

/// Plugin folders in load order: global, then the active profile's.
pub fn plugin_dirs(base: &Path, profile_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = vec![base.join("plugins")];
    if let Some(dir) = profile_dir {
        dirs.push(dir.join("plugins"));
    }
    dirs
}

/// `*.rb` files directly inside `dirs`, sorted by name within each folder.
pub fn discover(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        let mut found: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("rb"))
            })
            .collect();
        found.sort();
        files.extend(found);
    }
    files
}

/// Copies the bundled examples into `dir`, never overwriting a file the
/// user may have edited. Returns how many were written.
pub fn install_examples(dir: &Path) -> Result<usize, String> {
    fs::create_dir_all(dir)
        .map_err(|error| format!("could not create {}: {error}", dir.display()))?;
    let mut written = 0;
    for (name, source) in EXAMPLES {
        let path = dir.join(name);
        if path.exists() {
            continue;
        }
        fs::write(&path, source)
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        written += 1;
    }
    Ok(written)
}

/// Function keys plugins may bind (F1 = Help and F3 = 3D are reserved).
pub fn key_from_name(name: &str) -> Option<KeyboardKey> {
    use KeyboardKey::*;
    Some(match name.trim().to_ascii_uppercase().as_str() {
        "F2" => KEY_F2,
        "F4" => KEY_F4,
        "F5" => KEY_F5,
        "F6" => KEY_F6,
        "F7" => KEY_F7,
        "F8" => KEY_F8,
        "F9" => KEY_F9,
        "F10" => KEY_F10,
        "F11" => KEY_F11,
        "F12" => KEY_F12,
        _ => return None,
    })
}

// ---------------------------------------------------------------- UI ----

#[derive(Clone, Debug, PartialEq)]
pub enum PanelAction {
    Close,
    Reload,
    OpenFolder,
    InstallExamples,
    Toggle(String),
    Invoke(i64),
}

enum Element {
    Text {
        text: String,
        x: f32,
        y: f32,
        size: f32,
        color: Color,
    },
    Button {
        action: PanelAction,
        rect: Rectangle,
        label: String,
        accent: bool,
    },
    Rule {
        y: f32,
    },
}

pub struct PanelLayout {
    pub frame: Rectangle,
    pub content: Rectangle,
    header: Vec<Element>,
    body: Vec<Element>,
    pub content_height: f32,
    scroll: f32,
}

const TITLE: Color = Color::new(235, 235, 240, 255);
const DIM: Color = Color::new(150, 150, 170, 255);
const ERROR: Color = Color::new(240, 110, 110, 255);
const OK: Color = Color::new(120, 210, 150, 255);
const BG: Color = Color::new(24, 24, 32, 248);
const BORDER: Color = Color::new(100, 100, 115, 255);
const BUTTON: Color = Color::new(48, 48, 62, 255);
const BUTTON_HOVER: Color = Color::new(75, 75, 98, 255);
const ACCENT: Color = Color::new(80, 55, 120, 255);

pub fn panel_layout(
    plugins: &[PluginInfo],
    dirs: &[PathBuf],
    screen_width: i32,
    screen_height: i32,
    scroll: f32,
    available: bool,
) -> PanelLayout {
    let scale = ui::scale(screen_width, screen_height);
    let width = (screen_width as f32 * 0.86).min(980.0 * scale);
    let height = screen_height as f32 * 0.86;
    let frame = Rectangle::new(
        (screen_width as f32 - width) / 2.0,
        (screen_height as f32 - height) / 2.0,
        width,
        height,
    );
    let pad = 14.0 * scale;
    let title = 24.0 * scale;
    let text = 16.0 * scale;
    let small = 14.0 * scale;
    let button_height = text + 12.0 * scale;
    let inner = width - pad * 2.0;

    // Header: title + action buttons (never scrolls).
    let mut header = vec![Element::Text {
        text: "Plugins".to_owned(),
        x: frame.x + pad,
        y: frame.y + pad,
        size: title,
        color: TITLE,
    }];
    let mut x = frame.x + width - pad;
    for (action, label) in [
        (PanelAction::Close, "Close"),
        (PanelAction::OpenFolder, "Open folder"),
        (PanelAction::InstallExamples, "Install examples"),
        (PanelAction::Reload, "Reload"),
    ] {
        let button_width = ui::measure(label, text) + pad * 1.4;
        x -= button_width;
        header.push(Element::Button {
            action,
            rect: Rectangle::new(x, frame.y + pad, button_width, button_height),
            label: label.to_owned(),
            accent: false,
        });
        x -= pad * 0.6;
    }
    let mut header_bottom = frame.y + pad + button_height.max(title) + pad * 0.5;
    let folders = format!(
        "Folders: {}",
        dirs.iter()
            .map(|dir| dir.display().to_string())
            .collect::<Vec<_>>()
            .join("   ")
    );
    for line in ui::wrap(&folders, small, inner) {
        header.push(Element::Text {
            text: line,
            x: frame.x + pad,
            y: header_bottom,
            size: small,
            color: DIM,
        });
        header_bottom += small * 1.3;
    }
    header_bottom += pad * 0.4;
    let content = Rectangle::new(
        frame.x,
        header_bottom,
        width,
        frame.y + height - header_bottom - pad * 0.5,
    );

    // Body: one card per plugin, laid out from y = 0 and scrolled later.
    let mut body = Vec::new();
    let mut y = 0.0;
    let paragraph = |body: &mut Vec<Element>, y: &mut f32, text: &str, size: f32, color: Color| {
        for line in ui::wrap(text, size, inner) {
            body.push(Element::Text {
                text: line,
                x: frame.x + pad,
                y: *y,
                size,
                color,
            });
            *y += size * 1.35;
        }
    };
    if !available {
        paragraph(
            &mut body,
            &mut y,
            "Plugins are Ruby files and need the scripting build: cargo build --release --features scripting",
            text,
            ERROR,
        );
        y += pad;
    }
    if plugins.is_empty() {
        paragraph(
            &mut body,
            &mut y,
            "No plugins yet. Click \"Install examples\" to copy the bundled examples into the global plugins folder, or drop your own *.rb files there and click Reload. See docs/plugins.md.",
            text,
            DIM,
        );
    }
    for plugin in plugins {
        body.push(Element::Rule { y });
        y += pad * 0.6;
        let toggle = if plugin.enabled { "Disable" } else { "Enable" };
        let toggle_width = ui::measure(toggle, text) + pad * 1.4;
        body.push(Element::Button {
            action: PanelAction::Toggle(plugin.file_name()),
            rect: Rectangle::new(
                frame.x + width - pad - toggle_width,
                y,
                toggle_width,
                button_height,
            ),
            label: toggle.to_owned(),
            accent: !plugin.enabled,
        });
        let mut name = plugin.name.clone();
        if !plugin.version.is_empty() {
            name.push_str(&format!("  v{}", plugin.version));
        }
        let status = if !plugin.enabled {
            ("  (disabled)", DIM)
        } else if plugin.error.is_some() {
            ("  (error)", ERROR)
        } else {
            ("  (loaded)", OK)
        };
        let name = ui::fit(&name, text * 1.1, inner - toggle_width - pad * 8.0);
        let name_width = ui::measure(&name, text * 1.1);
        body.push(Element::Text {
            text: name,
            x: frame.x + pad,
            y: y + (button_height - text * 1.1) / 2.0,
            size: text * 1.1,
            color: TITLE,
        });
        body.push(Element::Text {
            text: status.0.to_owned(),
            x: frame.x + pad + name_width,
            y: y + (button_height - small) / 2.0,
            size: small,
            color: status.1,
        });
        y += button_height + 4.0 * scale;
        if !plugin.description.is_empty() {
            paragraph(&mut body, &mut y, &plugin.description, text, TITLE);
        }
        paragraph(
            &mut body,
            &mut y,
            &plugin.file.display().to_string(),
            small,
            DIM,
        );
        if let Some(error) = &plugin.error {
            paragraph(&mut body, &mut y, error, small, ERROR);
        }

        let mut summary = Vec::new();
        for (kind, label) in [
            (ItemKind::Menu, "menu"),
            (ItemKind::Command, "commands"),
            (ItemKind::Timer, "timers"),
            (ItemKind::Hook, "hooks"),
        ] {
            let names: Vec<String> = plugin
                .items
                .iter()
                .filter(|item| item.kind == kind)
                .map(|item| match kind {
                    ItemKind::Command => format!("Selenite.run({:?})", item.label),
                    ItemKind::Hook => format!(":{}", item.label),
                    _ => item.label.clone(),
                })
                .collect();
            if !names.is_empty() {
                summary.push(format!("{label}: {}", names.join(", ")));
            }
        }
        if !summary.is_empty() {
            paragraph(&mut body, &mut y, &summary.join("   |   "), small, DIM);
        }

        let mut x = frame.x + pad;
        let mut row_used = false;
        for item in plugin
            .items
            .iter()
            .filter(|item| item.kind == ItemKind::Button)
        {
            let label = match &item.key {
                Some(key) => format!("{}  [{key}]", item.label),
                None => item.label.clone(),
            };
            let button_width = (ui::measure(&label, text) + pad * 1.4).min(inner);
            if row_used && x + button_width > frame.x + width - pad {
                x = frame.x + pad;
                y += button_height + 6.0 * scale;
            }
            body.push(Element::Button {
                action: PanelAction::Invoke(item.id),
                rect: Rectangle::new(x, y + 2.0 * scale, button_width, button_height),
                label,
                accent: true,
            });
            x += button_width + pad * 0.6;
            row_used = true;
        }
        if row_used {
            y += button_height + 8.0 * scale;
        }
        y += pad * 0.6;
    }

    let content_height = y;
    let max_scroll = (content_height - content.height).max(0.0);
    let scroll = scroll.clamp(0.0, max_scroll);
    for element in &mut body {
        match element {
            Element::Text { y, .. } | Element::Rule { y } => *y += content.y - scroll,
            Element::Button { rect, .. } => rect.y += content.y - scroll,
        }
    }
    PanelLayout {
        frame,
        content,
        header,
        body,
        content_height,
        scroll,
    }
}

impl PanelLayout {
    pub fn max_scroll(&self) -> f32 {
        (self.content_height - self.content.height).max(0.0)
    }

    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    fn visible(&self, top: f32, bottom: f32) -> bool {
        top >= self.content.y - 0.5 && bottom <= self.content.y + self.content.height + 0.5
    }

    fn buttons(&self) -> impl Iterator<Item = (&PanelAction, Rectangle, &str, bool)> {
        self.header
            .iter()
            .map(|element| (element, true))
            .chain(self.body.iter().map(|element| (element, false)))
            .filter_map(move |(element, fixed)| match element {
                Element::Button {
                    action,
                    rect,
                    label,
                    accent,
                } if fixed || self.visible(rect.y, rect.y + rect.height) => {
                    Some((action, *rect, label.as_str(), *accent))
                }
                _ => None,
            })
    }

    pub fn action_at(&self, mouse: Vector2) -> Option<PanelAction> {
        self.buttons()
            .find(|(_, rect, _, _)| point_in(mouse, *rect))
            .map(|(action, ..)| action.clone())
    }

    pub fn draw<D: RaylibDraw>(
        &self,
        d: &mut D,
        screen_width: i32,
        screen_height: i32,
        mouse: Vector2,
    ) {
        d.draw_rectangle(0, 0, screen_width, screen_height, Color::new(0, 0, 0, 150));
        d.draw_rectangle_rec(self.frame, BG);
        d.draw_rectangle_lines_ex(self.frame, 1.0, BORDER);
        for element in self.header.iter().chain(&self.body) {
            match element {
                Element::Text {
                    text,
                    x,
                    y,
                    size,
                    color,
                } => {
                    let fixed = self
                        .header
                        .iter()
                        .any(|header| std::ptr::eq(header, element));
                    if fixed || self.visible(*y, *y + *size) {
                        ui::draw_text(d, text, *x, *y, *size, *color);
                    }
                }
                Element::Rule { y } => {
                    if self.visible(*y, *y) {
                        d.draw_line_ex(
                            Vector2::new(self.frame.x + 10.0, *y),
                            Vector2::new(self.frame.x + self.frame.width - 10.0, *y),
                            1.0,
                            BORDER,
                        );
                    }
                }
                Element::Button { .. } => {}
            }
        }
        for (_, rect, label, accent) in self.buttons() {
            let bg = if point_in(mouse, rect) {
                BUTTON_HOVER
            } else if accent {
                ACCENT
            } else {
                BUTTON
            };
            d.draw_rectangle_rec(rect, bg);
            let size = (rect.height - 12.0).max(8.0) * 0.95;
            ui::draw_text_centered(
                d,
                &ui::fit(label, size, rect.width - 8.0),
                rect,
                size,
                TITLE,
            );
        }
        if self.max_scroll() > 0.0 {
            let track = self.content;
            let fraction = track.height / self.content_height;
            let thumb_height = (track.height * fraction).max(20.0);
            let thumb_y =
                track.y + (track.height - thumb_height) * (self.scroll / self.max_scroll());
            d.draw_rectangle_rec(
                Rectangle::new(track.x + track.width - 6.0, thumb_y, 4.0, thumb_height),
                BORDER,
            );
        }
    }
}

fn point_in(point: Vector2, rect: Rectangle) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<PluginInfo> {
        let mut plugin = PluginInfo::for_file(Path::new("/p/hello.rb"), true, None);
        plugin.name = "Hello".to_owned();
        plugin.items = vec![
            PluginItem {
                id: 1,
                kind: ItemKind::Button,
                label: "Say hi".to_owned(),
                key: Some("F6".to_owned()),
                kinds: Vec::new(),
                interval: 0.0,
            },
            PluginItem {
                id: 2,
                kind: ItemKind::Menu,
                label: "Inspect".to_owned(),
                key: None,
                kinds: vec!["image".to_owned(), "file".to_owned()],
                interval: 0.0,
            },
        ];
        let broken =
            PluginInfo::for_file(Path::new("/p/broken.rb"), true, Some("SyntaxError".into()));
        vec![plugin, broken]
    }

    #[test]
    fn discovers_and_installs_examples() {
        let dir = std::env::temp_dir().join(format!("selenite_plugins_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert!(discover(std::slice::from_ref(&dir)).is_empty());
        assert_eq!(install_examples(&dir).unwrap(), EXAMPLES.len());
        assert_eq!(install_examples(&dir).unwrap(), 0, "never overwrites");
        fs::write(dir.join("notes.txt"), "x").unwrap();
        let found = discover(std::slice::from_ref(&dir));
        assert_eq!(found.len(), EXAMPLES.len());
        assert!(found.windows(2).all(|pair| pair[0] < pair[1]));
        let _ = fs::remove_dir_all(&dir);

        let dirs = plugin_dirs(Path::new("/base"), Some(Path::new("/base/profiles/a")));
        assert_eq!(dirs[0], Path::new("/base/plugins"));
        assert_eq!(dirs[1], Path::new("/base/profiles/a/plugins"));
    }

    #[test]
    fn menu_kinds_and_keys() {
        let plugins = sample();
        let menu = &plugins[0].items[1];
        assert!(menu.applies_to(Some("image")));
        assert!(
            menu.applies_to(Some("audio")),
            "`file` matches any file kind"
        );
        assert!(!menu.applies_to(Some("grid")));
        assert!(!menu.applies_to(None));
        assert!(plugins[0].items[0].applies_to(None));
        assert_eq!(key_from_name("f6"), Some(KeyboardKey::KEY_F6));
        assert_eq!(key_from_name("F1"), None, "F1 is Help");
        assert_eq!(ItemKind::parse("timer"), Some(ItemKind::Timer));
        assert_eq!(plugins[1].file_name(), "broken.rb");
    }

    #[test]
    fn panel_buttons_are_clickable_and_scroll_clamps() {
        let plugins = sample();
        let dirs = vec![PathBuf::from("/base/plugins")];
        let layout = panel_layout(&plugins, &dirs, 1280, 800, 1e9, true);
        assert!(layout.scroll() <= layout.max_scroll());
        let layout = panel_layout(&plugins, &dirs, 1280, 800, 0.0, true);
        let invoke = layout
            .buttons()
            .find(|(action, ..)| **action == PanelAction::Invoke(1))
            .map(|(_, rect, label, _)| (rect, label.to_owned()))
            .expect("button item is shown");
        assert_eq!(invoke.1, "Say hi  [F6]");
        let center = Vector2::new(invoke.0.x + 2.0, invoke.0.y + 2.0);
        assert_eq!(layout.action_at(center), Some(PanelAction::Invoke(1)));
        assert!(layout
            .buttons()
            .any(|(action, ..)| *action == PanelAction::Toggle("broken.rb".into())));
        assert!(layout
            .buttons()
            .any(|(action, ..)| *action == PanelAction::Reload));
    }
}

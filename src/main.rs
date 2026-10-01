mod camera;
mod download;
#[cfg(feature = "scripting")]
mod game;
mod grid_inventory;
mod help;
mod history;
mod imaging;
mod inventory;
mod item_panel;
mod music;
mod panels;
mod persistence;
mod plugins;
mod profiles;
#[cfg(feature = "scripting")]
mod scripting;
mod search;
mod selection;
mod settings;
mod ui;
mod view3d;

use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::{SystemTime, UNIX_EPOCH},
};

use camera::Camera;
use download::Download;
use imaging::{ThumbnailCache, TiledImage};
use panels::{InputKind, PanelAction, ProfilePanel};
use persistence::{CellKind, SavedGrid};
use plugins::{ItemKind, PluginInfo};
use profiles::{Profile, ProfileStore};
use raylib::prelude::*;
use settings::Settings;
use view3d::Orbit;

const WINDOW_WIDTH: i32 = 960;
const WINDOW_HEIGHT: i32 = 720;
const DEFAULT_CELL_SIZE: f64 = 72.0;
const ZOOM_STEP: f32 = 0.12;
const MAX_VISIBLE_CELLS_PER_AXIS: i64 = 256;
const VIEWER_MARGIN: f32 = 24.0;

const BACKGROUND: Color = Color::new(30, 30, 36, 255);
const GRID_LINE: Color = Color::new(70, 70, 82, 255);
const SELECTION_COLOR: Color = Color::new(250, 210, 60, 255);
const NESTED_GRID_COLOR: Color = Color::new(120, 200, 160, 255);
const AXIS_COLOR: Color = Color::new(115, 115, 140, 255);
const TOOLBAR_BG: Color = Color::new(20, 20, 26, 255);
const BUTTON_BG: Color = Color::new(60, 90, 160, 255);
const BUTTON_HOVER_BG: Color = Color::new(80, 115, 200, 255);
const BUTTON_TEXT: Color = Color::new(240, 240, 245, 255);
const WHITE: Color = Color::new(255, 255, 255, 255);
const VIEWER_BACKGROUND: Color = Color::new(18, 18, 22, 255);
const VIEWER_TEXT: Color = Color::new(220, 220, 225, 255);
const CONSOLE_BORDER: Color = Color::new(90, 90, 110, 255);
const CONSOLE_TEXT: Color = Color::new(210, 210, 220, 255);
const CONSOLE_PROMPT: Color = Color::new(140, 230, 170, 255);
const BUTTON_DISABLED_BG: Color = Color::new(45, 45, 54, 255);
const BUTTON_DISABLED_TEXT: Color = Color::new(120, 120, 130, 255);
const DANGER_BG: Color = Color::new(170, 55, 55, 255);
const DANGER_HOVER_BG: Color = Color::new(205, 70, 70, 255);
const DIALOG_BG: Color = Color::new(28, 28, 34, 245);
const DIALOG_BORDER: Color = Color::new(100, 100, 115, 255);
const SEARCH_COLOR: Color = Color::new(80, 220, 235, 255);
const NOW_PLAYING_COLOR: Color = Color::new(180, 120, 250, 255);
const STATUS_FADE_SECONDS: f64 = 8.0;

#[derive(Clone)]
struct NavFrame {
    grid: Arc<Mutex<SavedGrid>>,
    camera: Camera,
    orbit: Orbit,
    selected: Option<(i64, i64)>,
}

#[derive(Clone)]
enum Mode {
    Main,
    Inventory,
    ImageViewer(PathBuf),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ToolbarAction {
    Profile,
    Save,
    Undo,
    Redo,
    NewGrid,
    Delete,
    Paste,
    Copy,
    OpenRun,
    Find,
    PaExport,
    PaImport,
    OpenInventory,
    Items,
    GridFolder,
    Console,
    Home,
    View3d,
    Labels,
    CopyFiles,
    Music,
    Plugins,
    Help,
    CancelDownloads,
    Refresh,
    Back,
}

impl ToolbarAction {
    fn tooltip(self) -> &'static str {
        match self {
            Self::Profile => "Local profiles: each one is a separate account with its own grids, assets and exports. Click to switch, create, rename or delete.",
            Self::Save => "Save this profile's grids now (Ctrl+S). Edits are also saved automatically.",
            Self::Undo => "Undo the last edit (Ctrl+Z): paste, drop, move, delete, new grid, PA import, or a script's changes.",
            Self::Redo => "Redo the last undone edit (Ctrl+Y or Ctrl+Shift+Z).",
            Self::Find => "Find cells by file name, path or type across every nested grid (Ctrl+F). Enter/↓ next, Shift+Enter/↑ previous.",
            Self::NewGrid => "Create a nested grid in the selected empty cell; double-click it to go inside.",
            Self::Delete => "Delete the selected cell(s) and everything stacked on them (Del). Shift+Del removes only the top item. Non-empty nested grids ask first.",
            Self::Paste => "Paste into the selected cell (Ctrl+V): an image of any size, copied files, file paths, or URLs (one per line). URLs download in the background with no size limit. Ctrl+Shift+V stacks everything onto the one cell.",
            Self::Copy => "Copy the selected cells' files to the clipboard (Ctrl+C); Ctrl+X cuts.",
            Self::OpenRun => "Open the selected cell's top item (Enter): images in the viewer, .rb scripts in embedded Ruby, music in the built-in player, video and other files in the system player.",
            Self::PaExport => help::PA_EXPORT_TIP,
            Self::PaImport => help::PA_IMPORT_TIP,
            Self::OpenInventory => "Open a separate window listing every file across all nested grids.",
            Self::Items => "This grid's game inventory (B): named items with counts and metadata, shared with Ruby as Selenite.grid.inventory.",
            Self::GridFolder => "Open this grid's own folder in your file manager (O). Every grid gets a folder when first used; files added to the grid are copied into its assets/ subfolder, along with pasted images and downloads.",
            Self::Console => "Toggle the embedded Ruby console (` key). Type Selenite.help for the API.",
            Self::Home => "Reset pan and zoom (and the 3D camera) back to the origin.",
            Self::View3d => help::VIEW_3D_TIP,
            Self::Labels => help::LABELS_TIP,
            Self::CopyFiles => help::COPY_FILES_TIP,
            Self::Music => help::MUSIC_TIP,
            Self::Plugins => help::PLUGINS_TIP,
            Self::Help => "Show all buttons, shortcuts and the Ruby API (F1).",
            Self::CancelDownloads => "Cancel all running downloads (Esc).",
            Self::Refresh => "Reload from disk.",
            Self::Back => "Leave this nested grid (Backspace).",
        }
    }
}

struct ToolbarButton {
    label: String,
    rect: Rectangle,
    action: ToolbarAction,
    enabled: bool,
}

/// Scaled, wrapped toolbar layout shared by hit-testing and rendering.
struct ToolbarLayout {
    buttons: Vec<ToolbarButton>,
    height: f32,
    font_size: f32,
    pad_x: f32,
    depth_label: Option<(String, Vector2)>,
}

/// Lays out labelled items left-to-right, wrapping onto new rows when the
/// window is too narrow. Returns item rectangles and the total bar height.
fn wrap_toolbar_items(
    widths: &[f32],
    screen_width: f32,
    item_height: f32,
    margin: f32,
    spacing: f32,
) -> (Vec<Rectangle>, f32) {
    if widths.is_empty() {
        return (Vec::new(), 0.0);
    }
    let mut rects = Vec::with_capacity(widths.len());
    let (mut x, mut y) = (margin, margin);
    for &width in widths {
        if x > margin && x + width > screen_width - margin {
            x = margin;
            y += item_height + spacing;
        }
        rects.push(Rectangle::new(x, y, width, item_height));
        x += width + spacing;
    }
    (rects, y + item_height + margin)
}

/// A background download and where its file goes when it finishes.
struct PendingDownload {
    download: Download,
    grid: Arc<Mutex<SavedGrid>>,
    root: Arc<Mutex<SavedGrid>>,
    json_path: PathBuf,
    col: i64,
    row: i64,
    /// Replace a (non-grid) occupant of the target cell instead of moving
    /// to the next free cell.
    replace: bool,
    /// Push the file on top of the target cell's stack instead.
    stack: bool,
    /// Ruby hook fired when the file lands: "download", "drop" or "paste".
    hook: &'static str,
}

struct DragState {
    from: (i64, i64),
    start: Vector2,
    active: bool,
    /// Dragging the whole multi-selection rather than one cell.
    group: bool,
}

/// A rubber-band selection rectangle started on an empty cell.
struct BandState {
    from: (i64, i64),
    start: Vector2,
    active: bool,
}

/// A mouse button held down over the 3D view: becomes a rotate/pan drag
/// once it moves, otherwise a click on release.
#[derive(Clone, Copy)]
struct OrbitPress {
    button: MouseButton,
    start: Vector2,
    moved: bool,
}

struct SearchState {
    query: String,
    hits: Vec<search::Hit>,
    index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MenuItem {
    Open,
    QueueAudio,
    Info,
    CopyFile,
    CopyPath,
    ShowInFolder,
    Paste,
    NewGrid,
    Delete,
    /// Bring the next stacked item to the top.
    CycleStack,
    /// Remove only the top item of a stack.
    RemoveTop,
    /// Spread a stack out into neighbouring free cells.
    Unstack,
    /// Move every other selected cell's items onto this cell.
    StackSelectionHere,
    /// Open this grid's game inventory panel.
    Items,
    /// Open the current grid's folder in the file manager.
    GridFolder,
    /// Open a nested grid cell's own folder.
    NestedGridFolder,
    /// Run a `.rb` cell as a game in its own window (`selenite --game`).
    RunGame,
    /// Export a `.rb` cell as a self-contained game folder.
    ExportGame,
    /// A plugin's `p.menu` entry, by action id.
    Plugin(i64),
}

impl MenuItem {
    fn label(self) -> &'static str {
        match self {
            Self::Open => "Open / Run",
            Self::QueueAudio => "Add to playlist",
            Self::Info => "Info",
            Self::CopyFile => "Copy file",
            Self::CopyPath => "Copy path",
            Self::ShowInFolder => "Show in folder",
            Self::Paste => "Paste here",
            Self::NewGrid => "New grid here",
            Self::Delete => "Delete",
            Self::CycleStack => "Next in stack (Tab)",
            Self::RemoveTop => "Remove top item (Shift+Del)",
            Self::Unstack => "Unstack into free cells",
            Self::StackSelectionHere => "Stack selection here",
            Self::Items => "Grid inventory… (B)",
            Self::GridFolder => "Open grid folder (O)",
            Self::NestedGridFolder => "Open its grid folder",
            Self::RunGame => "Run as game",
            Self::ExportGame => "Export standalone game",
            Self::Plugin(_) => "Plugin",
        }
    }

    /// Menu for a cell whose top item has `kind`, holding `stack_len`
    /// items; `multi` is set when other cells are selected too.
    fn for_cell(kind: Option<CellKind>, stack_len: usize, multi: bool) -> Vec<Self> {
        let mut items = match kind {
            None => vec![Self::Paste, Self::NewGrid],
            Some(CellKind::Grid) => vec![Self::Open, Self::NestedGridFolder, Self::Info],
            Some(kind) => {
                let mut items = vec![Self::Open];
                if kind == CellKind::Audio {
                    items.push(Self::QueueAudio);
                }
                if kind == CellKind::RubyScript {
                    items.extend([Self::RunGame, Self::ExportGame]);
                }
                items.extend([
                    Self::Info,
                    Self::CopyFile,
                    Self::CopyPath,
                    Self::ShowInFolder,
                    Self::Paste,
                ]);
                items
            }
        };
        if stack_len > 1 {
            items.extend([Self::CycleStack, Self::RemoveTop, Self::Unstack]);
        }
        if multi {
            items.push(Self::StackSelectionHere);
        }
        if kind.is_some() {
            items.push(Self::Delete);
        }
        items.push(Self::GridFolder);
        items.push(Self::Items);
        items
    }
}

struct ContextMenu {
    cell: (i64, i64),
    position: Vector2,
    items: Vec<MenuItem>,
    /// Labels of the plugin items, by action id.
    plugin_labels: Vec<(i64, String)>,
}

impl ContextMenu {
    fn item_label(&self, item: MenuItem) -> String {
        match item {
            MenuItem::Plugin(id) => self
                .plugin_labels
                .iter()
                .find(|(item_id, _)| *item_id == id)
                .map(|(_, label)| format!("» {label}"))
                .unwrap_or_else(|| "Plugin".to_owned()),
            item => item.label().to_owned(),
        }
    }

    fn layout(&self, screen_width: i32, screen_height: i32) -> (Rectangle, Vec<Rectangle>, f32) {
        let scale = ui::scale(screen_width, screen_height);
        let size = 17.0 * scale;
        let pad = 10.0 * scale;
        let item_height = size + pad;
        let width = self
            .items
            .iter()
            .map(|item| ui::measure(&self.item_label(*item), size))
            .fold(ui::measure("(-0000, -0000)", size), f32::max)
            .min(screen_width as f32 * 0.6)
            + pad * 2.0;
        let height = item_height * (self.items.len() + 1) as f32 + pad * 0.5;
        let x = self
            .position
            .x
            .min(screen_width as f32 - width - 4.0)
            .max(0.0);
        let y = self
            .position
            .y
            .min(screen_height as f32 - height - 4.0)
            .max(0.0);
        let frame = Rectangle::new(x, y, width, height);
        let rows = (0..self.items.len())
            .map(|index| {
                Rectangle::new(x, y + item_height * (index + 1) as f32, width, item_height)
            })
            .collect();
        (frame, rows, size)
    }
}

/// Info-panel lines for a cell, keyed by the cell and the file it held.
type InfoCache = ((i64, i64), Option<PathBuf>, Vec<String>);

struct App {
    thumbnails: ThumbnailCache,
    root_grid: Arc<Mutex<SavedGrid>>,
    grid: Arc<Mutex<SavedGrid>>,
    camera: Camera,
    cell_size: f64,
    selected: Option<(i64, i64)>,
    /// Every selected cell; `selected` is the primary (focused) one.
    selection: selection::Selection,
    band: Option<BandState>,
    nav_stack: Vec<NavFrame>,
    json_path: PathBuf,
    mode: Mode,
    panning: bool,
    /// Cells awaiting confirmation before deletion (set only when one of
    /// them holds a non-empty nested grid, since that destroys everything
    /// inside it); empty means no confirmation dialog is showing.
    pending_delete: Vec<(i64, i64)>,
    item_panel: Option<item_panel::ItemPanel>,
    console_open: bool,
    console_input: String,
    console_history: Vec<String>,
    /// Previously submitted console lines, for Up/Down recall.
    console_inputs: Vec<String>,
    console_recall: Option<usize>,
    console_scroll: usize,
    status: Option<String>,
    profiles: ProfileStore,
    profile: Option<String>,
    profile_panel: Option<ProfilePanel>,
    help_open: bool,
    help_scroll: f32,
    hover: Option<(ToolbarAction, f64)>,
    downloads: Vec<PendingDownload>,
    drag: Option<DragState>,
    view_3d: bool,
    orbit: Orbit,
    orbit_press: Option<OrbitPress>,
    history: history::History,
    search: Option<SearchState>,
    context_menu: Option<ContextMenu>,
    info_open: bool,
    info_cache: Option<InfoCache>,
    minimap_open: bool,
    minimap_drag: bool,
    status_seen: (Option<String>, f64),
    title_dirty: bool,
    clipboard: Option<arboard::Clipboard>,
    settings: Settings,
    settings_path: PathBuf,
    player: music::Player,
    player_drag: Option<PlayerDrag>,
    plugins: Vec<PluginInfo>,
    /// Scroll offset of the plugin manager; `None` while it is closed.
    plugin_panel: Option<f32>,
    plugin_timers: Vec<PluginTimer>,
    /// Games started with "Run as game" that are still running.
    games: Vec<GameRun>,
    #[cfg(feature = "scripting")]
    script_engine: Option<scripting::ScriptEngine>,
}

/// A `selenite --game` child process and the save file it may change.
struct GameRun {
    child: std::process::Child,
    name: String,
    json_path: PathBuf,
    modified: Option<std::time::SystemTime>,
}

/// What the mouse is dragging on the music player bar.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerDrag {
    /// Seek preview, as a 0..=1 fraction of the track; applied on release.
    Seek(f32),
    Volume,
}

/// A plugin `every(seconds)` action and when it next fires.
#[cfg_attr(not(feature = "scripting"), allow(dead_code))]
struct PluginTimer {
    id: i64,
    interval: f64,
    due: std::time::Instant,
}

impl App {
    fn new(mode: Mode, json_path: PathBuf) -> Result<Self, String> {
        let root_grid = match mode {
            Mode::Main => load_or_new_root(&json_path)?,
            Mode::Inventory => load_existing_root(&json_path)?,
            Mode::ImageViewer(_) => unreachable!(),
        };
        let profiles = ProfileStore::open_default();
        let settings_path = Settings::path_in(profiles.base());
        let settings = Settings::load(&settings_path);
        let player = music::Player::new(settings.volume, settings.shuffle, settings.repeat);
        Ok(Self {
            thumbnails: ThumbnailCache::new(),
            grid: Arc::clone(&root_grid),
            root_grid,
            camera: Camera::new(),
            cell_size: DEFAULT_CELL_SIZE,
            selected: None,
            selection: selection::Selection::default(),
            band: None,
            nav_stack: Vec::new(),
            json_path,
            mode,
            panning: false,
            pending_delete: Vec::new(),
            item_panel: None,
            console_open: false,
            console_input: String::new(),
            console_history: Vec::new(),
            console_inputs: Vec::new(),
            console_recall: None,
            console_scroll: 0,
            status: None,
            profiles,
            profile: None,
            profile_panel: None,
            help_open: false,
            help_scroll: 0.0,
            hover: None,
            downloads: Vec::new(),
            drag: None,
            view_3d: false,
            orbit: Orbit::default(),
            orbit_press: None,
            history: history::History::default(),
            search: None,
            context_menu: None,
            info_open: false,
            info_cache: None,
            minimap_open: settings.minimap,
            minimap_drag: false,
            status_seen: (None, 0.0),
            title_dirty: true,
            clipboard: None,
            settings,
            settings_path,
            player,
            player_drag: None,
            plugins: Vec::new(),
            plugin_panel: None,
            plugin_timers: Vec::new(),
            games: Vec::new(),
            #[cfg(feature = "scripting")]
            script_engine: None,
        })
    }

    fn with_profile(store: ProfileStore, profile: Profile) -> Result<Self, String> {
        let mut app = Self::new(Mode::Main, profile.grid_path())?;
        app.settings_path = Settings::path_in(store.base());
        app.settings = Settings::load(&app.settings_path);
        app.minimap_open = app.settings.minimap;
        app.player = music::Player::new(
            app.settings.volume,
            app.settings.shuffle,
            app.settings.repeat,
        );
        app.profiles = store;
        app.profile = Some(profile.name);
        Ok(app)
    }

    fn labels(&self) -> bool {
        self.settings.labels
    }

    fn set_labels(&mut self, on: bool) {
        self.settings.labels = on;
        self.status = Some(format!("Grid labels {} (L)", if on { "on" } else { "off" }));
    }

    fn set_copy_files(&mut self, on: bool) {
        self.settings.copy_files = on;
        self.status = Some(if on {
            "Copy In on: dropped and pasted files are copied into the grid's assets folder"
                .to_string()
        } else {
            "Copy In off: dropped and pasted files are linked in place".to_string()
        });
    }

    /// Copies live state into the settings and writes them when changed.
    fn persist_settings(&mut self) {
        if self.player_drag.is_some() {
            return;
        }
        let mut next = self.settings.clone();
        next.minimap = self.minimap_open;
        next.volume = self.player.volume();
        next.shuffle = self.player.playlist.shuffle();
        next.repeat = self.player.playlist.repeat;
        let path_on_disk = self.settings_path.is_file();
        if next != self.settings || !path_on_disk && next != Settings::default() {
            self.settings = next;
            if let Err(error) = self.settings.save(&self.settings_path) {
                self.status = Some(format!("Error saving settings: {error}"));
            }
        }
    }

    fn window_title(&self) -> String {
        let base = match self.mode {
            Mode::Inventory => "Selenite Inventory",
            _ => help::VERSION_LABEL,
        };
        match &self.profile {
            Some(name) => format!("{base} — {name}"),
            None => format!("{base} — {}", self.json_path.display()),
        }
    }

    fn run(mut self) -> Result<(), String> {
        let (mut rl, thread) = raylib::init()
            .size(WINDOW_WIDTH, WINDOW_HEIGHT)
            .title("Selenite")
            .resizable()
            .build();
        rl.set_target_fps(60);
        // Esc closes overlays / cancels downloads instead of quitting.
        rl.set_exit_key(None);
        ui::load_font(&thread);
        self.run_profile_init();

        while !rl.window_should_close() {
            if let Err(error) = self.update(&mut rl) {
                self.status = Some(format!("Error: {error}"));
            }
            self.poll_downloads();
            self.poll_games();
            self.process_script_requests();
            self.update_player();
            self.fire_plugin_timers();
            self.persist_settings();
            self.orbit.animate(rl.get_frame_time());
            self.request_visible_thumbnails(&rl);
            self.thumbnails.pump(&mut rl, &thread);
            if self.title_dirty {
                rl.set_window_title(&thread, &self.window_title());
                self.title_dirty = false;
            }
            self.draw(&mut rl, &thread);
            let now = rl.get_time();
            if self.status != self.status_seen.0 {
                self.status_seen = (self.status.clone(), now);
            } else if self.status.is_some() && now - self.status_seen.1 > STATUS_FADE_SECONDS {
                self.status = None;
            }
        }
        for pending in &self.downloads {
            pending.download.cancel();
        }
        let saved = if matches!(self.mode, Mode::Main) {
            self.save()
        } else {
            Ok(())
        };
        self.player_drag = None;
        self.persist_settings();
        // Shut Ruby down and free GPU textures while the window (and the GL
        // context/driver) is still alive; doing it afterwards segfaults.
        #[cfg(feature = "scripting")]
        drop(self.script_engine.take());
        self.thumbnails.clear();
        self.player.shutdown();
        drop(rl);
        saved
    }

    fn update(&mut self, rl: &mut RaylibHandle) -> Result<(), String> {
        self.sync_selection();
        if let Some(scale) = ui::handle_scale_keys(rl) {
            self.status = Some(format!(
                "UI scale {:.0}% (Ctrl +/- to adjust, Ctrl 0 to reset)",
                scale * 100.0
            ));
        }
        self.update_hover(rl);

        if self.help_open {
            self.handle_help_input(rl);
            return Ok(());
        }
        if self.plugin_panel.is_some() {
            return self.handle_plugin_panel(rl);
        }
        if self.profile_panel.is_some() {
            return self.handle_profile_panel(rl);
        }
        if self.item_panel.is_some() {
            return self.handle_item_panel(rl);
        }

        let just_toggled = rl.is_key_pressed(KeyboardKey::KEY_GRAVE);
        if just_toggled {
            self.toggle_console();
        }

        if self.console_open {
            if just_toggled {
                // Swallow the backtick character itself so it doesn't land
                // in the input buffer on the same frame the console opened.
                while rl.get_char_pressed().is_some() {}
            } else {
                self.handle_console_input(rl);
            }
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                let mouse = rl.get_mouse_position();
                if mouse.y < self.toolbar_layout(rl).height {
                    self.handle_left_click(rl)?;
                }
            }
            return Ok(());
        }

        if !self.pending_delete.is_empty() {
            return self.handle_delete_confirmation(rl);
        }
        if self.context_menu.is_some() {
            return self.handle_context_menu(rl);
        }

        let typing = self.search.is_some();
        if typing {
            self.handle_search_input(rl);
        } else {
            if rl.is_key_pressed(KeyboardKey::KEY_F1) {
                self.help_open = true;
                return Ok(());
            }
            if rl.is_key_pressed(KeyboardKey::KEY_F3) {
                self.toggle_view_3d();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_M) && !ctrl_down(rl) {
                self.minimap_open = !self.minimap_open;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_I) && !ctrl_down(rl) {
                self.info_open = !self.info_open;
            }
            if rl.is_key_pressed(KeyboardKey::KEY_B) && !ctrl_down(rl) {
                self.open_item_panel();
                return Ok(());
            }
            if rl.is_key_pressed(KeyboardKey::KEY_O) && !ctrl_down(rl) {
                let grid = Arc::clone(&self.grid);
                return self.open_grid_folder(&grid);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_Z) && !ctrl_down(rl) && !self.view_3d {
                self.zoom_to_fit(rl);
            }
            if !ctrl_down(rl) {
                if let Err(error) = self.handle_player_keys(rl) {
                    self.status = Some(format!("Error: {error}"));
                }
                self.handle_plugin_keys(rl);
            }
        }

        if matches!(self.mode, Mode::Main) {
            self.handle_drop(rl)?;
            if !typing {
                self.handle_shortcuts(rl)?;
            }
        }
        if ctrl_down(rl) && rl.is_key_pressed(KeyboardKey::KEY_F) {
            self.toggle_search();
        }
        if self.handle_player_mouse(rl) {
            return Ok(());
        }
        if self.minimap_click(rl) {
            return Ok(());
        }
        if self.view_3d {
            self.handle_3d_input(rl, typing)?;
        } else {
            self.handle_pan(rl);
            self.handle_zoom(rl);

            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                self.handle_left_click(rl)?;
            }
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
                let mouse = rl.get_mouse_position();
                if mouse.y >= self.toolbar_layout(rl).height {
                    if let Ok([col, row]) = self.camera.cell_at(
                        [mouse.x as f64, mouse.y as f64],
                        self.viewport(rl),
                        self.cell_size,
                    ) {
                        self.open_context_menu((col, row), mouse);
                    }
                }
            }
            self.handle_drag(rl)?;
            self.handle_band(rl);
        }
        if typing {
            return Ok(());
        }

        if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) && !self.nav_stack.is_empty() {
            self.exit_grid();
        }

        if matches!(self.mode, Mode::Main)
            && rl.is_key_pressed(KeyboardKey::KEY_DELETE)
            && self.selected.is_some()
        {
            if shift_down(rl) {
                self.edit("Remove top item", |app| app.remove_top_item())?;
            } else {
                self.edit("Delete", |app| app.delete_selection())?;
            }
        }

        Ok(())
    }

    /// Keeps the multi-selection consistent with `selected`, which the rest
    /// of the app (and Ruby) may set directly: a primary cell outside the
    /// selection collapses it to that cell, and no primary clears it.
    fn sync_selection(&mut self) {
        match self.selected {
            Some(cell) if !self.selection.contains(cell) => self.selection.set_single(Some(cell)),
            None if !self.selection.is_empty() => self.selection.clear(),
            _ => {}
        }
    }

    fn set_single_selection(&mut self, cell: Option<(i64, i64)>) {
        self.selected = cell;
        self.selection.set_single(cell);
    }

    /// Every selected cell, primary included, in row-major order.
    fn selected_cells(&self) -> Vec<(i64, i64)> {
        if self.selection.is_multi() {
            self.selection.cells().collect()
        } else {
            self.selected.into_iter().collect()
        }
    }

    /// Frames the multi-selection, or every occupied cell when at most one
    /// cell is selected (Z).
    fn zoom_to_fit(&mut self, rl: &RaylibHandle) {
        let cells = if self.selection.is_multi() {
            self.selected_cells()
        } else {
            lock(&self.grid)
                .entries()
                .map(|(col, row, _)| (col, row))
                .collect()
        };
        let Some(((min_col, min_row), (max_col, max_row))) = selection::bounds(cells) else {
            self.status = Some("Nothing to zoom to: the grid is empty".into());
            return;
        };
        let toolbar = f64::from(self.toolbar_layout(rl).height);
        let viewport = self.viewport(rl);
        let margin = 40.0 + toolbar;
        match self.camera.fit_cells(
            [min_col, min_row],
            [max_col, max_row],
            viewport,
            self.cell_size,
            margin,
        ) {
            Ok(()) => {
                self.status = Some(format!(
                    "Zoomed to fit {}×{} cells",
                    max_col - min_col + 1,
                    max_row - min_row + 1
                ))
            }
            Err(error) => self.status = Some(format!("Error: {error}")),
        }
    }

    fn update_hover(&mut self, rl: &RaylibHandle) {
        let mouse = rl.get_mouse_position();
        let hovered = self
            .toolbar_layout(rl)
            .buttons
            .iter()
            .find(|button| point_in_rect(mouse, button.rect))
            .map(|button| button.action);
        self.hover = match (hovered, self.hover) {
            (Some(action), Some((current, since))) if action == current => Some((current, since)),
            (Some(action), _) => Some((action, rl.get_time())),
            (None, _) => None,
        };
    }

    fn handle_shortcuts(&mut self, rl: &RaylibHandle) -> Result<(), String> {
        let ctrl = ctrl_down(rl);
        if ctrl {
            let shift = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
            if rl.is_key_pressed(KeyboardKey::KEY_Z) {
                return if shift { self.redo() } else { self.undo() };
            }
            if rl.is_key_pressed(KeyboardKey::KEY_Y) {
                return self.redo();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_V) {
                if shift {
                    return self.edit("Paste as stack", |app| app.paste_clipboard_with(true));
                }
                return self.edit("Paste", |app| app.paste_clipboard());
            }
            if rl.is_key_pressed(KeyboardKey::KEY_C) {
                return self.copy_selection();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_X) {
                return self.cut_selection();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_A) {
                self.select_all_cells();
                return Ok(());
            }
            if rl.is_key_pressed(KeyboardKey::KEY_S) {
                self.save()?;
                self.status = Some(format!("Saved {}", self.json_path.display()));
            }
            return Ok(());
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            if !self.downloads.is_empty() {
                self.cancel_downloads();
            } else {
                self.selected = None;
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ENTER) || rl.is_key_pressed(KeyboardKey::KEY_KP_ENTER)
        {
            self.open_or_run_selection()?;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_HOME) {
            self.reset_view();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_TAB) {
            let back = shift_down(rl);
            self.edit("Cycle stack", |app| app.cycle_selected_stack(back))?;
        }
        let steps = [
            (KeyboardKey::KEY_LEFT, (-1, 0)),
            (KeyboardKey::KEY_RIGHT, (1, 0)),
            (KeyboardKey::KEY_UP, (0, -1)),
            (KeyboardKey::KEY_DOWN, (0, 1)),
        ];
        for (key, (dx, dy)) in steps {
            if rl.is_key_pressed(key) || rl.is_key_pressed_repeat(key) {
                let (col, row) = self.selected.unwrap_or((0, 0));
                let next = if self.selected.is_some() {
                    (col + dx, row + dy)
                } else {
                    (col, row)
                };
                self.set_single_selection(Some(next));
                self.ensure_cell_visible(rl, next);
            }
        }
        Ok(())
    }

    fn ensure_cell_visible(&mut self, rl: &RaylibHandle, (col, row): (i64, i64)) {
        if self.view_3d {
            self.orbit.focus(col, row);
            return;
        }
        let viewport = self.viewport(rl);
        let toolbar = self.toolbar_layout(rl).height as f64;
        if let Ok(rect) = self.camera.cell_rect([col, row], viewport, self.cell_size) {
            let outside = rect[0] < 0.0
                || rect[1] < toolbar
                || rect[0] + rect[2] > viewport[0]
                || rect[1] + rect[3] > viewport[1];
            if outside {
                self.camera.center_on_cell(col, row, self.cell_size);
            }
        }
    }

    fn handle_drag(&mut self, rl: &RaylibHandle) -> Result<(), String> {
        let Some(drag) = &mut self.drag else {
            return Ok(());
        };
        let mouse = rl.get_mouse_position();
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            let threshold = 8.0 * ui::scale(rl.get_screen_width(), rl.get_screen_height());
            let (dx, dy) = (mouse.x - drag.start.x, mouse.y - drag.start.y);
            if !drag.active && dx * dx + dy * dy > threshold * threshold {
                drag.active = true;
            }
            return Ok(());
        }
        let drag = self.drag.take().expect("checked above");
        if !drag.active {
            // A press on an already multi-selected cell that never became
            // a drag is a plain click: collapse to that cell.
            if drag.group {
                self.set_single_selection(Some(drag.from));
            }
            return Ok(());
        }
        if mouse.y < self.toolbar_layout(rl).height {
            return Ok(());
        }
        let [col, row] = self
            .camera
            .cell_at(
                [mouse.x as f64, mouse.y as f64],
                self.viewport(rl),
                self.cell_size,
            )
            .map_err(|error| error.to_string())?;
        if (col, row) == drag.from {
            return Ok(());
        }
        if drag.group {
            return self.move_selection((col - drag.from.0, row - drag.from.1));
        }
        let before = lock(&self.grid).snapshot();
        if shift_down(rl) {
            let moved = lock(&self.grid).merge_cells(drag.from, (col, row));
            self.history.record(&self.grid, before, "Stack");
            self.status = Some(format!(
                "Stacked {moved} item(s) from ({}, {}) onto ({col}, {row}) — Tab cycles the stack",
                drag.from.0, drag.from.1
            ));
        } else {
            lock(&self.grid).swap_cells(drag.from, (col, row));
            self.history.record(&self.grid, before, "Move");
            self.status = Some(format!(
                "Moved ({}, {}) to ({col}, {row}) (hold Shift while dropping to stack instead)",
                drag.from.0, drag.from.1
            ));
        }
        self.set_single_selection(Some((col, row)));
        self.save()
    }

    /// Moves every selected, occupied cell by `offset` as one undoable step,
    /// refusing if a destination holds something that isn't moving too.
    fn move_selection(&mut self, offset: (i64, i64)) -> Result<(), String> {
        let cells = self.selected_cells();
        let plan = {
            let grid = lock(&self.grid);
            selection::plan_move(cells, offset, |(col, row)| grid.occupied(col, row))
        };
        let moves = match plan {
            Ok(moves) if !moves.is_empty() => moves,
            Ok(_) => return Ok(()),
            Err((col, row)) => {
                self.status = Some(format!(
                    "Can't move the selection: ({col}, {row}) is occupied. Drop somewhere free, or move that cell first."
                ));
                return Ok(());
            }
        };
        let before = lock(&self.grid).snapshot();
        let moved = lock(&self.grid).move_many(&moves);
        self.history.record(&self.grid, before, "Move selection");
        self.selection.translate(offset);
        self.selected = self
            .selected
            .map(|(col, row)| (col + offset.0, row + offset.1));
        self.status = Some(format!(
            "Moved {moved} cell(s) by ({}, {})",
            offset.0, offset.1
        ));
        self.save()
    }

    /// Destination cells for a group drag hovering `target`, plus whether
    /// the drop would be blocked.
    fn group_drop_preview(&self, target: Option<(i64, i64)>) -> Option<(Vec<(i64, i64)>, bool)> {
        let drag = self
            .drag
            .as_ref()
            .filter(|drag| drag.active && drag.group)?;
        let target = target?;
        let offset = (target.0 - drag.from.0, target.1 - drag.from.1);
        let grid = lock(&self.grid);
        let cells: Vec<(i64, i64)> = self
            .selection
            .cells()
            .filter(|&(col, row)| grid.occupied(col, row))
            .collect();
        let blocked =
            selection::plan_move(cells.iter().copied(), offset, |(c, r)| grid.occupied(c, r))
                .is_err();
        Some((
            cells
                .iter()
                .map(|(col, row)| (col + offset.0, row + offset.1))
                .collect(),
            blocked,
        ))
    }

    fn group_drop_blocked(&self, target: Option<(i64, i64)>) -> bool {
        self.group_drop_preview(target)
            .is_some_and(|(_, blocked)| blocked)
    }

    /// Rubber-band selection: dragging from an empty cell selects the
    /// rectangle of cells it spans.
    fn handle_band(&mut self, rl: &RaylibHandle) {
        let Some(band) = &mut self.band else {
            return;
        };
        let mouse = rl.get_mouse_position();
        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            let threshold = 8.0 * ui::scale(rl.get_screen_width(), rl.get_screen_height());
            let (dx, dy) = (mouse.x - band.start.x, mouse.y - band.start.y);
            if !band.active && dx * dx + dy * dy > threshold * threshold {
                band.active = true;
            }
            return;
        }
        let band = self.band.take().expect("checked above");
        if !band.active {
            return;
        }
        let Ok([col, row]) = self.camera.cell_at(
            [mouse.x as f64, mouse.y as f64],
            self.viewport(rl),
            self.cell_size,
        ) else {
            return;
        };
        match self.selection.set_rect(band.from, (col, row), false) {
            Ok(()) => {
                self.selected = Some((col, row));
                let occupied = {
                    let grid = lock(&self.grid);
                    self.selection
                        .cells()
                        .filter(|(c, r)| grid.occupied(*c, *r))
                        .count()
                };
                self.status = Some(format!(
                    "{} cell(s) selected, {occupied} occupied — drag one to move them all, Del deletes, Ctrl+C copies",
                    self.selection.len()
                ));
            }
            Err(error) => self.status = Some(format!("Selection: {error}")),
        }
    }

    /// Ctrl+A: every occupied cell of the current grid.
    fn select_all_cells(&mut self) {
        let cells: Vec<(i64, i64)> = lock(&self.grid)
            .entries()
            .map(|(col, row, _)| (col, row))
            .collect();
        if cells.is_empty() {
            self.status = Some("Nothing to select in this grid".into());
            return;
        }
        self.selection.select_all(cells.iter().copied());
        self.selected = self.selection.anchor();
        self.status = Some(format!("Selected all {} occupied cell(s)", cells.len()));
    }

    fn cycle_selected_stack(&mut self, back: bool) -> Result<(), String> {
        let Some((col, row)) = self.selected else {
            return Ok(());
        };
        let (moved, len) = {
            let mut grid = lock(&self.grid);
            let moved = grid.cycle_stack(col, row, if back { -1 } else { 1 });
            (moved, grid.stack_len(col, row))
        };
        if moved {
            self.info_cache = None;
            let top = lock(&self.grid)
                .items_at(col, row)
                .first()
                .map(content_label)
                .unwrap_or_default();
            self.status = Some(format!(
                "Stack at ({col}, {row}): now showing {top} (1 of {len}; Tab / Shift+Tab to cycle)"
            ));
            self.save()?;
        } else if len < 2 {
            self.status = Some(
                "Only one item here. Shift+drag a cell onto another, Ctrl+Shift+V or drop with Shift to stack.".into(),
            );
        }
        Ok(())
    }

    fn remove_top_item(&mut self) -> Result<(), String> {
        let Some((col, row)) = self.selected else {
            return Ok(());
        };
        let removed = {
            let mut grid = lock(&self.grid);
            if grid.stack_len(col, row) < 2 {
                None
            } else {
                grid.pop_item(col, row, 0)
            }
        };
        match removed {
            Some(item) => {
                self.info_cache = None;
                self.status = Some(format!(
                    "Removed {} from the stack at ({col}, {row})",
                    content_label(&item)
                ));
                self.save()
            }
            // A single item: same as a normal delete (with its confirmation).
            None => self.delete_selection(),
        }
    }

    fn unstack_selected(&mut self, (col, row): (i64, i64)) -> Result<(), String> {
        let placed = lock(&self.grid).unstack(col, row);
        if placed.is_empty() {
            self.status = Some("Nothing to unstack".into());
            return Ok(());
        }
        self.selection
            .select_all(std::iter::once((col, row)).chain(placed.iter().copied()));
        self.selected = Some((col, row));
        self.info_cache = None;
        self.status = Some(format!(
            "Unstacked {} item(s) into the free cells to the right",
            placed.len()
        ));
        self.save()
    }

    /// Moves every other selected cell's items onto `target`.
    fn stack_selection_onto(&mut self, target: (i64, i64)) -> Result<(), String> {
        let sources: Vec<(i64, i64)> = self
            .selected_cells()
            .into_iter()
            .filter(|cell| *cell != target)
            .collect();
        let mut moved = 0;
        {
            let mut grid = lock(&self.grid);
            for source in sources.iter().rev() {
                moved += grid.merge_cells(*source, target);
            }
        }
        self.set_single_selection(Some(target));
        self.info_cache = None;
        self.status = Some(format!(
            "Stacked {moved} item(s) onto ({}, {}) — Tab cycles through them",
            target.0, target.1
        ));
        self.save()
    }

    fn toggle_console(&mut self) {
        self.console_open = !self.console_open;
        if self.console_open {
            self.console_input.clear();
            self.ensure_script_engine();
        }
    }

    #[cfg(feature = "scripting")]
    fn ensure_script_engine(&mut self) {
        if self.script_engine.is_none() {
            match scripting::ScriptEngine::new() {
                Ok(engine) => {
                    self.console_print(
                        "Ruby scripting console ready. Try: Selenite.help   (Up/Down history, Ctrl+V paste, PgUp/PgDn scroll, Ctrl+L clear)",
                    );
                    self.script_engine = Some(engine);
                }
                Err(error) => {
                    self.console_print(&format!("error starting Ruby: {error}"));
                }
            }
        }
    }

    #[cfg(not(feature = "scripting"))]
    fn ensure_script_engine(&mut self) {
        if self.console_history.is_empty() {
            self.console_history.push(
                "Scripting not enabled -- rebuild with `cargo build --release --features scripting`."
                    .to_owned(),
            );
        }
    }

    fn handle_console_input(&mut self, rl: &mut RaylibHandle) {
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.console_open = false;
            return;
        }
        let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);

        while let Some(c) = rl.get_char_pressed() {
            if !c.is_control() && !ctrl {
                self.console_input.push(c);
            }
        }
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_V) {
            let pasted = self
                .clipboard()
                .ok()
                .and_then(|clipboard| clipboard.get_text().ok())
                .unwrap_or_default();
            // Multi-line pastes become one line of Ruby joined by `;`.
            let joined = pasted
                .lines()
                .map(str::trim_end)
                .filter(|line| !line.trim().is_empty())
                .collect::<Vec<_>>()
                .join("; ");
            self.console_input.push_str(&joined);
        }
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_L) {
            self.console_history.clear();
            self.console_scroll = 0;
        }
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_U) {
            self.console_input.clear();
        }

        if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE)
            || rl.is_key_pressed_repeat(KeyboardKey::KEY_BACKSPACE)
        {
            self.console_input.pop();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_UP) && !self.console_inputs.is_empty() {
            let index = match self.console_recall {
                Some(index) => index.saturating_sub(1),
                None => self.console_inputs.len() - 1,
            };
            self.console_recall = Some(index);
            self.console_input = self.console_inputs[index].clone();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            if let Some(index) = self.console_recall {
                if index + 1 < self.console_inputs.len() {
                    self.console_recall = Some(index + 1);
                    self.console_input = self.console_inputs[index + 1].clone();
                } else {
                    self.console_recall = None;
                    self.console_input.clear();
                }
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_PAGE_UP) {
            self.console_scroll = (self.console_scroll + 5).min(self.console_history.len());
        }
        if rl.is_key_pressed(KeyboardKey::KEY_PAGE_DOWN) {
            self.console_scroll = self.console_scroll.saturating_sub(5);
        }
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            let lines = (wheel.abs().ceil() as usize) * 2;
            self.console_scroll = if wheel > 0.0 {
                (self.console_scroll + lines).min(self.console_history.len())
            } else {
                self.console_scroll.saturating_sub(lines)
            };
        }

        if rl.is_key_pressed(KeyboardKey::KEY_ENTER) || rl.is_key_pressed(KeyboardKey::KEY_KP_ENTER)
        {
            self.submit_console_line();
        }
    }

    fn console_print(&mut self, text: &str) {
        for line in text.trim_end_matches('\n').split('\n') {
            self.console_history.push(line.to_owned());
        }
        const MAX_HISTORY: usize = 500;
        if self.console_history.len() > MAX_HISTORY {
            let overflow = self.console_history.len() - MAX_HISTORY;
            self.console_history.drain(0..overflow);
        }
        self.console_scroll = 0;
    }

    fn submit_console_line(&mut self) {
        let line = std::mem::take(&mut self.console_input);
        self.console_recall = None;
        if line.trim().is_empty() {
            return;
        }
        if self.console_inputs.last() != Some(&line) {
            self.console_inputs.push(line.clone());
        }
        self.console_print(&format!("> {line}"));

        #[cfg(feature = "scripting")]
        {
            self.ensure_script_engine();
            if let Some(engine) = &self.script_engine {
                engine.set_context(self.script_context());
                let grid = Arc::clone(&self.grid);
                let before = lock(&grid).snapshot();
                let (printed, result) = engine.eval_captured(&line);
                if self.history.record(&grid, before, "Ruby console") {
                    let _ = self.save();
                }
                if !printed.is_empty() {
                    self.console_print(&printed);
                }
                self.console_print(&format!("=> {result}"));
            }
        }
        #[cfg(not(feature = "scripting"))]
        {
            self.ensure_script_engine();
        }
    }

    #[cfg(feature = "scripting")]
    fn script_context(&self) -> scripting::ScriptContext {
        let mut context = scripting::ScriptContext::new(
            Arc::clone(&self.grid),
            Arc::clone(&self.root_grid),
            self.json_path.clone(),
        );
        context.selected = self.selected;
        context.selection = self.selected_cells();
        context.depth = self.nav_stack.len();
        context.profile = self.profile.clone();
        context.profiles = Some(self.profiles.clone());
        context.history = (
            self.history.undo_label().map(str::to_owned),
            self.history.redo_label().map(str::to_owned),
        );
        context.labels = self.labels();
        context.music = self.player.snapshot();
        context
    }

    /// Runs Ruby hooks for `event`; returns true when a hook returned
    /// `:handled`. Ruby is not started just to fire hooks.
    #[cfg(feature = "scripting")]
    fn emit_hook(&mut self, event: &str, args: Vec<HookValue>) -> bool {
        let Some(engine) = &self.script_engine else {
            return false;
        };
        if !engine.has_hooks(event) {
            return false;
        }
        engine.set_context(self.script_context());
        let (handled, output) = engine.emit(event, args);
        if !output.trim().is_empty() {
            let last = output
                .trim_end()
                .lines()
                .last()
                .unwrap_or_default()
                .to_owned();
            self.console_print(&output);
            self.status = Some(format!("[{event} hook] {last}"));
        }
        handled
    }

    #[cfg(not(feature = "scripting"))]
    fn emit_hook(&mut self, _event: &str, _args: Vec<HookValue>) -> bool {
        false
    }

    /// Runs the active profile's `init.rb` (if any), loads plugins and
    /// fires `:profile`.
    fn run_profile_init(&mut self) {
        let Some(name) = self.profile.clone() else {
            self.load_plugins();
            return;
        };
        let script = self.profiles.profile(&name).init_script();
        #[cfg(feature = "scripting")]
        if script.is_file() {
            self.ensure_script_engine();
            if let Some(engine) = &self.script_engine {
                engine.set_context(self.script_context());
                let (printed, result) = engine.eval_file_captured(&script);
                self.console_print(&format!("> load {}", script.display()));
                if !printed.is_empty() {
                    self.console_print(&printed);
                }
                self.console_print(&format!("=> {result}"));
            }
        }
        #[cfg(not(feature = "scripting"))]
        let _ = script;
        self.load_plugins();
        self.emit_hook("profile", vec![HookValue::Str(name)]);
    }

    #[cfg(feature = "scripting")]
    fn process_script_requests(&mut self) {
        let Some(engine) = &self.script_engine else {
            return;
        };
        let requests = engine.drain_requests();
        for request in requests {
            if let Err(error) = self.apply_script_request(request) {
                self.status = Some(format!("Error: {error}"));
            }
        }
    }

    #[cfg(not(feature = "scripting"))]
    fn process_script_requests(&mut self) {}

    #[cfg(feature = "scripting")]
    fn apply_script_request(&mut self, request: scripting::AppRequest) -> Result<(), String> {
        use scripting::AppRequest;
        match request {
            AppRequest::Status(text) => self.status = Some(text),
            AppRequest::Select(col, row) => self.selected = Some((col, row)),
            AppRequest::Goto(col, row) => {
                self.selected = Some((col, row));
                self.camera.center_on_cell(col, row, self.cell_size);
                self.orbit.focus(col, row);
            }
            AppRequest::View3d(enabled) => {
                if enabled != self.view_3d {
                    self.toggle_view_3d();
                }
            }
            AppRequest::Orbit {
                yaw,
                pitch,
                distance,
            } => self.orbit.set_angles(yaw, pitch, distance),
            AppRequest::Open(col, row) => {
                self.selected = Some((col, row));
                self.activate_cell(col, row)?;
            }
            AppRequest::Enter(col, row) => {
                let nested = lock(&self.grid).grid_at(col, row);
                match nested {
                    Some(nested) => self.enter_grid(nested),
                    None => return Err(format!("({col}, {row}) is not a grid")),
                }
            }
            AppRequest::Back => self.exit_grid(),
            AppRequest::Download {
                url,
                col,
                row,
                stack,
            } => {
                self.start_download(&url, col, row, false, stack)?;
                self.status = Some(format!("Downloading {url} in the background…"));
            }
            AppRequest::SelectCells(cells) => {
                if cells.len() as i64 > selection::MAX_RECT_CELLS {
                    return Err(format!(
                        "can't select {} cells (max {})",
                        cells.len(),
                        selection::MAX_RECT_CELLS
                    ));
                }
                let primary = cells.first().copied();
                self.selection.select_all(cells);
                self.selected = primary;
                if let Some(primary) = primary {
                    self.selection.set_anchor(primary);
                }
                self.info_cache = None;
            }
            AppRequest::SwitchProfile(name) => self.switch_profile(&name)?,
            AppRequest::Undo => self.undo()?,
            AppRequest::Redo => self.redo()?,
            AppRequest::Find(query) => {
                self.search = Some(SearchState {
                    query,
                    hits: Vec::new(),
                    index: 0,
                });
                self.refresh_search();
                if let Some(search) = &self.search {
                    self.status = Some(format!("{} match(es)", search.hits.len()));
                }
            }
            AppRequest::Labels(on) => self.set_labels(on),
            AppRequest::CopyText(text) => {
                self.clipboard()?
                    .set_text(text)
                    .map_err(|error| format!("clipboard: {error}"))?;
                self.status = Some("Copied text to the clipboard".to_owned());
            }
            AppRequest::ReloadPlugins => self.load_plugins(),
            AppRequest::Music(command) => self.apply_music_command(command)?,
        }
        Ok(())
    }

    // ------------------------------------------------------ music player --

    /// Playable audio cells of the current grid in reading order.
    fn grid_audio(&self) -> Vec<((i64, i64), PathBuf)> {
        let grid = lock(&self.grid);
        let mut tracks: Vec<((i64, i64), PathBuf)> = grid
            .entries()
            .filter(|(_, _, kind)| *kind == CellKind::Audio)
            .filter_map(|(col, row, _)| {
                grid.file_at(col, row)
                    .filter(|path| music::is_playable(path))
                    .map(|path| ((col, row), path.to_path_buf()))
            })
            .collect();
        tracks.sort_by_key(|((col, row), _)| (*row, *col));
        tracks
    }

    /// Plays `path` with the rest of this grid's audio as the playlist.
    fn play_audio(&mut self, path: PathBuf) -> Result<(), String> {
        let tracks = self.grid_audio();
        let start = tracks.iter().position(|(_, track)| *track == path);
        let (list, start) = match start {
            Some(start) => (tracks.into_iter().map(|(_, track)| track).collect(), start),
            None => (vec![path], 0),
        };
        let count = list.len();
        self.player.play_list(list, start)?;
        self.status = Some(format!("Playing {count} track(s) from this grid"));
        Ok(())
    }

    fn play_grid_audio(&mut self) -> Result<(), String> {
        let tracks: Vec<PathBuf> = self
            .grid_audio()
            .into_iter()
            .map(|(_, track)| track)
            .collect();
        if tracks.is_empty() {
            self.player.visible = true;
            return Err("no playable audio (wav/ogg/mp3/flac/qoa/xm/mod) in this grid".to_owned());
        }
        let count = tracks.len();
        self.player.play_list(tracks, 0)?;
        self.status = Some(format!("Playing {count} track(s) from this grid"));
        Ok(())
    }

    fn player_control(&mut self, control: music::Control) -> Result<(), String> {
        use music::Control;
        match control {
            Control::Previous => self.player.previous()?,
            Control::PlayPause => {
                if self.player.playlist.is_empty() {
                    self.play_grid_audio()?;
                } else {
                    self.player.toggle()?;
                }
            }
            Control::Next => self.player.next()?,
            Control::Stop => self.player.stop(),
            Control::Shuffle => {
                let on = !self.player.playlist.shuffle();
                self.player.set_shuffle(on);
                self.status = Some(format!("Shuffle {}", if on { "on" } else { "off" }));
            }
            Control::Repeat => {
                self.player.playlist.repeat = self.player.playlist.repeat.cycle();
                self.status = Some(format!("Repeat {}", self.player.playlist.repeat.name()));
            }
            Control::Playlist => {
                self.player.visible = true;
                self.player.playlist_open = !self.player.playlist_open;
            }
            Control::Close => {
                self.player.stop();
                self.player.visible = false;
                self.player.playlist_open = false;
            }
        }
        Ok(())
    }

    fn toggle_player(&mut self) {
        self.player.visible = !self.player.visible;
        if !self.player.visible {
            self.player.playlist_open = false;
        } else if self.player.playlist.is_empty() {
            self.status = Some(
                "Music player: double-click an audio cell, or press Play to play this grid's audio"
                    .to_owned(),
            );
        }
    }

    fn handle_player_keys(&mut self, rl: &RaylibHandle) -> Result<(), String> {
        use music::Control;
        let pressed = |key| rl.is_key_pressed(key) || rl.is_key_pressed_repeat(key);
        if rl.is_key_pressed(KeyboardKey::KEY_L) {
            self.set_labels(!self.labels());
        }
        if rl.is_key_pressed(KeyboardKey::KEY_P) {
            self.player_control(Control::Playlist)?;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_SPACE) {
            self.player_control(Control::PlayPause)?;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_LEFT_BRACKET) {
            self.player_control(Control::Previous)?;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_RIGHT_BRACKET) {
            self.player_control(Control::Next)?;
        }
        if pressed(KeyboardKey::KEY_COMMA) {
            self.player.seek_by(-5.0);
        }
        if pressed(KeyboardKey::KEY_PERIOD) {
            self.player.seek_by(5.0);
        }
        for (key, delta) in [
            (KeyboardKey::KEY_NINE, -0.05),
            (KeyboardKey::KEY_ZERO, 0.05),
        ] {
            if pressed(key) {
                self.player.set_volume(self.player.volume() + delta);
                self.status = Some(format!("Volume {:.0}%", self.player.volume() * 100.0));
            }
        }
        Ok(())
    }

    /// Routes mouse input on the player bar / playlist. Returns true when
    /// the event was consumed (so the grid underneath ignores it).
    fn handle_player_mouse(&mut self, rl: &RaylibHandle) -> bool {
        if !self.player.visible {
            self.player_drag = None;
            return false;
        }
        let (width, height) = (rl.get_screen_width(), rl.get_screen_height());
        let mouse = rl.get_mouse_position();
        let bar = music::bar_layout(&self.player, width, height);
        if let Some(drag) = self.player_drag {
            let down = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
            match drag {
                PlayerDrag::Seek(_) if down => {
                    self.player_drag =
                        Some(PlayerDrag::Seek(music::slider_fraction(bar.seek, mouse)));
                }
                PlayerDrag::Seek(fraction) => {
                    self.player.seek(fraction * self.player.length());
                    self.player_drag = None;
                }
                PlayerDrag::Volume => {
                    self.player
                        .set_volume(music::slider_fraction(bar.volume, mouse));
                    if !down {
                        self.player_drag = None;
                    }
                }
            }
            return true;
        }

        let playlist = self.player.playlist_open.then(|| {
            music::playlist_layout(&self.player, width, height, self.toolbar_layout(rl).height)
        });
        let in_list = playlist
            .as_ref()
            .is_some_and(|list| point_in_rect(mouse, list.frame));
        let in_bar = point_in_rect(mouse, bar.frame);
        if !in_list && !in_bar {
            return false;
        }
        let wheel = rl.get_mouse_wheel_move();
        let pressed = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let other = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT)
            || rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_MIDDLE);
        if let (Some(list), true) = (&playlist, in_list) {
            if wheel != 0.0 {
                let max = self.player.playlist.len().saturating_sub(list.visible_rows);
                let lines = (wheel.abs().ceil() as usize) * 3;
                self.player.playlist_scroll = if wheel > 0.0 {
                    self.player.playlist_scroll.min(max).saturating_sub(lines)
                } else {
                    (self.player.playlist_scroll + lines).min(max)
                };
            }
            if pressed {
                let hit = list
                    .rows
                    .iter()
                    .find(|(_, rect)| point_in_rect(mouse, *rect))
                    .map(|(index, _)| *index);
                if let Some(index) = hit {
                    if let Err(error) = self.player.jump(index) {
                        self.status = Some(format!("Error: {error}"));
                    }
                }
            }
        } else {
            if wheel != 0.0 {
                self.player.set_volume(self.player.volume() + wheel * 0.05);
                self.status = Some(format!("Volume {:.0}%", self.player.volume() * 100.0));
            }
            if pressed {
                let control = bar
                    .buttons
                    .iter()
                    .find(|(_, rect, _)| point_in_rect(mouse, *rect))
                    .map(|(control, _, _)| *control);
                if let Some(control) = control {
                    if let Err(error) = self.player_control(control) {
                        self.status = Some(format!("Error: {error}"));
                    }
                } else if self.player.has_track()
                    && point_in_rect(mouse, music::slider_hit(bar.seek, bar.scale))
                {
                    self.player_drag =
                        Some(PlayerDrag::Seek(music::slider_fraction(bar.seek, mouse)));
                } else if point_in_rect(mouse, music::slider_hit(bar.volume, bar.scale)) {
                    self.player_drag = Some(PlayerDrag::Volume);
                    self.player
                        .set_volume(music::slider_fraction(bar.volume, mouse));
                }
            }
        }
        wheel != 0.0 || pressed || other
    }

    /// Feeds audio, auto-advances and fires the `:track` hook.
    fn update_player(&mut self) {
        if let Err(error) = self.player.update() {
            self.status = Some(format!("Music: {error}"));
        }
        if let Some(path) = self.player.take_changed() {
            let index = self
                .player
                .playlist
                .current()
                .map_or(-1, |index| index as i64);
            let count = self.player.playlist.len() as i64;
            self.status = Some(format!("Now playing: {}", music::track_name(&path)));
            self.emit_hook(
                "track",
                vec![
                    HookValue::Str(path.display().to_string()),
                    HookValue::Int(index),
                    HookValue::Int(count),
                ],
            );
        }
    }

    #[cfg(feature = "scripting")]
    fn apply_music_command(&mut self, command: scripting::MusicCommand) -> Result<(), String> {
        use scripting::MusicCommand;
        match command {
            MusicCommand::Play(None) => {
                if self.player.has_track() {
                    self.player.resume();
                } else {
                    self.player_control(music::Control::PlayPause)?;
                }
            }
            MusicCommand::Play(Some(path)) => {
                let path = PathBuf::from(path);
                if !music::is_playable(&path) {
                    return Err(format!("{} is not a playable audio file", path.display()));
                }
                self.play_audio(path)?;
            }
            MusicCommand::PlayList(paths, start) => {
                self.player
                    .play_list(paths.into_iter().map(PathBuf::from).collect(), start)?;
            }
            MusicCommand::Queue(path) => self.player.enqueue(PathBuf::from(path))?,
            MusicCommand::Toggle => self.player_control(music::Control::PlayPause)?,
            MusicCommand::Pause => self.player.pause(),
            MusicCommand::Resume => self.player.resume(),
            MusicCommand::Stop => self.player.stop(),
            MusicCommand::Next => self.player.next()?,
            MusicCommand::Previous => self.player.previous()?,
            MusicCommand::Clear => self.player.clear(),
            MusicCommand::Show(on) => {
                self.player.visible = on;
                if !on {
                    self.player.playlist_open = false;
                }
            }
            MusicCommand::Seek(seconds) => self.player.seek(seconds as f32),
            MusicCommand::Volume(level) => self.player.set_volume(level as f32),
            MusicCommand::Shuffle(on) => self.player.set_shuffle(on),
            MusicCommand::Repeat(mode) => self.player.playlist.repeat = mode,
        }
        Ok(())
    }

    // ----------------------------------------------------------- plugins --

    fn plugin_dirs(&self) -> Vec<PathBuf> {
        let profile_dir = self
            .profile
            .as_ref()
            .map(|name| self.profiles.profile(name).dir);
        plugins::plugin_dirs(self.profiles.base(), profile_dir.as_deref())
    }

    fn plugin_files(&self) -> Vec<(PathBuf, bool)> {
        plugins::discover(&self.plugin_dirs())
            .into_iter()
            .map(|path| {
                let enabled = self.settings.plugin_enabled(&plugins::file_name(&path));
                (path, enabled)
            })
            .collect()
    }

    /// (Re)loads every plugin file; previous plugins' hooks are removed.
    #[cfg(feature = "scripting")]
    fn load_plugins(&mut self) {
        let files = self.plugin_files();
        if files.is_empty() && self.script_engine.is_none() {
            self.plugins.clear();
            self.plugin_timers.clear();
            return;
        }
        self.ensure_script_engine();
        let context = self.script_context();
        let Some(engine) = &self.script_engine else {
            return;
        };
        engine.set_context(context);
        let (infos, output) = engine.load_plugins(&files);
        self.plugins = infos;
        if !output.trim().is_empty() {
            self.console_print(&output);
        }
        let now = std::time::Instant::now();
        self.plugin_timers = self
            .plugins
            .iter()
            .flat_map(|plugin| &plugin.items)
            .filter(|item| item.kind == ItemKind::Timer)
            .map(|item| PluginTimer {
                id: item.id,
                interval: item.interval,
                due: now + std::time::Duration::from_secs_f64(item.interval.max(0.25)),
            })
            .collect();
        let failed: Vec<&str> = self
            .plugins
            .iter()
            .filter(|plugin| plugin.error.is_some())
            .map(|plugin| plugin.name.as_str())
            .collect();
        let loaded = self
            .plugins
            .iter()
            .filter(|plugin| plugin.enabled && plugin.error.is_none())
            .count();
        if !failed.is_empty() {
            self.status = Some(format!(
                "{} plugin(s) failed to load: {} (see Plugins)",
                failed.len(),
                failed.join(", ")
            ));
        } else if loaded > 0 {
            self.console_print(&format!("Loaded {loaded} plugin(s)."));
        }
    }

    #[cfg(not(feature = "scripting"))]
    fn load_plugins(&mut self) {
        self.plugins = self
            .plugin_files()
            .into_iter()
            .map(|(path, enabled)| PluginInfo::for_file(&path, enabled, None))
            .collect();
        self.plugin_timers.clear();
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    fn plugin_item_label(&self, id: i64) -> String {
        self.plugins
            .iter()
            .find_map(|plugin| {
                plugin
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .map(|item| format!("{} › {}", plugin.name, item.label))
            })
            .unwrap_or_else(|| format!("plugin action {id}"))
    }

    /// Runs a plugin action as one undoable edit.
    #[cfg(feature = "scripting")]
    fn invoke_plugin(&mut self, id: i64, cell: Option<(i64, i64)>) -> Result<(), String> {
        let label = self.plugin_item_label(id);
        let context = self.script_context();
        let Some(engine) = &self.script_engine else {
            return Err("Ruby is not running".to_owned());
        };
        engine.set_context(context);
        let grid = Arc::clone(&self.grid);
        let before = lock(&grid).snapshot();
        let (output, error) = engine.invoke_plugin(id, cell);
        if self
            .history
            .record(&grid, before, &format!("Plugin: {label}"))
        {
            self.save()?;
        }
        if !output.trim().is_empty() {
            let last = output
                .trim_end()
                .lines()
                .last()
                .unwrap_or_default()
                .to_owned();
            self.console_print(&output);
            self.status = Some(last);
        }
        match error {
            Some(error) => {
                self.console_print(&format!("[{label}] {error}"));
                Err(format!("{label}: {error}"))
            }
            None => Ok(()),
        }
    }

    #[cfg(not(feature = "scripting"))]
    fn invoke_plugin(&mut self, _id: i64, _cell: Option<(i64, i64)>) -> Result<(), String> {
        Err("plugins need the scripting build (--features scripting)".to_owned())
    }

    fn handle_plugin_keys(&mut self, rl: &RaylibHandle) {
        let pressed: Vec<i64> = self
            .plugins
            .iter()
            .filter(|plugin| plugin.enabled && plugin.error.is_none())
            .flat_map(|plugin| &plugin.items)
            .filter(|item| item.kind == ItemKind::Button)
            .filter(|item| {
                item.key
                    .as_deref()
                    .and_then(plugins::key_from_name)
                    .is_some_and(|key| rl.is_key_pressed(key))
            })
            .map(|item| item.id)
            .collect();
        for id in pressed {
            if let Err(error) = self.invoke_plugin(id, self.selected) {
                self.status = Some(format!("Error: {error}"));
            }
        }
    }

    fn fire_plugin_timers(&mut self) {
        let now = std::time::Instant::now();
        let due: Vec<i64> = self
            .plugin_timers
            .iter_mut()
            .filter(|timer| timer.due <= now)
            .map(|timer| {
                timer.due = now + std::time::Duration::from_secs_f64(timer.interval.max(0.25));
                timer.id
            })
            .collect();
        for id in due {
            if let Err(error) = self.invoke_plugin(id, None) {
                self.plugin_timers.retain(|timer| timer.id != id);
                self.status = Some(format!("Error: {error} (timer stopped)"));
            }
        }
    }

    fn plugin_panel_layout(&self, width: i32, height: i32) -> plugins::PanelLayout {
        plugins::panel_layout(
            &self.plugins,
            &self.plugin_dirs(),
            width,
            height,
            self.plugin_panel.unwrap_or(0.0),
            cfg!(feature = "scripting"),
        )
    }

    fn handle_plugin_panel(&mut self, rl: &RaylibHandle) -> Result<(), String> {
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.plugin_panel = None;
            return Ok(());
        }
        let (width, height) = (rl.get_screen_width(), rl.get_screen_height());
        let layout = self.plugin_panel_layout(width, height);
        let step = 40.0 * ui::scale(width, height);
        let mut scroll = layout.scroll() - rl.get_mouse_wheel_move() * step * 1.5;
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN)
            || rl.is_key_pressed_repeat(KeyboardKey::KEY_DOWN)
        {
            scroll += step;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_UP) || rl.is_key_pressed_repeat(KeyboardKey::KEY_UP) {
            scroll -= step;
        }
        self.plugin_panel = Some(scroll.clamp(0.0, layout.max_scroll()));
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let mouse = rl.get_mouse_position();
            match layout.action_at(mouse) {
                Some(action) => self.perform_plugin_panel_action(action)?,
                None if !point_in_rect(mouse, layout.frame) => self.plugin_panel = None,
                None => {}
            }
        }
        Ok(())
    }

    fn perform_plugin_panel_action(&mut self, action: plugins::PanelAction) -> Result<(), String> {
        use plugins::PanelAction;
        let global_dir = self.plugin_dirs().into_iter().next().unwrap_or_default();
        match action {
            PanelAction::Close => self.plugin_panel = None,
            PanelAction::Reload => {
                self.load_plugins();
                if !self.plugins.iter().any(|p| p.error.is_some()) {
                    self.status = Some(format!("Reloaded {} plugin file(s)", self.plugins.len()));
                }
            }
            PanelAction::OpenFolder => {
                fs::create_dir_all(&global_dir)
                    .map_err(|error| format!("creating {}: {error}", global_dir.display()))?;
                open_with_system(&global_dir)?;
            }
            PanelAction::InstallExamples => {
                let count = plugins::install_examples(&global_dir)?;
                self.load_plugins();
                self.status = Some(format!(
                    "Installed {count} example plugin(s) into {}",
                    global_dir.display()
                ));
            }
            PanelAction::Toggle(file_name) => {
                let enabled = !self.settings.plugin_enabled(&file_name);
                self.settings.set_plugin_enabled(&file_name, enabled);
                self.settings.save(&self.settings_path)?;
                self.load_plugins();
                self.status = Some(format!(
                    "{file_name} {}",
                    if enabled { "enabled" } else { "disabled" }
                ));
            }
            PanelAction::Invoke(id) => {
                self.plugin_panel = None;
                self.invoke_plugin(id, self.selected)?;
            }
        }
        Ok(())
    }

    fn handle_help_input(&mut self, rl: &RaylibHandle) {
        let (width, height) = (rl.get_screen_width(), rl.get_screen_height());
        let layout = panels::help_layout(width, height);
        let max = panels::help_max_scroll(&layout);
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) || rl.is_key_pressed(KeyboardKey::KEY_F1) {
            self.help_open = false;
            return;
        }
        let step = 40.0 * layout.scale;
        let mut scroll = self.help_scroll - rl.get_mouse_wheel_move() * step * 1.5;
        if rl.is_key_pressed(KeyboardKey::KEY_DOWN)
            || rl.is_key_pressed_repeat(KeyboardKey::KEY_DOWN)
        {
            scroll += step;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_UP) || rl.is_key_pressed_repeat(KeyboardKey::KEY_UP) {
            scroll -= step;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_PAGE_DOWN) {
            scroll += layout.body.height * 0.9;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_PAGE_UP) {
            scroll -= layout.body.height * 0.9;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_HOME) {
            scroll = 0.0;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_END) {
            scroll = max;
        }
        self.help_scroll = scroll.clamp(0.0, max);
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let mouse = rl.get_mouse_position();
            if point_in_rect(mouse, layout.close) || !point_in_rect(mouse, layout.frame) {
                self.help_open = false;
            }
        }
    }

    fn open_item_panel(&mut self) {
        if !matches!(self.mode, Mode::Main) {
            return;
        }
        self.item_panel = Some(item_panel::ItemPanel::default());
    }

    fn item_panel_title(&self) -> String {
        if self.nav_stack.is_empty() {
            "Inventory — root grid".to_owned()
        } else {
            format!("Inventory — nested grid (depth {})", self.nav_stack.len())
        }
    }

    fn handle_item_panel(&mut self, rl: &mut RaylibHandle) -> Result<(), String> {
        let (width, height) = (rl.get_screen_width(), rl.get_screen_height());
        let mouse = rl.get_mouse_position();
        let inventory = lock(&self.grid).inventory().clone();
        let Some(panel) = self.item_panel.as_mut() else {
            return Ok(());
        };
        let layout = panel.layout(&inventory, width, height);

        let mut action = None;
        while let Some(c) = rl.get_char_pressed() {
            if !c.is_control() && panel.input.chars().count() < 80 {
                panel.input.push(c);
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE)
            || rl.is_key_pressed_repeat(KeyboardKey::KEY_BACKSPACE)
        {
            panel.input.pop();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ENTER) || rl.is_key_pressed(KeyboardKey::KEY_KP_ENTER)
        {
            action = Some(item_panel::ItemAction::Add);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            action = Some(item_panel::ItemAction::Close);
        }
        if ctrl_down(rl) && rl.is_key_pressed(KeyboardKey::KEY_Z) {
            self.item_panel = None;
            let result = self.undo();
            self.item_panel = Some(item_panel::ItemPanel::default());
            return result;
        }
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            panel.scroll_by(wheel.signum() as i32, inventory.len(), layout.visible_rows);
        }
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if let Some(clicked) = panel.action_at(&layout, mouse) {
                action = Some(clicked);
            } else if !point_in_rect(mouse, layout.frame) {
                action = Some(item_panel::ItemAction::Close);
            }
        }
        match action {
            Some(action) => self.apply_item_action(action),
            None => Ok(()),
        }
    }

    /// Applies one inventory-panel action as an undoable edit.
    fn apply_item_action(&mut self, action: item_panel::ItemAction) -> Result<(), String> {
        use item_panel::ItemAction;
        let Some(panel) = self.item_panel.as_mut() else {
            return Ok(());
        };
        let edit: Result<String, String> = match action {
            ItemAction::Close => {
                self.item_panel = None;
                return Ok(());
            }
            ItemAction::Select(name) => {
                panel.input = name.clone();
                panel.selected = Some(name);
                return Ok(());
            }
            ItemAction::Add => match item_panel::parse_command(&panel.input) {
                Ok(command) => self.edit("Inventory", |app| {
                    use item_panel::EntryCommand;
                    let mut grid = lock(&app.grid);
                    let inventory = grid.inventory_mut();
                    match command {
                        EntryCommand::Add(name, amount) if amount < 0 => inventory
                            .remove(&name, -amount)
                            .map(|left| format!("Removed {} {name} ({left} left)", -amount)),
                        EntryCommand::Add(name, amount) => inventory
                            .add(&name, amount)
                            .map(|now| format!("Added {amount} {name} (now {now})")),
                        EntryCommand::Set(name, count) => inventory
                            .set(&name, count)
                            .map(|()| format!("{name} set to {}", count.max(0))),
                        EntryCommand::Rename(from, to) => inventory
                            .rename(&from, &to)
                            .map(|()| format!("Renamed {from} to {to}")),
                        EntryCommand::Meta(name, key, value) => {
                            let cleared = value.is_null();
                            inventory.set_meta(&name, &key, value).map(|()| {
                                if cleared {
                                    format!("Cleared {name}.{key}")
                                } else {
                                    format!("Set {name}.{key}")
                                }
                            })
                        }
                    }
                }),
                Err(error) => Err(error),
            },
            ItemAction::Increment(name) => self.edit("Inventory", |app| {
                lock(&app.grid)
                    .inventory_mut()
                    .add(&name, 1)
                    .map(|now| format!("{name}: {now}"))
            }),
            ItemAction::Decrement(name) => self.edit("Inventory", |app| {
                lock(&app.grid)
                    .inventory_mut()
                    .remove(&name, 1)
                    .map(|left| format!("{name}: {left}"))
            }),
            ItemAction::Delete(name) => self.edit("Inventory", |app| {
                lock(&app.grid).inventory_mut().delete(&name);
                Ok(format!("Deleted {name} (Ctrl+Z restores it)"))
            }),
            ItemAction::ClearAll => self.edit("Inventory", |app| {
                lock(&app.grid).inventory_mut().clear();
                Ok("Cleared the inventory (Ctrl+Z restores it)".to_owned())
            }),
        };
        let added = matches!(edit, Ok(_));
        let message = match edit {
            Ok(message) => (message, false),
            Err(error) => (error, true),
        };
        if added {
            self.save()?;
            self.emit_hook("inventory", vec![HookValue::Str(message.0.clone())]);
        }
        if let Some(panel) = self.item_panel.as_mut() {
            if added {
                panel.input.clear();
            }
            panel.message = Some(message);
        }
        Ok(())
    }

    fn open_profile_panel(&mut self) {
        let mut panel = ProfilePanel::new(self.profiles.list());
        if self.profile.is_none() {
            panel.set_info(format!(
                "Editing {} directly; pick a profile to switch to it.",
                self.json_path.display()
            ));
        }
        self.profile_panel = Some(panel);
    }

    fn handle_profile_panel(&mut self, rl: &mut RaylibHandle) -> Result<(), String> {
        let (width, height) = (rl.get_screen_width(), rl.get_screen_height());
        let mouse = rl.get_mouse_position();
        let current = self.profile.clone();
        let Some(panel) = self.profile_panel.as_mut() else {
            return Ok(());
        };
        let layout = panel.layout(width, height, current.as_deref());

        let mut action = None;
        if let Some((_, text)) = panel.input.as_mut() {
            while let Some(c) = rl.get_char_pressed() {
                if !c.is_control() && text.chars().count() < 40 {
                    text.push(c);
                }
            }
            if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE)
                || rl.is_key_pressed_repeat(KeyboardKey::KEY_BACKSPACE)
            {
                text.pop();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_ENTER)
                || rl.is_key_pressed(KeyboardKey::KEY_KP_ENTER)
            {
                action = Some(PanelAction::Submit);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                action = Some(PanelAction::CancelInput);
            }
        } else if panel.confirm_delete.is_some() {
            if rl.is_key_pressed(KeyboardKey::KEY_ENTER) || rl.is_key_pressed(KeyboardKey::KEY_Y) {
                action = Some(PanelAction::ConfirmDelete);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) || rl.is_key_pressed(KeyboardKey::KEY_N) {
                action = Some(PanelAction::CancelDelete);
            }
        } else {
            while rl.get_char_pressed().is_some() {}
            if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
                action = Some(PanelAction::Close);
            }
            if rl.is_key_pressed(KeyboardKey::KEY_N) {
                action = Some(PanelAction::StartNew);
            }
        }

        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            panel.scroll_by(wheel.signum() as i32, layout.visible_rows);
        }
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if let Some(clicked) = panel.action_at(&layout, mouse) {
                action = Some(clicked);
            } else if !point_in_rect(mouse, layout.frame) && panel.input.is_none() {
                action = Some(PanelAction::Close);
            }
        }

        match action {
            Some(action) => self.apply_panel_action(action),
            None => Ok(()),
        }
    }

    fn apply_panel_action(&mut self, action: PanelAction) -> Result<(), String> {
        let Some(panel) = self.profile_panel.as_mut() else {
            return Ok(());
        };
        match action {
            PanelAction::Close => self.profile_panel = None,
            PanelAction::StartNew => {
                panel.confirm_delete = None;
                panel.input = Some((InputKind::Create, String::new()));
                panel.message = None;
            }
            PanelAction::StartRename(name) => {
                panel.confirm_delete = None;
                panel.input = Some((InputKind::Rename(name.clone()), name));
                panel.message = None;
            }
            PanelAction::AskDelete(name) => {
                panel.input = None;
                panel.confirm_delete = Some(name);
            }
            PanelAction::CancelInput => panel.input = None,
            PanelAction::CancelDelete => panel.confirm_delete = None,
            PanelAction::ConfirmDelete => {
                let Some(name) = panel.confirm_delete.take() else {
                    return Ok(());
                };
                let result = self.delete_profile(&name);
                self.panel_report(result, format!("Deleted profile '{name}'"));
            }
            PanelAction::Switch(name) => {
                let result = self.switch_profile(&name);
                match result {
                    Ok(()) => self.profile_panel = None,
                    Err(error) => self.panel_report(Err(error), String::new()),
                }
            }
            PanelAction::Submit => {
                let Some((kind, text)) = panel.input.take() else {
                    return Ok(());
                };
                let result = match &kind {
                    InputKind::Create => self.create_profile(&text),
                    InputKind::Rename(old) => self.rename_profile(old, &text),
                };
                match (result, kind) {
                    (Ok(()), InputKind::Create) => self.profile_panel = None,
                    (Ok(()), InputKind::Rename(old)) => {
                        self.panel_report(Ok(()), format!("Renamed '{old}' to '{}'", text.trim()));
                    }
                    (Err(error), kind) => {
                        if let Some(panel) = self.profile_panel.as_mut() {
                            panel.input = Some((kind, text));
                            panel.set_error(error);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Refreshes the panel's profile list and shows `result` in it.
    fn panel_report(&mut self, result: Result<(), String>, success: String) {
        let names = self.profiles.list();
        if let Some(panel) = self.profile_panel.as_mut() {
            panel.names = names;
            panel.scroll = panel.scroll.min(panel.names.len().saturating_sub(1));
            match result {
                Ok(()) => panel.set_info(success),
                Err(error) => panel.set_error(error),
            }
        }
    }

    fn switch_profile(&mut self, name: &str) -> Result<(), String> {
        if self.profile.as_deref() == Some(name) {
            return Ok(());
        }
        if !self.profiles.exists(name) {
            return Err(format!("no profile named '{name}'"));
        }
        let profile = self.profiles.profile(name);
        let root = load_or_new_root(&profile.grid_path())?;
        if matches!(self.mode, Mode::Main) {
            self.save()?;
        }
        // Downloads keep their own root/json handles and finish into the
        // profile they were started from.
        self.root_grid = Arc::clone(&root);
        self.grid = root;
        self.json_path = profile.grid_path();
        self.profile = Some(profile.name.clone());
        self.nav_stack.clear();
        self.selected = None;
        self.drag = None;
        self.camera = Camera::new();
        self.orbit = self.orbit.recentered();
        self.history.clear();
        self.search = None;
        self.context_menu = None;
        self.thumbnails.clear();
        self.title_dirty = true;
        self.profiles.set_last_used(&profile.name)?;
        self.status = Some(format!("Switched to profile '{}'", profile.name));
        self.run_profile_init();
        Ok(())
    }

    fn create_profile(&mut self, name: &str) -> Result<(), String> {
        let profile = self.profiles.create(name)?;
        self.switch_profile(&profile.name)?;
        self.status = Some(format!(
            "Created profile '{}' — a fresh set of grids",
            profile.name
        ));
        Ok(())
    }

    fn rename_profile(&mut self, old: &str, new: &str) -> Result<(), String> {
        let is_current = self.profile.as_deref() == Some(old);
        if is_current {
            if !self.downloads.is_empty() {
                return Err("wait for downloads to finish before renaming this profile".to_owned());
            }
            self.save()?;
        }
        let renamed = self.profiles.rename(old, new)?;
        if is_current {
            let root = load_or_new_root(&renamed.grid_path())?;
            self.root_grid = Arc::clone(&root);
            self.grid = root;
            self.json_path = renamed.grid_path();
            self.profile = Some(renamed.name.clone());
            self.nav_stack.clear();
            self.selected = None;
            self.thumbnails.clear();
            self.title_dirty = true;
        }
        Ok(())
    }

    fn delete_profile(&mut self, name: &str) -> Result<(), String> {
        if self.profile.as_deref() == Some(name) {
            return Err("switch to another profile before deleting this one".to_owned());
        }
        if self
            .downloads
            .iter()
            .any(|pending| pending.json_path == self.profiles.profile(name).grid_path())
        {
            return Err("that profile still has downloads running".to_owned());
        }
        self.profiles.delete(name)
    }

    fn handle_drop(&mut self, rl: &mut RaylibHandle) -> Result<(), String> {
        if !rl.is_file_dropped() {
            return Ok(());
        }

        let mouse = rl.get_mouse_position();
        if mouse.y < self.toolbar_layout(rl).height {
            let _ = rl.load_dropped_files();
            return Ok(());
        }

        let dropped = rl.load_dropped_files();
        let paths: Vec<PathBuf> = dropped.iter().map(PathBuf::from).collect();
        if paths.is_empty() {
            return Ok(());
        }

        let viewport = self.viewport(rl);
        let (col, row) = if self.view_3d {
            self.pick_3d(rl, mouse)
                .ok_or_else(|| "drop onto the grid, not the sky".to_owned())?
        } else {
            let [col, row] = self
                .camera
                .cell_at([mouse.x as f64, mouse.y as f64], viewport, self.cell_size)
                .map_err(|error| error.to_string())?;
            (col, row)
        };

        let before = lock(&self.grid).snapshot();
        let stack = {
            let grid = lock(&self.grid);
            shift_down(rl) || matches!(grid.kind_at(col, row), Some(kind) if kind != CellKind::Grid)
        };
        let mut linked = Vec::new();
        let mut copying = 0;
        for (offset, path) in paths.iter().enumerate() {
            let target = if stack { col } else { col + offset as i64 };
            if let Some(assets) = self.copy_target(path)? {
                self.start_copy(path, &assets, target, row, !stack, stack, "drop");
                copying += 1;
                continue;
            }
            let mut grid = lock(&self.grid);
            if stack {
                linked.push(path.clone());
            } else if grid.insert_files_fanout(target, row, [path.clone()]) > 0 {
                linked.push(path.clone());
            }
        }
        if stack && !linked.is_empty() {
            lock(&self.grid).push_files(col, row, linked.clone());
        }
        let inserted = linked.len();
        if inserted > 0 {
            self.history.record(&self.grid, before, "Drop");
        }

        let copy_note = if copying > 0 {
            format!("copying {copying} file(s) into the grid's assets folder (Esc cancels)…")
        } else {
            String::new()
        };
        if inserted > 0 {
            let mut message = if stack {
                format!(
                    "Stacked {inserted} file(s) on ({col}, {row}) — Tab cycles, right-click → Unstack spreads them out"
                )
            } else {
                format!("Dropped {inserted} file(s) at ({col}, {row})")
            };
            if copying > 0 {
                message.push_str("; ");
                message.push_str(&copy_note);
            }
            self.status = Some(message);
            self.info_cache = None;
            self.save()?;
            let list = linked
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            self.emit_hook(
                "drop",
                vec![
                    HookValue::Int(col),
                    HookValue::Int(row),
                    HookValue::List(list),
                ],
            );
        } else if copying > 0 {
            self.status = Some(format!(
                "Copying {copying} file(s) dropped on ({col}, {row}) into the grid's assets folder (Esc cancels)…"
            ));
        }

        Ok(())
    }

    fn handle_pan(&mut self, rl: &RaylibHandle) {
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_MIDDLE) {
            self.panning = true;
        }
        if rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_MIDDLE) {
            self.panning = false;
        }
        if self.panning && rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_MIDDLE) {
            let delta = rl.get_mouse_delta();
            if delta.x != 0.0 || delta.y != 0.0 {
                self.camera.pan(delta.x as f64, delta.y as f64);
            }
        }
    }

    fn handle_zoom(&mut self, rl: &RaylibHandle) {
        let wheel = rl.get_mouse_wheel_move();
        if wheel == 0.0 {
            return;
        }
        let factor = 1.0 + (wheel * ZOOM_STEP);
        if factor <= 0.0 {
            return;
        }

        let mouse = rl.get_mouse_position();
        let _ = self.camera.zoom_at(
            [mouse.x as f64, mouse.y as f64],
            self.viewport(rl),
            factor as f64,
        );
    }

    fn handle_left_click(&mut self, rl: &RaylibHandle) -> Result<(), String> {
        let mouse = rl.get_mouse_position();
        let toolbar = self.toolbar_layout(rl);
        let action = toolbar
            .buttons
            .iter()
            .find(|button| button.enabled && point_in_rect(mouse, button.rect))
            .map(|button| button.action);
        if let Some(action) = action {
            if let Err(error) = self.perform_toolbar_action(action) {
                self.status = Some(format!("Error: {error}"));
            }
            return Ok(());
        }
        if mouse.y < toolbar.height {
            return Ok(());
        }

        let [col, row] = self
            .camera
            .cell_at(
                [mouse.x as f64, mouse.y as f64],
                self.viewport(rl),
                self.cell_size,
            )
            .map_err(|error| error.to_string())?;

        let ctrl = ctrl_down(rl);
        let shift = shift_down(rl);
        if (ctrl || shift) && matches!(self.mode, Mode::Main) {
            self.drag = None;
            if shift {
                if let Err(error) = self.selection.extend_to(self.selected, (col, row), ctrl) {
                    self.status = Some(format!("Selection: {error}"));
                    return Ok(());
                }
                self.selected = Some((col, row));
            } else {
                self.selected = self.selection.toggle(self.selected, (col, row));
            }
            if self.selection.is_multi() {
                self.status = Some(format!(
                    "{} cell(s) selected (Ctrl+click toggles, Shift+click extends; drag one to move them all)",
                    self.selection.len()
                ));
            }
            return Ok(());
        }

        if self.camera.register_click(col, row, rl.get_time()) {
            self.drag = None;
            self.band = None;
            self.set_single_selection(Some((col, row)));
            self.activate_cell(col, row)?;
        } else {
            let occupied = lock(&self.grid).occupied(col, row);
            let group = self.selection.is_multi() && self.selection.contains((col, row));
            if group {
                // Keep the group so it can be dragged; a release without a
                // drag collapses to this cell (see handle_drag).
                self.selected = Some((col, row));
            } else {
                self.set_single_selection(Some((col, row)));
            }
            if matches!(self.mode, Mode::Main) {
                if occupied || group {
                    self.drag = Some(DragState {
                        from: (col, row),
                        start: mouse,
                        active: false,
                        group,
                    });
                } else {
                    self.band = Some(BandState {
                        from: (col, row),
                        start: mouse,
                        active: false,
                    });
                }
            }
        }

        Ok(())
    }

    fn activate_cell(&mut self, col: i64, row: i64) -> Result<(), String> {
        let entry = {
            let grid = lock(&self.grid);
            match grid.kind_at(col, row) {
                Some(CellKind::Grid) => grid.grid_at(col, row).map(ActivatedCell::Grid),
                Some(kind) => grid
                    .file_at(col, row)
                    .map(PathBuf::from)
                    .map(|path| ActivatedCell::File(kind, path)),
                None => None,
            }
        };

        let hook_args = match &entry {
            Some(ActivatedCell::File(kind, path)) => {
                Some((kind.name(), HookValue::Str(path.display().to_string())))
            }
            Some(ActivatedCell::Grid(_)) => Some(("grid", HookValue::Nil)),
            None => None,
        };
        if let Some((kind, path)) = hook_args {
            let handled = self.emit_hook(
                "activate",
                vec![
                    HookValue::Int(col),
                    HookValue::Int(row),
                    HookValue::Str(kind.to_owned()),
                    path,
                ],
            );
            if handled {
                return Ok(());
            }
        }

        match entry {
            Some(ActivatedCell::File(CellKind::Image, path)) => {
                spawn_mode(Mode::ImageViewer(path), None)
            }
            Some(ActivatedCell::File(CellKind::RubyScript, path)) => {
                self.edit("Ruby script", |app| app.run_ruby_file(&path))
            }
            Some(ActivatedCell::File(CellKind::Audio, path)) if music::is_playable(&path) => {
                self.play_audio(path)
            }
            Some(ActivatedCell::File(_, path)) => open_with_system(&path),
            Some(ActivatedCell::Grid(grid)) => {
                self.enter_grid(grid);
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn perform_toolbar_action(&mut self, action: ToolbarAction) -> Result<(), String> {
        match action {
            ToolbarAction::Profile => {
                self.open_profile_panel();
                Ok(())
            }
            ToolbarAction::Save => {
                self.save()?;
                self.status = Some(format!("Saved {}", self.json_path.display()));
                let path = self.json_path.display().to_string();
                self.emit_hook("save", vec![HookValue::Str(path)]);
                Ok(())
            }
            ToolbarAction::Undo => self.undo(),
            ToolbarAction::Redo => self.redo(),
            ToolbarAction::Find => {
                self.toggle_search();
                Ok(())
            }
            ToolbarAction::NewGrid => self.edit("New Grid", |app| app.create_grid_at_selection()),
            ToolbarAction::Delete => self.edit("Delete", |app| app.delete_selection()),
            ToolbarAction::Paste => self.edit("Paste", |app| app.paste_clipboard()),
            ToolbarAction::Copy => self.copy_selection(),
            ToolbarAction::OpenRun => self.open_or_run_selection(),
            ToolbarAction::PaExport => self.partitioned_array_transfer(true),
            ToolbarAction::PaImport => {
                self.edit("PA Import", |app| app.partitioned_array_transfer(false))
            }
            ToolbarAction::OpenInventory => {
                self.save()?;
                spawn_mode(Mode::Inventory, Some(self.json_path.clone()))
            }
            ToolbarAction::Items => {
                self.open_item_panel();
                Ok(())
            }
            ToolbarAction::GridFolder => {
                let grid = Arc::clone(&self.grid);
                self.open_grid_folder(&grid)
            }
            ToolbarAction::Console => {
                self.toggle_console();
                Ok(())
            }
            ToolbarAction::Home => {
                self.reset_view();
                Ok(())
            }
            ToolbarAction::View3d => {
                self.toggle_view_3d();
                Ok(())
            }
            ToolbarAction::Labels => {
                self.set_labels(!self.labels());
                Ok(())
            }
            ToolbarAction::CopyFiles => {
                self.set_copy_files(!self.settings.copy_files);
                Ok(())
            }
            ToolbarAction::Music => {
                self.toggle_player();
                Ok(())
            }
            ToolbarAction::Plugins => {
                self.plugin_panel = Some(0.0);
                Ok(())
            }
            ToolbarAction::Help => {
                self.help_open = true;
                Ok(())
            }
            ToolbarAction::CancelDownloads => {
                self.cancel_downloads();
                Ok(())
            }
            ToolbarAction::Refresh => self.refresh(),
            ToolbarAction::Back => {
                self.exit_grid();
                Ok(())
            }
        }
    }

    fn open_or_run_selection(&mut self) -> Result<(), String> {
        let Some((col, row)) = self.selected else {
            return Ok(());
        };
        self.activate_cell(col, row)
    }

    fn clipboard(&mut self) -> Result<&mut arboard::Clipboard, String> {
        // Kept alive for the app's lifetime: on X11/Wayland the copied data
        // is served by this process and vanishes if the handle is dropped.
        if self.clipboard.is_none() {
            self.clipboard = Some(
                arboard::Clipboard::new()
                    .map_err(|error| format!("clipboard unavailable: {error}"))?,
            );
        }
        Ok(self.clipboard.as_mut().expect("just initialised"))
    }

    fn paste_clipboard(&mut self) -> Result<(), String> {
        self.paste_clipboard_with(false)
    }

    /// Pastes into the selected cell; with `stack` every item is pushed on
    /// top of that one cell instead of spreading to the right.
    fn paste_clipboard_with(&mut self, stack: bool) -> Result<(), String> {
        let Some((col, row)) = self.selected else {
            return Err("select a cell to paste into first".to_owned());
        };
        let asset_dir = self.asset_directory()?;
        let clipboard = self.clipboard()?;

        // 1. Files copied in a file manager.
        if let Ok(files) = clipboard.get().file_list() {
            let files: Vec<PathBuf> = files.into_iter().filter(|path| path.exists()).collect();
            if !files.is_empty() {
                return self.place_items(
                    col,
                    row,
                    files.into_iter().map(PasteItem::Path).collect(),
                    stack,
                );
            }
        }

        // 2. Raw image data (screenshots, "Copy image"), any resolution.
        if let Ok(image) = clipboard.get_image() {
            let path = asset_dir.join(format!("clipboard-{}.png", timestamp()));
            let (width, height) = (image.width, image.height);
            let pixels = image.bytes.into_owned();
            let buffer = image::RgbaImage::from_raw(width as u32, height as u32, pixels)
                .ok_or_else(|| "clipboard returned invalid image data".to_owned())?;
            buffer
                .save(&path)
                .map_err(|error| format!("could not save clipboard image: {error}"))?;
            self.status = Some(format!("Pasted {width}×{height} image"));
            return self.place_items(col, row, vec![PasteItem::Path(path)], stack);
        }

        // 3. Text: URLs and/or paths, one per line.
        let text = clipboard.get_text().map_err(|error| {
            format!("clipboard contains no usable files, image or text: {error}")
        })?;
        let mut items = Vec::new();
        let mut rejected = Vec::new();
        for entry in download::clipboard_entries(&text) {
            if download::is_url(&entry) {
                items.push(PasteItem::Url(entry));
            } else if let Some(path) = download::file_url_to_path(&entry) {
                items.push(PasteItem::Path(path));
            } else {
                let path = expand_home(&entry);
                if path.exists() {
                    items.push(PasteItem::Path(path));
                } else {
                    rejected.push(entry);
                }
            }
        }
        if items.is_empty() {
            return Err(match rejected.first() {
                Some(first) => format!("clipboard text is not a URL or existing path: {first}"),
                None => "clipboard is empty".to_owned(),
            });
        }
        self.place_items(col, row, items, stack)?;
        if !rejected.is_empty() {
            self.status = Some(format!(
                "{} — skipped {} line(s) that weren't URLs or existing paths",
                self.status.clone().unwrap_or_default(),
                rejected.len()
            ));
        }
        Ok(())
    }

    /// Places pasted items starting at `(col, row)`: the first replaces a
    /// file already in the selected cell, the rest go to the next free
    /// cells to the right. With `stack` they all go on top of `(col, row)`
    /// instead. URLs download in the background.
    fn place_items(
        &mut self,
        col: i64,
        row: i64,
        items: Vec<PasteItem>,
        stack: bool,
    ) -> Result<(), String> {
        let mut placed = Vec::new();
        let mut queued = 0;
        let mut copying = 0;
        let mut next_col = col;
        for (index, item) in items.into_iter().enumerate() {
            let replace_first =
                !stack && index == 0 && lock(&self.grid).kind_at(col, row) != Some(CellKind::Grid);
            let target = if replace_first || stack {
                col
            } else {
                self.next_unreserved(next_col, row)
            };
            next_col = target + 1;
            match item {
                PasteItem::Path(path) => {
                    if let Some(assets) = self.copy_target(&path)? {
                        self.start_copy(&path, &assets, target, row, replace_first, stack, "paste");
                        copying += 1;
                        continue;
                    }
                    {
                        let mut grid = lock(&self.grid);
                        if stack {
                            grid.push_file(target, row, path.clone());
                        } else {
                            if replace_first {
                                grid.pop_item(target, row, 0);
                            }
                            grid.push_file(target, row, path.clone());
                        }
                    }
                    placed.push((target, path));
                }
                PasteItem::Url(url) => {
                    self.start_download(&url, target, row, replace_first, stack)?;
                    queued += 1;
                }
            }
        }
        if !placed.is_empty() {
            self.info_cache = None;
            self.save()?;
        }
        let place = if stack {
            format!(" onto the stack at ({col}, {row})")
        } else {
            String::new()
        };
        let mut message = match (placed.len(), queued) {
            (0, 0) => String::new(),
            (1, 0) => format!("Pasted {}{place}", placed[0].1.display()),
            (count, 0) => format!("Pasted {count} items{place}"),
            (0, count) => format!("Downloading {count} URL(s) in the background{place}…"),
            (count, urls) => format!("Pasted {count} item(s), downloading {urls} URL(s){place}…"),
        };
        if copying > 0 {
            let verb = if message.is_empty() {
                "Copying"
            } else {
                "; copying"
            };
            message.push_str(&format!(
                "{verb} {copying} file(s) into the grid's assets folder{place}…"
            ));
        }
        self.status = Some(message);
        for (target, path) in placed {
            self.emit_hook(
                "paste",
                vec![
                    HookValue::Int(target),
                    HookValue::Int(row),
                    HookValue::Str(path.display().to_string()),
                ],
            );
        }
        Ok(())
    }

    /// First column at or after `col` in `row` that is neither occupied nor
    /// the target of a running download into the current grid.
    fn next_unreserved(&self, col: i64, row: i64) -> i64 {
        let grid = lock(&self.grid);
        let mut col = col;
        loop {
            col = grid.next_free_in_row(col, row);
            let reserved = self.downloads.iter().any(|pending| {
                Arc::ptr_eq(&pending.grid, &self.grid) && pending.col == col && pending.row == row
            });
            if !reserved {
                return col;
            }
            col += 1;
        }
    }

    fn start_download(
        &mut self,
        url: &str,
        col: i64,
        row: i64,
        replace: bool,
        stack: bool,
    ) -> Result<(), String> {
        let directory = self.asset_directory()?;
        self.downloads.push(PendingDownload {
            download: Download::start(url, &directory),
            grid: Arc::clone(&self.grid),
            root: Arc::clone(&self.root_grid),
            json_path: self.json_path.clone(),
            col,
            row,
            replace,
            stack,
            hook: "download",
        });
        Ok(())
    }

    /// Queues a background copy of `source` into the current grid's assets
    /// folder; the copy lands on (`col`, `row`) like a download.
    fn start_copy(
        &mut self,
        source: &Path,
        directory: &Path,
        col: i64,
        row: i64,
        replace: bool,
        stack: bool,
        hook: &'static str,
    ) {
        self.downloads.push(PendingDownload {
            download: Download::copy(source, directory),
            grid: Arc::clone(&self.grid),
            root: Arc::clone(&self.root_grid),
            json_path: self.json_path.clone(),
            col,
            row,
            replace,
            stack,
            hook,
        });
    }

    /// The assets folder to copy `path` into, or `None` when it should be
    /// linked in place (Copy In off, a folder, or already in assets/).
    fn copy_target(&self, path: &Path) -> Result<Option<PathBuf>, String> {
        if !self.settings.copy_files || !path.is_file() {
            return Ok(None);
        }
        let assets = self.asset_directory()?;
        Ok(download::needs_copy(path, &assets).then_some(assets))
    }

    fn cancel_downloads(&mut self) {
        for pending in &self.downloads {
            pending.download.cancel();
        }
        if !self.downloads.is_empty() {
            self.status = Some(format!("Cancelling {} download(s)…", self.downloads.len()));
        }
    }

    fn poll_downloads(&mut self) {
        let mut index = 0;
        while index < self.downloads.len() {
            match self.downloads[index].download.poll() {
                Some(outcome) => {
                    let pending = self.downloads.remove(index);
                    if let Err(error) = self.finish_download(pending, outcome) {
                        self.status = Some(format!("Error: {error}"));
                    }
                }
                None => index += 1,
            }
        }
    }

    fn finish_download(
        &mut self,
        pending: PendingDownload,
        outcome: Result<PathBuf, String>,
    ) -> Result<(), String> {
        let path = outcome?;
        let before = lock(&pending.grid).snapshot();
        let (col, row) = {
            let mut grid = lock(&pending.grid);
            let col = match grid.kind_at(pending.col, pending.row) {
                _ if pending.stack => pending.col,
                None => pending.col,
                Some(CellKind::Grid) => grid.next_free_in_row(pending.col, pending.row),
                Some(_) if pending.replace => {
                    grid.pop_item(pending.col, pending.row, 0);
                    pending.col
                }
                Some(_) => grid.next_free_in_row(pending.col, pending.row),
            };
            grid.push_file(col, pending.row, path.clone());
            (col, pending.row)
        };
        if Arc::ptr_eq(&pending.root, &self.root_grid) {
            let label = match pending.hook {
                "drop" => "Drop",
                "paste" => "Paste",
                _ => "Download",
            };
            self.history.record(&pending.grid, before, label);
            self.save()?;
        } else {
            lock(&pending.root)
                .save(&pending.json_path)
                .map_err(|error| error.to_string())?;
        }
        let size = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file");
        self.status = Some(if pending.download.is_copy {
            format!(
                "Copied {name} ({}) into the grid's assets folder at ({col}, {row})",
                download::human_bytes(size)
            )
        } else {
            format!(
                "Downloaded {name} ({}) to ({col}, {row})",
                download::human_bytes(size)
            )
        });
        let shown = path.display().to_string();
        let value = if pending.hook == "drop" {
            HookValue::List(vec![shown])
        } else {
            HookValue::Str(shown)
        };
        self.emit_hook(
            pending.hook,
            vec![HookValue::Int(col), HookValue::Int(row), value],
        );
        Ok(())
    }

    fn copy_selection(&mut self) -> Result<(), String> {
        if self.selected.is_none() {
            return Err("select a cell to copy first".to_owned());
        }
        let paths: Vec<PathBuf> = {
            let grid = lock(&self.grid);
            self.selected_cells()
                .into_iter()
                .filter_map(|(col, row)| grid.file_at(col, row).map(Path::to_path_buf))
                .map(|path| fs::canonicalize(&path).unwrap_or(path))
                .collect()
        };
        if paths.is_empty() {
            return Err("only cells holding a file can be copied".to_owned());
        }
        let clipboard = self.clipboard()?;
        let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
        if clipboard.set().file_list(&refs).is_err() {
            let text = paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join("\n");
            clipboard
                .set_text(text)
                .map_err(|error| format!("could not copy: {error}"))?;
        }
        self.status = Some(match paths.as_slice() {
            [one] => format!("Copied {}", one.display()),
            many => format!("Copied {} files", many.len()),
        });
        Ok(())
    }

    /// Ctrl+X: copy the selected files, then delete the cells (undoable).
    fn cut_selection(&mut self) -> Result<(), String> {
        self.copy_selection()?;
        let copied = self.status.clone().unwrap_or_default();
        self.edit("Cut", |app| app.delete_selection())?;
        if self.pending_delete.is_empty() {
            self.status = Some(format!(
                "{copied} and removed them (Ctrl+V pastes, Ctrl+Z undoes)"
            ));
        }
        Ok(())
    }

    /// Where files added to the current grid are stored: the `assets/`
    /// subfolder of that grid's own folder.
    fn asset_directory(&self) -> Result<PathBuf, String> {
        let (assets, new) =
            persistence::grid_assets_folder(&self.json_path, &self.grid, &self.root_grid)
                .map_err(|error| format!("could not create the grid's assets folder: {error}"))?;
        if new {
            self.save()?;
        }
        Ok(assets)
    }

    /// `grid`'s folder, created on first use (saving if it just got an id).
    fn folder_of(&self, grid: &Arc<Mutex<SavedGrid>>) -> Result<PathBuf, String> {
        let (folder, new) = persistence::grid_folder(&self.json_path, grid, &self.root_grid)
            .map_err(|error| format!("could not create the grid folder: {error}"))?;
        if new {
            self.save()?;
        }
        Ok(folder)
    }

    /// Opens `grid`'s folder in the system file manager.
    fn open_grid_folder(&mut self, grid: &Arc<Mutex<SavedGrid>>) -> Result<(), String> {
        let folder = self.folder_of(grid)?;
        open_with_system(&folder)?;
        self.status = Some(format!("Opened grid folder {}", folder.display()));
        Ok(())
    }

    #[cfg(feature = "scripting")]
    fn run_ruby_file(&mut self, path: &Path) -> Result<(), String> {
        self.ensure_script_engine();
        let Some(engine) = &self.script_engine else {
            return Err("Ruby scripting engine could not be started".to_owned());
        };
        engine.set_context(self.script_context());
        let (printed, result) = engine.eval_file_captured(path);
        self.console_print(&format!("> load {}", path.display()));
        if !printed.is_empty() {
            self.console_print(&printed);
        }
        self.console_print(&format!("=> {result}"));
        let first_line = printed.lines().last().unwrap_or(&result).to_owned();
        self.status = Some(format!("Ran {}: {}", path.display(), first_line));
        self.save()
    }

    #[cfg(not(feature = "scripting"))]
    fn run_ruby_file(&mut self, _path: &Path) -> Result<(), String> {
        Err("Ruby scripting is unavailable; install the scripting-enabled build".to_owned())
    }

    #[cfg(feature = "scripting")]
    fn partitioned_array_dir(&self) -> PathBuf {
        self.json_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("selenite_partitioned_array")
    }

    #[cfg(feature = "scripting")]
    fn partitioned_array_transfer(&mut self, export: bool) -> Result<(), String> {
        self.ensure_script_engine();
        let Some(engine) = &self.script_engine else {
            return Err("Ruby scripting engine could not be started".to_owned());
        };
        engine.set_context(self.script_context());
        let dir = self.partitioned_array_dir();
        if export {
            let count = engine.pa_export(&dir)?;
            self.status = Some(format!("Exported {count} cells to {}", dir.display()));
            Ok(())
        } else {
            let count = engine.pa_import(&dir)?;
            self.status = Some(format!("Imported {count} cells from {}", dir.display()));
            self.save()
        }
    }

    #[cfg(not(feature = "scripting"))]
    fn partitioned_array_transfer(&mut self, _export: bool) -> Result<(), String> {
        Err("partitioned_array integration requires the scripting-enabled build".to_owned())
    }

    fn create_grid_at_selection(&mut self) -> Result<(), String> {
        let Some((col, row)) = self.selected else {
            return Ok(());
        };

        {
            let mut grid = lock(&self.grid);
            if grid.occupied(col, row) {
                return Ok(());
            }
            grid.create_grid(col, row)
                .map_err(|error| error.to_string())?;
        }

        self.save()
    }

    fn delete_selection(&mut self) -> Result<(), String> {
        let cells: Vec<(i64, i64)> = {
            let grid = lock(&self.grid);
            self.selected_cells()
                .into_iter()
                .filter(|(col, row)| grid.occupied(*col, *row))
                .collect()
        };
        if cells.is_empty() {
            return Ok(());
        }

        // Deleting a nested grid destroys everything inside it, so require
        // an explicit confirmation when any stack being deleted holds a
        // non-empty grid. Files and empty grids go immediately (Ctrl+Z).
        let has_nonempty_grid = {
            let grid = lock(&self.grid);
            cells.iter().any(|(col, row)| {
                grid.items_at(*col, *row).iter().any(|item| match item {
                    persistence::CellContent::Grid(nested) => !lock(nested).is_empty(),
                    _ => false,
                })
            })
        };

        if has_nonempty_grid {
            self.pending_delete = cells;
            return Ok(());
        }

        self.remove_cells(&cells)
    }

    fn remove_cells(&mut self, cells: &[(i64, i64)]) -> Result<(), String> {
        let removed: usize = {
            let mut grid = lock(&self.grid);
            cells
                .iter()
                .filter_map(|(col, row)| grid.remove(*col, *row))
                .map(|items| items.len())
                .sum()
        };
        self.set_single_selection(None);
        self.info_cache = None;
        if cells.len() > 1 || removed > 1 {
            self.status = Some(format!(
                "Deleted {removed} item(s) from {} cell(s) — Ctrl+Z brings them back",
                cells.len()
            ));
        }
        self.save()
    }

    fn confirm_pending_delete(&mut self) -> Result<(), String> {
        let cells = std::mem::take(&mut self.pending_delete);
        if !cells.is_empty() {
            self.edit("Delete grid", |app| app.remove_cells(&cells))?;
            self.status = Some(match cells.as_slice() {
                [(col, row)] => format!("Deleted ({col}, {row}) — Ctrl+Z brings it back"),
                _ => format!(
                    "Deleted {} cells including nested grids — Ctrl+Z brings them back",
                    cells.len()
                ),
            });
        }
        Ok(())
    }

    fn cancel_pending_delete(&mut self) {
        self.pending_delete.clear();
    }

    fn handle_delete_confirmation(&mut self, rl: &RaylibHandle) -> Result<(), String> {
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) || rl.is_key_pressed(KeyboardKey::KEY_N) {
            self.cancel_pending_delete();
            return Ok(());
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ENTER) || rl.is_key_pressed(KeyboardKey::KEY_Y) {
            return self.confirm_pending_delete();
        }
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let (confirm_rect, cancel_rect) =
                confirm_dialog_rects(rl.get_screen_width(), rl.get_screen_height());
            let mouse = rl.get_mouse_position();
            if point_in_rect(mouse, confirm_rect) {
                return self.confirm_pending_delete();
            }
            if point_in_rect(mouse, cancel_rect) {
                self.cancel_pending_delete();
            }
        }
        Ok(())
    }

    fn refresh(&mut self) -> Result<(), String> {
        let root = load_existing_root(&self.json_path)?;
        self.root_grid = Arc::clone(&root);
        self.grid = root;
        self.nav_stack.clear();
        self.selected = None;
        self.camera = Camera::new();
        self.orbit = self.orbit.recentered();
        self.history.clear();
        self.search = None;
        Ok(())
    }

    fn save(&self) -> Result<(), String> {
        let grid = lock(&self.root_grid);
        grid.save(&self.json_path)
            .map_err(|error| error.to_string())
    }

    fn enter_grid(&mut self, sub_grid: Arc<Mutex<SavedGrid>>) {
        self.nav_stack.push(NavFrame {
            grid: Arc::clone(&self.grid),
            camera: self.camera.clone(),
            orbit: self.orbit.clone(),
            selected: self.selected,
        });
        self.grid = sub_grid;
        self.camera = Camera::new();
        self.orbit = self.orbit.recentered();
        self.selected = None;
    }

    fn exit_grid(&mut self) {
        if let Some(frame) = self.nav_stack.pop() {
            self.grid = frame.grid;
            self.camera = frame.camera;
            self.orbit = frame.orbit;
            self.selected = frame.selected;
        }
    }

    fn reset_view(&mut self) {
        self.camera = Camera::new();
        self.orbit = Orbit::default();
    }

    /// Runs an edit of the current grid and records it for undo if it
    /// changed anything.
    fn edit<T>(
        &mut self,
        label: &str,
        action: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        let grid = Arc::clone(&self.grid);
        let before = lock(&grid).snapshot();
        let result = action(self);
        if self.history.record(&grid, before, label) {
            self.info_cache = None;
        }
        result
    }

    fn undo(&mut self) -> Result<(), String> {
        match self.history.undo() {
            Some((grid, label)) => self.after_history_step(&grid, "Undid", &label),
            None => {
                self.status = Some("Nothing to undo".to_owned());
                Ok(())
            }
        }
    }

    fn redo(&mut self) -> Result<(), String> {
        match self.history.redo() {
            Some((grid, label)) => self.after_history_step(&grid, "Redid", &label),
            None => {
                self.status = Some("Nothing to redo".to_owned());
                Ok(())
            }
        }
    }

    fn after_history_step(
        &mut self,
        grid: &Arc<Mutex<SavedGrid>>,
        verb: &str,
        label: &str,
    ) -> Result<(), String> {
        let here = Arc::ptr_eq(grid, &self.grid);
        let left = self.repair_navigation();
        self.info_cache = None;
        self.status = Some(format!(
            "{verb} {label}{}{}",
            if here { "" } else { " (in another grid)" },
            if left {
                " — left a nested grid that no longer exists"
            } else {
                ""
            }
        ));
        self.save()
    }

    /// Cells of the nested grids leading from the root to the current
    /// grid, or `None` if one of them was removed.
    fn current_path(&self) -> Option<Vec<(i64, i64)>> {
        let mut chain: Vec<&Arc<Mutex<SavedGrid>>> =
            self.nav_stack.iter().map(|frame| &frame.grid).collect();
        chain.push(&self.grid);
        chain
            .windows(2)
            .map(|pair| lock(pair[0]).find_grid(pair[1]))
            .collect()
    }

    /// Leaves any nested grid that is no longer attached to its parent
    /// (e.g. after undoing its creation). Returns whether it had to.
    fn repair_navigation(&mut self) -> bool {
        let mut chain: Vec<Arc<Mutex<SavedGrid>>> = self
            .nav_stack
            .iter()
            .map(|frame| Arc::clone(&frame.grid))
            .collect();
        chain.push(Arc::clone(&self.grid));
        let broken = chain
            .windows(2)
            .position(|pair| lock(&pair[0]).find_grid(&pair[1]).is_none());
        match broken {
            Some(level) => {
                while self.nav_stack.len() > level {
                    self.exit_grid();
                }
                true
            }
            None => false,
        }
    }

    fn navigate_to_path(&mut self, target: &[(i64, i64)]) -> Result<(), String> {
        let current = self.current_path().unwrap_or_default();
        let common = current
            .iter()
            .zip(target)
            .take_while(|(a, b)| a == b)
            .count();
        let common = if current.len() == self.nav_stack.len() {
            common
        } else {
            0
        };
        while self.nav_stack.len() > common {
            self.exit_grid();
        }
        for &(col, row) in &target[common..] {
            let nested = lock(&self.grid)
                .grid_in(col, row)
                .ok_or_else(|| format!("nested grid ({col}, {row}) no longer exists"))?;
            self.enter_grid(nested);
        }
        Ok(())
    }

    fn breadcrumb(&self) -> String {
        match self.current_path() {
            Some(path) => search::Hit {
                path,
                cell: (0, 0),
                index: 0,
                name: String::new(),
            }
            .location(),
            None => format!("Depth {}", self.nav_stack.len()),
        }
    }

    fn toggle_search(&mut self) {
        self.search = match self.search {
            Some(_) => None,
            None => Some(SearchState {
                query: String::new(),
                hits: Vec::new(),
                index: 0,
            }),
        };
    }

    fn refresh_search(&mut self) {
        let Some(search) = &mut self.search else {
            return;
        };
        search.hits = search::search(&lock(&self.root_grid), &search.query);
        search.index = 0;
        if !search.hits.is_empty() {
            self.jump_to_hit(0);
        }
    }

    fn jump_to_hit(&mut self, index: usize) {
        let Some(hit) = self
            .search
            .as_ref()
            .and_then(|search| search.hits.get(index))
            .cloned()
        else {
            return;
        };
        if let Some(search) = &mut self.search {
            search.index = index;
        }
        if let Err(error) = self.navigate_to_path(&hit.path) {
            self.status = Some(format!("Error: {error}"));
            return;
        }
        let (col, row) = hit.cell;
        if hit.index > 0 {
            self.status = Some(format!(
                "{} is item {} of the stack at ({col}, {row}) — Tab cycles the stack",
                hit.name,
                hit.index + 1
            ));
        }
        self.set_single_selection(Some((col, row)));
        self.camera.center_on_cell(col, row, self.cell_size);
        self.orbit.focus(col, row);
    }

    fn step_search(&mut self, forward: bool) {
        let Some(search) = &self.search else {
            return;
        };
        let count = search.hits.len();
        if count == 0 {
            return;
        }
        let index = if forward {
            (search.index + 1) % count
        } else {
            (search.index + count - 1) % count
        };
        self.jump_to_hit(index);
    }

    fn handle_search_input(&mut self, rl: &mut RaylibHandle) {
        let ctrl = ctrl_down(rl);
        let shift = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        let mut changed = false;
        while let Some(character) = rl.get_char_pressed() {
            if !ctrl && !character.is_control() {
                if let Some(search) = &mut self.search {
                    search.query.push(character);
                    changed = true;
                }
            }
        }
        let held = |key| rl.is_key_pressed(key) || rl.is_key_pressed_repeat(key);
        if held(KeyboardKey::KEY_BACKSPACE) {
            if let Some(search) = &mut self.search {
                changed |= search.query.pop().is_some();
            }
        }
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_V) {
            let text = self
                .clipboard()
                .ok()
                .and_then(|clipboard| clipboard.get_text().ok())
                .unwrap_or_default();
            if let (Some(line), Some(search)) = (text.lines().next(), &mut self.search) {
                search.query.push_str(line.trim());
                changed = true;
            }
        }
        if changed {
            self.refresh_search();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.search = None;
            return;
        }
        let enter = held(KeyboardKey::KEY_ENTER) || held(KeyboardKey::KEY_KP_ENTER);
        if (enter && !shift) || held(KeyboardKey::KEY_DOWN) {
            self.step_search(true);
        }
        if (enter && shift) || held(KeyboardKey::KEY_UP) {
            self.step_search(false);
        }
    }

    fn open_context_menu(&mut self, cell: (i64, i64), position: Vector2) {
        // Right-clicking inside a multi-selection keeps the group.
        if self.selection.is_multi() && self.selection.contains(cell) {
            self.selected = Some(cell);
        } else {
            self.set_single_selection(Some(cell));
        }
        let multi = self.selection.is_multi();
        let (kind, playable, stack_len) = {
            let grid = lock(&self.grid);
            let playable = grid.file_at(cell.0, cell.1).is_some_and(music::is_playable);
            (
                grid.kind_at(cell.0, cell.1),
                playable,
                grid.stack_len(cell.0, cell.1),
            )
        };
        let mut items = MenuItem::for_cell(kind, stack_len, multi);
        if !playable {
            items.retain(|item| *item != MenuItem::QueueAudio);
        }
        if !matches!(self.mode, Mode::Main) {
            items.retain(|item| {
                !matches!(
                    item,
                    MenuItem::Paste
                        | MenuItem::NewGrid
                        | MenuItem::Delete
                        | MenuItem::CycleStack
                        | MenuItem::RemoveTop
                        | MenuItem::Unstack
                        | MenuItem::StackSelectionHere
                        | MenuItem::Items
                )
            });
        }
        let kind_name = kind.map(|kind| kind.name());
        let plugin_labels: Vec<(i64, String)> = self
            .plugins
            .iter()
            .filter(|plugin| plugin.enabled && plugin.error.is_none())
            .flat_map(|plugin| &plugin.items)
            .filter(|item| item.kind == ItemKind::Menu && item.applies_to(kind_name))
            .map(|item| (item.id, item.label.clone()))
            .collect();
        items.extend(plugin_labels.iter().map(|(id, _)| MenuItem::Plugin(*id)));
        if items.is_empty() {
            return;
        }
        self.context_menu = Some(ContextMenu {
            cell,
            position,
            items,
            plugin_labels,
        });
    }

    fn handle_context_menu(&mut self, rl: &RaylibHandle) -> Result<(), String> {
        let Some(menu) = &self.context_menu else {
            return Ok(());
        };
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.context_menu = None;
            return Ok(());
        }
        let left = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let right = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT);
        if !left && !right {
            return Ok(());
        }
        let mouse = rl.get_mouse_position();
        let (_, rows, _) = menu.layout(rl.get_screen_width(), rl.get_screen_height());
        let chosen = rows
            .iter()
            .position(|rect| point_in_rect(mouse, *rect))
            .map(|index| menu.items[index]);
        let cell = menu.cell;
        self.context_menu = None;
        match chosen {
            Some(item) if left => self.perform_menu_item(item, cell),
            _ => Ok(()),
        }
    }

    fn perform_menu_item(&mut self, item: MenuItem, (col, row): (i64, i64)) -> Result<(), String> {
        self.selected = Some((col, row));
        let path = lock(&self.grid).file_at(col, row).map(Path::to_path_buf);
        match item {
            MenuItem::Open => self.activate_cell(col, row),
            MenuItem::Info => {
                self.info_open = true;
                Ok(())
            }
            MenuItem::CopyFile => self.copy_selection(),
            MenuItem::CopyPath => {
                let path = path.ok_or_else(|| "this cell has no file".to_owned())?;
                let absolute = fs::canonicalize(&path).unwrap_or(path);
                let text = absolute.display().to_string();
                self.clipboard()?
                    .set_text(text.clone())
                    .map_err(|error| format!("could not copy: {error}"))?;
                self.status = Some(format!("Copied path {text}"));
                Ok(())
            }
            MenuItem::ShowInFolder => {
                let path = path.ok_or_else(|| "this cell has no file".to_owned())?;
                let folder = path
                    .parent()
                    .filter(|parent| !parent.as_os_str().is_empty())
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."));
                open_with_system(&folder)
            }
            MenuItem::Paste => self.edit("Paste", |app| app.paste_clipboard()),
            MenuItem::NewGrid => self.edit("New Grid", |app| app.create_grid_at_selection()),
            MenuItem::Delete => self.edit("Delete", |app| app.delete_selection()),
            MenuItem::CycleStack => self.edit("Cycle stack", |app| app.cycle_selected_stack(false)),
            MenuItem::RemoveTop => self.edit("Remove top item", |app| app.remove_top_item()),
            MenuItem::Unstack => self.edit("Unstack", |app| app.unstack_selected((col, row))),
            MenuItem::StackSelectionHere => self.edit("Stack selection", |app| {
                app.stack_selection_onto((col, row))
            }),
            MenuItem::Items => {
                self.open_item_panel();
                Ok(())
            }
            MenuItem::GridFolder => {
                let grid = Arc::clone(&self.grid);
                self.open_grid_folder(&grid)
            }
            MenuItem::NestedGridFolder => {
                let nested = lock(&self.grid)
                    .grid_at(col, row)
                    .ok_or_else(|| "this cell is not a grid".to_owned())?;
                self.open_grid_folder(&nested)
            }
            MenuItem::QueueAudio => {
                let path = path.ok_or_else(|| "this cell has no file".to_owned())?;
                let name = music::track_name(&path);
                self.player.enqueue(path)?;
                self.status = Some(format!(
                    "Added {name} to the playlist ({} track(s))",
                    self.player.playlist.len()
                ));
                Ok(())
            }
            MenuItem::Plugin(id) => self.invoke_plugin(id, Some((col, row))),
            MenuItem::RunGame => {
                let path = path.ok_or_else(|| "this cell has no file".to_owned())?;
                self.run_game(&path)
            }
            MenuItem::ExportGame => {
                let path = path.ok_or_else(|| "this cell has no file".to_owned())?;
                let folder = export_game(&path, &self.json_path)?;
                self.status = Some(format!(
                    "Exported game to {} (run ./run-game.sh)",
                    folder.display()
                ));
                Ok(())
            }
        }
    }

    /// Saves, then runs `script` as `selenite --game` in a child process
    /// against this profile/grid. When it exits, the grid is reloaded if the
    /// game saved changes (e.g. inventory) to it.
    fn run_game(&mut self, script: &Path) -> Result<(), String> {
        if !matches!(self.mode, Mode::Main) {
            return Err("games run from the main window".to_owned());
        }
        self.save()?;
        let exe = env::current_exe().map_err(|error| error.to_string())?;
        let mut command = Command::new(exe);
        command.arg("--game").arg(script);
        match &self.profile {
            Some(name) => command.arg("--profile").arg(name),
            None => command.arg("--grid").arg(&self.json_path),
        };
        let child = command
            .spawn()
            .map_err(|error| format!("could not start the game: {error}"))?;
        let name = script
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.status = Some(format!("Running {name} — close its window to return"));
        self.games.push(GameRun {
            child,
            name,
            json_path: self.json_path.clone(),
            modified: modified_time(&self.json_path),
        });
        Ok(())
    }

    fn poll_games(&mut self) {
        let mut finished = Vec::new();
        self.games.retain_mut(|run| match run.child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                finished.push((
                    run.name.clone(),
                    run.json_path.clone(),
                    run.modified,
                    status.success(),
                ));
                false
            }
            Err(_) => false,
        });
        for (name, json_path, modified, success) in finished {
            let changed = modified_time(&json_path) != modified;
            let mut message = if success {
                format!("Game {name} finished")
            } else {
                format!("Game {name} exited with an error (run it with --game in a terminal for details)")
            };
            if changed && json_path == self.json_path {
                match load_or_new_root(&json_path) {
                    Ok(root) => {
                        self.root_grid = Arc::clone(&root);
                        self.grid = root;
                        self.nav_stack.clear();
                        self.selected = None;
                        self.selection.clear();
                        self.history.clear();
                        self.thumbnails.clear();
                        self.info_cache = None;
                        message.push_str(" — reloaded the grid it saved");
                    }
                    Err(error) => message.push_str(&format!(" — could not reload: {error}")),
                }
            }
            self.status = Some(message);
        }
    }

    /// Height reserved at the bottom of the window by the player bar.
    fn bottom_inset(&self, screen_width: i32, screen_height: i32) -> f32 {
        if self.player.visible {
            music::bar_height(screen_width, screen_height)
        } else {
            0.0
        }
    }

    /// Screen rectangle of the minimap and the square of grid cells it
    /// shows: `(rect, min_col, min_row, cells_across)`.
    fn minimap_geometry(
        &self,
        screen_width: i32,
        screen_height: i32,
    ) -> Option<(Rectangle, f64, f64, f64)> {
        // The playlist panel covers the minimap's corner, so it yields.
        if !self.minimap_open || self.player.playlist_open || !matches!(self.mode, Mode::Main) {
            return None;
        }
        let scale = ui::scale(screen_width, screen_height);
        let size = (190.0 * scale)
            .min(screen_width as f32 * 0.3)
            .min(screen_height as f32 * 0.35);
        if size < 60.0 {
            return None;
        }
        let margin = 10.0 * scale;
        let rect = Rectangle::new(
            screen_width as f32 - size - margin,
            screen_height as f32
                - size
                - margin
                - 34.0 * scale
                - self.bottom_inset(screen_width, screen_height),
            size,
            size,
        );
        let (mut min_x, mut min_y, mut max_x, mut max_y) =
            match self.view_window(screen_width, screen_height) {
                Some((x0, y0, x1, y1)) => (x0, y0, x1, y1),
                None => (-4.0, -4.0, 4.0, 4.0),
            };
        for (col, row, _) in lock(&self.grid).entries() {
            min_x = min_x.min(col as f64);
            min_y = min_y.min(row as f64);
            max_x = max_x.max(col as f64 + 1.0);
            max_y = max_y.max(row as f64 + 1.0);
        }
        let span = (max_x - min_x).max(max_y - min_y).max(8.0) * 1.1;
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;
        Some((rect, center_x - span / 2.0, center_y - span / 2.0, span))
    }

    /// The area of the grid currently in view, in cell units.
    fn view_window(&self, screen_width: i32, screen_height: i32) -> Option<(f64, f64, f64, f64)> {
        if self.view_3d {
            let (col, row) = self.orbit.center_cell();
            let reach = (self.orbit.distance as f64 * 0.6).max(2.0);
            let (x, y) = (col as f64 + 0.5, row as f64 + 0.5);
            return Some((x - reach, y - reach, x + reach, y + reach));
        }
        let (min, max) = self
            .camera
            .visible_range([screen_width as f64, screen_height as f64], self.cell_size)
            .ok()?;
        Some((
            min[0] as f64,
            min[1] as f64,
            max[0] as f64 + 1.0,
            max[1] as f64 + 1.0,
        ))
    }

    /// Click or drag on the minimap to move the view. Returns whether the
    /// mouse is being used by the minimap this frame.
    fn minimap_click(&mut self, rl: &RaylibHandle) -> bool {
        let Some((rect, min_x, min_y, span)) =
            self.minimap_geometry(rl.get_screen_width(), rl.get_screen_height())
        else {
            self.minimap_drag = false;
            return false;
        };
        let mouse = rl.get_mouse_position();
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) && point_in_rect(mouse, rect)
        {
            self.minimap_drag = true;
        }
        if !rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) {
            self.minimap_drag = false;
        }
        if !self.minimap_drag {
            return point_in_rect(mouse, rect)
                && (rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT)
                    || rl.get_mouse_wheel_move() != 0.0);
        }
        let x = min_x + ((mouse.x - rect.x).clamp(0.0, rect.width) / rect.width) as f64 * span;
        let y = min_y + ((mouse.y - rect.y).clamp(0.0, rect.height) / rect.height) as f64 * span;
        let (col, row) = (x.floor() as i64, y.floor() as i64);
        self.camera.center_on_cell(col, row, self.cell_size);
        self.orbit.focus(col, row);
        true
    }

    fn draw_minimap(&self, d: &mut RaylibDrawHandle<'_>, search_cells: &[(i64, i64)]) {
        let (screen_width, screen_height) = (d.get_screen_width(), d.get_screen_height());
        let Some((rect, min_x, min_y, span)) = self.minimap_geometry(screen_width, screen_height)
        else {
            return;
        };
        let to_screen = |x: f64, y: f64| {
            Vector2::new(
                rect.x + ((x - min_x) / span) as f32 * rect.width,
                rect.y + ((y - min_y) / span) as f32 * rect.height,
            )
        };
        d.draw_rectangle_rec(rect, Color::new(12, 12, 18, 215));
        d.draw_rectangle_lines_ex(rect, 1.0, DIALOG_BORDER);
        let cell = (rect.width / span as f32).max(2.0);
        let entries: Vec<(i64, i64, CellKind)> = lock(&self.grid).entries().collect();
        for (col, row, kind) in entries {
            let point = to_screen(col as f64, row as f64);
            d.draw_rectangle_rec(
                Rectangle::new(point.x, point.y, cell * 0.85, cell * 0.85),
                kind_color(kind),
            );
        }
        for &(col, row) in search_cells {
            let point = to_screen(col as f64, row as f64);
            d.draw_rectangle_lines_ex(
                Rectangle::new(point.x - 1.0, point.y - 1.0, cell + 2.0, cell + 2.0),
                1.5,
                SEARCH_COLOR,
            );
        }
        let origin = to_screen(0.0, 0.0);
        if point_in_rect(origin, rect) {
            d.draw_circle_v(origin, 2.5, AXIS_COLOR);
        }
        if let Some((col, row)) = self.selected {
            let point = to_screen(col as f64 + 0.5, row as f64 + 0.5);
            if point_in_rect(point, rect) {
                d.draw_circle_v(point, 3.0, SELECTION_COLOR);
            }
        }
        if let Some((x0, y0, x1, y1)) = self.view_window(screen_width, screen_height) {
            let a = to_screen(x0, y0);
            let b = to_screen(x1, y1);
            let x = a.x.max(rect.x);
            let y = a.y.max(rect.y);
            let view = Rectangle::new(
                x,
                y,
                (b.x.min(rect.x + rect.width) - x).max(2.0),
                (b.y.min(rect.y + rect.height) - y).max(2.0),
            );
            d.draw_rectangle_lines_ex(view, 1.5, Color::new(235, 235, 245, 200));
        }
        let size = 12.0 * ui::scale(screen_width, screen_height);
        ui::draw_text(
            d,
            "map (M)",
            rect.x + 4.0,
            rect.y + 2.0,
            size,
            Color::new(150, 150, 170, 255),
        );
    }

    fn info_lines(&mut self) -> Vec<String> {
        let mut lines = self.cell_detail_lines();
        let Some((col, row)) = self.selected else {
            return lines;
        };
        let stack: Vec<String> = lock(&self.grid)
            .items_at(col, row)
            .iter()
            .map(content_label)
            .collect();
        if stack.len() > 1 {
            lines.push(format!("Stack of {} (Tab cycles, top first):", stack.len()));
            for (index, label) in stack.iter().enumerate().take(8) {
                let marker = if index == 0 { "›" } else { " " };
                lines.push(format!("{marker} {}. {label}", index + 1));
            }
            if stack.len() > 8 {
                lines.push(format!("  … {} more", stack.len() - 8));
            }
        }
        let count = self.selection.cells().count();
        if count > 1 {
            lines.insert(
                0,
                format!("{count} cells selected (drag to move, Delete removes all)"),
            );
        }
        let items = lock(&self.grid).inventory().len();
        if items > 0 {
            lines.push(format!("Grid inventory: {items} item type(s) — B to open"));
        }
        lines
    }

    fn cell_detail_lines(&mut self) -> Vec<String> {
        let Some((col, row)) = self.selected else {
            return vec!["No cell selected".to_owned()];
        };
        let (kind, path, nested) = {
            let grid = lock(&self.grid);
            (
                grid.kind_at(col, row),
                grid.file_at(col, row).map(Path::to_path_buf),
                grid.grid_at(col, row),
            )
        };
        let mut lines = vec![format!("Cell ({col}, {row})")];
        match (kind, nested) {
            (None, _) => {
                lines.push("Empty — paste, drop a file, or right-click.".to_owned());
                return lines;
            }
            (Some(CellKind::Grid), Some(nested)) => {
                let nested = lock(&nested);
                lines.push("Nested grid".to_owned());
                lines.push(format!(
                    "{} cell(s), {} item(s) here, {} cell(s) in total",
                    nested.len(),
                    nested.item_count(),
                    nested.total_len()
                ));
                lines.push("Double-click or Enter to go inside".to_owned());
                return lines;
            }
            _ => {}
        }
        if let Some((cached_cell, cached_path, cached)) = &self.info_cache {
            if *cached_cell == (col, row) && *cached_path == path {
                return cached.clone();
            }
        }
        if let (Some(kind), Some(path)) = (kind, &path) {
            lines.push(format!(
                "{}  ·  {}",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file"),
                kind.label()
            ));
            lines.push(path.display().to_string());
            match fs::metadata(path) {
                Ok(meta) => {
                    lines.push(format!("Size {}", download::human_bytes(meta.len())));
                    if let Ok(modified) = meta.modified() {
                        lines.push(format!("Modified {}", human_age(modified)));
                    }
                    if kind == CellKind::Image {
                        if let Ok((width, height)) = image::image_dimensions(path) {
                            lines.push(format!("{width} × {height} px"));
                        }
                    }
                }
                Err(_) => lines.push("! file is missing".to_owned()),
            }
        }
        self.info_cache = Some(((col, row), path, lines.clone()));
        lines
    }

    fn draw_info_panel(&mut self, d: &mut RaylibDrawHandle<'_>, top: f32) {
        if !self.info_open {
            return;
        }
        let lines = self.info_lines();
        let (screen_width, screen_height) = (d.get_screen_width(), d.get_screen_height());
        let scale = ui::scale(screen_width, screen_height);
        let size = 15.0 * scale;
        let pad = 10.0 * scale;
        let width = (360.0 * scale).min(screen_width as f32 * 0.45);
        let mut wrapped = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            for part in ui::wrap(line, size, width - pad * 2.0) {
                wrapped.push((index == 0, part));
            }
        }
        wrapped.push((false, "I to close".to_owned()));
        let line_height = size * 1.35;
        let rect = Rectangle::new(
            screen_width as f32 - width - pad,
            top + pad,
            width,
            line_height * wrapped.len() as f32 + pad * 1.5,
        );
        d.draw_rectangle_rec(rect, DIALOG_BG);
        d.draw_rectangle_lines_ex(rect, 1.0, DIALOG_BORDER);
        let last = wrapped.len() - 1;
        for (index, (heading, line)) in wrapped.iter().enumerate() {
            let color = if *heading {
                SELECTION_COLOR
            } else if index == last {
                Color::new(130, 130, 150, 255)
            } else {
                VIEWER_TEXT
            };
            ui::draw_text(
                d,
                line,
                rect.x + pad,
                rect.y + pad * 0.75 + line_height * index as f32,
                size,
                color,
            );
        }
    }

    fn draw_search_bar(&self, d: &mut RaylibDrawHandle<'_>, top: f32, time: f64) {
        let Some(search) = &self.search else {
            return;
        };
        let (screen_width, screen_height) = (d.get_screen_width(), d.get_screen_height());
        let scale = ui::scale(screen_width, screen_height);
        let size = 17.0 * scale;
        let pad = 9.0 * scale;
        let width = (520.0 * scale).min(screen_width as f32 - pad * 2.0);
        let rect = Rectangle::new(pad, top + pad, width, size * 2.0 + pad * 2.2);
        d.draw_rectangle_rec(rect, DIALOG_BG);
        d.draw_rectangle_lines_ex(rect, 1.5, SEARCH_COLOR);
        let cursor = if (time * 2.0) as i64 % 2 == 0 {
            "|"
        } else {
            " "
        };
        let counter = if search.query.trim().is_empty() {
            "type to search".to_owned()
        } else if search.hits.is_empty() {
            "no matches".to_owned()
        } else {
            format!("{}/{}", search.index + 1, search.hits.len())
        };
        let counter_width = ui::measure(&counter, size);
        let query = ui::fit(
            &format!("Find: {}{cursor}", search.query),
            size,
            width - counter_width - pad * 3.0,
        );
        ui::draw_text(d, &query, rect.x + pad, rect.y + pad, size, BUTTON_TEXT);
        ui::draw_text(
            d,
            &counter,
            rect.x + width - counter_width - pad,
            rect.y + pad,
            size,
            SEARCH_COLOR,
        );
        let detail = match search.hits.get(search.index) {
            Some(hit) => format!(
                "{}  ·  {} ({},{})",
                hit.name,
                hit.location(),
                hit.cell.0,
                hit.cell.1
            ),
            None => "Enter/↓ next · Shift+Enter/↑ previous · Esc close".to_owned(),
        };
        let detail_size = size * 0.8;
        let detail = ui::fit(&detail, detail_size, width - pad * 2.0);
        ui::draw_text(
            d,
            &detail,
            rect.x + pad,
            rect.y + pad + size * 1.25,
            detail_size,
            Color::new(160, 160, 180, 255),
        );
    }

    fn draw_context_menu(&self, d: &mut RaylibDrawHandle<'_>, mouse: Vector2) {
        let Some(menu) = &self.context_menu else {
            return;
        };
        let (frame, rows, size) = menu.layout(d.get_screen_width(), d.get_screen_height());
        d.draw_rectangle_rec(frame, DIALOG_BG);
        d.draw_rectangle_lines_ex(frame, 1.0, DIALOG_BORDER);
        let heading = Rectangle::new(frame.x, frame.y, frame.width, rows[0].height);
        ui::draw_text_in(
            d,
            &format!("({}, {})", menu.cell.0, menu.cell.1),
            heading,
            size * 0.6,
            size * 0.85,
            Color::new(150, 150, 170, 255),
        );
        for (item, rect) in menu.items.iter().zip(&rows) {
            if point_in_rect(mouse, *rect) {
                let color = if *item == MenuItem::Delete {
                    DANGER_BG
                } else {
                    BUTTON_HOVER_BG
                };
                d.draw_rectangle_rec(*rect, color);
            }
            ui::draw_text_in(
                d,
                &menu.item_label(*item),
                *rect,
                size * 0.6,
                size,
                BUTTON_TEXT,
            );
        }
    }

    fn toggle_view_3d(&mut self) {
        self.view_3d = !self.view_3d;
        self.drag = None;
        self.orbit_press = None;
        self.panning = false;
        if self.view_3d {
            if let Some((col, row)) = self.selected {
                self.orbit.focus(col, row);
            }
            self.status = Some(
                "3D view: left-drag rotate · right/middle or Shift+drag pan · wheel zoom · Q/E turn · PgUp/PgDn tilt · F3 back to 2D"
                    .to_owned(),
            );
        } else {
            self.status = Some("2D view".to_owned());
        }
    }

    /// Mouse and keyboard handling for the 3D view: left-drag orbits,
    /// right/middle or Shift+left drags pan, a left click selects and a
    /// double click opens.
    fn handle_3d_input(&mut self, rl: &RaylibHandle, typing: bool) -> Result<(), String> {
        let mouse = rl.get_mouse_position();
        let toolbar_height = self.toolbar_layout(rl).height;
        let over_scene = mouse.y >= toolbar_height;
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 && over_scene {
            self.orbit.zoom(wheel);
        }

        if self.orbit_press.is_none() {
            for button in [
                MouseButton::MOUSE_BUTTON_LEFT,
                MouseButton::MOUSE_BUTTON_RIGHT,
                MouseButton::MOUSE_BUTTON_MIDDLE,
            ] {
                if !rl.is_mouse_button_pressed(button) {
                    continue;
                }
                if over_scene {
                    self.orbit_press = Some(OrbitPress {
                        button,
                        start: mouse,
                        moved: false,
                    });
                } else if matches!(button, MouseButton::MOUSE_BUTTON_LEFT) {
                    self.handle_left_click(rl)?;
                }
                break;
            }
        }

        if let Some(mut press) = self.orbit_press {
            let threshold = 6.0 * ui::scale(rl.get_screen_width(), rl.get_screen_height());
            if (mouse - press.start).length() > threshold {
                press.moved = true;
            }
            let delta = rl.get_mouse_delta();
            if press.moved && (delta.x != 0.0 || delta.y != 0.0) {
                let shift = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                    || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
                if shift || !matches!(press.button, MouseButton::MOUSE_BUTTON_LEFT) {
                    self.orbit
                        .pan(delta.x, delta.y, rl.get_screen_height() as f32);
                } else {
                    self.orbit.rotate(delta.x, delta.y);
                }
            }
            if rl.is_mouse_button_released(press.button) {
                self.orbit_press = None;
                if !press.moved {
                    let picked = self.pick_3d(rl, press.start);
                    match (press.button, picked) {
                        (MouseButton::MOUSE_BUTTON_LEFT, Some((col, row))) => {
                            if self.camera.register_click(col, row, rl.get_time()) {
                                self.activate_cell(col, row)?;
                            } else {
                                self.selected = Some((col, row));
                            }
                        }
                        (MouseButton::MOUSE_BUTTON_RIGHT, Some(cell)) => {
                            self.open_context_menu(cell, press.start);
                        }
                        _ => {}
                    }
                }
            } else {
                self.orbit_press = Some(press);
            }
        }

        if typing {
            return Ok(());
        }
        let turn = 15f32.to_radians();
        let held = |key| rl.is_key_pressed(key) || rl.is_key_pressed_repeat(key);
        if held(KeyboardKey::KEY_Q) {
            self.orbit.rotate_by(turn, 0.0);
        }
        if held(KeyboardKey::KEY_E) {
            self.orbit.rotate_by(-turn, 0.0);
        }
        if held(KeyboardKey::KEY_PAGE_UP) {
            self.orbit.rotate_by(0.0, turn / 2.0);
        }
        if held(KeyboardKey::KEY_PAGE_DOWN) {
            self.orbit.rotate_by(0.0, -turn / 2.0);
        }
        if !ctrl_down(rl) {
            let axis = |plus: KeyboardKey, minus: KeyboardKey| {
                rl.is_key_down(plus) as i32 as f32 - rl.is_key_down(minus) as i32 as f32
            };
            let right = axis(KeyboardKey::KEY_D, KeyboardKey::KEY_A);
            let forward = axis(KeyboardKey::KEY_W, KeyboardKey::KEY_S);
            if right != 0.0 || forward != 0.0 {
                let speed = 900.0 * rl.get_frame_time().min(0.1);
                self.orbit.pan(
                    -right * speed,
                    -forward * speed,
                    rl.get_screen_height() as f32,
                );
            }
        }
        Ok(())
    }

    /// Occupied cells within the 3D view's draw range, with their block
    /// heights (used for both picking and drawing).
    fn blocks_3d(&self) -> Vec<(i64, i64, CellKind)> {
        let (min_col, max_col, min_row, max_row) = self.orbit.visible_span();
        lock(&self.grid)
            .entries()
            .filter(|&(col, row, _)| {
                (min_col..=max_col).contains(&col) && (min_row..=max_row).contains(&row)
            })
            .collect()
    }

    fn pick_3d(&self, rl: &RaylibHandle, point: Vector2) -> Option<(i64, i64)> {
        let ray = rl.get_screen_to_world_ray(point, self.orbit.camera());
        let blocks = self.blocks_3d();
        view3d::pick_cell(
            ray,
            blocks
                .into_iter()
                .map(|(col, row, kind)| (col, row, block_height(kind))),
        )
    }

    fn request_visible_thumbnails(&mut self, rl: &RaylibHandle) {
        if !matches!(self.mode, Mode::Main | Mode::Inventory) {
            return;
        }
        if self.view_3d {
            let blocks = self.blocks_3d();
            let paths: Vec<PathBuf> = {
                let grid = lock(&self.grid);
                blocks
                    .into_iter()
                    .filter_map(|(col, row, _)| grid.image_at(col, row).map(Path::to_path_buf))
                    .collect()
            };
            for path in paths {
                self.thumbnails.request(&path);
            }
            return;
        }
        let viewport = self.viewport(rl);
        let Ok((min_cell, max_cell)) = self.camera.visible_range(viewport, self.cell_size) else {
            return;
        };
        let (min_col, max_col) = clamp_span(min_cell[0], max_cell[0]);
        let (min_row, max_row) = clamp_span(min_cell[1], max_cell[1]);

        let visible_paths: Vec<PathBuf> = {
            let grid = lock(&self.grid);
            let mut paths = Vec::new();
            for row in min_row..=max_row {
                for col in min_col..=max_col {
                    if let Some(path) = grid.image_at(col, row) {
                        paths.push(path.to_path_buf());
                    }
                }
            }
            paths
        };

        for path in visible_paths {
            self.thumbnails.request(&path);
        }
    }

    fn draw(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        let mouse = rl.get_mouse_position();
        let viewport = self.viewport(rl);
        let visible_range = self.camera.visible_range(viewport, self.cell_size).ok();
        let toolbar = self.toolbar_layout(rl);
        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();
        let scale = ui::scale(screen_width, screen_height);
        let time = rl.get_time();
        let drag_target = self
            .drag
            .as_ref()
            .filter(|drag| drag.active && mouse.y >= toolbar.height)
            .and_then(|_| {
                self.camera
                    .cell_at([mouse.x as f64, mouse.y as f64], viewport, self.cell_size)
                    .ok()
            })
            .map(|[col, row]| (col, row));
        let hovered_cell_for_drag = drag_target;
        let group_preview = self.group_drop_preview(drag_target);
        let band_rect = self.band.as_ref().filter(|band| band.active).map(|band| {
            Rectangle::new(
                band.start.x.min(mouse.x),
                band.start.y.min(mouse.y),
                (band.start.x - mouse.x).abs(),
                (band.start.y - mouse.y).abs(),
            )
        });
        let shift_down_draw = shift_down(rl);
        let download_cells: Vec<((i64, i64), Option<f32>)> = self
            .downloads
            .iter()
            .filter(|pending| Arc::ptr_eq(&pending.grid, &self.grid))
            .map(|pending| ((pending.col, pending.row), pending.download.fraction()))
            .collect();
        let tooltip = self
            .hover
            .filter(|(_, since)| time - since > 0.45)
            .and_then(|(action, _)| {
                toolbar
                    .buttons
                    .iter()
                    .find(|button| button.action == action)
                    .map(|button| (action.tooltip(), button.rect))
            });
        let hover_3d = (self.view_3d
            && mouse.y >= toolbar.height
            && self.orbit_press.is_none_or(|press| !press.moved))
        .then(|| self.pick_3d(rl, mouse))
        .flatten();
        let search_cells: Vec<(i64, i64)> = match (&self.search, self.current_path()) {
            (Some(search), Some(path)) => search
                .hits
                .iter()
                .filter(|hit| hit.path == path)
                .map(|hit| hit.cell)
                .collect(),
            _ => Vec::new(),
        };
        let mut d = rl.begin_drawing(thread);
        d.clear_background(BACKGROUND);

        if self.view_3d {
            self.draw_scene_3d(
                &mut d,
                hover_3d,
                &download_cells,
                &search_cells,
                time,
                toolbar.height,
            );
        } else if let Some((min_cell, max_cell)) = visible_range {
            let (min_col, max_col) = clamp_span(min_cell[0], max_cell[0]);
            let (min_row, max_row) = clamp_span(min_cell[1], max_cell[1]);
            for row in min_row..=max_row {
                for col in min_col..=max_col {
                    if let Ok(rect) = self.camera.cell_rect([col, row], viewport, self.cell_size) {
                        let rect = Rectangle::new(
                            rect[0] as f32,
                            rect[1] as f32,
                            rect[2] as f32,
                            rect[3] as f32,
                        );
                        d.draw_rectangle_lines_ex(rect, 1.0, GRID_LINE);
                        self.draw_cell(&mut d, col, row, rect);
                        let label_size = ui::cell_text_size(rect.height, 0.15, 64.0);
                        let show_label = self.labels() || self.selected == Some((col, row));
                        if show_label && label_size >= 9.0 {
                            let inset = rect.height * 0.05;
                            let coordinates = ui::fit(
                                &format!("{col},{row}"),
                                label_size,
                                rect.width - inset * 2.0,
                            );
                            ui::draw_text(
                                &mut d,
                                &coordinates,
                                rect.x + inset,
                                rect.y + inset * 0.6,
                                label_size,
                                Color::new(145, 145, 165, 255),
                            );
                        }
                        if search_cells.contains(&(col, row)) {
                            let thickness = (rect.height * 0.03).clamp(1.5, 6.0);
                            let inset = thickness * 1.5;
                            d.draw_rectangle_lines_ex(
                                Rectangle::new(
                                    rect.x + inset,
                                    rect.y + inset,
                                    rect.width - inset * 2.0,
                                    rect.height - inset * 2.0,
                                ),
                                thickness,
                                SEARCH_COLOR,
                            );
                        }
                        if self.selection.is_multi() && self.selection.contains((col, row)) {
                            d.draw_rectangle_rec(rect, Color::new(250, 210, 60, 38));
                            let thickness = (rect.height * 0.03).clamp(1.5, 6.0);
                            d.draw_rectangle_lines_ex(
                                rect,
                                thickness,
                                Color::new(250, 210, 60, 170),
                            );
                        }
                        let stack_len = lock(&self.grid).stack_len(col, row);
                        if stack_len > 1 {
                            draw_stack_badge(&mut d, rect, stack_len);
                        }
                        if self.selected == Some((col, row)) {
                            let thickness = (rect.height * 0.04).clamp(2.0, 8.0);
                            d.draw_rectangle_lines_ex(rect, thickness, SELECTION_COLOR);
                        }
                        if let Some((cells, blocked)) = &group_preview {
                            if cells.contains(&(col, row)) {
                                let tint = if *blocked {
                                    Color::new(235, 80, 80, 70)
                                } else {
                                    Color::new(90, 220, 120, 60)
                                };
                                d.draw_rectangle_rec(rect, tint);
                                d.draw_rectangle_lines_ex(
                                    rect,
                                    (rect.height * 0.04).clamp(2.0, 8.0),
                                    Color::new(tint.r, tint.g, tint.b, 230),
                                );
                            }
                        }
                        if drag_target == Some((col, row)) && group_preview.is_none() {
                            d.draw_rectangle_rec(rect, Color::new(250, 210, 60, 50));
                            let thickness = (rect.height * 0.05).clamp(2.0, 10.0);
                            d.draw_rectangle_lines_ex(
                                rect,
                                thickness,
                                Color::new(255, 240, 150, 255),
                            );
                        }
                        if let Some((_, fraction)) =
                            download_cells.iter().find(|(cell, _)| *cell == (col, row))
                        {
                            draw_download_progress(&mut d, rect, *fraction, time);
                        }
                    }
                }
            }
            if let Ok(origin) = self.camera.cell_rect([0, 0], viewport, self.cell_size) {
                let origin_x = origin[0] as f32;
                let origin_y = origin[1] as f32;
                d.draw_line_ex(
                    Vector2::new(origin_x, toolbar.height),
                    Vector2::new(origin_x, screen_height as f32),
                    2.0,
                    AXIS_COLOR,
                );
                d.draw_line_ex(
                    Vector2::new(0.0, origin_y),
                    Vector2::new(screen_width as f32, origin_y),
                    2.0,
                    AXIS_COLOR,
                );
            }
        }

        if let Some(band) = band_rect {
            d.draw_rectangle_rec(band, Color::new(250, 210, 60, 40));
            d.draw_rectangle_lines_ex(band, 1.5, SELECTION_COLOR);
        }

        d.draw_rectangle(0, 0, screen_width, toolbar.height.ceil() as i32, TOOLBAR_BG);
        for button in &toolbar.buttons {
            let hovered = button.enabled && point_in_rect(mouse, button.rect);
            let bg = if !button.enabled {
                BUTTON_DISABLED_BG
            } else if matches!(button.action, ToolbarAction::Delete) {
                if hovered {
                    DANGER_HOVER_BG
                } else {
                    DANGER_BG
                }
            } else if hovered {
                BUTTON_HOVER_BG
            } else {
                BUTTON_BG
            };
            let text_color = if button.enabled {
                BUTTON_TEXT
            } else {
                BUTTON_DISABLED_TEXT
            };
            d.draw_rectangle_rec(button.rect, bg);
            ui::draw_text_in(
                &mut d,
                &button.label,
                button.rect,
                toolbar.pad_x,
                toolbar.font_size,
                text_color,
            );
        }
        if let Some((text, position)) = &toolbar.depth_label {
            ui::draw_text(
                &mut d,
                text,
                position.x,
                position.y,
                toolbar.font_size,
                BUTTON_TEXT,
            );
        }
        let mut status_text = self.status.clone();
        let pending_thumbnails = self.thumbnails.pending_count();
        if status_text.is_none() && pending_thumbnails > 0 {
            status_text = Some(format!("Loading {pending_thumbnails} thumbnail(s)…"));
        }
        if !self.downloads.is_empty() {
            let summary = self
                .downloads
                .iter()
                .map(|pending| pending.download.describe())
                .collect::<Vec<_>>()
                .join("  |  ");
            status_text = Some(match status_text {
                Some(status) => format!("{summary}  —  {status}"),
                None => summary,
            });
        }
        let bottom_inset = self.bottom_inset(screen_width, screen_height);
        if self.player.visible {
            let bar = music::bar_layout(&self.player, screen_width, screen_height);
            let preview = match self.player_drag {
                Some(PlayerDrag::Seek(fraction)) => Some(fraction),
                _ => None,
            };
            music::draw_bar(&mut d, &self.player, &bar, mouse, preview);
            if self.player_drag.is_none() && self.plugin_panel.is_none() && !self.help_open {
                if let Some((control, rect, _)) = bar
                    .buttons
                    .iter()
                    .find(|(_, rect, _)| point_in_rect(mouse, *rect))
                {
                    panels::draw_tooltip(
                        &mut d,
                        control.tooltip(),
                        *rect,
                        screen_width,
                        screen_height,
                    );
                }
            }
            if self.player.playlist_open {
                let list = music::playlist_layout(
                    &self.player,
                    screen_width,
                    screen_height,
                    toolbar.height,
                );
                music::draw_playlist(&mut d, &self.player, &list, mouse);
            }
        }
        if let Some(status) = &status_text {
            let size = 16.0 * scale;
            let pad = 8.0 * scale;
            let bar_height = size + pad * 2.0;
            let bottom = screen_height as f32 - bottom_inset;
            let text = ui::fit(status, size, screen_width as f32 - pad * 2.0);
            let width = ui::measure(&text, size) + pad * 2.0;
            d.draw_rectangle_rec(
                Rectangle::new(
                    0.0,
                    bottom - bar_height,
                    width.min(screen_width as f32),
                    bar_height,
                ),
                Color::new(12, 12, 16, 220),
            );
            ui::draw_text(
                &mut d,
                &text,
                pad,
                bottom - bar_height + pad,
                size,
                VIEWER_TEXT,
            );
        }

        let overlay_top = toolbar.height;
        if matches!(self.mode, Mode::Main | Mode::Inventory) && !self.console_open {
            self.draw_minimap(&mut d, &search_cells);
            self.draw_search_bar(&mut d, overlay_top, time);
            let info_top = if self.search.is_some() {
                overlay_top + 2.0 * (17.0 * scale) + 9.0 * scale * 3.2
            } else {
                overlay_top
            };
            self.draw_info_panel(&mut d, info_top);
        }

        if self.console_open {
            draw_console(
                &mut d,
                screen_width,
                screen_height,
                toolbar.height,
                &self.console_history,
                &self.console_input,
                self.console_scroll,
            );
        }

        if let Some(&(col, row)) = self.pending_delete.first() {
            draw_confirm_dialog(
                &mut d,
                screen_width,
                screen_height,
                col,
                row,
                self.pending_delete.len(),
                mouse,
            );
        }

        if let Some(drag) = self.drag.as_ref().filter(|drag| drag.active) {
            let size = 15.0 * scale;
            let label = if drag.group {
                let count = self.selection.cells().count();
                let blocked = self.group_drop_blocked(hovered_cell_for_drag);
                if blocked {
                    format!("move {count} cells — blocked")
                } else {
                    format!("move {count} cells")
                }
            } else if shift_down_draw {
                format!("stack ({}, {}) onto target", drag.from.0, drag.from.1)
            } else {
                format!("move ({}, {})", drag.from.0, drag.from.1)
            };
            let width = ui::measure(&label, size) + 16.0 * scale;
            let ghost = Rectangle::new(
                mouse.x + 14.0 * scale,
                mouse.y + 10.0 * scale,
                width,
                size + 12.0 * scale,
            );
            d.draw_rectangle_rec(ghost, Color::new(250, 210, 60, 220));
            ui::draw_text_centered(&mut d, &label, ghost, size, Color::new(20, 20, 26, 255));
        }

        if let Some((text, rect)) = tooltip {
            if self.profile_panel.is_none()
                && self.item_panel.is_none()
                && !self.help_open
                && self.plugin_panel.is_none()
            {
                panels::draw_tooltip(&mut d, text, rect, screen_width, screen_height);
            }
        }

        self.draw_context_menu(&mut d, mouse);

        if let Some(panel) = &self.profile_panel {
            let layout = panel.layout(screen_width, screen_height, self.profile.as_deref());
            let base = self.profiles.base().display().to_string();
            panel.draw(&mut d, &layout, screen_width, screen_height, mouse, &base);
        }

        if let Some(panel) = &self.item_panel {
            let inventory = lock(&self.grid).inventory().clone();
            let layout = panel.layout(&inventory, screen_width, screen_height);
            let title = self.item_panel_title();
            panel.draw(
                &mut d,
                &layout,
                &inventory,
                &title,
                screen_width,
                screen_height,
                mouse,
            );
        }

        if self.plugin_panel.is_some() {
            let layout = self.plugin_panel_layout(screen_width, screen_height);
            layout.draw(&mut d, screen_width, screen_height, mouse);
        }

        if self.help_open {
            panels::draw_help(&mut d, screen_width, screen_height, self.help_scroll, mouse);
        }
    }

    fn draw_scene_3d(
        &self,
        d: &mut RaylibDrawHandle<'_>,
        hover: Option<(i64, i64)>,
        downloads: &[((i64, i64), Option<f32>)],
        search_cells: &[(i64, i64)],
        time: f64,
        toolbar_height: f32,
    ) {
        let screen_width = d.get_screen_width();
        let screen_height = d.get_screen_height();
        let scale = ui::scale(screen_width, screen_height);
        d.draw_rectangle_gradient_v(
            0,
            0,
            screen_width,
            screen_height,
            Color::new(14, 16, 30, 255),
            Color::new(46, 40, 62, 255),
        );

        let camera = self.orbit.camera();
        let eye = self.orbit.position();
        let target = self.orbit.target();
        let (min_col, max_col, min_row, max_row) = self.orbit.visible_span();
        let radius = ((max_col - min_col) as f32 / 2.0).max(1.0);
        let cells: Vec<(i64, i64, CellKind, Option<PathBuf>, Vec<CellKind>)> = {
            let blocks = self.blocks_3d();
            let grid = lock(&self.grid);
            blocks
                .into_iter()
                .map(|(col, row, kind)| {
                    let under: Vec<CellKind> = grid
                        .items_at(col, row)
                        .iter()
                        .skip(1)
                        .take(5)
                        .map(|item| item.kind())
                        .collect();
                    (
                        col,
                        row,
                        kind,
                        grid.file_at(col, row).map(PathBuf::from),
                        under,
                    )
                })
                .collect()
        };
        // Fade distant geometry into the background like fog.
        let fade = |x: f32, z: f32| {
            let distance = ((x - target.x).powi(2) + (z - target.z).powi(2)).sqrt();
            (1.0 - distance / (radius + 1.0)).clamp(0.15, 1.0)
        };
        let with_alpha = |color: Color, alpha: f32| {
            Color::new(color.r, color.g, color.b, (color.a as f32 * alpha) as u8)
        };

        {
            let mut d3 = d.begin_mode3D(camera);
            d3.draw_plane(
                Vector3::new(target.x, -0.02, target.z),
                Vector2::new(radius * 2.0 + 2.0, radius * 2.0 + 2.0),
                Color::new(24, 24, 32, 255),
            );
            for col in min_col..=max_col + 1 {
                let x = col as f32;
                let color = with_alpha(GRID_LINE, fade(x, target.z));
                d3.draw_line3D(
                    Vector3::new(x, 0.0, min_row as f32),
                    Vector3::new(x, 0.0, (max_row + 1) as f32),
                    color,
                );
            }
            for row in min_row..=max_row + 1 {
                let z = row as f32;
                let color = with_alpha(GRID_LINE, fade(target.x, z));
                d3.draw_line3D(
                    Vector3::new(min_col as f32, 0.0, z),
                    Vector3::new((max_col + 1) as f32, 0.0, z),
                    color,
                );
            }
            if (min_row..=max_row + 1).contains(&0) {
                d3.draw_line3D(
                    Vector3::new(min_col as f32, 0.005, 0.0),
                    Vector3::new((max_col + 1) as f32, 0.005, 0.0),
                    Color::new(210, 95, 95, 255),
                );
            }
            if (min_col..=max_col + 1).contains(&0) {
                d3.draw_line3D(
                    Vector3::new(0.0, 0.005, min_row as f32),
                    Vector3::new(0.0, 0.005, (max_row + 1) as f32),
                    Color::new(95, 140, 220, 255),
                );
            }
            d3.draw_line3D(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 1.2, 0.0),
                Color::new(110, 210, 130, 255),
            );

            for (col, row, kind, path, under) in &cells {
                let (x, z) = (*col as f32, *row as f32);
                let alpha = fade(x + 0.5, z + 0.5);
                // Items underneath the top of a stack show as thin plates
                // beside the block, one per hidden item.
                for (layer, under_kind) in under.iter().enumerate() {
                    let y = 0.03 + layer as f32 * 0.07;
                    d3.draw_cube(
                        Vector3::new(x + 0.5, y, z + 0.5),
                        0.99,
                        0.05,
                        0.99,
                        with_alpha(kind_color(*under_kind), alpha * 0.8),
                    );
                }
                let height = block_height(*kind) + under.len() as f32 * 0.07;
                let center = Vector3::new(x + 0.5, height / 2.0, z + 0.5);
                let color = kind_color(*kind);
                match kind {
                    CellKind::Image => {
                        d3.draw_cube(center, 0.94, height, 0.94, with_alpha(color, alpha));
                        match path.as_ref().and_then(|path| self.thumbnails.get(path)) {
                            Some(texture) => view3d::draw_texture_flat(
                                &mut d3,
                                texture,
                                x + 0.04,
                                z + 0.04,
                                0.92,
                                height + 0.003,
                            ),
                            None => view3d::draw_square_outline(
                                &mut d3,
                                x + 0.2,
                                z + 0.2,
                                0.6,
                                height + 0.003,
                                with_alpha(BUTTON_TEXT, alpha),
                            ),
                        }
                    }
                    CellKind::Grid => {
                        let body = Color::new(40, 80, 64, 255);
                        d3.draw_cube(center, 0.9, height, 0.9, with_alpha(body, alpha));
                        d3.draw_cube_wires(
                            center,
                            0.9,
                            height,
                            0.9,
                            with_alpha(NESTED_GRID_COLOR, alpha),
                        );
                        // A 3×3 lattice on top marks it as a grid you can enter.
                        for sub_row in 0..3 {
                            for sub_col in 0..3 {
                                view3d::draw_square_outline(
                                    &mut d3,
                                    x + 0.08 + sub_col as f32 * 0.28,
                                    z + 0.08 + sub_row as f32 * 0.28,
                                    0.26,
                                    height + 0.003,
                                    with_alpha(NESTED_GRID_COLOR, alpha),
                                );
                            }
                        }
                    }
                    _ => {
                        d3.draw_cube(center, 0.82, height, 0.82, with_alpha(color, alpha));
                        d3.draw_cube_wires(
                            center,
                            0.82,
                            height,
                            0.82,
                            with_alpha(Color::new(235, 235, 245, 120), alpha),
                        );
                    }
                }
            }

            let raised = |col: i64, row: i64| {
                cells
                    .iter()
                    .find(|(c, r, ..)| (*c, *r) == (col, row))
                    .map_or(0.0, |(_, _, kind, _, under)| {
                        block_height(*kind) + under.len() as f32 * 0.07
                    })
            };
            if let Some((col, row)) = hover {
                view3d::draw_square_outline(
                    &mut d3,
                    col as f32,
                    row as f32,
                    1.0,
                    raised(col, row) + 0.01,
                    Color::new(200, 205, 235, 220),
                );
            }
            for &(col, row) in search_cells {
                view3d::draw_square_outline(
                    &mut d3,
                    col as f32 + 0.06,
                    row as f32 + 0.06,
                    0.88,
                    raised(col, row) + 0.02,
                    SEARCH_COLOR,
                );
            }
            if let Some((col, row)) = self.selected {
                for (sel_col, sel_row) in self.selection.cells() {
                    if (sel_col, sel_row) != (col, row) {
                        view3d::draw_square_outline(
                            &mut d3,
                            sel_col as f32 + 0.03,
                            sel_row as f32 + 0.03,
                            0.94,
                            raised(sel_col, sel_row) + 0.03,
                            Color::new(250, 210, 60, 200),
                        );
                    }
                }
                let height = raised(col, row) + 0.12;
                d3.draw_cube_wires(
                    Vector3::new(col as f32 + 0.5, height / 2.0, row as f32 + 0.5),
                    1.02,
                    height,
                    1.02,
                    SELECTION_COLOR,
                );
                d3.draw_cube(
                    Vector3::new(col as f32 + 0.5, 0.004, row as f32 + 0.5),
                    1.0,
                    0.008,
                    1.0,
                    Color::new(250, 210, 60, 70),
                );
            }
            for ((col, row), fraction) in downloads {
                let level = fraction.unwrap_or(((time * 1.5).sin() as f32 + 1.0) / 2.0);
                let height = 0.05 + level * 0.9;
                d3.draw_cube_wires(
                    Vector3::new(*col as f32 + 0.5, height / 2.0, *row as f32 + 0.5),
                    0.96,
                    height,
                    0.96,
                    Color::new(120, 220, 255, 255),
                );
            }
        }

        // Screen-space labels, drawn far-to-near so close ones stay on top.
        let forward = (target - eye).normalize();
        let mut labels: Vec<(f32, Vector2, f32, String, Color)> = Vec::new();
        let names = if self.labels() { cells.as_slice() } else { &[] };
        for (col, row, kind, path, under) in names {
            let top = Vector3::new(
                *col as f32 + 0.5,
                block_height(*kind) + under.len() as f32 * 0.07 + 0.02,
                *row as f32 + 0.5,
            );
            let depth = (top - eye).dot(forward);
            if depth <= 0.1 {
                continue;
            }
            let size = (self.orbit.pixels_per_unit(top, screen_height as f32) * 0.14).min(34.0);
            if size < 10.0 {
                continue;
            }
            let text = match (kind, path) {
                (CellKind::Grid, _) => "grid".to_owned(),
                (_, Some(path)) => path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file")
                    .to_owned(),
                _ => kind.label().to_owned(),
            };
            let text = if under.is_empty() {
                text
            } else {
                format!("{text} ×{}", under.len() + 1)
            };
            let point = d.get_world_to_screen(top, camera);
            labels.push((depth, point, size, text, BUTTON_TEXT));
        }
        for (col, row) in self.selected.into_iter().chain(hover) {
            let point = Vector3::new(col as f32 + 0.5, 0.0, row as f32 + 1.0);
            if (point - eye).dot(forward) > 0.1 {
                let screen = d.get_world_to_screen(point, camera);
                labels.push((
                    0.0,
                    Vector2::new(screen.x, screen.y + 14.0 * scale),
                    15.0 * scale,
                    format!("{col},{row}"),
                    Color::new(250, 225, 140, 255),
                ));
            }
        }
        labels.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, point, size, text, color) in labels.iter().take(400) {
            if point.y < toolbar_height {
                continue;
            }
            let text = ui::fit(text, *size, size * 9.0);
            let width = ui::measure(&text, *size) + size * 0.6;
            let rect = Rectangle::new(
                point.x - width / 2.0,
                point.y - size * 1.5,
                width,
                size * 1.3,
            );
            d.draw_rectangle_rec(rect, Color::new(12, 12, 18, 170));
            ui::draw_text_centered(d, &text, rect, *size, *color);
        }

        for ((col, row), fraction) in downloads {
            let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(dx, dz)| {
                d.get_world_to_screen(
                    Vector3::new(*col as f32 + dx, 0.0, *row as f32 + dz),
                    camera,
                )
            });
            let min_x = corners.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
            let max_x = corners
                .iter()
                .map(|p| p.x)
                .fold(f32::NEG_INFINITY, f32::max);
            let min_y = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
            let max_y = corners
                .iter()
                .map(|p| p.y)
                .fold(f32::NEG_INFINITY, f32::max);
            if max_x - min_x > 8.0 && max_x - min_x < screen_width as f32 {
                let rect = Rectangle::new(min_x, min_y, max_x - min_x, max_y - min_y);
                draw_download_progress(d, rect, *fraction, time);
            }
        }

        let hud = format!(
            "3D  ·  yaw {:.0}°  pitch {:.0}°  ·  {} block(s)",
            self.orbit.yaw.to_degrees(),
            self.orbit.pitch.to_degrees(),
            cells.len()
        );
        let size = 14.0 * scale;
        let pad = 8.0 * scale;
        let width = ui::measure(&hud, size) + pad * 2.0;
        let rect = Rectangle::new(
            screen_width as f32 - width - pad,
            toolbar_height + pad,
            width,
            size + pad * 1.4,
        );
        d.draw_rectangle_rec(rect, Color::new(12, 12, 18, 190));
        ui::draw_text_centered(d, &hud, rect, size, VIEWER_TEXT);
    }

    fn draw_cell(&self, d: &mut RaylibDrawHandle<'_>, col: i64, row: i64, rect: Rectangle) {
        let content = {
            let grid = lock(&self.grid);
            match grid.kind_at(col, row) {
                Some(CellKind::Grid) => Some(DrawCell::Grid),
                Some(kind) => grid
                    .file_at(col, row)
                    .map(PathBuf::from)
                    .map(|path| DrawCell::File(kind, path)),
                None => None,
            }
        };

        match content {
            Some(DrawCell::File(kind, path)) => {
                if kind == CellKind::Image {
                    if let Some(texture) = self.thumbnails.get(&path) {
                        draw_texture_fit(d, texture, rect);
                    } else if self.thumbnails.is_pending(&path) && rect.height > 24.0 {
                        let size = ui::cell_text_size(rect.height, 0.12, 28.0);
                        ui::draw_text_centered(
                            d,
                            "loading…",
                            rect,
                            size,
                            Color::new(150, 150, 170, 255),
                        );
                    } else if self.thumbnails.failure(&path).is_some() && rect.height > 24.0 {
                        let size = ui::cell_text_size(rect.height, 0.12, 28.0);
                        ui::draw_text_centered(d, "unreadable", rect, size, DANGER_HOVER_BG);
                    }
                }
                draw_file_badge(d, rect, kind, &path, self.labels());
                if self.player.playlist.current_path() == Some(path.as_path())
                    && self.player.state() != music::State::Stopped
                {
                    let thickness = (rect.height * 0.035).clamp(1.5, 6.0);
                    let inset = thickness * 2.5;
                    d.draw_rectangle_lines_ex(
                        Rectangle::new(
                            rect.x + inset,
                            rect.y + inset,
                            rect.width - inset * 2.0,
                            rect.height - inset * 2.0,
                        ),
                        thickness,
                        NOW_PLAYING_COLOR,
                    );
                }
            }
            Some(DrawCell::Grid) => draw_nested_grid_icon(d, rect),
            None => {}
        }
    }

    fn toolbar_layout(&self, rl: &RaylibHandle) -> ToolbarLayout {
        // `New Grid` needs an empty selected cell; `Delete` needs an
        // occupied one. Both are disabled with no selection at all.
        let selection_occupied = self
            .selected
            .map(|(col, row)| lock(&self.grid).occupied(col, row));

        let specs: Vec<(String, ToolbarAction, bool)> = match self.mode {
            Mode::Main => {
                let profile_label = match &self.profile {
                    Some(name) => format!("Profile: {}", ui::fit(name, 20.0, 160.0)),
                    None => "Profile: (file)".to_owned(),
                };
                let mut items = vec![
                    (profile_label, ToolbarAction::Profile, true),
                    ("Save".to_owned(), ToolbarAction::Save, true),
                    (
                        "New Grid".to_owned(),
                        ToolbarAction::NewGrid,
                        selection_occupied == Some(false),
                    ),
                    (
                        "Delete".to_owned(),
                        ToolbarAction::Delete,
                        selection_occupied == Some(true),
                    ),
                    (
                        "Paste".to_owned(),
                        ToolbarAction::Paste,
                        self.selected.is_some(),
                    ),
                    (
                        "Copy".to_owned(),
                        ToolbarAction::Copy,
                        selection_occupied == Some(true),
                    ),
                    (
                        "Open / Run".to_owned(),
                        ToolbarAction::OpenRun,
                        selection_occupied == Some(true),
                    ),
                    (
                        "PA Export".to_owned(),
                        ToolbarAction::PaExport,
                        cfg!(feature = "scripting"),
                    ),
                    (
                        "PA Import".to_owned(),
                        ToolbarAction::PaImport,
                        cfg!(feature = "scripting"),
                    ),
                    ("File List".to_owned(), ToolbarAction::OpenInventory, true),
                    ("Items".to_owned(), ToolbarAction::Items, true),
                    ("Grid Folder".to_owned(), ToolbarAction::GridFolder, true),
                    (
                        "Undo".to_owned(),
                        ToolbarAction::Undo,
                        self.history.can_undo(),
                    ),
                    (
                        "Redo".to_owned(),
                        ToolbarAction::Redo,
                        self.history.can_redo(),
                    ),
                    ("Find".to_owned(), ToolbarAction::Find, true),
                    ("Ruby".to_owned(), ToolbarAction::Console, true),
                    ("Home".to_owned(), ToolbarAction::Home, true),
                    (
                        if self.view_3d { "2D View" } else { "3D View" }.to_owned(),
                        ToolbarAction::View3d,
                        true,
                    ),
                    (
                        format!("Labels: {}", if self.labels() { "On" } else { "Off" }),
                        ToolbarAction::Labels,
                        true,
                    ),
                    (
                        format!(
                            "Copy In: {}",
                            if self.settings.copy_files {
                                "On"
                            } else {
                                "Off"
                            }
                        ),
                        ToolbarAction::CopyFiles,
                        true,
                    ),
                    (
                        if self.player.visible {
                            "Hide Music"
                        } else {
                            "Music"
                        }
                        .to_owned(),
                        ToolbarAction::Music,
                        true,
                    ),
                    (
                        format!(
                            "Plugins ({})",
                            self.plugins
                                .iter()
                                .filter(|plugin| plugin.enabled && plugin.error.is_none())
                                .count()
                        ),
                        ToolbarAction::Plugins,
                        true,
                    ),
                    ("Help".to_owned(), ToolbarAction::Help, true),
                ];
                if !self.downloads.is_empty() {
                    items.push((
                        format!("Cancel DL ({})", self.downloads.len()),
                        ToolbarAction::CancelDownloads,
                        true,
                    ));
                }
                if !self.nav_stack.is_empty() {
                    items.push(("Back".to_owned(), ToolbarAction::Back, true));
                }
                items
            }
            Mode::Inventory => {
                let mut items = vec![
                    ("Refresh".to_owned(), ToolbarAction::Refresh, true),
                    ("Find".to_owned(), ToolbarAction::Find, true),
                    (
                        if self.view_3d { "2D View" } else { "3D View" }.to_owned(),
                        ToolbarAction::View3d,
                        true,
                    ),
                    (
                        format!("Labels: {}", if self.labels() { "On" } else { "Off" }),
                        ToolbarAction::Labels,
                        true,
                    ),
                    ("Music".to_owned(), ToolbarAction::Music, true),
                ];
                if !self.nav_stack.is_empty() {
                    items.push(("Back".to_owned(), ToolbarAction::Back, true));
                }
                items
            }
            Mode::ImageViewer(_) => Vec::new(),
        };

        let screen_width = rl.get_screen_width();
        let scale = ui::scale(screen_width, rl.get_screen_height());
        let font_size = 20.0 * scale;
        let pad_x = 9.0 * scale;
        let item_height = font_size + 14.0 * scale;
        let margin = 8.0 * scale;
        let spacing = 6.0 * scale;

        let depth_text = (!self.nav_stack.is_empty())
            .then(|| ui::fit(&self.breadcrumb(), font_size, screen_width as f32 * 0.5));
        let mut widths: Vec<f32> = specs
            .iter()
            .map(|(label, _, _)| ui::measure(label, font_size) + pad_x * 2.0)
            .collect();
        if let Some(text) = &depth_text {
            widths.push(ui::measure(text, font_size) + pad_x);
        }
        let (mut rects, height) =
            wrap_toolbar_items(&widths, screen_width as f32, item_height, margin, spacing);
        let depth_label = depth_text.map(|text| {
            let rect = rects.pop().expect("depth label was laid out");
            let position = Vector2::new(rect.x, rect.y + (rect.height - font_size) / 2.0);
            (text, position)
        });

        let buttons = specs
            .into_iter()
            .zip(rects)
            .map(|((label, action, enabled), rect)| ToolbarButton {
                label,
                rect,
                action,
                enabled,
            })
            .collect();
        ToolbarLayout {
            buttons,
            height,
            font_size,
            pad_x,
            depth_label,
        }
    }

    fn viewport(&self, rl: &RaylibHandle) -> [f64; 2] {
        [rl.get_screen_width() as f64, rl.get_screen_height() as f64]
    }
}

enum ActivatedCell {
    File(CellKind, PathBuf),
    Grid(Arc<Mutex<SavedGrid>>),
}

enum DrawCell {
    File(CellKind, PathBuf),
    Grid,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<OsString> = env::args_os().collect();
    match parse_mode(&args)? {
        StartupMode::Usage => {
            println!("{}", usage());
            Ok(())
        }
        StartupMode::ListProfiles => {
            let store = ProfileStore::open_default();
            let last = store.last_used();
            println!("Profiles in {}:", store.base().display());
            for name in store.list() {
                let marker = if last.as_deref() == Some(name.as_str()) {
                    "*"
                } else {
                    " "
                };
                println!(" {marker} {name}");
            }
            Ok(())
        }
        StartupMode::Image(path) => run_image_viewer(&path),
        StartupMode::Inventory(json_path) => inventory::InventoryApp::new(json_path)?.run(),
        StartupMode::Script(script_path, target) => run_script_file(&script_path, target),
        StartupMode::Game(script_path, target) => run_game_file(&script_path, target),
        StartupMode::Grid(json_path) => App::new(Mode::Main, json_path)?.run(),
        StartupMode::Profile(requested) => {
            let store = ProfileStore::open_default();
            let profile = store.startup_profile(requested.as_deref())?;
            App::with_profile(store, profile)?.run()
        }
    }
}

/// Where a session's grids come from: a named local profile (the default)
/// or an explicit grid file.
#[derive(Debug, PartialEq, Eq)]
enum Target {
    Profile(Option<String>),
    Grid(PathBuf),
}

#[derive(Debug, PartialEq, Eq)]
enum StartupMode {
    Profile(Option<String>),
    Grid(PathBuf),
    Inventory(PathBuf),
    Image(PathBuf),
    Script(PathBuf, Target),
    Game(PathBuf, Target),
    ListProfiles,
    Usage,
}

fn usage() -> String {
    let lines = [
        "Usage:",
        "  selenite                         open the last-used profile (\"{default}\" on first run)",
        "  selenite --profile NAME          open (or create) the profile NAME",
        "  selenite --grid FILE.json        edit a grid file directly, outside any profile",
        "  selenite --list-profiles         list local profiles",
        "  selenite --script FILE.rb [--profile NAME | --grid FILE.json | FILE.json]",
        "                                   run a Ruby script headless against a profile or grid",
        "  selenite --game FILE.rb [--profile NAME | --grid FILE.json | FILE.json]",
        "                                   run a Ruby game (Raylib API) with access to the grid",
        "  selenite --image FILE            open the image viewer",
        "  selenite --inventory FILE.json   open the inventory window",
    ];
    format!(
        "Selenite {} — infinite nested grids for any file\n\n{}\n\nProfiles live in {} (override with SELENITE_HOME).",
        env!("CARGO_PKG_VERSION"),
        lines.join("\n").replace("{default}", profiles::DEFAULT_PROFILE),
        profiles::default_base_dir().display()
    )
}

fn parse_mode(args: &[OsString]) -> Result<StartupMode, String> {
    let rest: Vec<&str> = args
        .iter()
        .skip(1)
        .map(|arg| {
            arg.to_str()
                .ok_or_else(|| format!("argument is not valid UTF-8: {arg:?}"))
        })
        .collect::<Result<_, _>>()?;
    let value = |index: usize, flag: &str| -> Result<String, String> {
        rest.get(index)
            .map(|value| (*value).to_owned())
            .ok_or_else(|| format!("{flag} needs a value\n\n{}", usage()))
    };
    let target = |index: usize| -> Result<Target, String> {
        match rest.get(index).copied() {
            None => Ok(Target::Profile(None)),
            Some("--profile") => Ok(Target::Profile(Some(value(index + 1, "--profile")?))),
            Some("--grid") => Ok(Target::Grid(PathBuf::from(value(index + 1, "--grid")?))),
            Some(path) => Ok(Target::Grid(PathBuf::from(path))),
        }
    };
    Ok(match rest.first().copied() {
        None => StartupMode::Profile(None),
        Some("-h" | "--help") => StartupMode::Usage,
        Some("--list-profiles") => StartupMode::ListProfiles,
        Some("--profile") => StartupMode::Profile(Some(value(1, "--profile")?)),
        Some("--grid") => StartupMode::Grid(PathBuf::from(value(1, "--grid")?)),
        Some("--image") => StartupMode::Image(PathBuf::from(value(1, "--image")?)),
        Some("--inventory") => StartupMode::Inventory(PathBuf::from(value(1, "--inventory")?)),
        Some("--script") => StartupMode::Script(PathBuf::from(value(1, "--script")?), target(2)?),
        Some("--game") => StartupMode::Game(PathBuf::from(value(1, "--game")?), target(2)?),
        Some(other) if other.ends_with(".json") => StartupMode::Grid(PathBuf::from(other)),
        Some(other) => return Err(format!("unknown argument: {other}\n\n{}", usage())),
    })
}

#[cfg(feature = "scripting")]
fn script_engine_for(
    target: Target,
) -> Result<(scripting::ScriptEngine, Arc<Mutex<SavedGrid>>, PathBuf), String> {
    let (json_path, profile, store) = match target {
        Target::Grid(path) => (path, None, ProfileStore::open_default()),
        Target::Profile(requested) => {
            let store = ProfileStore::open_default();
            let profile = store.startup_profile(requested.as_deref())?;
            (profile.grid_path(), Some(profile.name), store)
        }
    };
    let root = load_or_new_root(&json_path)?;
    let engine = scripting::ScriptEngine::new()?;
    let mut context =
        scripting::ScriptContext::new(Arc::clone(&root), Arc::clone(&root), json_path.clone());
    context.profile = profile;
    context.profiles = Some(store);
    engine.set_context(context);
    Ok((engine, root, json_path))
}

/// Runs a Ruby game: the Raylib module may open its own window. The grid is
/// only written when the game saves (`Selenite.save` / `Game autosave:`).
#[cfg(feature = "scripting")]
fn run_game_file(script_path: &Path, target: Target) -> Result<(), String> {
    let (engine, _root, _json_path) = script_engine_for(target)?;
    game::enable();
    let (printed, result) = engine.eval_file_captured(script_path);
    game::shutdown();
    print!("{printed}");
    for request in engine.drain_requests() {
        if let scripting::AppRequest::Status(text) = request {
            println!("{text}");
        }
    }
    if result.starts_with("error") {
        return Err(result);
    }
    Ok(())
}

#[cfg(not(feature = "scripting"))]
fn run_game_file(_script_path: &Path, _target: Target) -> Result<(), String> {
    Err("Ruby games need the scripting-enabled build".to_owned())
}

#[cfg(feature = "scripting")]
fn run_script_file(script_path: &Path, target: Target) -> Result<(), String> {
    let (engine, root, json_path) = script_engine_for(target)?;
    let (printed, result) = engine.eval_file_captured(script_path);
    print!("{printed}");
    if result.starts_with("error") {
        return Err(result);
    }
    for request in engine.drain_requests() {
        if let scripting::AppRequest::Status(text) = request {
            println!("{text}");
        }
    }
    let saved = lock(&root)
        .save(&json_path)
        .map_err(|error| error.to_string());
    saved
}

#[cfg(not(feature = "scripting"))]
fn run_script_file(_script_path: &Path, _target: Target) -> Result<(), String> {
    Err("Ruby scripting is unavailable; install the scripting-enabled build".to_owned())
}

enum ViewerImage {
    Decoding(std::sync::mpsc::Receiver<Result<image::RgbaImage, String>>),
    Ready(TiledImage),
    Failed(String),
}

fn run_image_viewer(path: &Path) -> Result<(), String> {
    let path = path.to_path_buf();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image")
        .to_owned();
    // Header-only read: instant even for gigapixel files.
    let (width, height) = imaging::dimensions(&path)?;

    let (mut rl, thread) = raylib::init()
        .size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .title(&format!("Selenite Viewer - {name} ({width}×{height})"))
        .resizable()
        .build();
    rl.set_target_fps(60);
    ui::load_font(&thread);

    let monitor = raylib::core::window::get_current_monitor();
    let max_w = (raylib::core::window::get_monitor_width(monitor) - 80).max(320);
    let max_h = (raylib::core::window::get_monitor_height(monitor) - 80).max(240);
    rl.set_window_size(
        (width.min(i32::MAX as u32) as i32).clamp(320, max_w),
        (height.min(i32::MAX as u32) as i32).clamp(240, max_h),
    );

    let (sender, receiver) = std::sync::mpsc::channel();
    {
        let path = path.clone();
        std::thread::spawn(move || {
            let _ = sender.send(imaging::decode_full(&path));
        });
    }
    let mut state = ViewerImage::Decoding(receiver);

    let fit_camera = |sw: f64, sh: f64| {
        let scale = ((sw - VIEWER_MARGIN as f64 * 2.0) / width as f64)
            .min((sh - VIEWER_MARGIN as f64 * 2.0) / height as f64);
        let mut camera = Camera::new();
        let _ = camera.zoom_at([sw / 2.0, sh / 2.0], [sw, sh], scale.max(1e-6));
        camera
    };
    let mut camera = Camera::new();
    let mut fitted = false;
    let mut panning = false;

    while !rl.window_should_close() {
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            break;
        }
        ui::handle_scale_keys(&rl);
        let (sw, sh) = (rl.get_screen_width() as f64, rl.get_screen_height() as f64);
        if !fitted {
            // Start fitted when the image is larger than the window.
            if width as f64 > sw - VIEWER_MARGIN as f64 * 2.0
                || height as f64 > sh - VIEWER_MARGIN as f64 * 2.0
            {
                camera = fit_camera(sw, sh);
            }
            fitted = true;
        }

        if let ViewerImage::Decoding(receiver) = &state {
            if let Ok(result) = receiver.try_recv() {
                state = match result.and_then(|image| TiledImage::upload(&mut rl, &thread, &image))
                {
                    Ok(tiled) => ViewerImage::Ready(tiled),
                    Err(error) => ViewerImage::Failed(error),
                };
            }
        }

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_MIDDLE)
            || rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT)
            || rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
        {
            panning = true;
        }
        if !(rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_MIDDLE)
            || rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT)
            || rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT))
        {
            panning = false;
        }
        if panning {
            let delta = rl.get_mouse_delta();
            if delta.x != 0.0 || delta.y != 0.0 {
                camera.pan(delta.x as f64, delta.y as f64);
            }
        }

        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            let factor = 1.0 + (wheel * ZOOM_STEP);
            if factor > 0.0 {
                let mouse = rl.get_mouse_position();
                let _ = camera.zoom_at([mouse.x as f64, mouse.y as f64], [sw, sh], factor as f64);
            }
        }
        for (key, factor) in [
            (KeyboardKey::KEY_EQUAL, 1.25),
            (KeyboardKey::KEY_KP_ADD, 1.25),
            (KeyboardKey::KEY_MINUS, 0.8),
            (KeyboardKey::KEY_KP_SUBTRACT, 0.8),
        ] {
            let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
                || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
            if !ctrl && rl.is_key_pressed(key) {
                let _ = camera.zoom_at([sw / 2.0, sh / 2.0], [sw, sh], factor);
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_R) || rl.is_key_pressed(KeyboardKey::KEY_ONE) {
            camera = Camera::new();
        } else if rl.is_key_pressed(KeyboardKey::KEY_F) {
            camera = fit_camera(sw, sh);
        }

        let screen_width = rl.get_screen_width();
        let screen_height = rl.get_screen_height();
        let viewport = [sw, sh];
        let scale = ui::scale(screen_width, screen_height);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(VIEWER_BACKGROUND);

        let top_left =
            camera.world_to_screen([-(width as f64) / 2.0, -(height as f64) / 2.0], viewport);
        let zoom = camera.zoom();
        let screen = Rectangle::new(0.0, 0.0, screen_width as f32, screen_height as f32);
        match &state {
            ViewerImage::Ready(tiled) => tiled.draw(
                &mut d,
                Vector2::new(top_left[0] as f32, top_left[1] as f32),
                zoom as f32,
                screen,
            ),
            ViewerImage::Decoding(_) => {
                let frame = Rectangle::new(
                    top_left[0] as f32,
                    top_left[1] as f32,
                    (width as f64 * zoom) as f32,
                    (height as f64 * zoom) as f32,
                );
                d.draw_rectangle_lines_ex(frame, 2.0, GRID_LINE);
                ui::draw_text_centered(
                    &mut d,
                    &format!("Decoding {width}×{height}…"),
                    screen,
                    22.0 * scale,
                    VIEWER_TEXT,
                );
            }
            ViewerImage::Failed(error) => ui::draw_text_centered(
                &mut d,
                &format!("Could not open image: {error}"),
                screen,
                18.0 * scale,
                DANGER_HOVER_BG,
            ),
        }

        let zoom_pct = (zoom * 100.0).round() as i64;
        let tiles = match &state {
            ViewerImage::Ready(tiled) if tiled.tile_count() > 1 => format!(
                " | {}×{} in {} GPU tiles",
                tiled.width,
                tiled.height,
                tiled.tile_count()
            ),
            _ => String::new(),
        };
        let info = format!(
            "{name}  {width}×{height}  {zoom_pct}%{tiles}  [Wheel/+/-: Zoom | Drag: Pan | F: Fit | R/1: 100% | Ctrl +/-: Text | Esc: Close]"
        );
        let size = 16.0 * scale;
        let pad = 8.0 * scale;
        let bar_height = size + pad * 2.0;
        let info = ui::fit(&info, size, screen_width as f32 - pad * 2.0);
        d.draw_rectangle_rec(
            Rectangle::new(
                0.0,
                screen_height as f32 - bar_height,
                screen_width as f32,
                bar_height,
            ),
            Color::new(12, 12, 16, 200),
        );
        ui::draw_text(
            &mut d,
            &info,
            pad,
            screen_height as f32 - bar_height + pad,
            size,
            VIEWER_TEXT,
        );
    }

    Ok(())
}

fn load_or_new_root(path: &Path) -> Result<Arc<Mutex<SavedGrid>>, String> {
    Ok(Arc::new(Mutex::new(
        SavedGrid::load(path)
            .map_err(|error| error.to_string())?
            .unwrap_or_else(SavedGrid::new),
    )))
}

fn load_existing_root(path: &Path) -> Result<Arc<Mutex<SavedGrid>>, String> {
    let Some(grid) = SavedGrid::load(path).map_err(|error| error.to_string())? else {
        return Err(format!("No saved grid found at {}", path.display()));
    };
    Ok(Arc::new(Mutex::new(grid)))
}

fn spawn_mode(mode: Mode, json_path: Option<PathBuf>) -> Result<(), String> {
    let exe = env::current_exe().map_err(|error| error.to_string())?;
    let mut command = Command::new(exe);
    match mode {
        Mode::ImageViewer(path) => {
            command.arg("--image").arg(path);
        }
        Mode::Inventory => {
            let path = json_path.ok_or_else(|| "missing inventory path".to_owned())?;
            command.arg("--inventory").arg(path);
        }
        Mode::Main => {}
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn modified_time(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

/// Launcher written into exported game folders: prefers the bundled
/// runtime, then a `selenite` on PATH.
const RUN_GAME_SH: &str = r#"#!/usr/bin/env bash
# Generated by Selenite "Export standalone game".
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"
if [ -x "$DIR/runtime/run.sh" ]; then
    exec "$DIR/runtime/run.sh" --game "$DIR/game.rb" --grid "$DIR/save.json" "$@"
elif [ -x "$DIR/runtime/selenite" ]; then
    exec "$DIR/runtime/selenite" --game "$DIR/game.rb" --grid "$DIR/save.json" "$@"
else
    exec selenite --game "$DIR/game.rb" --grid "$DIR/save.json" "$@"
fi
"#;

/// Writes `<script stem>-game/` next to `script`:
/// - `game.rb`: the script.
/// - `save.json`: a snapshot of the current grid (world data and inventory).
/// - `assets/`: copied if the script has a sibling `assets/` folder.
/// - `runtime/`: the whole portable bundle when Selenite runs from one,
///   otherwise just this binary.
/// - `run-game.sh`: the launcher.
fn export_game(script: &Path, grid_json: &Path) -> Result<PathBuf, String> {
    let stem = script
        .file_stem()
        .ok_or_else(|| "the script has no file name".to_owned())?
        .to_string_lossy()
        .into_owned();
    let parent = script
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let folder = parent.join(format!("{stem}-game"));
    let io = |error: std::io::Error| format!("export failed: {error}");
    fs::create_dir_all(&folder).map_err(io)?;
    fs::copy(script, folder.join("game.rb")).map_err(io)?;
    if grid_json.exists() {
        fs::copy(grid_json, folder.join("save.json")).map_err(io)?;
    }
    let assets = parent.join("assets");
    if assets.is_dir() {
        copy_tree(&assets, &folder.join("assets")).map_err(io)?;
    }
    let exe = env::current_exe().map_err(io)?;
    let runtime = folder.join("runtime");
    fs::create_dir_all(&runtime).map_err(io)?;
    let bundle = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    if bundle.join("run.sh").is_file() && bundle.join("lib").is_dir() {
        fs::copy(bundle.join("run.sh"), runtime.join("run.sh")).map_err(io)?;
        copy_tree(&bundle.join("lib"), &runtime.join("lib")).map_err(io)?;
        for extra in ["licenses", "partitioned_array"] {
            if bundle.join(extra).is_dir() {
                copy_tree(&bundle.join(extra), &runtime.join(extra)).map_err(io)?;
            }
        }
    }
    fs::copy(&exe, runtime.join("selenite")).map_err(io)?;
    let launcher = folder.join("run-game.sh");
    fs::write(&launcher, RUN_GAME_SH).map_err(io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).map_err(io)?;
    }
    Ok(folder)
}

/// Recursive copy that keeps symlinks as symlinks on Unix.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            #[cfg(unix)]
            {
                let _ = fs::remove_file(&target);
                std::os::unix::fs::symlink(fs::read_link(entry.path())?, &target)?;
            }
            #[cfg(not(unix))]
            fs::copy(entry.path(), &target).map(|_| ())?;
        } else if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn content_label(content: &persistence::CellContent) -> String {
    content.label()
}

fn shift_down(rl: &RaylibHandle) -> bool {
    rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT)
}

fn ctrl_down(rl: &RaylibHandle) -> bool {
    [
        KeyboardKey::KEY_LEFT_CONTROL,
        KeyboardKey::KEY_RIGHT_CONTROL,
        KeyboardKey::KEY_LEFT_SUPER,
        KeyboardKey::KEY_RIGHT_SUPER,
    ]
    .into_iter()
    .any(|key| rl.is_key_down(key))
}

/// "just now", "5 min ago", "3 days ago", … for a file timestamp.
fn human_age(time: std::time::SystemTime) -> String {
    let Ok(age) = std::time::SystemTime::now().duration_since(time) else {
        return "just now".to_owned();
    };
    let seconds = age.as_secs();
    let (amount, unit) = match seconds {
        0..60 => return "just now".to_owned(),
        60..3_600 => (seconds / 60, "min"),
        3_600..86_400 => (seconds / 3_600, "hour"),
        86_400..2_592_000 => (seconds / 86_400, "day"),
        2_592_000..31_536_000 => (seconds / 2_592_000, "month"),
        _ => (seconds / 31_536_000, "year"),
    };
    let plural = if amount == 1 || unit == "min" {
        ""
    } else {
        "s"
    };
    format!("{amount} {unit}{plural} ago")
}

fn point_in_rect(point: Vector2, rect: Rectangle) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

fn timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn extension_for_content_type(content_type: &str) -> &'static str {
    match content_type.split(';').next().unwrap_or_default().trim() {
        "image/png" => ".png",
        "image/jpeg" => ".jpg",
        "image/gif" => ".gif",
        "image/webp" => ".webp",
        "audio/mpeg" => ".mp3",
        "audio/ogg" => ".ogg",
        "audio/wav" | "audio/x-wav" => ".wav",
        "video/mp4" => ".mp4",
        "video/webm" => ".webm",
        "text/x-ruby" | "application/x-ruby" => ".rb",
        _ => ".bin",
    }
}

fn open_with_system(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    };

    command
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("could not open {}: {error}", path.display()))
}

/// Decodes an image file with the `image` crate and uploads it as a raylib
/// `Texture2D`.
///
/// Raylib's own loader (stb_image, used by `load_texture`) only supports the
/// handful of formats it was compiled with and can fail on perfectly valid
/// files (e.g. progressive/CMYK JPEGs) with "Data format not supported". The
/// `image` crate has broader, more tolerant decoders, so we decode to a plain
/// RGBA8 buffer ourselves and hand raylib already-decoded pixels instead of a
/// file path.
#[cfg(feature = "scripting")]
use scripting::HookArg as HookValue;

/// Hook arguments; hooks never fire without the scripting feature.
#[cfg(not(feature = "scripting"))]
#[allow(dead_code)]
enum HookValue {
    Int(i64),
    Str(String),
    List(Vec<String>),
    Nil,
}

enum PasteItem {
    Path(PathBuf),
    Url(String),
}

/// Expands a leading `~` to the home directory.
fn expand_home(text: &str) -> PathBuf {
    let home = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"));
    match (text.strip_prefix('~'), home) {
        (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with(['/', '\\']) => {
            PathBuf::from(home).join(rest.trim_start_matches(['/', '\\']))
        }
        _ => PathBuf::from(text),
    }
}

/// Progress bar along the bottom of a cell that is receiving a download;
/// unknown sizes get a sliding indeterminate bar.
fn draw_download_progress(
    d: &mut RaylibDrawHandle<'_>,
    rect: Rectangle,
    fraction: Option<f32>,
    time: f64,
) {
    d.draw_rectangle_rec(rect, Color::new(10, 10, 14, 120));
    let height = (rect.height * 0.12).clamp(3.0, 18.0);
    let track = Rectangle::new(
        rect.x + 2.0,
        rect.y + rect.height - height - 2.0,
        rect.width - 4.0,
        height,
    );
    d.draw_rectangle_rec(track, Color::new(40, 40, 50, 230));
    let fill = match fraction {
        Some(fraction) => Rectangle::new(
            track.x,
            track.y,
            track.width * fraction.clamp(0.0, 1.0),
            track.height,
        ),
        None => {
            let width = track.width * 0.3;
            let phase = (time * 0.8).fract() as f32;
            Rectangle::new(
                track.x + (track.width - width) * phase,
                track.y,
                width,
                track.height,
            )
        }
    };
    d.draw_rectangle_rec(fill, Color::new(90, 190, 255, 255));
    let size = ui::cell_text_size(rect.height, 0.14, 28.0);
    if size >= 9.0 {
        let label = match fraction {
            Some(fraction) => format!("{:.0}%", fraction * 100.0),
            None => "downloading".to_owned(),
        };
        let area = Rectangle::new(rect.x, rect.y, rect.width, rect.height - height - 4.0);
        ui::draw_text_centered(
            d,
            &ui::fit(&label, size, rect.width - 4.0),
            area,
            size,
            WHITE,
        );
    }
}

fn clamp_span(min_value: i64, max_value: i64) -> (i64, i64) {
    if max_value - min_value <= MAX_VISIBLE_CELLS_PER_AXIS {
        return (min_value, max_value);
    }
    let center = (min_value + max_value) / 2;
    let half = MAX_VISIBLE_CELLS_PER_AXIS / 2;
    (center - half, center + half)
}

fn draw_texture_fit(d: &mut RaylibDrawHandle<'_>, texture: &Texture2D, rect: Rectangle) {
    let scale = (rect.width / texture.width as f32).min(rect.height / texture.height as f32);
    let dest_width = texture.width as f32 * scale;
    let dest_height = texture.height as f32 * scale;
    let dest = Rectangle::new(
        rect.x + ((rect.width - dest_width) / 2.0),
        rect.y + ((rect.height - dest_height) / 2.0),
        dest_width,
        dest_height,
    );
    let source = Rectangle::new(0.0, 0.0, texture.width as f32, texture.height as f32);
    d.draw_texture_pro(texture, source, dest, Vector2::new(0.0, 0.0), 0.0, WHITE);
}

fn draw_nested_grid_icon(d: &mut RaylibDrawHandle<'_>, rect: Rectangle) {
    let thickness = (rect.height * 0.03).clamp(1.5, 6.0);
    d.draw_rectangle_lines_ex(rect, thickness, NESTED_GRID_COLOR);
    if rect.width > 16.0 && rect.height > 16.0 {
        // Inner square sits below the coordinate label.
        let size = rect.height * 0.62;
        let inset = Rectangle::new(
            rect.x + (rect.width - size) / 2.0,
            rect.y + rect.height * 0.28,
            size,
            size,
        );
        d.draw_rectangle_lines_ex(inset, thickness, NESTED_GRID_COLOR);
    }
}

fn kind_color(kind: CellKind) -> Color {
    match kind {
        CellKind::Image => Color::new(70, 120, 190, 230),
        CellKind::Audio => Color::new(145, 85, 190, 230),
        CellKind::Video => Color::new(200, 90, 80, 230),
        CellKind::RubyScript => Color::new(190, 55, 65, 230),
        CellKind::File => Color::new(90, 105, 120, 230),
        CellKind::Grid => NESTED_GRID_COLOR,
    }
}

/// Height of a cell's block in the 3D view (one cell = one world unit).
fn block_height(kind: CellKind) -> f32 {
    match kind {
        CellKind::Image => 0.06,
        CellKind::Audio => 0.32,
        CellKind::Video => 0.42,
        CellKind::RubyScript => 0.26,
        CellKind::File => 0.2,
        CellKind::Grid => 0.6,
    }
}

fn draw_file_badge(
    d: &mut RaylibDrawHandle<'_>,
    rect: Rectangle,
    kind: CellKind,
    path: &Path,
    show_name: bool,
) {
    let color = kind_color(kind);
    // Every badge metric is proportional to the on-screen cell size so the
    // text follows the camera zoom.
    let inset = rect.height * 0.05;
    let badge_height = (rect.height * 0.24).max(2.0);
    let badge = Rectangle::new(
        rect.x + inset,
        rect.y + rect.height - badge_height - inset,
        (rect.width - inset * 2.0).max(0.0),
        badge_height,
    );
    d.draw_rectangle_rec(badge, color);
    let badge_text = badge_height * 0.72;
    if badge_text >= 7.0 {
        let label = ui::fit(kind.label(), badge_text, badge.width - inset * 2.0);
        ui::draw_text_in(d, &label, badge, inset, badge_text, WHITE);
    }

    let name_size = ui::cell_text_size(rect.height, 0.16, 72.0);
    if show_name && kind != CellKind::Image && name_size >= 9.0 {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file");
        let name = ui::fit(name, name_size, rect.width - inset * 2.0);
        ui::draw_text(
            d,
            &name,
            rect.x + inset,
            rect.y + rect.height * 0.42 - name_size / 2.0,
            name_size,
            BUTTON_TEXT,
        );
    }
}

/// Quake-style scripting console overlay: dims the bottom portion of the
/// screen, shows recent scrollback, and the line currently being typed.
fn draw_console(
    d: &mut RaylibDrawHandle<'_>,
    screen_width: i32,
    screen_height: i32,
    toolbar_height: f32,
    history: &[String],
    input: &str,
    scroll: usize,
) {
    const MAX_VISIBLE_LINES: usize = 14;

    let scale = ui::scale(screen_width, screen_height);
    let text_size = 16.0 * scale;
    let prompt_size = 18.0 * scale;
    let line_height = text_size * 1.3;
    let padding = 10.0 * scale;
    let width = screen_width as f32;

    let available = screen_height as f32 - toolbar_height;
    let fitting_lines = ((available - padding * 2.0 - prompt_size * 1.6) / line_height)
        .floor()
        .max(0.0) as usize;
    let visible_lines = MAX_VISIBLE_LINES.min(fitting_lines);
    let height = (line_height * visible_lines as f32 + prompt_size * 1.6 + padding * 2.0)
        .min(available.max(0.0));
    let top = screen_height as f32 - height;
    let panel = Rectangle::new(0.0, top, width, height);
    d.draw_rectangle_rec(panel, Color::new(10, 10, 14, 235));
    d.draw_rectangle_lines_ex(panel, 1.0, CONSOLE_BORDER);

    let mut y = top + padding;
    let max_text_width = width - padding * 2.0;
    let end = history.len().saturating_sub(scroll);
    let start = end.saturating_sub(visible_lines);
    for line in &history[start..end] {
        let line = ui::fit(line, text_size, max_text_width);
        ui::draw_text(d, &line, padding, y, text_size, CONSOLE_TEXT);
        y += line_height;
    }
    if scroll > 0 {
        let note = format!("↑ scrolled back {scroll} line(s) — PgDn to return");
        let note_width = ui::measure(&note, text_size);
        ui::draw_text(
            d,
            &note,
            width - padding - note_width,
            top + padding,
            text_size,
            CONSOLE_PROMPT,
        );
    }

    // Keep the end of long input visible by trimming from the left.
    let mut prompt = format!("> {input}_");
    while ui::measure(&prompt, prompt_size) > max_text_width && prompt.chars().count() > 3 {
        let mut chars = prompt.chars();
        chars.nth(2);
        prompt = format!("> {}", chars.as_str());
    }
    ui::draw_text(
        d,
        &prompt,
        padding,
        top + height - padding - prompt_size * 1.2,
        prompt_size,
        CONSOLE_PROMPT,
    );
}

/// Scale and outer rectangle of the delete-confirmation dialog, shrunk if
/// needed so it always fits inside the window.
fn confirm_dialog_frame(screen_width: i32, screen_height: i32) -> (f32, Rectangle) {
    const DIALOG_WIDTH: f32 = 470.0;
    const DIALOG_HEIGHT: f32 = 170.0;

    let scale = ui::scale(screen_width, screen_height)
        .min((screen_width as f32 - 20.0) / DIALOG_WIDTH)
        .min((screen_height as f32 - 20.0) / DIALOG_HEIGHT)
        .max(0.1);
    let width = DIALOG_WIDTH * scale;
    let height = DIALOG_HEIGHT * scale;
    let rect = Rectangle::new(
        (screen_width as f32 - width) / 2.0,
        (screen_height as f32 - height) / 2.0,
        width,
        height,
    );
    (scale, rect)
}

/// Computes the on-screen rectangles for the "Delete" / "Cancel" buttons of
/// the delete-confirmation dialog, shared between hit-testing (`update`) and
/// rendering (`draw`) so the clickable area always matches what's drawn.
fn confirm_dialog_rects(screen_width: i32, screen_height: i32) -> (Rectangle, Rectangle) {
    let (scale, dialog) = confirm_dialog_frame(screen_width, screen_height);
    let button_width = 130.0 * scale;
    let button_height = 40.0 * scale;
    let button_spacing = 20.0 * scale;

    let buttons_y = dialog.y + dialog.height - button_height - 16.0 * scale;
    let total_width = button_width * 2.0 + button_spacing;
    let start_x = dialog.x + (dialog.width - total_width) / 2.0;

    let confirm_rect = Rectangle::new(start_x, buttons_y, button_width, button_height);
    let cancel_rect = Rectangle::new(
        start_x + button_width + button_spacing,
        buttons_y,
        button_width,
        button_height,
    );
    (confirm_rect, cancel_rect)
}

/// Modal confirmation dialog shown before deleting a non-empty nested grid.
/// "×N" badge in the bottom-right corner of a stacked cell.
fn draw_stack_badge(d: &mut RaylibDrawHandle<'_>, rect: Rectangle, count: usize) {
    let size = ui::cell_text_size(rect.height, 0.17, 40.0);
    if size < 8.0 {
        return;
    }
    let label = format!("×{count}");
    let pad = size * 0.3;
    let width = ui::measure(&label, size) + pad * 2.0;
    let height = size + pad;
    let badge = Rectangle::new(
        rect.x + rect.width - width - rect.width * 0.04,
        rect.y + rect.height - height - rect.height * 0.04,
        width,
        height,
    );
    // Offset "plates" behind the badge hint at items stacked underneath.
    let plate = Color::new(250, 210, 60, 120);
    let step = (rect.height * 0.025).clamp(1.0, 4.0);
    for layer in (1..count.min(3)).rev() {
        let offset = step * layer as f32;
        d.draw_rectangle_lines_ex(
            Rectangle::new(
                rect.x + offset,
                rect.y + offset,
                rect.width - offset * 2.0,
                rect.height - offset * 2.0,
            ),
            1.0,
            plate,
        );
    }
    d.draw_rectangle_rec(badge, Color::new(20, 20, 26, 220));
    d.draw_rectangle_lines_ex(badge, 1.0, SELECTION_COLOR);
    ui::draw_text_centered(d, &label, badge, size, Color::new(250, 225, 140, 255));
}

fn draw_confirm_dialog(
    d: &mut RaylibDrawHandle<'_>,
    screen_width: i32,
    screen_height: i32,
    col: i64,
    row: i64,
    count: usize,
    mouse: Vector2,
) {
    d.draw_rectangle(0, 0, screen_width, screen_height, Color::new(0, 0, 0, 150));

    let (scale, dialog_rect) = confirm_dialog_frame(screen_width, screen_height);
    let (x, y) = (dialog_rect.x, dialog_rect.y);
    let pad = 16.0 * scale;
    d.draw_rectangle_rec(dialog_rect, DIALOG_BG);
    d.draw_rectangle_lines_ex(dialog_rect, 2.0, DIALOG_BORDER);

    ui::draw_text(
        d,
        if count > 1 {
            "Delete selected cells?"
        } else {
            "Delete nested grid?"
        },
        x + pad,
        y + pad,
        22.0 * scale,
        BUTTON_TEXT,
    );

    let message = if count > 1 {
        format!(
            "{count} cells selected; some hold grids with contents.\nUndo (Ctrl+Z) can bring them back."
        )
    } else {
        format!(
            "Cell ({col}, {row}) contains a grid with contents.\nUndo (Ctrl+Z) can bring it back."
        )
    };
    let body_size = 16.0 * scale;
    for (index, line) in message.split('\n').enumerate() {
        let line = ui::fit(line, body_size, dialog_rect.width - pad * 2.0);
        ui::draw_text(
            d,
            &line,
            x + pad,
            y + 52.0 * scale + index as f32 * body_size * 1.35,
            body_size,
            CONSOLE_TEXT,
        );
    }

    let (confirm_rect, cancel_rect) = confirm_dialog_rects(screen_width, screen_height);
    let confirm_hovered = point_in_rect(mouse, confirm_rect);
    let cancel_hovered = point_in_rect(mouse, cancel_rect);
    let button_text = 20.0 * scale;

    d.draw_rectangle_rec(
        confirm_rect,
        if confirm_hovered {
            DANGER_HOVER_BG
        } else {
            DANGER_BG
        },
    );
    ui::draw_text_centered(d, "Delete", confirm_rect, button_text, BUTTON_TEXT);

    d.draw_rectangle_rec(
        cancel_rect,
        if cancel_hovered {
            BUTTON_HOVER_BG
        } else {
            BUTTON_BG
        },
    );
    ui::draw_text_centered(d, "Cancel", cancel_rect, button_text, BUTTON_TEXT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_game_writes_a_runnable_folder() {
        let dir = env::temp_dir().join(format!("selenite-export-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("assets")).unwrap();
        let script = dir.join("pong.rb");
        fs::write(&script, "puts 1\n").unwrap();
        fs::write(dir.join("assets/ball.png"), b"png").unwrap();
        let grid = dir.join("grid.json");
        fs::write(&grid, "{}").unwrap();

        let folder = export_game(&script, &grid).unwrap();
        assert_eq!(folder, dir.join("pong-game"));
        assert_eq!(
            fs::read_to_string(folder.join("game.rb")).unwrap(),
            "puts 1\n"
        );
        assert_eq!(fs::read_to_string(folder.join("save.json")).unwrap(), "{}");
        assert!(folder.join("assets/ball.png").is_file());
        assert!(folder.join("runtime/selenite").is_file());
        let launcher = fs::read_to_string(folder.join("run-game.sh")).unwrap();
        assert!(launcher.contains("--game \"$DIR/game.rb\""));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(folder.join("run-game.sh"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111);
        }
        // Exporting again overwrites in place.
        export_game(&script, &grid).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn context_menu_items_and_layout_fit_the_screen() {
        assert_eq!(
            MenuItem::for_cell(None, 0, false),
            vec![
                MenuItem::Paste,
                MenuItem::NewGrid,
                MenuItem::GridFolder,
                MenuItem::Items
            ]
        );
        let nested = MenuItem::for_cell(Some(CellKind::Grid), 1, false);
        assert!(nested.contains(&MenuItem::Open));
        assert!(nested.contains(&MenuItem::NestedGridFolder));
        assert!(!MenuItem::for_cell(Some(CellKind::Image), 1, false)
            .contains(&MenuItem::NestedGridFolder));
        let audio = MenuItem::for_cell(Some(CellKind::Audio), 1, false);
        assert!(audio.contains(&MenuItem::CopyPath));
        assert!(audio.contains(&MenuItem::QueueAudio));
        assert!(!audio.contains(&MenuItem::CycleStack));
        assert!(!audio.contains(&MenuItem::StackSelectionHere));
        assert_eq!(audio.last(), Some(&MenuItem::Items));
        assert!(
            !MenuItem::for_cell(Some(CellKind::Image), 1, false).contains(&MenuItem::QueueAudio)
        );

        // Stacks get stack actions; multi-selections can be stacked here.
        let stacked = MenuItem::for_cell(Some(CellKind::Image), 3, false);
        for item in [MenuItem::CycleStack, MenuItem::RemoveTop, MenuItem::Unstack] {
            assert!(stacked.contains(&item), "{item:?} missing");
        }
        let multi = MenuItem::for_cell(Some(CellKind::Image), 1, true);
        assert!(multi.contains(&MenuItem::StackSelectionHere));
        assert!(multi.contains(&MenuItem::Delete));
        assert!(MenuItem::for_cell(None, 0, true).contains(&MenuItem::StackSelectionHere));
        assert!(!MenuItem::for_cell(None, 0, true).contains(&MenuItem::Delete));

        // Ruby scripts can be run or exported as games; other files cannot.
        let script = MenuItem::for_cell(Some(CellKind::RubyScript), 1, false);
        assert!(script.contains(&MenuItem::RunGame));
        assert!(script.contains(&MenuItem::ExportGame));
        assert!(!stacked.contains(&MenuItem::RunGame));

        let mut items = MenuItem::for_cell(Some(CellKind::Image), 1, false);
        items.push(MenuItem::Plugin(7));
        let menu = ContextMenu {
            cell: (3, -2),
            position: Vector2::new(790.0, 590.0),
            items,
            plugin_labels: vec![(7, "Inspect file".to_owned())],
        };
        assert_eq!(menu.item_label(MenuItem::Plugin(7)), "» Inspect file");
        assert_eq!(menu.item_label(MenuItem::Plugin(8)), "Plugin");
        assert_eq!(menu.item_label(MenuItem::Info), "Info");
        let (frame, rows, _) = menu.layout(800, 600);
        assert_eq!(rows.len(), menu.items.len());
        assert!(frame.x + frame.width <= 800.0 && frame.y + frame.height <= 600.0);
        assert!(rows[0].y > frame.y, "first row sits below the header");
    }

    #[test]
    fn human_age_is_readable() {
        let now = std::time::SystemTime::now();
        let ago = |seconds| human_age(now - std::time::Duration::from_secs(seconds));
        assert_eq!(ago(5), "just now");
        assert_eq!(ago(5 * 60), "5 min ago");
        assert_eq!(ago(3_600), "1 hour ago");
        assert_eq!(ago(3 * 86_400), "3 days ago");
    }

    #[test]
    fn toolbar_wraps_onto_new_rows_when_narrow() {
        let widths = [100.0, 100.0, 100.0];
        let (rects, height) = wrap_toolbar_items(&widths, 1000.0, 40.0, 10.0, 5.0);
        assert!(rects.iter().all(|rect| rect.y == 10.0));
        assert_eq!(height, 60.0);

        let (rects, height) = wrap_toolbar_items(&widths, 240.0, 40.0, 10.0, 5.0);
        assert_eq!(rects[0].y, 10.0);
        assert_eq!(rects[1].y, 10.0);
        assert_eq!(rects[2].y, 55.0);
        assert_eq!(rects[2].x, 10.0);
        assert_eq!(height, 105.0);

        let (rects, height) = wrap_toolbar_items(&[500.0], 100.0, 40.0, 10.0, 5.0);
        assert_eq!(rects[0].y, 10.0);
        assert_eq!(height, 60.0);
        assert_eq!(wrap_toolbar_items(&[], 100.0, 40.0, 10.0, 5.0).1, 0.0);
    }

    #[test]
    fn parses_command_line_modes() {
        let parse = |args: &[&str]| {
            let args: Vec<OsString> = std::iter::once("selenite")
                .chain(args.iter().copied())
                .map(OsString::from)
                .collect();
            parse_mode(&args)
        };
        assert_eq!(parse(&[]).unwrap(), StartupMode::Profile(None));
        assert_eq!(
            parse(&["--profile", "work"]).unwrap(),
            StartupMode::Profile(Some("work".into()))
        );
        assert_eq!(
            parse(&["--grid", "a.json"]).unwrap(),
            StartupMode::Grid("a.json".into())
        );
        assert_eq!(
            parse(&["b.json"]).unwrap(),
            StartupMode::Grid("b.json".into())
        );
        assert_eq!(
            parse(&["--script", "s.rb"]).unwrap(),
            StartupMode::Script("s.rb".into(), Target::Profile(None))
        );
        assert_eq!(
            parse(&["--script", "s.rb", "--profile", "p"]).unwrap(),
            StartupMode::Script("s.rb".into(), Target::Profile(Some("p".into())))
        );
        assert_eq!(
            parse(&["--script", "s.rb", "g.json"]).unwrap(),
            StartupMode::Script("s.rb".into(), Target::Grid("g.json".into()))
        );
        assert_eq!(
            parse(&["--game", "pong.rb", "--profile", "p"]).unwrap(),
            StartupMode::Game("pong.rb".into(), Target::Profile(Some("p".into())))
        );
        assert!(parse(&["--game"]).is_err());
        assert_eq!(
            parse(&["--list-profiles"]).unwrap(),
            StartupMode::ListProfiles
        );
        assert!(parse(&["--profile"]).is_err());
        assert!(parse(&["--bogus"]).is_err());
    }

    #[test]
    fn expands_home_directory() {
        if let Some(home) = env::var_os("HOME") {
            assert_eq!(expand_home("~/x.png"), PathBuf::from(home).join("x.png"));
        }
        assert_eq!(expand_home("/tmp/~x"), PathBuf::from("/tmp/~x"));
        assert_eq!(expand_home("~other"), PathBuf::from("~other"));
    }

    /// The same model calls the UI makes for: box-select, group drag,
    /// Stack selection here, Tab, Unstack and Shift+Delete.
    #[test]
    fn ux_flow_select_move_stack_cycle_unstack() {
        let mut grid = persistence::SavedGrid::new();
        for col in 0..3 {
            grid.push_file(col, 0, PathBuf::from(format!("/m/{col}.png")));
        }
        grid.push_file(4, 0, PathBuf::from("/m/blocker.txt"));

        let mut sel = selection::Selection::default();
        sel.set_rect((0, 0), (2, 0), false).unwrap();
        let cells: Vec<_> = sel.cells().collect();
        assert_eq!(cells.len(), 3);

        // Dragging two to the right would land (2,0) on the blocker.
        assert_eq!(
            selection::plan_move(cells.iter().copied(), (2, 0), |(c, r)| grid.occupied(c, r)),
            Err((4, 0))
        );
        // Dragging one row down is free.
        let moves =
            selection::plan_move(cells.iter().copied(), (0, 1), |(c, r)| grid.occupied(c, r))
                .unwrap();
        assert_eq!(grid.move_many(&moves), 3);
        sel.translate((0, 1));
        assert!((0..3).all(|col| grid.occupied(col, 1) && !grid.occupied(col, 0)));
        assert!(sel.contains((2, 1)));

        // Stack the whole selection onto (0, 1).
        for cell in sel
            .cells()
            .filter(|&cell| cell != (0, 1))
            .collect::<Vec<_>>()
        {
            grid.merge_cells(cell, (0, 1));
        }
        assert_eq!(grid.stack_len(0, 1), 3);
        assert_eq!(grid.len(), 2, "stack + blocker");
        let top =
            |grid: &persistence::SavedGrid| grid.items_at(0, 1)[0].path().unwrap().to_path_buf();
        let first = top(&grid);
        assert!(grid.cycle_stack(0, 1, 1));
        assert_ne!(top(&grid), first);
        assert!(grid.cycle_stack(0, 1, 1) && grid.cycle_stack(0, 1, 1));
        assert_eq!(top(&grid), first, "Tab wraps around");

        let placed = grid.unstack(0, 1);
        assert_eq!(placed, vec![(1, 1), (2, 1)]);
        assert_eq!(grid.stack_len(0, 1), 1);
        assert!(grid.pop_item(2, 1, 0).is_some());
        assert!(
            !grid.occupied(2, 1),
            "Shift+Delete on a single item empties it"
        );
        assert_eq!(grid.item_count(), 3);
    }

    #[test]
    fn context_menus_stay_on_screen_in_every_corner_and_size() {
        let mut items = MenuItem::for_cell(Some(CellKind::RubyScript), 3, true);
        items.extend([MenuItem::Plugin(1), MenuItem::Plugin(2)]);
        for (w, h) in [(640, 480), (1024, 600), (1920, 1080), (3840, 2160)] {
            for (x, y) in [
                (0.0, 0.0),
                (w as f32 - 1.0, 0.0),
                (0.0, h as f32 - 1.0),
                (w as f32 - 1.0, h as f32 - 1.0),
                (w as f32 / 2.0, h as f32 / 2.0),
            ] {
                let menu = ContextMenu {
                    cell: (0, 0),
                    position: Vector2::new(x, y),
                    items: items.clone(),
                    plugin_labels: vec![(1, "A".into()), (2, "B".into())],
                };
                let (frame, rows, _) = menu.layout(w, h);
                assert_eq!(rows.len(), items.len());
                assert!(
                    frame.x >= 0.0
                        && frame.y >= 0.0
                        && frame.x + frame.width <= w as f32 + 0.5
                        && frame.y + frame.height <= h as f32 + 0.5,
                    "menu off-screen at {w}x{h} from ({x}, {y})"
                );
                for pair in rows.windows(2) {
                    assert!(pair[0].y + pair[0].height <= pair[1].y + 0.5);
                }
            }
        }
    }
}

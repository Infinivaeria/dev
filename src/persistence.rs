//! Infinite, sparse, recursively-nestable grid persistence.
//!
//! A `SavedGrid` maps signed `(col, row)` coordinates to either a file path
//! or another nested `SavedGrid` (shared via `Arc<Mutex<_>>` so that a
//! Magnus wrapper handed out for a nested grid aliases the exact same data
//! as the parent cell -- mutating the nested grid "in place" needs no
//! write-back/bubbling logic when navigating back out of it).

use std::{
    collections::HashMap,
    error::Error,
    fmt,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::grid_inventory::GridInventory;

/// Folder that holds every grid's own folder for the save file at
/// `save_path`: `<dir>/<stem>-grids`.
pub fn grids_root(save_path: &Path) -> PathBuf {
    let parent = save_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stem = save_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "selenite".to_owned());
    parent.join(format!("{stem}-grids"))
}

/// The folder belonging to `grid` (created on first use). Returns the path
/// and whether the grid was just given its id, in which case the save file
/// should be written so the id sticks.
pub fn grid_folder(
    save_path: &Path,
    grid: &Arc<Mutex<SavedGrid>>,
    root: &Arc<Mutex<SavedGrid>>,
) -> io::Result<(PathBuf, bool)> {
    let is_root = Arc::ptr_eq(grid, root);
    let (id, new) = grid
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .ensure_id(is_root);
    let folder = grids_root(save_path).join(id);
    fs::create_dir_all(&folder)?;
    Ok((folder, new))
}

/// Subfolder of every grid folder that holds the grid's own copies of the
/// files added to it.
pub const ASSETS_DIR: &str = "assets";

/// `grid`'s assets folder (`<grid folder>/assets`), created on first use.
/// The flag means the same as for [`grid_folder`].
pub fn grid_assets_folder(
    save_path: &Path,
    grid: &Arc<Mutex<SavedGrid>>,
    root: &Arc<Mutex<SavedGrid>>,
) -> io::Result<(PathBuf, bool)> {
    let (folder, new) = grid_folder(save_path, grid, root)?;
    let assets = folder.join(ASSETS_DIR);
    fs::create_dir_all(&assets)?;
    Ok((assets, new))
}

fn valid_grid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A short unique id: time in nanoseconds plus a process-wide counter.
fn new_grid_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or_default();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("grid-{nanos:x}-{count:x}")
}

pub const SAVE_VERSION: u32 = 3;
/// Oldest save format `load` still understands (migrated on the next save).
pub const MIN_SAVE_VERSION: u32 = 2;
const IMAGE_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".bmp", ".gif", ".tga", ".qoi", ".psd", ".dds", ".hdr", ".ktx",
];
const AUDIO_EXTENSIONS: &[&str] = &[
    ".aac", ".aiff", ".alac", ".flac", ".m4a", ".mp3", ".ogg", ".opus", ".wav", ".wma",
];
const VIDEO_EXTENSIONS: &[&str] = &[
    ".avi", ".flv", ".m4v", ".mkv", ".mov", ".mp4", ".mpeg", ".mpg", ".ogv", ".webm", ".wmv",
];
const RUBY_EXTENSIONS: &[&str] = &[".rb", ".rake", ".gemspec"];

/// What a single grid cell holds. Cloning is shallow: a cloned `Grid`
/// shares the same nested grid.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CellValue {
    Image(PathBuf),
    File(PathBuf),
    Grid(Arc<Mutex<SavedGrid>>),
}

/// An item and its saved label visibility. Flattening preserves existing item JSON.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CellContent {
    #[serde(flatten)]
    pub content: CellValue,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    labels_hidden: bool,
}

impl From<CellValue> for CellContent {
    fn from(content: CellValue) -> Self {
        Self {
            content,
            labels_hidden: false,
        }
    }
}

/// Lightweight tag for a cell's content, without borrowing the content itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    Image,
    Audio,
    Video,
    RubyScript,
    File,
    Grid,
}

impl CellKind {
    /// Lower-case identifier used by the Ruby API and PA exports.
    pub fn name(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::RubyScript => "ruby",
            Self::File => "file",
            Self::Grid => "grid",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "Image",
            Self::Audio => "Audio",
            Self::Video => "Video",
            Self::RubyScript => "Ruby",
            Self::File => "File",
            Self::Grid => "Grid",
        }
    }
}

impl CellContent {
    pub fn from_path(path: PathBuf) -> Self {
        if SavedGrid::is_image_path(&path) {
            CellValue::Image(path).into()
        } else {
            CellValue::File(path).into()
        }
    }

    pub fn kind(&self) -> CellKind {
        match &self.content {
            CellValue::Image(_) => CellKind::Image,
            CellValue::File(path) => SavedGrid::classify_path(path),
            CellValue::Grid(_) => CellKind::Grid,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match &self.content {
            CellValue::Image(path) | CellValue::File(path) => Some(path.as_path()),
            CellValue::Grid(_) => None,
        }
    }

    /// Short display name: the file name, or "nested grid (N items)".
    pub fn label(&self) -> String {
        match &self.content {
            CellValue::Image(path) | CellValue::File(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
            CellValue::Grid(grid) => {
                let len = grid.lock().map(|grid| grid.len()).unwrap_or(0);
                format!("nested grid ({len} cells)")
            }
        }
    }

    /// Equal label visibility plus value equality for files or identity for nested grids.
    pub fn same_as(&self, other: &CellContent) -> bool {
        self.labels_hidden == other.labels_hidden
            && match (&self.content, &other.content) {
                (CellValue::Image(a), CellValue::Image(b))
                | (CellValue::File(a), CellValue::File(b)) => a == b,
                (CellValue::Grid(a), CellValue::Grid(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
    }
}

/// An infinite, sparse grid of cells keyed by signed `(col, row)`
/// coordinates. Only occupied cells are stored; each holds a non-empty
/// stack of items whose first element is the top (the one shown and opened).
/// Every grid also carries its own game-style inventory.
#[derive(Debug, Default)]
pub struct SavedGrid {
    cells: HashMap<(i64, i64), Vec<CellContent>>,
    inventory: GridInventory,
    /// Stable id naming this grid's folder on disk; assigned the first time
    /// the folder is used.
    id: Option<String>,
}

// JSON objects need string/simple keys, so `(i64, i64)` tuple keys can't
// serialize directly via `HashMap`'s blanket impl. Bridge through a flat
// list of `{col, row, items}` entries instead.
#[derive(Serialize)]
struct StackEntryRef<'a> {
    col: i64,
    row: i64,
    items: &'a [CellContent],
}

#[derive(Serialize)]
struct GridRef<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<&'a str>,
    cells: Vec<StackEntryRef<'a>>,
    #[serde(skip_serializing_if = "GridInventory::is_empty")]
    inventory: &'a GridInventory,
}

#[derive(Deserialize)]
struct CellEntryV2 {
    col: i64,
    row: i64,
    content: CellContent,
}

#[derive(Deserialize)]
struct StackEntry {
    col: i64,
    row: i64,
    items: Vec<CellContent>,
}

/// Version 2 grids are a bare list of single-item cells; version 3 grids
/// are objects with stacked cells and an inventory.
#[derive(Deserialize)]
#[serde(untagged)]
enum GridRepr {
    V3 {
        cells: Vec<StackEntry>,
        #[serde(default)]
        inventory: GridInventory,
        #[serde(default)]
        id: Option<String>,
    },
    V2(Vec<CellEntryV2>),
}

impl Serialize for SavedGrid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut cells: Vec<StackEntryRef<'_>> = self
            .cells
            .iter()
            .map(|(&(col, row), items)| StackEntryRef {
                col,
                row,
                items: items.as_slice(),
            })
            .collect();
        cells.sort_by_key(|entry| (entry.col, entry.row));
        GridRef {
            id: self.id.as_deref(),
            cells,
            inventory: &self.inventory,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SavedGrid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut grid = SavedGrid::new();
        match GridRepr::deserialize(deserializer)? {
            GridRepr::V2(entries) => {
                for entry in entries {
                    grid.cells
                        .insert((entry.col, entry.row), vec![entry.content]);
                }
            }
            GridRepr::V3 {
                cells,
                inventory,
                id,
            } => {
                grid.id = id.filter(|id| valid_grid_id(id));
                for entry in cells {
                    if !entry.items.is_empty() {
                        grid.cells
                            .entry((entry.col, entry.row))
                            .or_default()
                            .extend(entry.items);
                    }
                }
                grid.inventory = inventory;
            }
        }
        Ok(grid)
    }
}

#[derive(Serialize)]
struct SaveFileRef<'a> {
    version: u32,
    root: &'a SavedGrid,
}

#[derive(Deserialize)]
struct SaveFile {
    version: u32,
    root: SavedGrid,
}

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Json(serde_json::Error),
    UnsupportedVersion(u32),
    CellOccupied { col: i64, row: i64 },
}

impl fmt::Display for SaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "could not access save file: {error}"),
            Self::Json(error) => write!(formatter, "invalid save JSON: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported save file version: {version}")
            }
            Self::CellOccupied { col, row } => {
                write!(formatter, "cell ({col}, {row}) is already occupied")
            }
        }
    }
}

impl Error for SaveError {}

impl From<io::Error> for SaveError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for SaveError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl SavedGrid {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn is_image_path(path: &Path) -> bool {
        Self::extension_matches(path, IMAGE_EXTENSIONS)
    }

    pub fn classify_path(path: &Path) -> CellKind {
        if Self::is_image_path(path) {
            CellKind::Image
        } else if Self::extension_matches(path, AUDIO_EXTENSIONS) {
            CellKind::Audio
        } else if Self::extension_matches(path, VIDEO_EXTENSIONS) {
            CellKind::Video
        } else if Self::extension_matches(path, RUBY_EXTENSIONS) {
            CellKind::RubyScript
        } else {
            CellKind::File
        }
    }

    fn extension_matches(path: &Path, extensions: &[&str]) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!(".{}", extension.to_ascii_lowercase()))
            .is_some_and(|extension| extensions.contains(&extension.as_str()))
    }

    fn top(&self, col: i64, row: i64) -> Option<&CellContent> {
        self.cells.get(&(col, row)).and_then(|items| items.first())
    }

    /// Whether the top item's labels are enabled (empty cells inherit the global setting).
    pub fn labels_at(&self, col: i64, row: i64) -> bool {
        self.top(col, row).is_none_or(|item| !item.labels_hidden)
    }

    /// Toggles only the top item; its preference follows it through moves and stacks.
    pub fn toggle_labels_at(&mut self, col: i64, row: i64) -> Option<bool> {
        let item = self.cells.get_mut(&(col, row))?.first_mut()?;
        item.labels_hidden = !item.labels_hidden;
        Some(!item.labels_hidden)
    }

    pub fn kind_at(&self, col: i64, row: i64) -> Option<CellKind> {
        self.top(col, row).map(CellContent::kind)
    }

    pub fn occupied(&self, col: i64, row: i64) -> bool {
        self.cells.contains_key(&(col, row))
    }

    pub fn image_at(&self, col: i64, row: i64) -> Option<&Path> {
        match &self.top(col, row)?.content {
            CellValue::Image(path) => Some(path.as_path()),
            CellValue::File(_) | CellValue::Grid(_) => None,
        }
    }

    pub fn file_at(&self, col: i64, row: i64) -> Option<&Path> {
        self.top(col, row)?.path()
    }

    pub fn grid_at(&self, col: i64, row: i64) -> Option<Arc<Mutex<SavedGrid>>> {
        match &self.top(col, row)?.content {
            CellValue::Grid(grid) => Some(Arc::clone(grid)),
            CellValue::Image(_) | CellValue::File(_) => None,
        }
    }

    /// The grid a cell leads into: its top item if that's a grid, otherwise
    /// the highest grid buried in its stack.
    pub fn grid_in(&self, col: i64, row: i64) -> Option<Arc<Mutex<SavedGrid>>> {
        self.items_at(col, row)
            .iter()
            .find_map(|item| match &item.content {
                CellValue::Grid(grid) => Some(Arc::clone(grid)),
                _ => None,
            })
    }

    /// Every item stacked in a cell, top first (empty if unoccupied).
    pub fn items_at(&self, col: i64, row: i64) -> &[CellContent] {
        self.cells
            .get(&(col, row))
            .map_or(&[][..], |items| items.as_slice())
    }

    pub fn stack_len(&self, col: i64, row: i64) -> usize {
        self.cells.get(&(col, row)).map_or(0, Vec::len)
    }

    /// Puts `content` on top of the cell's stack (creating the cell if
    /// needed); returns the new stack height.
    pub fn push_content(&mut self, col: i64, row: i64, content: CellContent) -> usize {
        let items = self.cells.entry((col, row)).or_default();
        items.insert(0, content);
        items.len()
    }

    pub fn push_file(&mut self, col: i64, row: i64, path: PathBuf) -> usize {
        self.push_content(col, row, CellContent::from_path(path))
    }

    /// Stacks several files on one cell, the first ending up on top.
    pub fn push_files<I, P>(&mut self, col: i64, row: i64, paths: I) -> usize
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let fresh: Vec<CellContent> = paths
            .into_iter()
            .map(|path| CellContent::from_path(path.into()))
            .collect();
        let count = fresh.len();
        if count > 0 {
            let items = self.cells.entry((col, row)).or_default();
            items.splice(0..0, fresh);
        }
        count
    }

    /// Stacks a brand-new nested grid on a cell.
    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn push_grid(&mut self, col: i64, row: i64) -> Arc<Mutex<SavedGrid>> {
        let grid = Arc::new(Mutex::new(SavedGrid::new()));
        self.push_content(col, row, CellValue::Grid(Arc::clone(&grid)).into());
        grid
    }

    /// Removes the item at `index` (0 = top) from a stack; the cell is
    /// emptied when its last item goes.
    pub fn pop_item(&mut self, col: i64, row: i64, index: usize) -> Option<CellContent> {
        let items = self.cells.get_mut(&(col, row))?;
        if index >= items.len() {
            return None;
        }
        let item = items.remove(index);
        if items.is_empty() {
            self.cells.remove(&(col, row));
        }
        Some(item)
    }

    /// Rotates a stack so a different item is on top: `delta` 1 brings the
    /// second item up, -1 brings the bottom item up. False if nothing moved.
    pub fn cycle_stack(&mut self, col: i64, row: i64, delta: i64) -> bool {
        let Some(items) = self.cells.get_mut(&(col, row)) else {
            return false;
        };
        let len = items.len() as i64;
        if len < 2 {
            return false;
        }
        let shift = delta.rem_euclid(len) as usize;
        if shift == 0 {
            return false;
        }
        items.rotate_left(shift);
        true
    }

    /// Moves the item at `index` to the top without reordering the rest.
    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn raise_item(&mut self, col: i64, row: i64, index: usize) -> bool {
        let Some(items) = self.cells.get_mut(&(col, row)) else {
            return false;
        };
        if index == 0 || index >= items.len() {
            return false;
        }
        let item = items.remove(index);
        items.insert(0, item);
        true
    }

    /// Moves every item of `from` on top of `to` (a plain move if `to` is
    /// empty). Returns how many items moved.
    pub fn merge_cells(&mut self, from: (i64, i64), to: (i64, i64)) -> usize {
        if from == to {
            return 0;
        }
        let Some(mut moved) = self.cells.remove(&from) else {
            return 0;
        };
        let count = moved.len();
        let target = self.cells.entry(to).or_default();
        moved.append(target);
        *target = moved;
        count
    }

    /// Spreads a stack out: the top stays, every other item moves to the
    /// next free cells to the right in the same row. Returns the cells used.
    pub fn unstack(&mut self, col: i64, row: i64) -> Vec<(i64, i64)> {
        let Some(items) = self.cells.get_mut(&(col, row)) else {
            return Vec::new();
        };
        if items.len() < 2 {
            return Vec::new();
        }
        let rest: Vec<CellContent> = items.drain(1..).collect();
        let mut placed = Vec::with_capacity(rest.len());
        let mut next = col;
        for item in rest {
            next = self.next_free_in_row(next.saturating_add(1), row);
            self.cells.insert((next, row), vec![item]);
            placed.push((next, row));
        }
        placed
    }

    pub fn inventory(&self) -> &GridInventory {
        &self.inventory
    }

    /// Moves whole stacks in one step: every source is lifted before any is
    /// placed, so a block shifted onto itself works. A destination that is
    /// still occupied afterwards gets the moved stack on top (merged).
    pub fn move_many(&mut self, moves: &[((i64, i64), (i64, i64))]) -> usize {
        let lifted: Vec<((i64, i64), Vec<CellContent>)> = moves
            .iter()
            .filter_map(|(from, to)| self.cells.remove(from).map(|items| (*to, items)))
            .collect();
        let count = lifted.len();
        for (to, mut items) in lifted {
            let target = self.cells.entry(to).or_default();
            items.append(target);
            *target = items;
        }
        count
    }

    pub fn inventory_mut(&mut self) -> &mut GridInventory {
        &mut self.inventory
    }

    /// Number of items (not cells) at this level, counting stacks fully.
    pub fn item_count(&self) -> usize {
        self.cells.values().map(Vec::len).sum()
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn insert_image(&mut self, col: i64, row: i64, path: PathBuf) -> Result<(), SaveError> {
        if self.cells.contains_key(&(col, row)) {
            return Err(SaveError::CellOccupied { col, row });
        }
        self.cells
            .insert((col, row), vec![CellValue::Image(path).into()]);
        Ok(())
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn insert_file(&mut self, col: i64, row: i64, path: PathBuf) -> Result<(), SaveError> {
        if self.cells.contains_key(&(col, row)) {
            return Err(SaveError::CellOccupied { col, row });
        }
        self.cells
            .insert((col, row), vec![CellContent::from_path(path)]);
        Ok(())
    }

    /// Places files left-to-right from `start_col`. An occupied cell has its
    /// top item replaced (the rest of its stack is kept); cells whose top is
    /// a nested grid are skipped.
    pub fn insert_files_fanout<I, P>(&mut self, start_col: i64, row: i64, paths: I) -> usize
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let mut col = start_col;
        let mut inserted = 0;

        for path in paths {
            let content = CellContent::from_path(path.into());
            match self.cells.get_mut(&(col, row)) {
                Some(items)
                    if items
                        .first()
                        .is_some_and(|item| item.kind() == CellKind::Grid) => {}
                Some(items) => {
                    items[0] = content;
                    inserted += 1;
                }
                None => {
                    self.cells.insert((col, row), vec![content]);
                    inserted += 1;
                }
            }
            col += 1;
        }

        inserted
    }

    #[allow(dead_code)]
    pub fn insert_images_fanout<I, P>(&mut self, start_col: i64, row: i64, paths: I) -> usize
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        self.insert_files_fanout(
            start_col,
            row,
            paths
                .into_iter()
                .map(Into::into)
                .filter(|path| Self::is_image_path(path)),
        )
    }

    pub fn create_grid(&mut self, col: i64, row: i64) -> Result<Arc<Mutex<SavedGrid>>, SaveError> {
        if self.cells.contains_key(&(col, row)) {
            return Err(SaveError::CellOccupied { col, row });
        }
        let grid = Arc::new(Mutex::new(SavedGrid::new()));
        self.cells
            .insert((col, row), vec![CellValue::Grid(Arc::clone(&grid)).into()]);
        Ok(grid)
    }

    /// Empties a cell, returning its whole stack.
    pub fn remove(&mut self, col: i64, row: i64) -> Option<Vec<CellContent>> {
        self.cells.remove(&(col, row))
    }

    /// Moves a cell into an empty target cell.
    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn move_cell(&mut self, from: (i64, i64), to: (i64, i64)) -> Result<(), SaveError> {
        if from == to {
            return Ok(());
        }
        if self.cells.contains_key(&to) {
            return Err(SaveError::CellOccupied {
                col: to.0,
                row: to.1,
            });
        }
        if let Some(content) = self.cells.remove(&from) {
            self.cells.insert(to, content);
        }
        Ok(())
    }

    /// Exchanges the contents of two cells; either may be empty.
    pub fn swap_cells(&mut self, a: (i64, i64), b: (i64, i64)) {
        if a == b {
            return;
        }
        let first = self.cells.remove(&a);
        let second = self.cells.remove(&b);
        if let Some(content) = first {
            self.cells.insert(b, content);
        }
        if let Some(content) = second {
            self.cells.insert(a, content);
        }
    }

    /// First unoccupied column at or to the right of `col` in `row`.
    pub fn next_free_in_row(&self, col: i64, row: i64) -> i64 {
        let mut col = col;
        while self.cells.contains_key(&(col, row)) {
            col = col.saturating_add(1);
        }
        col
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn clear(&mut self) {
        self.cells.clear();
    }

    /// Shallow copy of this grid's cells and inventory (nested grids are
    /// shared, not copied), used for undo/redo.
    pub fn snapshot(&self) -> SavedGrid {
        SavedGrid {
            cells: self.cells.clone(),
            inventory: self.inventory.clone(),
            id: self.id.clone(),
        }
    }

    /// This grid's folder id, if its folder has been used.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Returns this grid's folder id, assigning one first if needed; the
    /// bool says whether it is new (so the grid should be saved).
    pub fn ensure_id(&mut self, is_root: bool) -> (String, bool) {
        if let Some(id) = &self.id {
            return (id.clone(), false);
        }
        let id = if is_root {
            "root".to_owned()
        } else {
            new_grid_id()
        };
        self.id = Some(id.clone());
        (id, true)
    }

    /// Keeps `id` across an undo/redo swap: folders aren't part of history.
    pub fn adopt_id(&mut self, id: Option<String>) {
        if id.is_some() {
            self.id = id;
        }
    }

    /// Whether both grids hold the same stacks and inventory at this level
    /// (nested grids compare by identity).
    pub fn same_cells(&self, other: &SavedGrid) -> bool {
        self.inventory == other.inventory
            && self.cells.len() == other.cells.len()
            && self.cells.iter().all(|(key, items)| {
                other.cells.get(key).is_some_and(|theirs| {
                    items.len() == theirs.len()
                        && items.iter().zip(theirs).all(|(a, b)| a.same_as(b))
                })
            })
    }

    /// Cell that holds `nested` (anywhere in its stack), if it's a direct
    /// child of this grid.
    pub fn find_grid(&self, nested: &Arc<Mutex<SavedGrid>>) -> Option<(i64, i64)> {
        self.cells.iter().find_map(|(&cell, items)| {
            items
                .iter()
                .any(|content| matches!(&content.content, CellValue::Grid(grid) if Arc::ptr_eq(grid, nested)))
                .then_some(cell)
        })
    }

    /// Number of items here and in every nested grid (stacks count fully).
    pub fn total_len(&self) -> usize {
        self.cells
            .values()
            .flatten()
            .map(|content| match &content.content {
                CellValue::Grid(grid) => {
                    1 + grid
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .total_len()
                }
                _ => 1,
            })
            .sum()
    }

    /// Rewrites file paths under `old_prefix` to live under `new_prefix`,
    /// recursing into nested grids. Returns how many paths changed.
    pub fn rewrite_path_prefix(&mut self, old_prefix: &Path, new_prefix: &Path) -> usize {
        let mut changed = 0;
        for content in self.cells.values_mut().flatten() {
            match &mut content.content {
                CellValue::Image(path) | CellValue::File(path) => {
                    if let Ok(rest) = path.strip_prefix(old_prefix) {
                        *path = new_prefix.join(rest);
                        changed += 1;
                    }
                }
                CellValue::Grid(grid) => {
                    changed += grid
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .rewrite_path_prefix(old_prefix, new_prefix);
                }
            }
        }
        changed
    }

    /// Recursively collects all items attached to this grid and all nested
    /// grids, including every item of a stack (top first).
    /// Returns a list of `(hierarchy_path, col, row, kind, path)`.
    pub fn collect_all_items(
        &self,
        current_prefix: &str,
    ) -> Vec<(String, i64, i64, CellKind, Option<PathBuf>)> {
        let mut items = Vec::new();
        let mut sorted_keys: Vec<(i64, i64)> = self.cells.keys().copied().collect();
        sorted_keys.sort();
        let loc = if current_prefix.is_empty() {
            "Root".to_string()
        } else {
            current_prefix.to_string()
        };

        for (col, row) in sorted_keys {
            for content in &self.cells[&(col, row)] {
                match &content.content {
                    CellValue::Image(path) | CellValue::File(path) => {
                        items.push((loc.clone(), col, row, content.kind(), Some(path.clone())));
                    }
                    CellValue::Grid(nested) => {
                        let grid_name = if current_prefix.is_empty() {
                            format!("Grid ({col}, {row})")
                        } else {
                            format!("{current_prefix} > Grid ({col}, {row})")
                        };
                        items.push((loc.clone(), col, row, CellKind::Grid, None));
                        let locked = nested.lock().unwrap_or_else(|e| e.into_inner());
                        items.extend(locked.collect_all_items(&grid_name));
                    }
                }
            }
        }
        items
    }

    /// All `(col, row, path)` triples for cells whose top item is an image.
    #[allow(dead_code)]
    pub fn images(&self) -> impl Iterator<Item = (i64, i64, &Path)> {
        self.cells
            .iter()
            .filter_map(|(&(col, row), items)| match &items.first()?.content {
                CellValue::Image(path) => Some((col, row, path.as_path())),
                CellValue::File(_) | CellValue::Grid(_) => None,
            })
    }

    /// All occupied `(col, row, top kind)` triples, for windowed rendering.
    pub fn entries(&self) -> impl Iterator<Item = (i64, i64, CellKind)> + '_ {
        self.cells.iter().filter_map(|(&(col, row), items)| {
            items.first().map(|content| (col, row, content.kind()))
        })
    }

    /// All occupied cells with their full stacks.
    pub fn stacks(&self) -> impl Iterator<Item = ((i64, i64), &[CellContent])> + '_ {
        self.cells
            .iter()
            .map(|(&cell, items)| (cell, items.as_slice()))
    }

    pub fn load(path: &Path) -> Result<Option<Self>, SaveError> {
        let data = match fs::read(path) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let file: SaveFile = serde_json::from_slice(&data)?;
        if !(MIN_SAVE_VERSION..=SAVE_VERSION).contains(&file.version) {
            return Err(SaveError::UnsupportedVersion(file.version));
        }
        Ok(Some(file.root))
    }

    pub fn save(&self, path: &Path) -> Result<(), SaveError> {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty());
        if let Some(parent) = parent {
            fs::create_dir_all(parent)?;
        }
        let mut temporary = path.as_os_str().to_os_string();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        let file = SaveFileRef {
            version: SAVE_VERSION,
            root: self,
        };
        let result = (|| -> Result<(), SaveError> {
            let mut handle = File::create(&temporary)?;
            serde_json::to_writer_pretty(&mut handle, &file)?;
            handle.write_all(b"\n")?;
            handle.sync_all()?;
            fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        time::SystemTime,
    };

    use super::{
        grid_assets_folder, grid_folder, grids_root, valid_grid_id, CellKind, SaveError, SavedGrid,
    };

    fn unique_file() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = PathBuf::from("test-artifacts");
        fs::create_dir_all(&dir).unwrap();
        dir.join(format!("selenite-save-{stamp}.json"))
    }

    #[test]
    fn item_labels_follow_moves_and_individual_stack_items() {
        let mut grid = SavedGrid::new();
        assert_eq!(grid.toggle_labels_at(0, 0), None);
        grid.push_file(0, 0, "same.mp4".into());
        assert_eq!(grid.toggle_labels_at(0, 0), Some(false));
        // Two occurrences of the same path have independent preferences.
        grid.push_file(0, 0, "same.mp4".into());
        assert!(grid.labels_at(0, 0));
        grid.cycle_stack(0, 0, 1);
        assert!(!grid.labels_at(0, 0));
        grid.move_cell((0, 0), (4, 2)).unwrap();
        assert!(!grid.labels_at(4, 2));
        assert_eq!(grid.unstack(4, 2), vec![(5, 2)]);
        assert!(!grid.labels_at(4, 2));
        assert!(grid.labels_at(5, 2));
        grid.swap_cells((4, 2), (5, 2));
        assert!(grid.labels_at(4, 2));
        assert!(!grid.labels_at(5, 2));
        assert_eq!(grid.toggle_labels_at(5, 2), Some(true));
    }

    #[test]
    fn item_labels_round_trip_and_old_items_default_to_visible() {
        let mut grid: SavedGrid = serde_json::from_str(
            r#"{"cells":[{"col":0,"row":0,"items":[{"Image":"a.png"},{"File":"b.mp3"}]}]}"#,
        )
        .unwrap();
        assert!(grid.labels_at(0, 0));
        grid.toggle_labels_at(0, 0);
        let nested = grid.create_grid(1, 0).unwrap();
        grid.toggle_labels_at(1, 0);
        nested.lock().unwrap().push_file(2, 0, "c.mp4".into());
        nested.lock().unwrap().toggle_labels_at(2, 0);

        let json = serde_json::to_value(&grid).unwrap();
        assert_eq!(json["cells"][0]["items"][0]["labels_hidden"], true);
        assert!(json["cells"][0]["items"][1].get("labels_hidden").is_none());
        let mut loaded: SavedGrid = serde_json::from_value(json).unwrap();
        assert!(!loaded.labels_at(0, 0));
        assert!(!loaded.labels_at(1, 0));
        assert!(!loaded
            .grid_at(1, 0)
            .unwrap()
            .lock()
            .unwrap()
            .labels_at(2, 0));
        loaded.cycle_stack(0, 0, 1);
        assert!(loaded.labels_at(0, 0));
        assert_eq!(loaded.kind_at(0, 0), Some(CellKind::Audio));

        let old: SavedGrid =
            serde_json::from_str(r#"[{"col":0,"row":0,"content":{"Image":"old.png"}}]"#).unwrap();
        assert!(old.labels_at(0, 0));
        assert_eq!(old.kind_at(0, 0), Some(CellKind::Image));
    }

    #[test]
    fn moves_swaps_and_rewrites_paths_recursively() {
        let mut grid = SavedGrid::new();
        grid.insert_file(0, 0, PathBuf::from("/old/profile/a.png"))
            .unwrap();
        grid.insert_file(1, 0, PathBuf::from("/elsewhere/b.mp3"))
            .unwrap();
        let nested = grid.create_grid(2, 0).unwrap();
        nested
            .lock()
            .unwrap()
            .insert_file(0, 0, PathBuf::from("/old/profile/c.txt"))
            .unwrap();

        assert_eq!(grid.next_free_in_row(0, 0), 3);
        assert!(matches!(
            grid.move_cell((0, 0), (1, 0)),
            Err(SaveError::CellOccupied { col: 1, row: 0 })
        ));
        grid.move_cell((0, 0), (5, 5)).unwrap();
        assert_eq!(grid.kind_at(5, 5), Some(CellKind::Image));
        assert!(!grid.occupied(0, 0));

        grid.swap_cells((5, 5), (1, 0));
        assert_eq!(grid.kind_at(5, 5), Some(CellKind::Audio));
        assert_eq!(grid.kind_at(1, 0), Some(CellKind::Image));
        grid.swap_cells((1, 0), (9, 9));
        assert!(!grid.occupied(1, 0));
        assert_eq!(grid.kind_at(9, 9), Some(CellKind::Image));

        let changed = grid.rewrite_path_prefix(Path::new("/old/profile"), Path::new("/new/p"));
        assert_eq!(changed, 2);
        assert_eq!(grid.file_at(9, 9), Some(Path::new("/new/p/a.png")));
        assert_eq!(grid.file_at(5, 5), Some(Path::new("/elsewhere/b.mp3")));
        assert_eq!(
            nested.lock().unwrap().file_at(0, 0),
            Some(Path::new("/new/p/c.txt"))
        );

        grid.clear();
        assert!(grid.is_empty());
    }

    #[test]
    fn accepts_supported_image_paths_case_insensitively() {
        assert!(SavedGrid::is_image_path(Path::new("thing.PNG")));
        assert!(SavedGrid::is_image_path(Path::new("thing.jpeg")));
        assert!(!SavedGrid::is_image_path(Path::new("thing.txt")));
        assert!(!SavedGrid::is_image_path(Path::new("thing")));
    }

    #[test]
    fn classifies_supported_file_categories_case_insensitively() {
        assert_eq!(
            SavedGrid::classify_path(Path::new("song.FLAC")),
            CellKind::Audio
        );
        assert_eq!(
            SavedGrid::classify_path(Path::new("movie.webm")),
            CellKind::Video
        );
        assert_eq!(
            SavedGrid::classify_path(Path::new("automation.RB")),
            CellKind::RubyScript
        );
        assert_eq!(
            SavedGrid::classify_path(Path::new("archive.zip")),
            CellKind::File
        );
    }

    #[test]
    fn round_trips_arbitrary_files() {
        let path = unique_file();
        let mut grid = SavedGrid::new();
        grid.insert_file(1, 2, PathBuf::from("music/song.mp3"))
            .unwrap();
        grid.insert_file(3, 4, PathBuf::from("scripts/layout.rb"))
            .unwrap();
        grid.save(&path).unwrap();

        let loaded = SavedGrid::load(&path).unwrap().unwrap();
        assert_eq!(loaded.kind_at(1, 2), Some(CellKind::Audio));
        assert_eq!(loaded.file_at(1, 2), Some(Path::new("music/song.mp3")));
        assert_eq!(loaded.kind_at(3, 4), Some(CellKind::RubyScript));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn round_trips_images_at_negative_and_positive_coordinates() {
        let path = unique_file();
        let mut grid = SavedGrid::new();
        grid.insert_image(-3, 4, PathBuf::from("images/star.jpg"))
            .unwrap();
        grid.insert_image(0, 0, PathBuf::from("images/origin.png"))
            .unwrap();
        grid.save(&path).unwrap();

        let loaded = SavedGrid::load(&path).unwrap().unwrap();
        assert_eq!(loaded.image_at(-3, 4), Some(Path::new("images/star.jpg")));
        assert_eq!(loaded.image_at(0, 0), Some(Path::new("images/origin.png")));
        assert_eq!(loaded.len(), 2);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn fanout_inserts_images_replaces_images_and_skips_grids() {
        let mut grid = SavedGrid::new();
        grid.insert_image(1, 5, PathBuf::from("old.png")).unwrap();
        grid.create_grid(2, 5).unwrap();

        let inserted = grid.insert_images_fanout(
            1,
            5,
            [
                "fresh.jpg",
                "nested-blocked.png",
                "ignored.txt",
                "third.png",
            ],
        );

        assert_eq!(inserted, 2);
        assert_eq!(grid.image_at(1, 5), Some(Path::new("fresh.jpg")));
        assert_eq!(grid.kind_at(2, 5), Some(CellKind::Grid));
        assert_eq!(grid.image_at(3, 5), Some(Path::new("third.png")));
    }

    #[test]
    fn nested_grids_share_mutations_through_the_parent() {
        let mut root = SavedGrid::new();
        let nested = root.create_grid(1, 1).unwrap();
        nested
            .lock()
            .unwrap()
            .insert_image(0, 0, PathBuf::from("nested.png"))
            .unwrap();

        let refetched = root.grid_at(1, 1).unwrap();
        assert_eq!(
            refetched.lock().unwrap().image_at(0, 0),
            Some(Path::new("nested.png"))
        );
    }

    #[test]
    fn round_trips_nested_grids_through_json() {
        let path = unique_file();
        let mut root = SavedGrid::new();
        let nested = root.create_grid(2, 2).unwrap();
        nested
            .lock()
            .unwrap()
            .insert_image(-1, -1, PathBuf::from("deep.png"))
            .unwrap();
        root.save(&path).unwrap();

        let loaded = SavedGrid::load(&path).unwrap().unwrap();
        assert_eq!(loaded.kind_at(2, 2), Some(CellKind::Grid));
        let loaded_nested = loaded.grid_at(2, 2).unwrap();
        assert_eq!(
            loaded_nested.lock().unwrap().image_at(-1, -1),
            Some(Path::new("deep.png"))
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn collects_all_items_recursively() {
        let mut root = SavedGrid::new();
        root.insert_image(0, 0, PathBuf::from("root.png")).unwrap();
        let nested = root.create_grid(1, 0).unwrap();
        nested
            .lock()
            .unwrap()
            .insert_image(5, 5, PathBuf::from("child.png"))
            .unwrap();

        let items = root.collect_all_items("");
        assert_eq!(items.len(), 3); // root.png, nested grid, child.png
        assert_eq!(items[0].0, "Root");
        assert_eq!(items[0].1, 0);
        assert_eq!(items[0].2, 0);
        assert_eq!(items[0].3, CellKind::Image);

        assert_eq!(items[1].0, "Root");
        assert_eq!(items[1].1, 1);
        assert_eq!(items[1].2, 0);
        assert_eq!(items[1].3, CellKind::Grid);

        assert_eq!(items[2].0, "Grid (1, 0)");
        assert_eq!(items[2].1, 5);
        assert_eq!(items[2].2, 5);
        assert_eq!(items[2].3, CellKind::Image);
    }

    #[test]
    fn stacks_push_cycle_merge_and_unstack() {
        let mut grid = SavedGrid::new();
        grid.insert_file(0, 0, PathBuf::from("a.png")).unwrap();
        assert_eq!(grid.push_file(0, 0, PathBuf::from("b.mp3")), 2);
        assert_eq!(grid.kind_at(0, 0), Some(CellKind::Audio));
        assert_eq!(grid.push_files(0, 0, ["c.txt", "d.rb"]), 2);
        let names: Vec<_> = grid
            .items_at(0, 0)
            .iter()
            .map(|item| item.path().unwrap().to_str().unwrap().to_owned())
            .collect();
        assert_eq!(names, ["c.txt", "d.rb", "b.mp3", "a.png"]);
        assert_eq!(grid.len(), 1);
        assert_eq!(grid.item_count(), 4);

        assert!(grid.cycle_stack(0, 0, 1));
        assert_eq!(grid.file_at(0, 0), Some(Path::new("d.rb")));
        assert!(grid.cycle_stack(0, 0, -1));
        assert_eq!(grid.file_at(0, 0), Some(Path::new("c.txt")));
        assert!(grid.raise_item(0, 0, 3));
        assert_eq!(grid.file_at(0, 0), Some(Path::new("a.png")));

        grid.insert_file(5, 0, PathBuf::from("e.png")).unwrap();
        assert_eq!(grid.merge_cells((5, 0), (0, 0)), 1);
        assert!(!grid.occupied(5, 0));
        assert_eq!(grid.stack_len(0, 0), 5);
        assert_eq!(grid.file_at(0, 0), Some(Path::new("e.png")));

        let nested = grid.push_grid(0, 0);
        assert!(Arc::ptr_eq(&grid.grid_at(0, 0).unwrap(), &nested));
        grid.cycle_stack(0, 0, 1);
        assert_eq!(grid.find_grid(&nested), Some((0, 0)));
        assert_eq!(grid.total_len(), 6);

        grid.insert_file(2, 0, PathBuf::from("blocker.txt"))
            .unwrap();
        let placed = grid.unstack(0, 0);
        assert_eq!(placed, [(1, 0), (3, 0), (4, 0), (5, 0), (6, 0)]);
        assert_eq!(grid.stack_len(0, 0), 1);
        assert_eq!(grid.len(), 7);

        assert!(grid.pop_item(0, 0, 1).is_none());
        assert!(grid.pop_item(0, 0, 0).is_some());
        assert!(!grid.occupied(0, 0));
    }

    #[test]
    fn grid_ids_round_trip_and_folders_are_created_on_use() {
        let path = unique_file();
        let root = Arc::new(Mutex::new(SavedGrid::new()));
        let nested = root.lock().unwrap().create_grid(2, 0).unwrap();
        let other = root.lock().unwrap().create_grid(3, 0).unwrap();
        assert_eq!(root.lock().unwrap().id(), None);

        let (root_folder, new) = grid_folder(&path, &root, &root).unwrap();
        assert!(new && root_folder.is_dir());
        assert!(root_folder.ends_with("root"));
        assert_eq!(root_folder.parent().unwrap(), grids_root(&path));
        let (again, new) = grid_folder(&path, &root, &root).unwrap();
        assert!(!new);
        assert_eq!(again, root_folder);

        let (nested_folder, _) = grid_folder(&path, &nested, &root).unwrap();
        let (other_folder, _) = grid_folder(&path, &other, &root).unwrap();
        assert!(nested_folder.is_dir() && other_folder.is_dir());
        assert_ne!(nested_folder, other_folder);
        assert_ne!(nested_folder, root_folder);

        // Grids that never used a folder stay id-less; ids survive a save.
        let unused = root.lock().unwrap().create_grid(4, 0).unwrap();
        root.lock().unwrap().save(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"id\": \"root\""));
        let loaded = Arc::new(Mutex::new(SavedGrid::load(&path).unwrap().unwrap()));
        assert_eq!(loaded.lock().unwrap().id(), Some("root"));
        let loaded_nested = loaded.lock().unwrap().grid_at(2, 0).unwrap();
        let (reloaded_folder, new) = grid_folder(&path, &loaded_nested, &loaded).unwrap();
        assert!(!new);
        assert_eq!(reloaded_folder, nested_folder);
        assert_eq!(unused.lock().unwrap().id(), None);
        let loaded_unused = loaded.lock().unwrap().grid_at(4, 0).unwrap();
        assert_eq!(loaded_unused.lock().unwrap().id(), None);

        // Unsafe ids in a hand-edited save are ignored.
        assert!(!valid_grid_id("../escape"));
        assert!(!valid_grid_id(""));
        assert!(valid_grid_id("grid-1a2b-0"));

        let (assets, new) = grid_assets_folder(&path, &loaded_nested, &loaded).unwrap();
        assert!(!new);
        assert_eq!(assets, nested_folder.join("assets"));
        assert!(assets.is_dir());

        fs::remove_dir_all(grids_root(&path)).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn migrates_version_2_saves_and_round_trips_stacks_and_inventory() {
        let path = unique_file();
        fs::write(
            &path,
            r#"{"version":2,"root":[
                {"col":0,"row":0,"content":{"Image":"a.png"}},
                {"col":1,"row":0,"content":{"Grid":[
                    {"col":-1,"row":2,"content":{"File":"deep.txt"}}
                ]}}
            ]}"#,
        )
        .unwrap();
        let mut grid = SavedGrid::load(&path).unwrap().unwrap();
        assert_eq!(grid.image_at(0, 0), Some(Path::new("a.png")));
        let nested = grid.grid_at(1, 0).unwrap();
        assert_eq!(
            nested.lock().unwrap().file_at(-1, 2),
            Some(Path::new("deep.txt"))
        );

        grid.push_file(0, 0, PathBuf::from("b.ogg"));
        grid.inventory_mut().add("coin", 12).unwrap();
        nested
            .lock()
            .unwrap()
            .inventory_mut()
            .add("key", 1)
            .unwrap();
        grid.save(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"version\": 3"));

        let loaded = SavedGrid::load(&path).unwrap().unwrap();
        assert_eq!(loaded.stack_len(0, 0), 2);
        assert_eq!(loaded.kind_at(0, 0), Some(CellKind::Audio));
        assert_eq!(loaded.inventory().count("coin"), 12);
        let nested = loaded.grid_at(1, 0).unwrap();
        assert_eq!(nested.lock().unwrap().inventory().count("key"), 1);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn snapshots_cover_stacks_and_inventory() {
        let mut grid = SavedGrid::new();
        grid.insert_file(0, 0, PathBuf::from("a.png")).unwrap();
        let before = grid.snapshot();
        assert!(grid.same_cells(&before));
        grid.push_file(0, 0, PathBuf::from("b.png"));
        assert!(!grid.same_cells(&before));
        let stacked = grid.snapshot();
        grid.inventory_mut().add("gem", 1).unwrap();
        assert!(!grid.same_cells(&stacked));
        grid.inventory_mut().remove("gem", 1).unwrap();
        assert!(grid.same_cells(&stacked));
    }

    #[test]
    fn rejects_occupied_cells_and_unsupported_versions() {
        let mut grid = SavedGrid::new();
        grid.insert_image(0, 0, PathBuf::from("a.png")).unwrap();
        assert!(matches!(
            grid.insert_image(0, 0, PathBuf::from("b.png")),
            Err(SaveError::CellOccupied { col: 0, row: 0 })
        ));

        let path = unique_file();
        fs::write(&path, r#"{"version":1,"root":[]}"#).unwrap();
        assert!(matches!(
            SavedGrid::load(&path),
            Err(SaveError::UnsupportedVersion(1))
        ));
        fs::remove_file(path).unwrap();
    }
}

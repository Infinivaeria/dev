//! Undo/redo for grid edits.
//!
//! Each entry remembers one grid and a shallow snapshot of its cells from
//! before the edit. Nested grids inside a snapshot are shared, so undoing
//! the deletion of a nested grid brings back its whole contents.

use std::sync::{Arc, Mutex, MutexGuard};

use crate::persistence::SavedGrid;

const LIMIT: usize = 200;

struct Entry {
    grid: Arc<Mutex<SavedGrid>>,
    cells: SavedGrid,
    label: String,
}

#[derive(Default)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
}

fn lock(grid: &Mutex<SavedGrid>) -> MutexGuard<'_, SavedGrid> {
    grid.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl History {
    /// Records an edit of `grid` if its cells now differ from `before`.
    /// Returns whether anything changed.
    pub fn record(&mut self, grid: &Arc<Mutex<SavedGrid>>, before: SavedGrid, label: &str) -> bool {
        if lock(grid).same_cells(&before) {
            return false;
        }
        self.undo.push(Entry {
            grid: Arc::clone(grid),
            cells: before,
            label: label.to_owned(),
        });
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|entry| entry.label.as_str())
    }

    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|entry| entry.label.as_str())
    }

    /// Restores the most recent edit; returns the grid it changed and the
    /// edit's label.
    pub fn undo(&mut self) -> Option<(Arc<Mutex<SavedGrid>>, String)> {
        let entry = self.undo.pop()?;
        let restored = Self::swap(entry);
        let result = (Arc::clone(&restored.grid), restored.label.clone());
        self.redo.push(restored);
        Some(result)
    }

    pub fn redo(&mut self) -> Option<(Arc<Mutex<SavedGrid>>, String)> {
        let entry = self.redo.pop()?;
        let restored = Self::swap(entry);
        let result = (Arc::clone(&restored.grid), restored.label.clone());
        self.undo.push(restored);
        Some(result)
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Puts the entry's cells into its grid and returns an entry holding
    /// the cells that were replaced.
    fn swap(entry: Entry) -> Entry {
        let mut guard = lock(&entry.grid);
        let mut cells = entry.cells;
        cells.adopt_id(guard.id().map(str::to_owned));
        let current = std::mem::replace(&mut *guard, cells);
        drop(guard);
        Entry {
            grid: entry.grid,
            cells: current,
            label: entry.label,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn grid() -> Arc<Mutex<SavedGrid>> {
        Arc::new(Mutex::new(SavedGrid::new()))
    }

    #[test]
    fn undo_keeps_a_grid_folder_id_assigned_after_the_snapshot() {
        let root = grid();
        let mut history = History::default();
        let before = lock(&root).snapshot();
        lock(&root)
            .insert_file(0, 0, PathBuf::from("/a/b.txt"))
            .unwrap();
        let (id, new) = lock(&root).ensure_id(true);
        assert!(new);
        assert!(history.record(&root, before, "Paste"));
        history.undo().unwrap();
        assert!(!lock(&root).occupied(0, 0));
        assert_eq!(lock(&root).id(), Some(id.as_str()));
        history.redo().unwrap();
        assert_eq!(lock(&root).id(), Some(id.as_str()));
    }

    #[test]
    fn undo_and_redo_restore_cells_and_nested_grids() {
        let root = grid();
        let mut history = History::default();

        let before = lock(&root).snapshot();
        let nested = lock(&root).create_grid(0, 0).unwrap();
        lock(&nested)
            .insert_file(3, 3, PathBuf::from("/a/song.mp3"))
            .unwrap();
        assert!(history.record(&root, before, "New Grid"));

        let before = lock(&root).snapshot();
        lock(&root).remove(0, 0);
        assert!(history.record(&root, before, "Delete"));
        assert!(!lock(&root).occupied(0, 0));

        let (changed, label) = history.undo().unwrap();
        assert!(Arc::ptr_eq(&changed, &root));
        assert_eq!(label, "Delete");
        let back = lock(&root).grid_at(0, 0).unwrap();
        assert!(Arc::ptr_eq(&back, &nested));
        assert!(lock(&back).occupied(3, 3));

        history.redo().unwrap();
        assert!(!lock(&root).occupied(0, 0));
        assert_eq!(history.undo_label(), Some("Delete"));
    }

    #[test]
    fn unchanged_edits_are_not_recorded_and_new_edits_clear_redo() {
        let root = grid();
        let mut history = History::default();
        let before = lock(&root).snapshot();
        assert!(!history.record(&root, before, "nothing"));
        assert!(!history.can_undo());

        let before = lock(&root).snapshot();
        lock(&root)
            .insert_file(0, 0, PathBuf::from("/a.txt"))
            .unwrap();
        history.record(&root, before, "Paste");
        history.undo();
        assert!(history.can_redo());
        let before = lock(&root).snapshot();
        lock(&root)
            .insert_file(1, 0, PathBuf::from("/b.txt"))
            .unwrap();
        history.record(&root, before, "Drop");
        assert!(!history.can_redo());
    }
}

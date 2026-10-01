//! Find cells by name across a profile's whole grid tree.

use std::path::Path;

use crate::persistence::SavedGrid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// Cells of the nested grids to enter, starting from the root.
    pub path: Vec<(i64, i64)>,
    pub cell: (i64, i64),
    /// Position of the matching item in the cell's stack (0 = top).
    pub index: usize,
    pub name: String,
}

impl Hit {
    pub fn location(&self) -> String {
        let mut text = String::from("Root");
        for (col, row) in &self.path {
            text.push_str(&format!(" › ({col},{row})"));
        }
        text
    }
}

/// Case-insensitive search of file names, full paths and kind names
/// (`image`, `audio`, `video`, `ruby`, `file`, `grid`). Results are in a
/// stable order: shallow grids first, then by row and column.
pub fn search(root: &SavedGrid, query: &str) -> Vec<Hit> {
    let needle = query.trim().to_lowercase();
    let mut hits = Vec::new();
    if !needle.is_empty() {
        walk(root, &needle, &mut Vec::new(), &mut hits, 0);
    }
    hits.sort_by(|a, b| {
        (a.path.len(), &a.path, a.cell.1, a.cell.0, a.index).cmp(&(
            b.path.len(),
            &b.path,
            b.cell.1,
            b.cell.0,
            b.index,
        ))
    });
    hits
}

const MAX_DEPTH: usize = 64;

fn walk(
    grid: &SavedGrid,
    needle: &str,
    path: &mut Vec<(i64, i64)>,
    hits: &mut Vec<Hit>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }
    for ((col, row), items) in grid.stacks() {
        for (index, item) in items.iter().enumerate() {
            let kind = item.kind();
            let file = item.path();
            let name = file
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("grid ({col},{row})"));
            let full = file
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            if name.to_lowercase().contains(needle)
                || full.to_lowercase().contains(needle)
                || kind.name() == needle
            {
                hits.push(Hit {
                    path: path.clone(),
                    cell: (col, row),
                    index,
                    name,
                });
            }
        }
        if let Some(nested) = grid.grid_in(col, row) {
            let nested = nested
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            path.push((col, row));
            walk(&nested, needle, path, hits, depth + 1);
            path.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn finds_names_kinds_and_nested_cells() {
        let mut root = SavedGrid::new();
        root.insert_file(0, 0, PathBuf::from("/music/Song.MP3"))
            .unwrap();
        root.insert_file(1, 0, PathBuf::from("/notes/todo.txt"))
            .unwrap();
        let nested = root.create_grid(2, 1).unwrap();
        nested
            .lock()
            .unwrap()
            .insert_file(5, -1, PathBuf::from("/music/other-song.flac"))
            .unwrap();

        let hits = search(&root, "song");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].cell, (0, 0));
        assert_eq!(hits[1].path, vec![(2, 1)]);
        assert_eq!(hits[1].cell, (5, -1));
        assert_eq!(hits[1].location(), "Root › (2,1)");

        assert_eq!(search(&root, "audio").len(), 2);
        assert_eq!(search(&root, "grid").len(), 1);
        assert_eq!(search(&root, "/notes").len(), 1);
        assert!(search(&root, "  ").is_empty());

        root.push_file(1, 0, PathBuf::from("/music/buried-song.ogg"));
        root.cycle_stack(1, 0, 1);
        let hits = search(&root, "buried");
        assert_eq!(hits.len(), 1);
        assert_eq!((hits[0].cell, hits[0].index), ((1, 0), 1));
    }
}

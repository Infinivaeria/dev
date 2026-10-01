//! Multi-cell selection: pure logic so it can be unit-tested without a
//! window. The app keeps a *primary* cell (`App::selected`, the one the
//! keyboard, info panel and single-cell actions use) and this set of extra
//! cells for group operations.

use std::collections::{BTreeSet, HashSet};

pub type Cell = (i64, i64);

/// Rectangle selections larger than this are refused so a stray drag
/// across a zoomed-out grid can't allocate millions of cells.
pub const MAX_RECT_CELLS: i64 = 65_536;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    cells: BTreeSet<Cell>,
    anchor: Option<Cell>,
}

impl Selection {
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// True when more than one cell is selected (group operations apply).
    pub fn is_multi(&self) -> bool {
        self.cells.len() > 1
    }

    pub fn contains(&self, cell: Cell) -> bool {
        self.cells.contains(&cell)
    }

    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        self.cells.iter().copied()
    }

    pub fn anchor(&self) -> Option<Cell> {
        self.anchor
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.anchor = None;
    }

    /// Plain click: exactly one cell (or nothing).
    pub fn set_single(&mut self, cell: Option<Cell>) {
        self.cells.clear();
        self.cells.extend(cell);
        self.anchor = cell;
    }

    /// Ctrl+click. `primary` is the currently focused cell, which joins the
    /// set first so Ctrl+clicking after a plain click selects both. Returns
    /// the new primary cell.
    pub fn toggle(&mut self, primary: Option<Cell>, cell: Cell) -> Option<Cell> {
        if self.cells.is_empty() {
            self.cells.extend(primary);
        }
        if self.cells.remove(&cell) {
            let next = self.cells.iter().next_back().copied();
            self.anchor = next;
            next
        } else {
            self.cells.insert(cell);
            self.anchor = Some(cell);
            Some(cell)
        }
    }

    /// Shift+click: the rectangle from the anchor (or `primary`) to `cell`.
    /// With `additive` (Ctrl+Shift) the rectangle is added to the set.
    pub fn extend_to(
        &mut self,
        primary: Option<Cell>,
        cell: Cell,
        additive: bool,
    ) -> Result<(), String> {
        let anchor = self.anchor.or(primary).unwrap_or(cell);
        let rect = rect_cells(anchor, cell)?;
        if !additive {
            self.cells.clear();
        }
        self.cells.extend(rect);
        self.anchor = Some(anchor);
        Ok(())
    }

    /// Rubber-band selection between two corners.
    pub fn set_rect(&mut self, a: Cell, b: Cell, additive: bool) -> Result<(), String> {
        let rect = rect_cells(a, b)?;
        if !additive {
            self.cells.clear();
        }
        self.cells.extend(rect);
        self.anchor = Some(a);
        Ok(())
    }

    pub fn select_all(&mut self, cells: impl IntoIterator<Item = Cell>) {
        self.cells = cells.into_iter().collect();
        self.anchor = self.cells.iter().next().copied();
    }

    /// Makes `cell` the anchor (adding it to the set if needed).
    #[cfg_attr(not(feature = "scripting"), allow(dead_code))]
    pub fn set_anchor(&mut self, cell: Cell) {
        self.cells.insert(cell);
        self.anchor = Some(cell);
    }

    pub fn translate(&mut self, (dx, dy): (i64, i64)) {
        self.cells = self
            .cells
            .iter()
            .map(|&(col, row)| (col + dx, row + dy))
            .collect();
        self.anchor = self.anchor.map(|(col, row)| (col + dx, row + dy));
    }
}

/// Every cell of the inclusive rectangle spanned by `a` and `b`.
pub fn rect_cells(a: Cell, b: Cell) -> Result<Vec<Cell>, String> {
    let (min_col, max_col) = (a.0.min(b.0), a.0.max(b.0));
    let (min_row, max_row) = (a.1.min(b.1), a.1.max(b.1));
    let width = max_col.saturating_sub(min_col).saturating_add(1);
    let height = max_row.saturating_sub(min_row).saturating_add(1);
    if width.saturating_mul(height) > MAX_RECT_CELLS {
        return Err(format!(
            "selection of {width}×{height} cells is too large (max {MAX_RECT_CELLS})"
        ));
    }
    Ok((min_row..=max_row)
        .flat_map(|row| (min_col..=max_col).map(move |col| (col, row)))
        .collect())
}

/// Inclusive bounding box `(min, max)` of some cells.
pub fn bounds(cells: impl IntoIterator<Item = Cell>) -> Option<(Cell, Cell)> {
    cells.into_iter().fold(None, |acc, (col, row)| match acc {
        None => Some(((col, row), (col, row))),
        Some(((min_col, min_row), (max_col, max_row))) => Some((
            (min_col.min(col), min_row.min(row)),
            (max_col.max(col), max_row.max(row)),
        )),
    })
}

/// Plans moving `cells` by `offset`. A destination may only be occupied by
/// a cell that is itself moving. Returns `(from, to)` pairs, or the first
/// blocking destination.
pub fn plan_move(
    cells: impl IntoIterator<Item = Cell>,
    (dx, dy): (i64, i64),
    occupied: impl Fn(Cell) -> bool,
) -> Result<Vec<(Cell, Cell)>, Cell> {
    let moving: Vec<Cell> = cells.into_iter().filter(|cell| occupied(*cell)).collect();
    let sources: HashSet<Cell> = moving.iter().copied().collect();
    let mut moves = Vec::with_capacity(moving.len());
    for (col, row) in moving {
        let to = (col + dx, row + dy);
        if occupied(to) && !sources.contains(&to) {
            return Err(to);
        }
        moves.push(((col, row), to));
    }
    Ok(moves)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_ctrl_click_and_shift_click() {
        let mut selection = Selection::default();
        selection.set_single(Some((0, 0)));
        assert_eq!(selection.len(), 1);

        let primary = selection.toggle(Some((0, 0)), (2, 0));
        assert_eq!(primary, Some((2, 0)));
        assert!(selection.contains((0, 0)) && selection.contains((2, 0)));
        assert!(selection.is_multi());

        let primary = selection.toggle(primary, (2, 0));
        assert_eq!(primary, Some((0, 0)));
        assert_eq!(selection.len(), 1);

        selection.set_single(Some((1, 1)));
        selection.extend_to(Some((1, 1)), (3, 2), false).unwrap();
        assert_eq!(selection.len(), 6);
        assert_eq!(selection.anchor(), Some((1, 1)));
        // Shift+click again re-anchors on the original corner.
        selection.extend_to(Some((3, 2)), (0, 1), false).unwrap();
        assert_eq!(selection.cells().collect::<Vec<_>>(), vec![(0, 1), (1, 1)]);
        selection.extend_to(None, (1, 3), true).unwrap();
        assert_eq!(selection.len(), 4);

        selection.translate((10, -1));
        assert!(selection.contains((10, 0)));
        assert_eq!(selection.len(), 4);
        selection.set_anchor((20, 20));
        assert_eq!(selection.anchor(), Some((20, 20)));
        assert_eq!(selection.len(), 5);

        selection.clear();
        assert!(selection.is_empty());
    }

    #[test]
    fn rects_are_capped_and_bounds_work() {
        assert_eq!(rect_cells((2, 2), (0, 1)).unwrap().len(), 6);
        assert!(rect_cells((0, 0), (1_000, 1_000)).is_err());
        assert!(rect_cells((i64::MIN, 0), (i64::MAX, 0)).is_err());
        assert_eq!(bounds([(3, 1), (-2, 5)]), Some(((-2, 1), (3, 5))));
        assert_eq!(bounds([]), None);
    }

    #[test]
    fn group_moves_detect_blockers_and_allow_overlap() {
        let occupied: HashSet<Cell> = [(0, 0), (1, 0), (5, 0)].into_iter().collect();
        let is_occupied = |cell: Cell| occupied.contains(&cell);
        // Shifting right by one overlaps itself, which is fine.
        let moves = plan_move([(0, 0), (1, 0), (9, 9)], (1, 0), is_occupied).unwrap();
        assert_eq!(moves, vec![((0, 0), (1, 0)), ((1, 0), (2, 0))]);
        // (5, 0) is occupied by something that isn't moving.
        assert_eq!(
            plan_move([(0, 0), (1, 0)], (4, 0), is_occupied),
            Err((5, 0))
        );
    }

    #[test]
    fn group_move_edge_cases() {
        let occupied: HashSet<Cell> = [(0, 0), (1, 0), (0, 1)].into_iter().collect();
        let is_occupied = |cell: Cell| occupied.contains(&cell);
        // A zero offset is a no-op move of every occupied cell onto itself.
        assert_eq!(
            plan_move([(0, 0), (1, 0)], (0, 0), is_occupied).unwrap(),
            vec![((0, 0), (0, 0)), ((1, 0), (1, 0))]
        );
        // Only empty cells selected: nothing to move.
        assert!(plan_move([(7, 7), (8, 8)], (1, 1), is_occupied)
            .unwrap()
            .is_empty());
        // Moving up-left into negative coordinates is fine.
        assert_eq!(
            plan_move([(0, 0), (0, 1)], (-3, -10), is_occupied).unwrap(),
            vec![((0, 0), (-3, -10)), ((0, 1), (-3, -9))]
        );
        // Shifting a column down onto its own cells is allowed.
        assert!(plan_move([(0, 0), (0, 1)], (0, 1), is_occupied).is_ok());
        // Moving only (0, 0) right is blocked by the unselected (1, 0).
        assert_eq!(plan_move([(0, 0)], (1, 0), is_occupied), Err((1, 0)));
    }

    #[test]
    fn select_all_then_ctrl_click_removes_one() {
        let mut selection = Selection::default();
        selection.select_all([(0, 0), (1, 0), (2, 0)]);
        assert_eq!(selection.len(), 3);
        let primary = selection.toggle(Some((0, 0)), (1, 0));
        assert!(!selection.contains((1, 0)));
        assert_eq!(selection.len(), 2);
        assert!(primary.is_some_and(|cell| selection.contains(cell)));
        // The drag box is additive when requested.
        selection.set_rect((5, 5), (6, 5), true).unwrap();
        assert_eq!(selection.len(), 4);
        selection.set_rect((5, 5), (6, 5), false).unwrap();
        assert_eq!(selection.len(), 2);
    }
}

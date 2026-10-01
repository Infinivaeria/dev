//! Infinite camera: world-space pan/zoom math shared by every grid window.
//!
//! The camera's `(x, y)` is the world-space point shown at the center of
//! the viewport; `zoom` is screen pixels per world unit. Zoom is clamped to
//! a very wide but finite range to avoid floating-point degeneracy while
//! still behaving like "infinite" zoom in practice.

use std::{error::Error, fmt};

const MIN_ZOOM: f64 = 1e-6;
const MAX_ZOOM: f64 = 1e6;
pub const DOUBLE_CLICK_SECONDS: f64 = 0.35;

#[derive(Debug)]
pub enum CameraError {
    InvalidViewport,
    InvalidCellSize,
    InvalidFactor,
}

impl fmt::Display for CameraError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidViewport => write!(formatter, "viewport dimensions must be positive"),
            Self::InvalidCellSize => write!(formatter, "cell size must be positive"),
            Self::InvalidFactor => write!(formatter, "zoom factor must be positive"),
        }
    }
}

impl Error for CameraError {}

fn check_viewport(viewport: [f64; 2]) -> Result<(), CameraError> {
    if viewport[0] > 0.0 && viewport[1] > 0.0 {
        Ok(())
    } else {
        Err(CameraError::InvalidViewport)
    }
}

fn check_cell_size(cell_size: f64) -> Result<(), CameraError> {
    if cell_size > 0.0 {
        Ok(())
    } else {
        Err(CameraError::InvalidCellSize)
    }
}

#[derive(Clone, Copy, Debug)]
struct ClickState {
    col: i64,
    row: i64,
    time: f64,
}

#[derive(Clone, Debug)]
pub struct Camera {
    x: f64,
    y: f64,
    zoom: f64,
    last_click: Option<ClickState>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
            last_click: None,
        }
    }
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn x(&self) -> f64 {
        self.x
    }

    #[allow(dead_code)]
    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// Pans by a screen-space pixel delta (e.g. mouse delta while a button
    /// is held), converting to world units via the current zoom.
    pub fn pan(&mut self, screen_dx: f64, screen_dy: f64) {
        self.x -= screen_dx / self.zoom;
        self.y -= screen_dy / self.zoom;
    }

    /// Centers the view on the middle of a cell, keeping the zoom.
    pub fn center_on_cell(&mut self, col: i64, row: i64, cell_size: f64) {
        self.x = (col as f64 + 0.5) * cell_size;
        self.y = (row as f64 + 0.5) * cell_size;
    }

    /// Centers on the inclusive cell rectangle `min..=max` and zooms so it
    /// fills `viewport` (minus `margin` pixels on every side).
    pub fn fit_cells(
        &mut self,
        min: [i64; 2],
        max: [i64; 2],
        viewport: [f64; 2],
        cell_size: f64,
        margin: f64,
    ) -> Result<(), CameraError> {
        check_viewport(viewport)?;
        check_cell_size(cell_size)?;
        let cols = (max[0] - min[0]).unsigned_abs() as f64 + 1.0;
        let rows = (max[1] - min[1]).unsigned_abs() as f64 + 1.0;
        let usable = [
            (viewport[0] - 2.0 * margin).max(viewport[0] * 0.25),
            (viewport[1] - 2.0 * margin).max(viewport[1] * 0.25),
        ];
        self.zoom = (usable[0] / (cols * cell_size))
            .min(usable[1] / (rows * cell_size))
            .clamp(MIN_ZOOM, MAX_ZOOM);
        self.x = (min[0].min(max[0]) as f64 + cols / 2.0) * cell_size;
        self.y = (min[1].min(max[1]) as f64 + rows / 2.0) * cell_size;
        Ok(())
    }

    /// Remembers the latest clicked cell and reports whether the current
    /// click qualifies as a double-click on the same cell.
    pub fn register_click(&mut self, col: i64, row: i64, now: f64) -> bool {
        let is_double = self.last_click.is_some_and(|last| {
            last.col == col && last.row == row && (now - last.time) <= DOUBLE_CLICK_SECONDS
        });
        self.last_click = Some(ClickState {
            col,
            row,
            time: now,
        });
        is_double
    }

    /// Multiplies the zoom by `factor` while keeping the world point under
    /// `screen_point` fixed on screen -- the standard "zoom toward cursor"
    /// technique. `factor > 1.0` zooms in, `factor < 1.0` zooms out.
    pub fn zoom_at(
        &mut self,
        screen_point: [f64; 2],
        viewport: [f64; 2],
        factor: f64,
    ) -> Result<(), CameraError> {
        check_viewport(viewport)?;
        if !(factor > 0.0) {
            return Err(CameraError::InvalidFactor);
        }
        let before = self.screen_to_world(screen_point, viewport);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.screen_to_world(screen_point, viewport);
        self.x += before[0] - after[0];
        self.y += before[1] - after[1];
        Ok(())
    }

    pub fn screen_to_world(&self, screen_point: [f64; 2], viewport: [f64; 2]) -> [f64; 2] {
        let center = [viewport[0] / 2.0, viewport[1] / 2.0];
        [
            self.x + (screen_point[0] - center[0]) / self.zoom,
            self.y + (screen_point[1] - center[1]) / self.zoom,
        ]
    }

    pub fn world_to_screen(&self, world_point: [f64; 2], viewport: [f64; 2]) -> [f64; 2] {
        let center = [viewport[0] / 2.0, viewport[1] / 2.0];
        [
            center[0] + (world_point[0] - self.x) * self.zoom,
            center[1] + (world_point[1] - self.y) * self.zoom,
        ]
    }

    /// Converts a screen point to the integer `(col, row)` cell it falls in.
    pub fn cell_at(
        &self,
        screen_point: [f64; 2],
        viewport: [f64; 2],
        cell_size: f64,
    ) -> Result<[i64; 2], CameraError> {
        check_viewport(viewport)?;
        check_cell_size(cell_size)?;
        let world = self.screen_to_world(screen_point, viewport);
        Ok([
            (world[0] / cell_size).floor() as i64,
            (world[1] / cell_size).floor() as i64,
        ])
    }

    /// The on-screen `[x, y, width, height]` rectangle for a cell.
    pub fn cell_rect(
        &self,
        cell: [i64; 2],
        viewport: [f64; 2],
        cell_size: f64,
    ) -> Result<[f64; 4], CameraError> {
        check_viewport(viewport)?;
        check_cell_size(cell_size)?;
        let world_origin = [cell[0] as f64 * cell_size, cell[1] as f64 * cell_size];
        let screen_origin = self.world_to_screen(world_origin, viewport);
        let size = cell_size * self.zoom;
        Ok([screen_origin[0], screen_origin[1], size, size])
    }

    /// The inclusive `(min_cell, max_cell)` bounding box of cells visible
    /// within the viewport, for windowed rendering.
    pub fn visible_range(
        &self,
        viewport: [f64; 2],
        cell_size: f64,
    ) -> Result<([i64; 2], [i64; 2]), CameraError> {
        let top_left = self.cell_at([0.0, 0.0], viewport, cell_size)?;
        let bottom_right = self.cell_at(viewport, viewport, cell_size)?;
        Ok((
            [
                top_left[0].min(bottom_right[0]),
                top_left[1].min(bottom_right[1]),
            ],
            [
                top_left[0].max(bottom_right[0]),
                top_left[1].max(bottom_right[1]),
            ],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{Camera, DOUBLE_CLICK_SECONDS};

    #[test]
    fn pans_by_screen_space_delta_scaled_by_zoom() {
        let mut camera = Camera::new();
        camera.zoom = 2.0;
        camera.pan(20.0, -10.0);
        assert_eq!((camera.x(), camera.y()), (-10.0, 5.0));
    }

    #[test]
    fn register_click_detects_double_clicks_per_cell() {
        let mut camera = Camera::new();
        assert!(!camera.register_click(1, 2, 10.0));
        assert!(camera.register_click(1, 2, 10.0 + (DOUBLE_CLICK_SECONDS / 2.0)));
        assert!(!camera.register_click(2, 2, 10.1));
        assert!(!camera.register_click(2, 2, 11.0));
    }

    #[test]
    fn zoom_at_keeps_the_cursor_world_point_fixed() {
        let mut camera = Camera::new();
        let viewport = [800.0, 600.0];
        let cursor = [500.0, 200.0];
        let before = camera.screen_to_world(cursor, viewport);

        camera.zoom_at(cursor, viewport, 3.0).unwrap();
        let after = camera.screen_to_world(cursor, viewport);

        assert!((before[0] - after[0]).abs() < 1e-9);
        assert!((before[1] - after[1]).abs() < 1e-9);
        assert!((camera.zoom() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn cell_at_round_trips_through_cell_rect() {
        let camera = Camera::new();
        let viewport = [800.0, 600.0];
        let cell_size = 64.0;

        for cell in [[0, 0], [5, -3], [-12, 9]] {
            let rect = camera.cell_rect(cell, viewport, cell_size).unwrap();
            let center = [rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0];
            assert_eq!(camera.cell_at(center, viewport, cell_size).unwrap(), cell);
        }
    }

    #[test]
    fn visible_range_contains_the_viewport_center_cell() {
        let mut camera = Camera::new();
        camera.pan(640.0, 320.0);
        let viewport = [800.0, 600.0];
        let cell_size = 64.0;

        let (min_cell, max_cell) = camera.visible_range(viewport, cell_size).unwrap();
        let center_cell = camera.cell_at([400.0, 300.0], viewport, cell_size).unwrap();
        assert!(center_cell[0] >= min_cell[0] && center_cell[0] <= max_cell[0]);
        assert!(center_cell[1] >= min_cell[1] && center_cell[1] <= max_cell[1]);
    }

    #[test]
    fn fit_cells_shows_the_whole_rectangle() {
        let mut camera = Camera::new();
        let viewport = [800.0, 600.0];
        camera
            .fit_cells([-2, 1], [5, 3], viewport, 64.0, 40.0)
            .unwrap();
        let (min_cell, max_cell) = camera.visible_range(viewport, 64.0).unwrap();
        assert!(min_cell[0] <= -2 && min_cell[1] <= 1);
        assert!(max_cell[0] >= 5 && max_cell[1] >= 3);
        // A single cell zooms in instead of out.
        camera
            .fit_cells([0, 0], [0, 0], viewport, 64.0, 40.0)
            .unwrap();
        assert!(camera.zoom() > 1.0);
        assert!(camera
            .fit_cells([0, 0], [0, 0], [0.0, 1.0], 64.0, 0.0)
            .is_err());
    }

    #[test]
    fn rejects_non_positive_viewport_and_cell_size() {
        let camera = Camera::new();
        assert!(camera.cell_at([0.0, 0.0], [0.0, 600.0], 64.0).is_err());
        assert!(camera.cell_at([0.0, 0.0], [800.0, 600.0], 0.0).is_err());
    }
}

//! Scalable raylib notebook front end for the built-in CAS.

use crate::cas::{execute_command, Notebook, NotebookCell};
use raylib::prelude::*;
use std::{
    env,
    error::Error,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub const LOGICAL_WIDTH: f32 = 1100.0;
pub const LOGICAL_HEIGHT: f32 = 760.0;

const CELL_HEIGHT: f32 = 154.0;
const VISIBLE_RESULTS: f32 = 470.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiGeometry {
    pub scale: f32,
    pub offset: Vector2,
}

impl UiGeometry {
    pub fn from_screen(width: f32, height: f32) -> Self {
        let scale = (width.max(1.0) / LOGICAL_WIDTH).min(height.max(1.0) / LOGICAL_HEIGHT);
        Self {
            scale,
            offset: Vector2::new(
                (width - LOGICAL_WIDTH * scale) * 0.5,
                (height - LOGICAL_HEIGHT * scale) * 0.5,
            ),
        }
    }

    pub fn to_logical(self, point: Vector2) -> Vector2 {
        (point - self.offset) * (1.0 / self.scale.max(f32::EPSILON))
    }
}

pub fn ui_geometry(width: f32, height: f32) -> UiGeometry {
    UiGeometry::from_screen(width, height)
}

pub fn hit_cell(logical_point: Vector2, bounds: Rectangle) -> bool {
    bounds.check_collision_point_rec(logical_point)
}

pub fn default_notebook_path() -> PathBuf {
    let root = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    root.join("selenite").join("cas").join("notebook.json")
}

fn empty_result() -> crate::cas::CasResult {
    crate::cas::CasResult {
        operation: String::new(),
        input: String::new(),
        output: String::new(),
        steps: Vec::new(),
        error: None,
    }
}

fn next_id(cells: &[NotebookCell]) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut id = format!("cell-{nanos:x}");
    let mut suffix = 0;
    while cells.iter().any(|cell| cell.id == id) {
        suffix += 1;
        id = format!("cell-{nanos:x}-{suffix}");
    }
    id
}

struct UiState {
    notebook: Notebook,
    path: PathBuf,
    selected: Option<usize>,
    scroll: f32,
    editing: bool,
    status: String,
    error: bool,
    help_open: bool,
    close: bool,
}

impl UiState {
    fn new(path: Option<PathBuf>) -> Self {
        let path = path.unwrap_or_else(default_notebook_path);
        let mut state = Self {
            notebook: Notebook::default(),
            path,
            selected: None,
            scroll: 0.0,
            editing: false,
            status: "Ready".to_owned(),
            error: false,
            help_open: false,
            close: false,
        };
        state.load();
        if state.notebook.cells.is_empty() {
            state.new_cell();
        }
        state
    }

    fn message(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.error = false;
    }

    fn problem(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.error = true;
    }

    fn new_cell(&mut self) {
        self.notebook.cells.push(NotebookCell {
            id: next_id(&self.notebook.cells),
            input: String::new(),
            result: empty_result(),
        });
        self.selected = Some(self.notebook.cells.len() - 1);
        self.editing = true;
        self.keep_selected_visible();
        self.message("Created a new cell");
    }

    fn selected(&self) -> Option<&NotebookCell> {
        self.selected
            .and_then(|index| self.notebook.cells.get(index))
    }

    fn selected_mut(&mut self) -> Option<&mut NotebookCell> {
        self.selected
            .and_then(|index| self.notebook.cells.get_mut(index))
    }

    fn require_selection(&mut self) -> bool {
        if self.selected().is_some() {
            true
        } else {
            self.problem("No operation performed: select a cell first");
            false
        }
    }

    fn run_selected(&mut self) {
        if !self.require_selection() {
            return;
        }
        let outcome = {
            let cell = self.selected_mut().unwrap();
            cell.result = execute_command(&cell.input);
            (cell.result.error.clone(), cell.result.operation.clone())
        };
        if let Some(error) = outcome.0 {
            self.problem(format!("Cell failed: {error}"));
        } else {
            self.message(format!("Ran {}", outcome.1));
        }
    }

    fn duplicate(&mut self) {
        let Some(cell) = self.selected().cloned() else {
            self.problem("No operation performed: select a cell first");
            return;
        };
        let index = self.selected.unwrap() + 1;
        self.notebook.cells.insert(
            index,
            NotebookCell {
                id: next_id(&self.notebook.cells),
                ..cell
            },
        );
        self.selected = Some(index);
        self.keep_selected_visible();
        self.message("Duplicated cell");
    }

    fn delete_selected(&mut self) {
        let Some(index) = self.selected else {
            self.problem("No operation performed: select a cell first");
            return;
        };
        if index >= self.notebook.cells.len() {
            self.selected = None;
            self.problem("No operation performed: select a cell first");
            return;
        }
        self.notebook.cells.remove(index);
        self.selected = if self.notebook.cells.is_empty() {
            None
        } else {
            Some(index.min(self.notebook.cells.len() - 1))
        };
        self.keep_selected_visible();
        self.message("Deleted cell");
    }

    fn select_relative(&mut self, delta: isize) {
        if self.notebook.cells.is_empty() {
            self.selected = None;
            return;
        }
        let current = self.selected.unwrap_or(0) as isize;
        self.selected =
            Some((current + delta).clamp(0, self.notebook.cells.len() as isize - 1) as usize);
        self.keep_selected_visible();
    }

    fn keep_selected_visible(&mut self) {
        let Some(index) = self.selected else {
            return;
        };
        let top = index as f32 * CELL_HEIGHT;
        let bottom = top + CELL_HEIGHT;
        if top < self.scroll {
            self.scroll = top;
        } else if bottom > self.scroll + VISIBLE_RESULTS {
            self.scroll = bottom - VISIBLE_RESULTS;
        }
    }

    fn save(&mut self) {
        if let Some(parent) = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            if let Err(error) = fs::create_dir_all(parent) {
                self.problem(format!("Cannot create notebook directory: {error}"));
                return;
            }
        }
        match self.notebook.save(&self.path) {
            Ok(()) => self.message(format!("Saved {}", self.path.display())),
            Err(error) => self.problem(error),
        }
    }

    fn load(&mut self) {
        if !self.path.exists() {
            self.message(format!("Notebook will be saved to {}", self.path.display()));
            return;
        }
        match Notebook::load(&self.path) {
            Ok(notebook) => {
                self.notebook = notebook;
                self.selected = (!self.notebook.cells.is_empty()).then_some(0);
                self.scroll = 0.0;
                self.message(format!("Loaded {}", self.path.display()));
            }
            Err(error) => self.problem(error),
        }
    }

    fn apply_template(&mut self, template: &str) {
        if !self.require_selection() {
            return;
        }
        self.selected_mut().unwrap().input = template.to_owned();
        self.editing = true;
        self.message("Command template inserted");
    }

    fn input(&mut self, rl: &mut RaylibHandle) {
        let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        let dismiss_key = rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) && self.help_open;
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            if self.help_open {
                self.help_open = false;
            } else {
                self.close = true;
            }
        }
        if dismiss_key {
            while rl.get_char_pressed().is_some() {}
            return;
        }
        let geometry = ui_geometry(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let mouse = geometry.to_logical(rl.get_mouse_position());
        let help_clicked = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
            && (hit_cell(mouse, rect(1000.0, 52.0, 82.0, 32.0))
                || (self.help_open && hit_cell(mouse, rect(894.0, 548.0, 78.0, 30.0))));
        if help_clicked {
            self.help_open = !self.help_open;
            self.editing = false;
            while rl.get_char_pressed().is_some() {}
            return;
        }
        if self.help_open {
            while rl.get_char_pressed().is_some() {}
            return;
        }
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
            self.run_selected();
        } else if ctrl && rl.is_key_pressed(KeyboardKey::KEY_S) {
            self.save();
        } else if ctrl && rl.is_key_pressed(KeyboardKey::KEY_N) {
            self.new_cell();
        } else if ctrl && rl.is_key_pressed(KeyboardKey::KEY_D) {
            self.duplicate();
        } else if rl.is_key_pressed(KeyboardKey::KEY_DELETE) {
            self.delete_selected();
        } else if rl.is_key_pressed(KeyboardKey::KEY_UP) {
            self.select_relative(-1);
        } else if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
            self.select_relative(1);
        }

        if self.editing && !ctrl {
            if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
                if let Some(cell) = self.selected_mut() {
                    cell.input.pop();
                }
            }
            while let Some(character) = rl.get_char_pressed() {
                if !character.is_control() && character != '\n' && character != '\r' {
                    if let Some(cell) = self.selected_mut() {
                        if cell.input.len() < 4096 {
                            cell.input.push(character);
                        }
                    }
                }
            }
        } else {
            while rl.get_char_pressed().is_some() {}
        }

        let clicked = |bounds: Rectangle| {
            rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) && hit_cell(mouse, bounds)
        };

        if clicked(rect(18.0, 52.0, 76.0, 32.0)) {
            self.run_selected();
        }
        if clicked(rect(102.0, 52.0, 92.0, 32.0)) {
            self.new_cell();
        }
        if clicked(rect(202.0, 52.0, 82.0, 32.0)) {
            self.delete_selected();
        }
        if clicked(rect(292.0, 52.0, 92.0, 32.0)) {
            self.save();
        }
        if clicked(rect(392.0, 52.0, 80.0, 32.0)) {
            self.load();
        }
        if clicked(rect(480.0, 52.0, 96.0, 32.0)) {
            self.duplicate();
        }

        if clicked(rect(18.0, 112.0, 1064.0, 38.0)) {
            self.editing = true;
        } else if clicked(rect(18.0, 166.0, 1064.0, 582.0)) {
            self.editing = false;
        }

        for (index, template) in CATALOG_TEMPLATES.iter().enumerate() {
            let row = index / 7;
            let column = index % 7;
            let bounds = rect(
                18.0 + column as f32 * 152.0,
                166.0 + row as f32 * 34.0,
                144.0,
                28.0,
            );
            if clicked(bounds) {
                self.apply_template(template.1);
            }
        }

        let result_bounds = rect(18.0, 266.0, 1064.0, VISIBLE_RESULTS - 24.0);
        if hit_cell(mouse, result_bounds) {
            let maximum = (self.notebook.cells.len() as f32 * CELL_HEIGHT
                - (VISIBLE_RESULTS - 24.0))
                .max(0.0);
            self.scroll = (self.scroll - rl.get_mouse_wheel_move() * 42.0).clamp(0.0, maximum);
            if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
                let index = ((mouse.y - result_bounds.y + self.scroll) / CELL_HEIGHT) as usize;
                if index < self.notebook.cells.len() {
                    self.selected = Some(index);
                    self.editing = true;
                }
            }
        }
    }

    fn draw(&self, d: &mut impl RaylibDraw, geometry: UiGeometry) {
        d.clear_background(Color::new(18, 21, 28, 255));
        d.draw_text("CAS Notebook", 18, 12, 28, Color::WHITE);
        d.draw_text(
            &truncate(&self.path.display().to_string(), 78),
            720,
            20,
            14,
            Color::new(155, 164, 181, 255),
        );

        for (bounds, label) in [
            (rect(18.0, 52.0, 76.0, 32.0), "Run"),
            (rect(102.0, 52.0, 92.0, 32.0), "New Cell"),
            (rect(202.0, 52.0, 82.0, 32.0), "Delete"),
            (rect(292.0, 52.0, 92.0, 32.0), "Save"),
            (rect(392.0, 52.0, 80.0, 32.0), "Load"),
            (rect(480.0, 52.0, 96.0, 32.0), "Duplicate"),
        ] {
            button(d, bounds, label, false);
        }
        button(d, rect(1000.0, 52.0, 82.0, 32.0), "Help", self.help_open);
        d.draw_text(
            "Ctrl+Enter run   Ctrl+S save   Ctrl+N new",
            610,
            62,
            14,
            Color::new(155, 164, 181, 255),
        );

        panel(d, rect(18.0, 92.0, 1064.0, 64.0), "Selected command");
        let editor = rect(18.0, 112.0, 1064.0, 38.0);
        d.draw_rectangle_rec(
            editor,
            if self.editing {
                Color::new(31, 42, 56, 255)
            } else {
                Color::new(27, 31, 40, 255)
            },
        );
        d.draw_rectangle_lines_ex(
            editor,
            if self.editing { 2.0 } else { 1.0 },
            Color::new(76, 139, 214, 255),
        );
        let command = self
            .selected()
            .map(|cell| cell.input.as_str())
            .unwrap_or("Select a cell to edit");
        d.draw_text(
            &truncate(command, 128),
            28,
            122,
            18,
            Color::new(238, 242, 248, 255),
        );
        if self.editing {
            d.draw_text(
                "|",
                29 + text_width(command, 18, 128) as i32,
                122,
                18,
                Color::WHITE,
            );
        }

        for (index, (label, _)) in CATALOG_TEMPLATES.iter().enumerate() {
            button(
                d,
                rect(
                    18.0 + (index % 7) as f32 * 152.0,
                    166.0 + (index / 7) as f32 * 34.0,
                    144.0,
                    28.0,
                ),
                label,
                false,
            );
        }

        let viewport = rect(18.0, 242.0, 1064.0, VISIBLE_RESULTS);
        panel(d, viewport, "Cells and results");
        {
            let mut clip = d.begin_scissor_mode(
                (geometry.offset.x + viewport.x * geometry.scale) as i32,
                (geometry.offset.y + (viewport.y + 24.0) * geometry.scale) as i32,
                (viewport.width * geometry.scale) as i32,
                ((viewport.height - 24.0) * geometry.scale) as i32,
            );
            for (index, cell) in self.notebook.cells.iter().enumerate() {
                let y = viewport.y + 24.0 + index as f32 * CELL_HEIGHT - self.scroll;
                if y + CELL_HEIGHT < viewport.y || y > viewport.y + viewport.height {
                    continue;
                }
                let bounds = rect(
                    viewport.x + 8.0,
                    y,
                    viewport.width - 16.0,
                    CELL_HEIGHT - 6.0,
                );
                clip.draw_rectangle_rec(
                    bounds,
                    if self.selected == Some(index) {
                        Color::new(36, 48, 64, 255)
                    } else {
                        Color::new(29, 34, 43, 255)
                    },
                );
                clip.draw_rectangle_lines_ex(
                    bounds,
                    1.0,
                    if self.selected == Some(index) {
                        Color::new(83, 154, 235, 255)
                    } else {
                        Color::new(60, 69, 84, 255)
                    },
                );
                clip.draw_text(
                    &format!("In [{}]  {}", index + 1, truncate(&cell.input, 112)),
                    bounds.x as i32 + 10,
                    bounds.y as i32 + 8,
                    16,
                    Color::new(214, 226, 242, 255),
                );
                if let Some(error) = &cell.result.error {
                    clip.draw_text(
                        &format!("Error: {}", truncate(error, 126)),
                        bounds.x as i32 + 10,
                        bounds.y as i32 + 34,
                        15,
                        Color::new(255, 104, 112, 255),
                    );
                } else if !cell.result.output.is_empty() {
                    clip.draw_text(
                        &format!(
                            "Out [{}]  {}",
                            index + 1,
                            truncate(&cell.result.output, 112)
                        ),
                        bounds.x as i32 + 10,
                        bounds.y as i32 + 34,
                        17,
                        Color::new(116, 224, 173, 255),
                    );
                } else {
                    clip.draw_text(
                        "Not run",
                        bounds.x as i32 + 10,
                        bounds.y as i32 + 34,
                        15,
                        Color::new(145, 151, 162, 255),
                    );
                }
                for (step_index, step) in cell.result.steps.iter().take(4).enumerate() {
                    clip.draw_text(
                        &format!("{}  {}", step_index + 1, truncate(step, 120)),
                        bounds.x as i32 + 14,
                        bounds.y as i32 + 63 + step_index as i32 * 18,
                        13,
                        Color::new(161, 170, 184, 255),
                    );
                }
            }
        }
        let status_y = 724.0;
        d.draw_rectangle_rec(
            rect(18.0, status_y, 1064.0, 28.0),
            if self.error {
                Color::new(75, 28, 34, 255)
            } else {
                Color::new(28, 49, 41, 255)
            },
        );
        d.draw_text(
            &truncate(&self.status, 145),
            28,
            status_y as i32 + 7,
            14,
            if self.error {
                Color::new(255, 197, 201, 255)
            } else {
                Color::new(205, 243, 222, 255)
            },
        );
        if self.help_open {
            draw_help(d);
        }
    }
}

const CATALOG_TEMPLATES: &[(&str, &str)] = &[
    ("simplify", "simplify (x^2-1)/(x-1)"),
    ("expand", "expand (x+1)^4"),
    ("factor", "factor x^2-1"),
    ("diff", "diff x^3+2*x; x"),
    ("integrate", "integrate x^2; x"),
    ("series", "series exp(x); x; 0; 5"),
    ("solve", "solve x+y=3,x-y=1; x,y"),
    ("determinant", "determinant [[1,2],[3,4]]"),
    ("inverse", "inverse [[1,2],[3,4]]"),
    ("matrix-solve", "matrix-solve [[2,1],[1,3]]; [5,7]"),
    ("rank", "rank [[1,2],[2,4]]"),
    ("eigenvalues", "eigenvalues [[2,1],[1,2]]"),
    ("eval-at", "eval-at x^2+y; x=3,y=4"),
];

pub fn run(path: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let (mut rl, thread) = raylib::init()
        .size(LOGICAL_WIDTH as i32, LOGICAL_HEIGHT as i32)
        .title("Selenite CAS Notebook")
        .resizable()
        .build();
    rl.set_target_fps(60);
    rl.set_window_min_size(800, 560);
    rl.set_exit_key(None);
    let mut state = UiState::new(path);
    while !rl.window_should_close() && !state.close {
        state.input(&mut rl);
        let geometry = ui_geometry(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let camera = Camera2D {
            offset: geometry.offset,
            target: Vector2::zero(),
            rotation: 0.0,
            zoom: geometry.scale,
        };
        {
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::BLACK);
            let mut d = d.begin_mode2D(camera);
            state.draw(&mut d, geometry);
        }
    }
    Ok(())
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
    Rectangle::new(x, y, width, height)
}

fn panel(d: &mut impl RaylibDraw, bounds: Rectangle, title: &str) {
    d.draw_rectangle_rec(bounds, Color::new(27, 32, 42, 255));
    d.draw_rectangle_lines_ex(bounds, 1.0, Color::new(60, 69, 84, 255));
    d.draw_text(
        title,
        bounds.x as i32 + 10,
        bounds.y as i32 + 6,
        14,
        Color::new(168, 178, 194, 255),
    );
}

fn button(d: &mut impl RaylibDraw, bounds: Rectangle, label: &str, active: bool) {
    d.draw_rectangle_rec(
        bounds,
        if active {
            Color::new(43, 110, 99, 255)
        } else {
            Color::new(49, 65, 85, 255)
        },
    );
    d.draw_rectangle_lines_ex(bounds, 1.0, Color::new(83, 105, 133, 255));
    let size = if label.len() > 12 { 12 } else { 14 };
    d.draw_text(
        label,
        (bounds.x + (bounds.width - text_width(label, size, usize::MAX) as f32) / 2.0) as i32,
        (bounds.y + (bounds.height - size as f32) / 2.0) as i32,
        size,
        Color::WHITE,
    );
}

fn draw_help(d: &mut impl RaylibDraw) {
    d.draw_rectangle_rec(
        rect(0.0, 0.0, LOGICAL_WIDTH, LOGICAL_HEIGHT),
        Color::new(0, 0, 0, 190),
    );
    let bounds = rect(110.0, 166.0, 880.0, 428.0);
    panel(d, bounds, "CAS Notebook Help");
    for (index, line) in [
        "Type a command in the Selected command field, then press Ctrl+Enter or Run.",
        "Use New Cell to add a command, Duplicate to copy the selected cell, and Delete to remove it.",
        "Save and Load store the notebook as JSON. The default notebook is saved under ~/.config/selenite/cas/.",
        "Examples: simplify (x^2-1)/(x-1)   |   expand (x+1)^4   |   diff x^3; x",
        "More examples: integrate x^2; x   |   solve x+y=3,x-y=1; x,y   |   determinant [[1,2],[3,4]]",
        "Matrix commands include determinant, inverse, matrix-solve, rank and eigenvalues.",
        "Select a template to insert it into the current command. Results and steps appear in the cell list.",
        "Keyboard: Ctrl+Enter run, Ctrl+S save, Ctrl+N new cell, Ctrl+D duplicate, Delete remove, Esc close help.",
    ]
    .iter()
    .enumerate()
    {
        d.draw_text(
            line,
            bounds.x as i32 + 22,
            bounds.y as i32 + 48 + index as i32 * 38,
            15,
            Color::new(224, 231, 242, 255),
        );
    }
    button(d, rect(894.0, 548.0, 78.0, 30.0), "Close", false);
}

fn text_width(text: &str, size: i32, limit: usize) -> i32 {
    text.chars().take(limit).count() as i32 * size * 58 / 100
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        format!(
            "{}…",
            text.chars().take(max.saturating_sub(1)).collect::<String>()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_scales_without_distortion_and_letterboxes() {
        let geometry = ui_geometry(2200.0, 760.0);
        assert_eq!(geometry.scale, 1.0);
        assert_eq!(geometry.offset, Vector2::new(550.0, 0.0));
        assert_eq!(
            geometry.to_logical(Vector2::new(600.0, 20.0)),
            Vector2::new(50.0, 20.0)
        );
    }

    #[test]
    fn no_selection_is_safe_and_reported() {
        let mut state = UiState {
            notebook: Notebook::default(),
            path: PathBuf::from("notebook.json"),
            selected: None,
            scroll: 0.0,
            editing: false,
            status: String::new(),
            error: false,
            help_open: false,
            close: false,
        };
        state.run_selected();
        state.duplicate();
        state.delete_selected();
        assert!(state.error);
        assert!(state.notebook.cells.is_empty());
        assert!(state.status.contains("select a cell"));
    }
}

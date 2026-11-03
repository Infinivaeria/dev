use raylib::prelude::*;

use crate::{
    point_in_rect, ui, BUTTON_BG, BUTTON_HOVER_BG, BUTTON_TEXT, DIALOG_BG, DIALOG_BORDER,
    SELECTION_COLOR,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Close,
    Paste,
    Unstack,
    Focus(usize),
    Extract(usize),
    Select(usize),
    Open(usize),
    Remove(usize),
    ConfirmRemove(usize),
    CancelRemove,
}

pub struct Panel {
    pub cell: (i64, i64),
    pub scroll: usize,
    pub pending_remove: Option<usize>,
    pub query: String,
    pub focused: Option<usize>,
}

pub struct Layout {
    pub frame: Rectangle,
    pub visible_rows: usize,
    pub buttons: Vec<(Rectangle, String, &'static str, Action)>,
    pub row_hits: Vec<(Rectangle, usize)>,
    pub search_rect: Rectangle,
    rows: Vec<(Rectangle, String, bool)>,
    size: f32,
    title: String,
    query: String,
    message: Option<String>,
}

impl Panel {
    pub fn matching_indices(&self, names: &[String]) -> Vec<usize> {
        let query = self.query.trim().to_lowercase();
        names
            .iter()
            .enumerate()
            .filter_map(|(index, name)| name.to_lowercase().contains(&query).then_some(index))
            .collect()
    }

    pub fn navigate(&mut self, names: &[String], delta: isize, visible_rows: usize) {
        let matches = self.matching_indices(names);
        if matches.is_empty() {
            self.focused = None;
            self.scroll = 0;
            return;
        }
        let position = self
            .focused
            .and_then(|index| matches.iter().position(|candidate| *candidate == index))
            .unwrap_or(0);
        let position = position.saturating_add_signed(delta).min(matches.len() - 1);
        self.focused = Some(matches[position]);
        if position < self.scroll {
            self.scroll = position;
        } else if position >= self.scroll + visible_rows {
            self.scroll = position.saturating_sub(visible_rows.saturating_sub(1));
        }
        self.scroll = self.scroll.min(matches.len().saturating_sub(visible_rows));
    }

    pub fn layout(&self, names: &[String], width: i32, height: i32) -> Layout {
        let scale = ui::scale(width, height);
        let size = 18.0 * scale;
        let pad = 12.0 * scale;
        let row_height = 40.0 * scale;
        let panel_width = (760.0 * scale).min(width as f32 - pad * 2.0).max(1.0);
        let panel_height = (520.0 * scale).min(height as f32 - pad * 2.0).max(1.0);
        let frame = Rectangle::new(
            (width as f32 - panel_width) / 2.0,
            (height as f32 - panel_height) / 2.0,
            panel_width,
            panel_height,
        );
        let visible_rows = ((panel_height - row_height * 4.0) / row_height).max(0.0) as usize;
        let search_rect = Rectangle::new(
            frame.x + pad,
            frame.y + row_height,
            panel_width - pad * 2.0,
            row_height - 4.0 * scale,
        );
        let mut buttons = Vec::new();
        let mut rows = Vec::new();
        let mut row_hits = Vec::new();
        let matches = self.matching_indices(names);
        for (offset, index) in matches
            .into_iter()
            .skip(self.scroll)
            .take(visible_rows)
            .enumerate()
        {
            let name = &names[index];
            let rect = Rectangle::new(
                frame.x + pad,
                frame.y + row_height * (offset + 2) as f32,
                panel_width - pad * 2.0,
                row_height - 4.0 * scale,
            );
            rows.push((
                rect,
                format!("{}. {name}", index + 1),
                self.focused == Some(index) || self.pending_remove == Some(index),
            ));
            if self.pending_remove.is_some() {
                continue;
            }
            row_hits.push((rect, index));
            for (slot, label, tooltip, action) in [
                (0, "^", "Bring to top", Action::Select(index)),
                (1, ">", "Open item", Action::Open(index)),
                (2, "+", "Extract into a free cell", Action::Extract(index)),
                (3, "X", "Remove item", Action::Remove(index)),
            ] {
                buttons.push((
                    Rectangle::new(
                        rect.x + rect.width - row_height * (4 - slot) as f32,
                        rect.y,
                        row_height - 4.0 * scale,
                        rect.height,
                    ),
                    label.to_owned(),
                    tooltip,
                    action,
                ));
            }
        }
        let commands = if let Some(index) = self.pending_remove {
            vec![
                (
                    "Delete grid",
                    "Delete this grid and its contents",
                    Action::ConfirmRemove(index),
                ),
                ("Cancel", "Cancel removal", Action::CancelRemove),
            ]
        } else {
            vec![
                (
                    "Paste into stack",
                    "Add clipboard items to this cell",
                    Action::Paste,
                ),
                ("Unstack", "Spread items into free cells", Action::Unstack),
                ("Close", "Close stack inventory", Action::Close),
            ]
        };
        let button_width = (panel_width - pad * 2.0) / commands.len() as f32;
        for (index, (label, tooltip, action)) in commands.into_iter().enumerate() {
            buttons.push((
                Rectangle::new(
                    frame.x + pad + index as f32 * button_width,
                    frame.y + panel_height - row_height - pad,
                    button_width - 4.0 * scale,
                    row_height,
                ),
                label.to_owned(),
                tooltip,
                action,
            ));
        }
        let title = match self.pending_remove.and_then(|index| names.get(index)) {
            Some(name) => format!("Delete {name} and its contents?"),
            None => format!(
                "Stack items ({}, {}) - {}",
                self.cell.0,
                self.cell.1,
                names.len()
            ),
        };
        Layout {
            frame,
            visible_rows,
            buttons,
            row_hits,
            search_rect,
            rows,
            size,
            title,
            query: self.query.clone(),
            message: None,
        }
    }
}

impl Layout {
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    pub fn draw(&self, draw: &mut RaylibDrawHandle<'_>, mouse: Vector2) {
        draw.draw_rectangle(
            0,
            0,
            draw.get_screen_width(),
            draw.get_screen_height(),
            Color::new(0, 0, 0, 150),
        );
        draw.draw_rectangle_rec(self.frame, DIALOG_BG);
        draw.draw_rectangle_lines_ex(self.frame, 1.0, DIALOG_BORDER);
        let pad = self.size * 0.65;
        let title = ui::fit(&self.title, self.size, self.frame.width - pad * 2.0);
        ui::draw_text(
            draw,
            &title,
            self.frame.x + pad,
            self.frame.y + pad,
            self.size,
            BUTTON_TEXT,
        );
        draw.draw_rectangle_rec(self.search_rect, BUTTON_BG);
        draw.draw_rectangle_lines_ex(self.search_rect, 1.0, SELECTION_COLOR);
        let query = if self.query.is_empty() {
            "Search"
        } else {
            &self.query
        };
        let query = ui::fit(query, self.size, self.search_rect.width - pad * 2.0);
        ui::draw_text(
            draw,
            &query,
            self.search_rect.x + pad,
            self.search_rect.y + (self.search_rect.height - self.size) / 2.0,
            self.size,
            BUTTON_TEXT,
        );
        for (rect, label, focused) in &self.rows {
            draw.draw_rectangle_rec(*rect, if *focused { BUTTON_HOVER_BG } else { BUTTON_BG });
            let text = ui::fit(
                label,
                self.size,
                (rect.width - rect.height * 4.5 - pad).max(0.0),
            );
            ui::draw_text(
                draw,
                &text,
                rect.x + pad,
                rect.y + (rect.height - self.size) / 2.0,
                self.size,
                BUTTON_TEXT,
            );
        }
        for (rect, label, _, _) in &self.buttons {
            draw.draw_rectangle_rec(
                *rect,
                if point_in_rect(mouse, *rect) {
                    BUTTON_HOVER_BG
                } else {
                    BUTTON_BG
                },
            );
            draw.draw_rectangle_lines_ex(*rect, 1.0, SELECTION_COLOR);
            let text = ui::fit(label, self.size, (rect.width - pad).max(0.0));
            ui::draw_text_centered(draw, &text, *rect, self.size, BUTTON_TEXT);
        }
        if let Some((_, _, tooltip, _)) = self
            .buttons
            .iter()
            .find(|(rect, _, _, _)| point_in_rect(mouse, *rect))
        {
            let text = ui::fit(tooltip, self.size * 0.85, self.frame.width - pad * 2.0);
            ui::draw_text(
                draw,
                &text,
                self.frame.x + pad,
                self.frame.y + self.frame.height - self.size * 4.2,
                self.size * 0.85,
                BUTTON_TEXT,
            );
        } else if let Some(message) = &self.message {
            let text = ui::fit(message, self.size * 0.85, self.frame.width - pad * 2.0);
            ui::draw_text(
                draw,
                &text,
                self.frame.x + pad,
                self.frame.y + self.frame.height - self.size * 4.2,
                self.size * 0.85,
                BUTTON_TEXT,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtered_stack_navigation_preserves_original_indices() {
        let names = vec![
            "cover.png".into(),
            "First.MP3".into(),
            "notes.txt".into(),
            "last.mp3".into(),
        ];
        let mut panel = Panel {
            cell: (0, 0),
            scroll: 0,
            pending_remove: None,
            query: "MP3".into(),
            focused: None,
        };
        assert_eq!(panel.matching_indices(&names), vec![1, 3]);
        panel.navigate(&names, 0, 1);
        assert_eq!(panel.focused, Some(1));
        panel.navigate(&names, 1, 1);
        assert_eq!(panel.focused, Some(3));
        assert_eq!(panel.scroll, 1);
        let layout = panel.layout(&names, 640, 480);
        assert!(layout
            .buttons
            .iter()
            .any(|(_, _, _, action)| *action == Action::Extract(3)));
        panel.query = "missing".into();
        panel.navigate(&names, 0, 1);
        assert_eq!(panel.focused, None);
        assert_eq!(panel.scroll, 0);
    }

    #[test]
    fn stack_layout_fits_windows_and_scrolls_to_last_item() {
        let names = (0..100)
            .map(|index| format!("track-{index}.mp3"))
            .collect::<Vec<_>>();
        for (width, height) in [(320, 240), (640, 480), (1280, 720), (3840, 2160)] {
            let mut panel = Panel {
                cell: (0, 0),
                scroll: 0,
                pending_remove: None,
                query: String::new(),
                focused: Some(0),
            };
            let layout = panel.layout(&names, width, height);
            assert!(layout.visible_rows > 0);
            assert!(layout.frame.x >= 0.0 && layout.frame.y >= 0.0);
            assert!(layout.frame.x + layout.frame.width <= width as f32);
            assert!(layout.frame.y + layout.frame.height <= height as f32);
            for (rect, _, _, _) in &layout.buttons {
                assert!(rect.x >= layout.frame.x && rect.y >= layout.frame.y);
                assert!(rect.x + rect.width <= layout.frame.x + layout.frame.width);
                assert!(rect.y + rect.height <= layout.frame.y + layout.frame.height);
            }
            panel.scroll = names.len().saturating_sub(layout.visible_rows);
            let layout = panel.layout(&names, width, height);
            assert!(layout
                .buttons
                .iter()
                .any(|(_, _, _, action)| *action == Action::Remove(99)));
            panel.pending_remove = Some(99);
            let layout = panel.layout(&names, width, height);
            assert!(layout.title.contains("track-99.mp3"));
            assert!(!layout.buttons.iter().any(|(_, _, _, action)| matches!(
                action,
                Action::Select(_) | Action::Open(_) | Action::Remove(_)
            )));
        }
    }
}

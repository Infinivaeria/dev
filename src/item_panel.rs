//! The game-inventory panel (B key / "Items" button): a simple overlay that
//! lists the current grid's named items with +/−/delete buttons and an
//! entry field. Layout is computed once and shared by hit-testing and
//! drawing, like the other panels.

use raylib::prelude::*;

use crate::grid_inventory::GridInventory;
use crate::{
    point_in_rect, ui, BUTTON_BG, BUTTON_HOVER_BG, BUTTON_TEXT, CONSOLE_PROMPT, DANGER_BG,
    DANGER_HOVER_BG, DIALOG_BG, DIALOG_BORDER, SELECTION_COLOR,
};

const MUTED_TEXT: Color = Color::new(150, 150, 170, 255);
const ERROR_TEXT: Color = Color::new(255, 140, 140, 255);
const ROW_BG: Color = Color::new(36, 36, 44, 255);
const CURRENT_ROW_BG: Color = Color::new(45, 60, 95, 255);
const COUNT_TEXT: Color = Color::new(250, 225, 140, 255);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemAction {
    Close,
    Add,
    Select(String),
    Increment(String),
    Decrement(String),
    Delete(String),
    ClearAll,
}

pub struct ItemButton {
    pub rect: Rectangle,
    pub label: &'static str,
    pub action: ItemAction,
    pub danger: bool,
}

pub struct ItemLayout {
    pub frame: Rectangle,
    pub scale: f32,
    /// Name, row rectangle, count and a one-line metadata summary.
    pub rows: Vec<(String, Rectangle, i64, String)>,
    pub buttons: Vec<ItemButton>,
    pub input_rect: Rectangle,
    pub visible_rows: usize,
}

#[derive(Default)]
pub struct ItemPanel {
    pub input: String,
    pub scroll: usize,
    pub selected: Option<String>,
    pub message: Option<(String, bool)>,
}

/// What the entry field asks for when Enter / Add is pressed.
#[derive(Clone, Debug, PartialEq)]
pub enum EntryCommand {
    /// `name`, `name x5`, `5 name`; negative amounts remove.
    Add(String, i64),
    /// `name = 5` (0 removes the item).
    Set(String, i64),
    /// `old -> new` (merges into an existing item).
    Rename(String, String),
    /// `name @key=value`; the value is JSON when it parses, else text.
    /// `name @key=` clears the key.
    Meta(String, String, serde_json::Value),
}

pub fn parse_command(text: &str) -> Result<EntryCommand, String> {
    let text = text.trim();
    if let Some((name, rest)) = text.split_once('@') {
        let name = name.trim();
        let (key, value) = rest.split_once('=').unwrap_or((rest, ""));
        let key = key.trim();
        if name.is_empty() || key.is_empty() {
            return Err("metadata syntax: name @key=value".into());
        }
        let value = value.trim();
        let value = if value.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(value)
                .unwrap_or_else(|_| serde_json::Value::String(value.to_owned()))
        };
        return Ok(EntryCommand::Meta(name.to_owned(), key.to_owned(), value));
    }
    if let Some((from, to)) = text.split_once("->") {
        let (from, to) = (from.trim(), to.trim());
        if from.is_empty() || to.is_empty() {
            return Err("rename syntax: old -> new".into());
        }
        return Ok(EntryCommand::Rename(from.to_owned(), to.to_owned()));
    }
    if let Some((name, count)) = text.rsplit_once('=') {
        let name = name.trim();
        let count = count
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("set syntax: name = count (got {:?})", count.trim()))?;
        if name.is_empty() {
            return Err("set syntax: name = count".into());
        }
        return Ok(EntryCommand::Set(name.to_owned(), count));
    }
    parse_entry(text).map(|(name, amount)| EntryCommand::Add(name, amount))
}

/// Parses the entry field: `name`, `name 5`, `name x5`, `name ×5` or
/// `5 name`. Negative amounts remove items.
pub fn parse_entry(text: &str) -> Result<(String, i64), String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("type a name (potion x3), name = 5, old -> new, or name @key=value".into());
    }
    let amount = |token: &str| -> Option<i64> {
        let token = token
            .strip_prefix('x')
            .or_else(|| token.strip_prefix('×'))
            .or_else(|| token.strip_prefix('*'))
            .unwrap_or(token);
        token.parse::<i64>().ok()
    };
    if let Some((rest, last)) = text.rsplit_once(char::is_whitespace) {
        if let Some(count) = amount(last) {
            return Ok((rest.trim().to_owned(), count));
        }
    }
    if let Some((first, rest)) = text.split_once(char::is_whitespace) {
        if let Ok(count) = first.parse::<i64>() {
            return Ok((rest.trim().to_owned(), count));
        }
    }
    Ok((text.to_owned(), 1))
}

/// Compact `key=value, …` summary of an item's metadata.
pub fn meta_summary(inventory: &GridInventory, name: &str) -> String {
    inventory
        .get(name)
        .map(|item| {
            item.meta
                .iter()
                .map(|(key, value)| match value {
                    serde_json::Value::String(text) => format!("{key}={text}"),
                    other => format!("{key}={other}"),
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

impl ItemPanel {
    pub fn layout(
        &self,
        inventory: &GridInventory,
        screen_width: i32,
        screen_height: i32,
    ) -> ItemLayout {
        const WIDTH: f32 = 700.0;
        const HEIGHT: f32 = 580.0;
        let s = ui::scale(screen_width, screen_height)
            .min((screen_width as f32 - 20.0) / WIDTH)
            .min((screen_height as f32 - 20.0) / HEIGHT)
            .max(0.1);
        let frame = Rectangle::new(
            (screen_width as f32 - WIDTH * s) / 2.0,
            (screen_height as f32 - HEIGHT * s) / 2.0,
            WIDTH * s,
            HEIGHT * s,
        );
        let pad = 18.0 * s;
        let row_h = 44.0 * s;
        let button_h = 34.0 * s;
        let small_w = 44.0 * s;
        let gap = 6.0 * s;
        let list_top = frame.y + 84.0 * s;
        let list_bottom = frame.y + frame.height - 130.0 * s;
        let visible_rows = (((list_bottom - list_top) / row_h).floor() as usize).max(1);

        let mut rows = Vec::new();
        let mut buttons = Vec::new();
        for (index, (name, item)) in inventory
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(visible_rows)
        {
            let y = list_top + (index - self.scroll) as f32 * row_h;
            let row = Rectangle::new(frame.x + pad, y, frame.width - pad * 2.0, row_h - 5.0 * s);
            rows.push((
                name.to_owned(),
                row,
                item.count,
                meta_summary(inventory, name),
            ));
            let by = row.y + (row.height - button_h) / 2.0;
            let mut x = row.x + row.width - gap - small_w;
            for (label, action, danger) in [
                ("x", ItemAction::Delete(name.to_owned()), true),
                ("+", ItemAction::Increment(name.to_owned()), false),
                ("-", ItemAction::Decrement(name.to_owned()), false),
            ] {
                buttons.push(ItemButton {
                    rect: Rectangle::new(x, by, small_w, button_h),
                    label,
                    action,
                    danger,
                });
                x -= small_w + gap;
            }
        }

        let bottom_y = frame.y + frame.height - pad - button_h;
        let input_y = bottom_y - button_h - 12.0 * s;
        let right = frame.x + frame.width - pad;
        let add_w = 90.0 * s;
        buttons.push(ItemButton {
            rect: Rectangle::new(right - add_w, input_y, add_w, button_h),
            label: "Add",
            action: ItemAction::Add,
            danger: false,
        });
        let input_rect = Rectangle::new(
            frame.x + pad,
            input_y,
            right - add_w - gap - (frame.x + pad),
            button_h,
        );
        let close_w = 110.0 * s;
        buttons.push(ItemButton {
            rect: Rectangle::new(right - close_w, bottom_y, close_w, button_h),
            label: "Close",
            action: ItemAction::Close,
            danger: false,
        });
        if !inventory.is_empty() {
            buttons.push(ItemButton {
                rect: Rectangle::new(frame.x + pad, bottom_y, 130.0 * s, button_h),
                label: "Clear all",
                action: ItemAction::ClearAll,
                danger: true,
            });
        }

        ItemLayout {
            frame,
            scale: s,
            rows,
            buttons,
            input_rect,
            visible_rows,
        }
    }

    pub fn action_at(&self, layout: &ItemLayout, mouse: Vector2) -> Option<ItemAction> {
        if let Some(button) = layout
            .buttons
            .iter()
            .find(|button| point_in_rect(mouse, button.rect))
        {
            return Some(button.action.clone());
        }
        layout
            .rows
            .iter()
            .find(|(_, rect, ..)| point_in_rect(mouse, *rect))
            .map(|(name, ..)| ItemAction::Select(name.clone()))
    }

    pub fn scroll_by(&mut self, delta: i32, total: usize, visible_rows: usize) {
        let max = total.saturating_sub(visible_rows);
        let next = self.scroll as i64 - delta as i64;
        self.scroll = next.clamp(0, max as i64) as usize;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw<D: RaylibDraw>(
        &self,
        d: &mut D,
        layout: &ItemLayout,
        inventory: &GridInventory,
        title: &str,
        screen_width: i32,
        screen_height: i32,
        mouse: Vector2,
    ) {
        let s = layout.scale;
        let frame = layout.frame;
        let pad = 18.0 * s;
        d.draw_rectangle(0, 0, screen_width, screen_height, Color::new(0, 0, 0, 160));
        d.draw_rectangle_rec(frame, DIALOG_BG);
        d.draw_rectangle_lines_ex(frame, 2.0, DIALOG_BORDER);
        ui::draw_text(
            d,
            &ui::fit(title, 26.0 * s, frame.width - pad * 2.0),
            frame.x + pad,
            frame.y + pad,
            26.0 * s,
            BUTTON_TEXT,
        );
        let subtitle = format!(
            "{} kind(s), {} in total.  Enter: gem x3 · gem = 5 · gem -> ruby · gem @color=red · Ctrl+Z undoes",
            inventory.len(),
            inventory.total()
        );
        ui::draw_text(
            d,
            &ui::fit(&subtitle, 14.0 * s, frame.width - pad * 2.0),
            frame.x + pad,
            frame.y + pad + 34.0 * s,
            14.0 * s,
            MUTED_TEXT,
        );

        let text_size = 19.0 * s;
        let buttons_w = (44.0 * 3.0 + 6.0 * 4.0) * s;
        if layout.rows.is_empty() {
            ui::draw_text(
                d,
                "No items yet — type a name below (e.g. \"gold x50\") and press Enter.",
                frame.x + pad,
                frame.y + 96.0 * s,
                16.0 * s,
                MUTED_TEXT,
            );
        }
        for (name, rect, count, meta) in &layout.rows {
            let current = self.selected.as_deref() == Some(name.as_str());
            d.draw_rectangle_rec(*rect, if current { CURRENT_ROW_BG } else { ROW_BG });
            if current {
                d.draw_rectangle_rec(
                    Rectangle::new(rect.x, rect.y, 4.0 * s, rect.height),
                    SELECTION_COLOR,
                );
            }
            let count_text = format!("×{count}");
            let count_w = ui::measure(&count_text, text_size);
            let available = rect.width - buttons_w - count_w - 36.0 * s;
            let label_x = rect.x + 12.0 * s;
            let label = ui::fit(name, text_size, available);
            let label_w = ui::measure(&label, text_size);
            ui::draw_text(
                d,
                &label,
                label_x,
                rect.y + (rect.height - text_size) / 2.0,
                text_size,
                BUTTON_TEXT,
            );
            ui::draw_text(
                d,
                &count_text,
                label_x + label_w + 10.0 * s,
                rect.y + (rect.height - text_size) / 2.0,
                text_size,
                COUNT_TEXT,
            );
            if !meta.is_empty() {
                let meta_size = 13.0 * s;
                let meta_x = label_x + label_w + count_w + 24.0 * s;
                let meta_w = rect.x + rect.width - buttons_w - meta_x - 6.0 * s;
                if meta_w > meta_size * 3.0 {
                    ui::draw_text(
                        d,
                        &ui::fit(meta, meta_size, meta_w),
                        meta_x,
                        rect.y + (rect.height - meta_size) / 2.0,
                        meta_size,
                        MUTED_TEXT,
                    );
                }
            }
        }
        if inventory.len() > layout.visible_rows {
            let info = format!(
                "{}-{} of {} (scroll for more)",
                self.scroll + 1,
                (self.scroll + layout.visible_rows).min(inventory.len()),
                inventory.len()
            );
            ui::draw_text(
                d,
                &info,
                frame.x + pad,
                layout.input_rect.y - 40.0 * s,
                13.0 * s,
                MUTED_TEXT,
            );
        }
        if let Some((message, error)) = &self.message {
            ui::draw_text(
                d,
                &ui::fit(message, 15.0 * s, frame.width - pad * 2.0),
                frame.x + pad,
                layout.input_rect.y - 22.0 * s,
                15.0 * s,
                if *error { ERROR_TEXT } else { MUTED_TEXT },
            );
        }

        let rect = layout.input_rect;
        d.draw_rectangle_rec(rect, Color::new(12, 12, 16, 255));
        d.draw_rectangle_lines_ex(rect, 1.5, SELECTION_COLOR);
        let size = 18.0 * s;
        let mut shown = format!("{}_", self.input);
        while ui::measure(&shown, size) > rect.width - 16.0 * s && shown.chars().count() > 1 {
            shown.remove(0);
        }
        ui::draw_text(
            d,
            &shown,
            rect.x + 8.0 * s,
            rect.y + (rect.height - size) / 2.0,
            size,
            CONSOLE_PROMPT,
        );

        for button in &layout.buttons {
            let hovered = point_in_rect(mouse, button.rect);
            let bg = match (button.danger, hovered) {
                (true, true) => DANGER_HOVER_BG,
                (true, false) => DANGER_BG,
                (false, true) => BUTTON_HOVER_BG,
                (false, false) => BUTTON_BG,
            };
            d.draw_rectangle_rec(button.rect, bg);
            ui::draw_text_centered(d, button.label, button.rect, 18.0 * s, BUTTON_TEXT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_amounts_in_several_styles() {
        assert_eq!(parse_entry("potion").unwrap(), ("potion".into(), 1));
        assert_eq!(
            parse_entry(" gold coin  x50 ").unwrap(),
            ("gold coin".into(), 50)
        );
        assert_eq!(parse_entry("arrow ×12").unwrap(), ("arrow".into(), 12));
        assert_eq!(parse_entry("key 3").unwrap(), ("key".into(), 3));
        assert_eq!(parse_entry("7 bombs").unwrap(), ("bombs".into(), 7));
        assert_eq!(parse_entry("herb -2").unwrap(), ("herb".into(), -2));
        assert!(parse_entry("   ").is_err());
    }

    #[test]
    fn parses_set_rename_and_meta_commands() {
        use serde_json::json;
        assert_eq!(
            parse_command("gold x5").unwrap(),
            EntryCommand::Add("gold".into(), 5)
        );
        assert_eq!(
            parse_command(" gold coin = 12 ").unwrap(),
            EntryCommand::Set("gold coin".into(), 12)
        );
        assert!(parse_command("gold = lots").is_err());
        assert!(parse_command(" = 3").is_err());
        assert_eq!(
            parse_command("arrow -> bolt").unwrap(),
            EntryCommand::Rename("arrow".into(), "bolt".into())
        );
        assert!(parse_command("arrow ->").is_err());
        assert_eq!(
            parse_command("gem @color=red").unwrap(),
            EntryCommand::Meta("gem".into(), "color".into(), json!("red"))
        );
        assert_eq!(
            parse_command("gem @value = 25").unwrap(),
            EntryCommand::Meta("gem".into(), "value".into(), json!(25))
        );
        assert_eq!(
            parse_command("gem @tags=[\"a\",1]").unwrap(),
            EntryCommand::Meta("gem".into(), "tags".into(), json!(["a", 1]))
        );
        assert_eq!(
            parse_command("gem @color=").unwrap(),
            EntryCommand::Meta("gem".into(), "color".into(), serde_json::Value::Null)
        );
        assert!(parse_command("@x=1").is_err());
        assert!(parse_command("").is_err());
    }

    #[test]
    fn layout_has_row_buttons_and_hit_tests() {
        let mut inventory = GridInventory::default();
        inventory.add("sword", 1).unwrap();
        inventory.add("gem", 4).unwrap();
        inventory
            .set_meta("gem", "color", serde_json::json!("red"))
            .unwrap();
        let panel = ItemPanel::default();
        let layout = panel.layout(&inventory, 1280, 800);
        assert_eq!(layout.rows.len(), 2);
        assert_eq!(meta_summary(&inventory, "gem"), "color=red");
        // 3 per row + Add + Close + Clear all.
        assert_eq!(layout.buttons.len(), 9);
        let plus = layout
            .buttons
            .iter()
            .find(|button| button.action == ItemAction::Increment("gem".into()))
            .unwrap();
        let center = Vector2::new(
            plus.rect.x + plus.rect.width / 2.0,
            plus.rect.y + plus.rect.height / 2.0,
        );
        assert_eq!(
            panel.action_at(&layout, center),
            Some(ItemAction::Increment("gem".into()))
        );
        let (_, row, ..) = &layout.rows[0];
        assert_eq!(
            panel.action_at(&layout, Vector2::new(row.x + 5.0, row.y + 5.0)),
            Some(ItemAction::Select("gem".into()))
        );
        // Every element stays inside the window.
        for button in &layout.buttons {
            assert!(button.rect.x >= 0.0 && button.rect.x + button.rect.width <= 1280.0);
        }
    }

    fn inside(inner: Rectangle, outer: Rectangle) -> bool {
        let eps = 0.5;
        inner.x >= outer.x - eps
            && inner.y >= outer.y - eps
            && inner.x + inner.width <= outer.x + outer.width + eps
            && inner.y + inner.height <= outer.y + outer.height + eps
    }

    fn overlap(a: Rectangle, b: Rectangle) -> bool {
        a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
    }

    #[test]
    fn layout_fits_and_never_overlaps_at_any_window_size() {
        let mut inventory = GridInventory::default();
        for index in 0..25 {
            inventory
                .add(&format!("item{index:02}"), index + 1)
                .unwrap();
        }
        let panel = ItemPanel::default();
        for (w, h) in [
            (320, 240),
            (640, 480),
            (800, 600),
            (1280, 720),
            (1920, 1080),
            (3840, 2160),
            (1600, 400),
            (500, 1400),
        ] {
            let layout = panel.layout(&inventory, w, h);
            let screen = Rectangle::new(0.0, 0.0, w as f32, h as f32);
            assert!(inside(layout.frame, screen), "frame off-screen at {w}x{h}");
            assert!(inside(layout.input_rect, layout.frame));
            assert!(layout.rows.len() <= layout.visible_rows);
            for (_, row, ..) in &layout.rows {
                assert!(inside(*row, layout.frame), "row outside at {w}x{h}");
                assert!(
                    row.y + row.height <= layout.input_rect.y,
                    "row covers the entry field at {w}x{h}"
                );
            }
            for (i, a) in layout.buttons.iter().enumerate() {
                assert!(
                    inside(a.rect, layout.frame),
                    "{} outside at {w}x{h}",
                    a.label
                );
                assert!(a.rect.width > 0.0 && a.rect.height > 0.0);
                for b in &layout.buttons[i + 1..] {
                    assert!(
                        !overlap(a.rect, b.rect),
                        "{} overlaps {} at {w}x{h}",
                        a.label,
                        b.label
                    );
                }
                // Every button is reachable by clicking its centre.
                let centre = Vector2::new(
                    a.rect.x + a.rect.width / 2.0,
                    a.rect.y + a.rect.height / 2.0,
                );
                assert_eq!(panel.action_at(&layout, centre), Some(a.action.clone()));
            }
        }
        // Text grows with the window.
        let small = panel.layout(&inventory, 800, 600).scale;
        let large = panel.layout(&inventory, 3840, 2160).scale;
        assert!(large > small);
    }

    #[test]
    fn scrolling_clamps_and_reveals_the_last_item() {
        let mut inventory = GridInventory::default();
        for index in 0..40 {
            inventory.add(&format!("item{index:02}"), 1).unwrap();
        }
        let mut panel = ItemPanel::default();
        let visible = panel.layout(&inventory, 1280, 800).visible_rows;
        assert!(visible < 40);
        panel.scroll_by(5, 40, visible);
        assert_eq!(panel.scroll, 0, "can't scroll above the top");
        panel.scroll_by(-1_000, 40, visible);
        assert_eq!(panel.scroll, 40 - visible);
        let layout = panel.layout(&inventory, 1280, 800);
        assert_eq!(layout.rows.last().unwrap().0, "item39");
        assert_eq!(layout.rows.len(), visible);
        panel.scroll_by(1, 40, visible);
        assert_eq!(panel.scroll, 40 - visible - 1);
        // Few items: nothing to scroll.
        panel.scroll_by(-10, 2, visible);
        assert_eq!(panel.scroll, 0);
    }

    #[test]
    fn empty_inventory_has_no_rows_or_clear_button() {
        let panel = ItemPanel::default();
        let layout = panel.layout(&GridInventory::default(), 1024, 768);
        assert!(layout.rows.is_empty());
        let actions: Vec<_> = layout.buttons.iter().map(|b| b.action.clone()).collect();
        assert_eq!(actions, vec![ItemAction::Add, ItemAction::Close]);
        // Clicking empty panel space does nothing.
        let frame = layout.frame;
        assert_eq!(
            panel.action_at(&layout, Vector2::new(frame.x + 30.0, frame.y + 120.0)),
            None
        );
    }
}

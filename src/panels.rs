//! Modal overlays: the profile manager, the Help panel and toolbar tooltips.
//! Each overlay computes one layout that is shared by hit-testing and
//! drawing so clickable areas always match what's on screen.

use raylib::prelude::*;

use crate::{
    help::HELP_SECTIONS, point_in_rect, ui, BUTTON_BG, BUTTON_DISABLED_BG, BUTTON_DISABLED_TEXT,
    BUTTON_HOVER_BG, BUTTON_TEXT, CONSOLE_PROMPT, CONSOLE_TEXT, DANGER_BG, DANGER_HOVER_BG,
    DIALOG_BG, DIALOG_BORDER, SELECTION_COLOR,
};

const ERROR_TEXT: Color = Color::new(255, 140, 140, 255);
const MUTED_TEXT: Color = Color::new(150, 150, 170, 255);
const CURRENT_ROW_BG: Color = Color::new(45, 60, 95, 255);
const ROW_BG: Color = Color::new(36, 36, 44, 255);

fn overlay_scale(screen_width: i32, screen_height: i32, width: f32, height: f32) -> f32 {
    ui::scale(screen_width, screen_height)
        .min((screen_width as f32 - 20.0) / width)
        .min((screen_height as f32 - 20.0) / height)
        .max(0.1)
}

fn centered(screen_width: i32, screen_height: i32, width: f32, height: f32) -> Rectangle {
    Rectangle::new(
        (screen_width as f32 - width) / 2.0,
        (screen_height as f32 - height) / 2.0,
        width,
        height,
    )
}

fn draw_button<D: RaylibDraw>(
    d: &mut D,
    rect: Rectangle,
    label: &str,
    size: f32,
    mouse: Vector2,
    enabled: bool,
    danger: bool,
) {
    let hovered = enabled && point_in_rect(mouse, rect);
    let bg = match (enabled, danger, hovered) {
        (false, _, _) => BUTTON_DISABLED_BG,
        (true, true, true) => DANGER_HOVER_BG,
        (true, true, false) => DANGER_BG,
        (true, false, true) => BUTTON_HOVER_BG,
        (true, false, false) => BUTTON_BG,
    };
    d.draw_rectangle_rec(rect, bg);
    let label = ui::fit(label, size, rect.width - size * 0.6);
    let color = if enabled {
        BUTTON_TEXT
    } else {
        BUTTON_DISABLED_TEXT
    };
    ui::draw_text_centered(d, &label, rect, size, color);
}

// ---------------------------------------------------------------- profiles

pub enum InputKind {
    Create,
    Rename(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PanelAction {
    Switch(String),
    StartRename(String),
    AskDelete(String),
    StartNew,
    Close,
    Submit,
    CancelInput,
    ConfirmDelete,
    CancelDelete,
}

pub struct PanelButton {
    pub rect: Rectangle,
    pub label: String,
    pub action: PanelAction,
    pub enabled: bool,
    pub danger: bool,
}

pub struct PanelLayout {
    pub frame: Rectangle,
    pub scale: f32,
    pub rows: Vec<(String, Rectangle, bool)>,
    pub buttons: Vec<PanelButton>,
    pub input_rect: Option<Rectangle>,
    pub message_y: f32,
    pub visible_rows: usize,
}

/// State of the profile manager overlay.
pub struct ProfilePanel {
    pub names: Vec<String>,
    pub input: Option<(InputKind, String)>,
    pub confirm_delete: Option<String>,
    pub message: Option<(String, bool)>,
    pub scroll: usize,
}

impl ProfilePanel {
    pub fn new(names: Vec<String>) -> Self {
        Self {
            names,
            input: None,
            confirm_delete: None,
            message: None,
            scroll: 0,
        }
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.message = Some((message.into(), true));
    }

    pub fn set_info(&mut self, message: impl Into<String>) {
        self.message = Some((message.into(), false));
    }

    pub fn layout(
        &self,
        screen_width: i32,
        screen_height: i32,
        current: Option<&str>,
    ) -> PanelLayout {
        const WIDTH: f32 = 680.0;
        const HEIGHT: f32 = 560.0;
        let s = overlay_scale(screen_width, screen_height, WIDTH, HEIGHT);
        let frame = centered(screen_width, screen_height, WIDTH * s, HEIGHT * s);
        let pad = 18.0 * s;
        let row_h = 46.0 * s;
        let button_h = 36.0 * s;
        let row_button_w = 96.0 * s;
        let gap = 8.0 * s;

        let list_top = frame.y + 92.0 * s;
        let footer_h = 110.0 * s;
        let list_bottom = frame.y + frame.height - footer_h;
        let visible_rows = (((list_bottom - list_top) / row_h).floor() as usize).max(1);

        let modal = self.input.is_some() || self.confirm_delete.is_some();
        let mut rows = Vec::new();
        let mut buttons = Vec::new();
        for (index, name) in self
            .names
            .iter()
            .enumerate()
            .skip(self.scroll)
            .take(visible_rows)
        {
            let y = list_top + (index - self.scroll) as f32 * row_h;
            let row = Rectangle::new(frame.x + pad, y, frame.width - pad * 2.0, row_h - 6.0 * s);
            let is_current = current == Some(name.as_str());
            rows.push((name.clone(), row, is_current));
            let by = row.y + (row.height - button_h) / 2.0;
            let mut x = row.x + row.width - gap - row_button_w;
            for (label, action, enabled, danger) in [
                (
                    "Delete",
                    PanelAction::AskDelete(name.clone()),
                    !is_current && !modal,
                    true,
                ),
                (
                    "Rename",
                    PanelAction::StartRename(name.clone()),
                    !modal,
                    false,
                ),
                (
                    if is_current { "Current" } else { "Open" },
                    PanelAction::Switch(name.clone()),
                    !is_current && !modal,
                    false,
                ),
            ] {
                buttons.push(PanelButton {
                    rect: Rectangle::new(x, by, row_button_w, button_h),
                    label: label.to_owned(),
                    action,
                    enabled,
                    danger,
                });
                x -= row_button_w + gap;
            }
        }

        let message_y = list_bottom + 8.0 * s;
        let bottom_y = frame.y + frame.height - pad - button_h;
        let right = frame.x + frame.width - pad;
        let mut input_rect = None;
        let mut footer = |label: &str, width: f32, action: PanelAction, danger: bool, x: f32| {
            buttons.push(PanelButton {
                rect: Rectangle::new(x, bottom_y, width, button_h),
                label: label.to_owned(),
                action,
                enabled: true,
                danger,
            });
        };
        if self.input.is_some() {
            let (ok_w, cancel_w) = (90.0 * s, 100.0 * s);
            footer(
                "Cancel",
                cancel_w,
                PanelAction::CancelInput,
                false,
                right - cancel_w,
            );
            footer(
                "OK",
                ok_w,
                PanelAction::Submit,
                false,
                right - cancel_w - gap - ok_w,
            );
            input_rect = Some(Rectangle::new(
                frame.x + pad,
                bottom_y,
                (right - cancel_w - ok_w - gap * 2.0) - (frame.x + pad) - gap,
                button_h,
            ));
        } else if self.confirm_delete.is_some() {
            let width = 150.0 * s;
            footer(
                "Cancel",
                width,
                PanelAction::CancelDelete,
                false,
                right - width,
            );
            footer(
                "Delete",
                width,
                PanelAction::ConfirmDelete,
                true,
                right - width * 2.0 - gap,
            );
        } else {
            let close_w = 110.0 * s;
            footer("Close", close_w, PanelAction::Close, false, right - close_w);
            footer(
                "New Profile",
                170.0 * s,
                PanelAction::StartNew,
                false,
                frame.x + pad,
            );
        }

        PanelLayout {
            frame,
            scale: s,
            rows,
            buttons,
            input_rect,
            message_y,
            visible_rows,
        }
    }

    pub fn action_at(&self, layout: &PanelLayout, mouse: Vector2) -> Option<PanelAction> {
        layout
            .buttons
            .iter()
            .find(|button| button.enabled && point_in_rect(mouse, button.rect))
            .map(|button| button.action.clone())
    }

    pub fn scroll_by(&mut self, delta: i32, visible_rows: usize) {
        let max = self.names.len().saturating_sub(visible_rows);
        let next = self.scroll as i64 - delta as i64;
        self.scroll = next.clamp(0, max as i64) as usize;
    }

    pub fn draw<D: RaylibDraw>(
        &self,
        d: &mut D,
        layout: &PanelLayout,
        screen_width: i32,
        screen_height: i32,
        mouse: Vector2,
        base_dir: &str,
    ) {
        let s = layout.scale;
        let frame = layout.frame;
        let pad = 18.0 * s;
        d.draw_rectangle(0, 0, screen_width, screen_height, Color::new(0, 0, 0, 160));
        d.draw_rectangle_rec(frame, DIALOG_BG);
        d.draw_rectangle_lines_ex(frame, 2.0, DIALOG_BORDER);

        ui::draw_text(
            d,
            "Profiles",
            frame.x + pad,
            frame.y + pad,
            26.0 * s,
            BUTTON_TEXT,
        );
        let subtitle = format!(
            "Each profile is a separate account with its own grids, assets, PA exports and init.rb. Stored in {base_dir}"
        );
        for (index, line) in ui::wrap(&subtitle, 14.0 * s, frame.width - pad * 2.0)
            .iter()
            .take(2)
            .enumerate()
        {
            ui::draw_text(
                d,
                line,
                frame.x + pad,
                frame.y + pad + 34.0 * s + index as f32 * 17.0 * s,
                14.0 * s,
                MUTED_TEXT,
            );
        }

        let text_size = 19.0 * s;
        for (name, rect, current) in &layout.rows {
            d.draw_rectangle_rec(*rect, if *current { CURRENT_ROW_BG } else { ROW_BG });
            if *current {
                d.draw_rectangle_rec(
                    Rectangle::new(rect.x, rect.y, 4.0 * s, rect.height),
                    SELECTION_COLOR,
                );
            }
            let label_width = rect.width - (96.0 * 3.0 + 8.0 * 4.0) * s - 12.0 * s;
            let label = ui::fit(name, text_size, label_width);
            ui::draw_text(
                d,
                &label,
                rect.x + 12.0 * s,
                rect.y + (rect.height - text_size) / 2.0,
                text_size,
                BUTTON_TEXT,
            );
        }
        if self.names.len() > layout.visible_rows {
            let info = format!(
                "{}-{} of {} (scroll for more)",
                self.scroll + 1,
                (self.scroll + layout.visible_rows).min(self.names.len()),
                self.names.len()
            );
            ui::draw_text(
                d,
                &info,
                frame.x + pad,
                layout.message_y - 22.0 * s,
                13.0 * s,
                MUTED_TEXT,
            );
        }

        let message = if let Some((kind, _)) = &self.input {
            Some((
                match kind {
                    InputKind::Create => {
                        "Name for the new profile (letters, numbers, space, - _ .):".to_owned()
                    }
                    InputKind::Rename(old) => format!("Rename '{old}' to:"),
                },
                false,
            ))
        } else if let Some(name) = &self.confirm_delete {
            Some((
                format!("Delete profile '{name}' with all of its grids, assets and exports? This cannot be undone."),
                true,
            ))
        } else {
            self.message.clone()
        };
        let mut message_y = layout.message_y;
        if let (Some((_, true)), true) = (&self.message, self.input.is_some()) {
            if let Some((error, _)) = &self.message {
                ui::draw_text(
                    d,
                    &ui::fit(error, 15.0 * s, frame.width - pad * 2.0),
                    frame.x + pad,
                    message_y,
                    15.0 * s,
                    ERROR_TEXT,
                );
                message_y += 19.0 * s;
            }
        }
        if let Some((text, is_error)) = message {
            for line in ui::wrap(&text, 16.0 * s, frame.width - pad * 2.0)
                .iter()
                .take(2)
            {
                ui::draw_text(
                    d,
                    line,
                    frame.x + pad,
                    message_y,
                    16.0 * s,
                    if is_error { ERROR_TEXT } else { CONSOLE_TEXT },
                );
                message_y += 20.0 * s;
            }
        }

        if let (Some(rect), Some((_, text))) = (layout.input_rect, &self.input) {
            d.draw_rectangle_rec(rect, Color::new(12, 12, 16, 255));
            d.draw_rectangle_lines_ex(rect, 1.5, SELECTION_COLOR);
            let size = 18.0 * s;
            let mut shown = format!("{text}_");
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
        }

        for button in &layout.buttons {
            draw_button(
                d,
                button.rect,
                &button.label,
                16.0 * s,
                mouse,
                button.enabled,
                button.danger,
            );
        }
    }
}

// -------------------------------------------------------------------- help

pub struct HelpLayout {
    pub frame: Rectangle,
    pub close: Rectangle,
    pub body: Rectangle,
    pub scale: f32,
}

pub fn help_layout(screen_width: i32, screen_height: i32) -> HelpLayout {
    const WIDTH: f32 = 860.0;
    const HEIGHT: f32 = 640.0;
    let s = overlay_scale(screen_width, screen_height, WIDTH, HEIGHT);
    // Use most of the window, but never more than the design size scaled.
    let width = (WIDTH * s).max((screen_width as f32 * 0.9).min(WIDTH * s * 1.3));
    let height = (HEIGHT * s).max((screen_height as f32 * 0.9).min(HEIGHT * s * 1.3));
    let width = width.min(screen_width as f32 - 20.0);
    let height = height.min(screen_height as f32 - 20.0);
    let frame = centered(screen_width, screen_height, width, height);
    let pad = 18.0 * s;
    let close = Rectangle::new(
        frame.x + frame.width - pad - 100.0 * s,
        frame.y + pad * 0.7,
        100.0 * s,
        34.0 * s,
    );
    let body = Rectangle::new(
        frame.x + pad,
        frame.y + 64.0 * s,
        frame.width - pad * 2.0,
        frame.height - 64.0 * s - pad,
    );
    HelpLayout {
        frame,
        close,
        body,
        scale: s,
    }
}

enum HelpLine {
    Heading(String),
    Item(Vec<String>, Vec<String>),
}

fn help_lines(layout: &HelpLayout) -> (Vec<HelpLine>, f32) {
    let s = layout.scale;
    let key_w = layout.body.width * 0.32;
    let desc_w = layout.body.width - key_w - 14.0 * s;
    let size = 15.0 * s;
    let line_h = size * 1.3;
    let mut lines = Vec::new();
    let mut height = 0.0;
    for (heading, items) in HELP_SECTIONS {
        lines.push(HelpLine::Heading((*heading).to_owned()));
        height += 34.0 * s;
        for (key, desc) in *items {
            let keys = ui::wrap(key, size, key_w);
            let descs = ui::wrap(desc, size, desc_w);
            height += keys.len().max(descs.len()) as f32 * line_h + 6.0 * s;
            lines.push(HelpLine::Item(keys, descs));
        }
        height += 8.0 * s;
    }
    (lines, height)
}

/// Maximum scroll offset for the help body.
pub fn help_max_scroll(layout: &HelpLayout) -> f32 {
    let (_, height) = help_lines(layout);
    (height - layout.body.height).max(0.0)
}

pub fn draw_help(
    d: &mut RaylibDrawHandle<'_>,
    screen_width: i32,
    screen_height: i32,
    scroll: f32,
    mouse: Vector2,
) {
    let layout = help_layout(screen_width, screen_height);
    let s = layout.scale;
    d.draw_rectangle(0, 0, screen_width, screen_height, Color::new(0, 0, 0, 170));
    d.draw_rectangle_rec(layout.frame, DIALOG_BG);
    d.draw_rectangle_lines_ex(layout.frame, 2.0, DIALOG_BORDER);
    ui::draw_text(
        d,
        "Selenite Help",
        layout.frame.x + 18.0 * s,
        layout.frame.y + 18.0 * s,
        26.0 * s,
        BUTTON_TEXT,
    );
    let hint = "F1 / Esc to close · wheel to scroll";
    let hint_x = layout.close.x - ui::measure(hint, 13.0 * s) - 14.0 * s;
    if hint_x > layout.frame.x + 220.0 * s {
        ui::draw_text(
            d,
            hint,
            hint_x,
            layout.frame.y + 28.0 * s,
            13.0 * s,
            MUTED_TEXT,
        );
    }
    draw_button(d, layout.close, "Close", 16.0 * s, mouse, true, false);

    let (lines, height) = help_lines(&layout);
    let body = layout.body;
    let size = 15.0 * s;
    let line_h = size * 1.3;
    let key_w = body.width * 0.32;
    {
        let mut clip = d.begin_scissor_mode(
            body.x as i32,
            body.y as i32,
            body.width.ceil() as i32,
            body.height.ceil() as i32,
        );
        let mut y = body.y - scroll;
        for line in &lines {
            match line {
                HelpLine::Heading(text) => {
                    y += 6.0 * s;
                    ui::draw_text(&mut clip, text, body.x, y, 20.0 * s, SELECTION_COLOR);
                    y += 28.0 * s;
                }
                HelpLine::Item(keys, descs) => {
                    let rows = keys.len().max(descs.len());
                    if y + rows as f32 * line_h >= body.y && y <= body.y + body.height {
                        for (index, key) in keys.iter().enumerate() {
                            ui::draw_text(
                                &mut clip,
                                key,
                                body.x,
                                y + index as f32 * line_h,
                                size,
                                CONSOLE_PROMPT,
                            );
                        }
                        for (index, desc) in descs.iter().enumerate() {
                            ui::draw_text(
                                &mut clip,
                                desc,
                                body.x + key_w + 14.0 * s,
                                y + index as f32 * line_h,
                                size,
                                CONSOLE_TEXT,
                            );
                        }
                    }
                    y += rows as f32 * line_h + 6.0 * s;
                }
            }
        }
        let _ = height;
    }
    if height > body.height {
        let track = Rectangle::new(body.x + body.width + 4.0 * s, body.y, 5.0 * s, body.height);
        d.draw_rectangle_rec(track, Color::new(40, 40, 50, 255));
        let thumb_h = (body.height / height * body.height).max(20.0 * s);
        let max_scroll = height - body.height;
        let thumb_y = body.y + (scroll / max_scroll) * (body.height - thumb_h);
        d.draw_rectangle_rec(
            Rectangle::new(track.x, thumb_y, track.width, thumb_h),
            MUTED_TEXT,
        );
    }
}

// ---------------------------------------------------------------- tooltips

pub fn draw_tooltip<D: RaylibDraw>(
    d: &mut D,
    text: &str,
    anchor: Rectangle,
    screen_width: i32,
    screen_height: i32,
) {
    let s = ui::scale(screen_width, screen_height);
    let size = 15.0 * s;
    let pad = 8.0 * s;
    let max_w = (440.0 * s).min(screen_width as f32 - 20.0);
    let lines = ui::wrap(text, size, max_w - pad * 2.0);
    let width = lines
        .iter()
        .map(|line| ui::measure(line, size))
        .fold(0.0, f32::max)
        + pad * 2.0;
    let line_h = size * 1.3;
    let height = lines.len() as f32 * line_h + pad * 2.0;
    let mut x = anchor.x.min(screen_width as f32 - width - 6.0).max(6.0);
    let mut y = anchor.y + anchor.height + 6.0 * s;
    if y + height > screen_height as f32 - 4.0 {
        y = (anchor.y - height - 6.0 * s).max(4.0);
    }
    if width > screen_width as f32 {
        x = 0.0;
    }
    let rect = Rectangle::new(x, y, width, height);
    d.draw_rectangle_rec(rect, Color::new(16, 16, 22, 245));
    d.draw_rectangle_lines_ex(rect, 1.0, SELECTION_COLOR);
    for (index, line) in lines.iter().enumerate() {
        ui::draw_text(
            d,
            line,
            x + pad,
            y + pad + index as f32 * line_h,
            size,
            CONSOLE_TEXT,
        );
    }
}

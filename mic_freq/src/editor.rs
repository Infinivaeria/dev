//! Side panel (raygui) for customizing each image's settings.

use crate::Scene;
use mic_freq::analysis::log_spaced_centers;
use mic_freq::config::{ImageSettings, DEFAULT_HI_HZ, DEFAULT_LO_HZ, MAX_HZ, MIN_HZ};
use raylib::prelude::*;

pub const PANEL_W: f32 = 380.0;
const PAD: f32 = 12.0;
const ROW: f32 = 24.0;
const GAP: f32 = 6.0;
const LABEL_W: f32 = 100.0;
const VALUE_W: f32 = 70.0;
const TITLE_H: f32 = 24.0;
const FOOTER_H: f32 = 64.0;
const NAME_CAP: usize = 40;

pub enum EditorAction {
    Save,
    Revert,
}

#[derive(Default)]
pub struct Editor {
    pub open: bool,
    pub selected: Option<usize>,
    pub dirty: bool,
    pub drop_replaces: bool,
    editing_name: bool,
    list_scroll: i32,
    list_focus: i32,
    scroll: f32,
    content_h: f32,
    status: Option<(String, f64)>,
}

/// Vertical layout cursor that hides rows scrolled outside the visible region.
struct Rows {
    x: f32,
    w: f32,
    y: f32,
    top: f32,
    bottom: f32,
    start: f32,
}

impl Rows {
    fn next(&mut self, h: f32) -> Option<Rectangle> {
        let r = Rectangle::new(self.x, self.y, self.w, h);
        self.y += h + GAP;
        (r.y >= self.top && r.y + h <= self.bottom).then_some(r)
    }
    fn used(&self) -> f32 {
        self.y - self.start
    }
}

impl Editor {
    pub fn wants_keyboard(&self) -> bool {
        self.open && self.editing_name
    }

    pub fn select(&mut self, index: Option<usize>) {
        if self.selected != index {
            self.editing_name = false;
        }
        self.selected = index;
    }

    pub fn clamp_selection(&mut self, len: usize) {
        if let Some(i) = self.selected {
            self.select(if len == 0 { None } else { Some(i.min(len - 1)) });
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>, now: f64) {
        self.status = Some((msg.into(), now + 4.0));
    }

    pub fn ui(&mut self, d: &mut RaylibDrawHandle, scene: &mut Scene, pitch: Option<f32>, area: Rectangle, now: f64) -> Option<EditorAction> {
        let before: Vec<ImageSettings> = scene.assets.iter().map(|a| a.settings.clone()).collect();
        let title = if self.dirty { "Image editor *  [Tab] hide" } else { "Image editor  [Tab] hide" };
        d.gui_panel(area, Some(title));

        let visible_top = area.y + TITLE_H + GAP;
        let visible_bottom = area.y + area.height - FOOTER_H;
        let mouse = d.get_mouse_position();
        if area.check_collision_point_rec(mouse) {
            self.scroll -= d.get_mouse_wheel_move() * 40.0;
        }
        self.scroll = self.scroll.clamp(0.0, (self.content_h - (visible_bottom - visible_top)).max(0.0));

        let start = visible_top - self.scroll;
        let mut rows = Rows { x: area.x + PAD, w: area.width - 2.0 * PAD, y: start, top: visible_top, bottom: visible_bottom, start };
        let mut action = None;

        self.image_list(d, scene, &mut rows);
        if let Some(i) = self.selected.filter(|&i| i < scene.assets.len()) {
            self.properties(d, &mut scene.assets[i].settings, pitch, &mut rows);
        } else if let Some(r) = rows.next(ROW) {
            d.gui_label(r, "Select an image (list or click it) to edit.");
        }
        self.content_h = rows.used();

        let fy = visible_bottom + GAP;
        let half = (area.width - 2.0 * PAD - GAP) / 2.0;
        if d.gui_button(Rectangle::new(area.x + PAD, fy, half, 28.0), "Save (Ctrl+S)") {
            action = Some(EditorAction::Save);
        }
        if d.gui_button(Rectangle::new(area.x + PAD + half + GAP, fy, half, 28.0), "Revert to saved") {
            action = Some(EditorAction::Revert);
        }
        if let Some((msg, until)) = &self.status {
            if now < *until {
                d.gui_label(Rectangle::new(area.x + PAD, fy + 30.0, area.width - 2.0 * PAD, 22.0), msg);
            } else {
                self.status = None;
            }
        }

        let after = scene.assets.iter().map(|a| &a.settings);
        if before.len() != scene.assets.len() || before.iter().zip(after).any(|(b, a)| b != a) {
            self.dirty = true;
        }
        action
    }

    fn image_list(&mut self, d: &mut RaylibDrawHandle, scene: &mut Scene, rows: &mut Rows) {
        if let Some(r) = rows.next(ROW) {
            d.gui_label(r, format!("Images ({})  - left = low, right = high", scene.assets.len()));
        }
        if let Some(r) = rows.next(132.0) {
            let items: Vec<String> = scene
                .assets
                .iter()
                .map(|a| {
                    let s = &a.settings;
                    format!("{}{}  ({:.0} Hz)", s.name, if s.visible { "" } else { " [hidden]" }, s.center_hz)
                })
                .collect();
            let mut active = self.selected.map_or(-1, |i| i as i32);
            // raylib-rs 6.0 labels these (focus, scroll, active) but forwards them
            // positionally to C's GuiListViewEx(scrollIndex, active, focus).
            d.gui_list_view_ex(r, &items, &mut self.list_scroll, &mut active, &mut self.list_focus);
            self.select(usize::try_from(active).ok().filter(|&i| i < scene.assets.len()));
        }

        if let Some(r) = rows.next(ROW) {
            let n = scene.assets.len();
            let bw = (r.width - 3.0 * GAP) / 4.0;
            let b = |k: f32| Rectangle::new(r.x + k * (bw + GAP), r.y, bw, r.height);
            let sel = self.selected;
            if sel.is_none() {
                d.gui_disable();
            }
            if d.gui_button(b(0.0), "< Left") {
                if let Some(i) = sel.filter(|&i| i > 0) {
                    scene.assets.swap(i, i - 1);
                    self.selected = Some(i - 1);
                }
            }
            if d.gui_button(b(1.0), "Right >") {
                if let Some(i) = sel.filter(|&i| i + 1 < n) {
                    scene.assets.swap(i, i + 1);
                    self.selected = Some(i + 1);
                }
            }
            if d.gui_button(b(2.0), "Remove") {
                if let Some(i) = sel {
                    let removed = scene.assets.remove(i);
                    if !removed.settings.is_generated() {
                        scene.ignored.push(removed.settings.source);
                    }
                    self.clamp_selection(scene.assets.len());
                }
            }
            d.gui_enable();
            if d.gui_button(b(3.0), "Spread") {
                let centers = log_spaced_centers(n, DEFAULT_LO_HZ, DEFAULT_HI_HZ);
                for (a, c) in scene.assets.iter_mut().zip(centers) {
                    a.settings.center_hz = c;
                }
            }
        }
        if let Some(r) = rows.next(20.0) {
            d.gui_check_box(Rectangle::new(r.x, r.y + 2.0, 16.0, 16.0), "Dropped file replaces selected image", &mut self.drop_replaces);
        }
        if let Some(r) = rows.next(20.0) {
            d.gui_label(r, "Drag image files onto the window to add them.");
        }
        if let Some(r) = rows.next(8.0) {
            d.gui_line(r, None::<&str>);
        }
    }

    fn properties(&mut self, d: &mut RaylibDrawHandle, s: &mut ImageSettings, pitch: Option<f32>, rows: &mut Rows) {
        if let Some(r) = rows.next(ROW) {
            let src = if s.is_generated() { "built-in placeholder".to_string() } else { s.source.clone() };
            d.gui_label(r, format!("Source: {src}"));
        }
        if let Some(r) = rows.next(ROW + 4.0) {
            d.gui_label(Rectangle::new(r.x, r.y, LABEL_W, r.height), "Name");
            let mut buf = String::with_capacity(NAME_CAP);
            buf.push_str(&s.name);
            let tb = Rectangle::new(r.x + LABEL_W, r.y, r.width - LABEL_W, r.height);
            if d.gui_text_box(tb, &mut buf, self.editing_name) {
                self.editing_name = !self.editing_name;
            }
            s.name = buf;
        } else {
            self.editing_name = false;
        }
        if let Some(r) = rows.next(20.0) {
            d.gui_check_box(Rectangle::new(r.x, r.y + 2.0, 16.0, 16.0), "Visible", &mut s.visible);
        }

        let old_log_hz = s.center_hz.log2();
        let mut log_hz = old_log_hz;
        slider(d, rows, "Frequency", &mut log_hz, MIN_HZ.log2(), MAX_HZ.log2(), |v| format!("{:.0} Hz", v.exp2()));
        if log_hz != old_log_hz {
            s.center_hz = log_hz.exp2().clamp(MIN_HZ, MAX_HZ);
        }
        if let Some(r) = rows.next(ROW) {
            let b = Rectangle::new(r.x + LABEL_W, r.y, r.width - LABEL_W, r.height);
            if pitch.is_none() {
                d.gui_disable();
            }
            let text = pitch.map_or("Use current pitch (silent)".into(), |p| format!("Use current pitch ({p:.0} Hz)"));
            if d.gui_button(b, &text) {
                if let Some(p) = pitch {
                    s.center_hz = p.clamp(MIN_HZ, MAX_HZ);
                }
            }
            d.gui_enable();
        }
        slider(d, rows, "Band width", &mut s.width_octaves, 0.1, 2.0, |v| format!("{v:.2} oct"));
        slider(d, rows, "Sensitivity", &mut s.gain, 0.0, 4.0, |v| format!("x{v:.2}"));
        slider(d, rows, "Size", &mut s.scale, 0.1, 3.0, |v| format!("x{v:.2}"));
        slider(d, rows, "Idle size", &mut s.min_size, 0.0, 1.0, |v| format!("{:.0}%", v * 100.0));
        slider(d, rows, "Idle opacity", &mut s.min_opacity, 0.0, 1.0, |v| format!("{:.0}%", v * 100.0));
        slider(d, rows, "Lift", &mut s.lift_px, 0.0, 200.0, |v| format!("{v:.0} px"));
        slider(d, rows, "Wobble", &mut s.wobble_deg, 0.0, 45.0, |v| format!("{v:.0} deg"));

        if let Some(r) = rows.next(ROW) {
            d.gui_label(r, format!("Tint  RGB({}, {}, {})", s.tint[0], s.tint[1], s.tint[2]));
        }
        if let Some(r) = rows.next(96.0) {
            let mut c = Color::new(s.tint[0], s.tint[1], s.tint[2], 255);
            d.gui_color_picker(Rectangle::new(r.x, r.y, r.width - 70.0, r.height), "", &mut c);
            // The picker round-trips through HSV every frame; only accept edits made with the mouse.
            let pick_area = Rectangle::new(r.x, r.y, r.width - 34.0, r.height);
            if d.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT) && pick_area.check_collision_point_rec(d.get_mouse_position()) {
                s.tint = [c.r, c.g, c.b];
            }
            let preview = Color::new(s.tint[0], s.tint[1], s.tint[2], 255);
            d.draw_rectangle((r.x + r.width - 28.0) as i32, r.y as i32, 28, r.height as i32, preview);
        }
        if let Some(r) = rows.next(ROW) {
            let half = (r.width - GAP) / 2.0;
            if d.gui_button(Rectangle::new(r.x, r.y, half, r.height), "Reset tint") {
                s.tint = [255, 255, 255];
            }
            if d.gui_button(Rectangle::new(r.x + half + GAP, r.y, half, r.height), "Reset all settings") {
                s.reset_style();
            }
        }
    }
}

fn slider(d: &mut RaylibDrawHandle, rows: &mut Rows, label: &str, value: &mut f32, min: f32, max: f32, fmt: impl Fn(f32) -> String) {
    if let Some(r) = rows.next(ROW) {
        let b = Rectangle::new(r.x + LABEL_W, r.y + 2.0, r.width - LABEL_W - VALUE_W, r.height - 4.0);
        let text = fmt(*value);
        d.gui_slider_bar(b, label, text, value, min, max);
    }
}

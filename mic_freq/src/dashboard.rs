use crate::launch::Launcher;
use chrono::Local;
use mic_freq::time_tools::{display_duration, parse_duration, Alarm, Stopwatch, Timer};
use raylib::prelude::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const WIDTH: f32 = 1000.0;
pub const HEIGHT: f32 = 820.0;
const INK: Color = Color::new(228, 234, 245, 255);
const MUTED: Color = Color::new(159, 174, 199, 255);
const ACCENT: Color = Color::new(94, 220, 190, 255);
const WARNING: Color = Color::new(255, 184, 100, 255);

pub struct Dashboard {
    pub alarm: Alarm,
    pub timer: Timer,
    pub watch: Stopwatch,
    pub launcher: Launcher,
    pub message: String,
    pub avatar_only: bool,
    pub muted: bool,
    visible: [bool; 3],
    alarm_text: String,
    timer_text: String,
    focus: Option<usize>,
    timer_configured: bool,
    launch_enabled: [bool; 3],
}

impl Dashboard {
    pub fn new(executable: PathBuf) -> Self {
        Self {
            alarm: Alarm::default(),
            timer: Timer::default(),
            watch: Stopwatch::default(),
            launcher: Launcher::new(executable),
            message: String::new(),
            avatar_only: false,
            muted: false,
            visible: [true; 3],
            alarm_text: "09:00".into(),
            timer_text: "05:00".into(),
            focus: None,
            timer_configured: false,
            launch_enabled: [false; 3],
        }
    }
    pub fn editing(&self) -> bool {
        self.focus.is_some()
    }
    pub fn ringing(&self) -> bool {
        self.alarm.ringing || self.timer.ringing
    }
    fn launch(&mut self, index: usize) {
        if self.launch_enabled[index] {
            if self.launcher.target.is_none() {
                self.message =
                    "No grid target selected. Drop a .json grid or .rb game script.".into();
            } else if let Err(e) = self.launcher.launch() {
                self.message = e;
            }
        }
    }
    pub fn tick(&mut self, now: Instant) {
        if let Some(error) = self.launcher.poll() {
            self.message = error;
        }
        if self.alarm.tick(Local::now()) {
            self.launch(0);
        }
        if self.timer.tick(now) {
            self.launch(1);
        }
    }
    fn timer_toggle(&mut self, now: Instant) {
        if self.timer.running() {
            self.timer.pause(now);
        } else if self.timer_configured && !self.timer.remaining(now).is_zero() {
            self.timer.resume(now);
        } else {
            match parse_duration(&self.timer_text) {
                Ok(duration) => {
                    self.timer.start(duration, now);
                    self.timer_configured = true;
                    self.message.clear();
                }
                Err(error) => self.message = error.into(),
            }
        }
    }
    fn arm(&mut self) {
        match self.alarm.arm(&self.alarm_text, Local::now()) {
            Ok(()) => self.message.clear(),
            Err(error) => self.message = error.into(),
        }
    }
    pub fn input(&mut self, rl: &mut RaylibHandle, now: Instant) {
        if rl.is_file_dropped() {
            let files: Vec<_> = rl
                .load_dropped_files()
                .paths()
                .iter()
                .map(PathBuf::from)
                .collect();
            self.message = if files.len() != 1 {
                "Drop one grid or game script at a time".into()
            } else {
                match self.launcher.select(files[0].clone()) {
                    Ok(()) => {
                        "Target selected. Enable 'Launch grid' on the tools that should use it."
                            .into()
                    }
                    Err(e) => e,
                }
            };
        }
        let (scale, offset) =
            transform(rl.get_screen_width() as f32, rl.get_screen_height() as f32);
        let mouse = (rl.get_mouse_position() - offset) * (1.0 / scale);
        let click = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        if click {
            self.focus = None;
        }
        let hit = |r: Rectangle| click && r.check_collision_point_rec(mouse);
        if hit(rect(20., 60., 200., 34.))
            || (!self.editing() && rl.is_key_pressed(KeyboardKey::KEY_F))
        {
            self.avatar_only = !self.avatar_only;
            self.focus = None;
        }
        for (i, key) in [KeyboardKey::KEY_A, KeyboardKey::KEY_T, KeyboardKey::KEY_S]
            .into_iter()
            .enumerate()
        {
            if hit(rect(240. + i as f32 * 165., 60., 155., 34.))
                || (!self.editing() && rl.is_key_pressed(key))
            {
                self.visible[i] = !self.visible[i];
            }
        }
        if hit(rect(750., 60., 230., 34.)) {
            self.muted = !self.muted;
        }
        if self.ringing() {
            if hit(rect(720., 107., 120., 34.)) {
                self.alarm.ringing = false;
                self.timer.ringing = false;
            }
            if self.alarm.ringing && hit(rect(850., 107., 130., 34.)) {
                self.alarm.snooze(Local::now());
            }
        }
        if !self.avatar_only {
            if self.visible[0] {
                if hit(rect(530., 222., 200., 42.)) {
                    self.focus = Some(0);
                }
                if hit(rect(530., 288., 130., 36.)) {
                    self.arm();
                }
                if hit(rect(670., 288., 130., 36.)) {
                    self.alarm = Alarm::default();
                }
                if hit(rect(530., 349., 420., 32.)) {
                    self.launch_enabled[0] = !self.launch_enabled[0];
                }
            }
            if self.visible[1] {
                if !self.timer_configured && hit(rect(40., 490., 200., 42.)) {
                    self.focus = Some(1);
                }
                if hit(rect(40., 594., 130., 36.)) {
                    self.timer_toggle(now);
                }
                if hit(rect(180., 594., 130., 36.)) {
                    self.timer = Timer::default();
                    self.timer_configured = false;
                }
                if hit(rect(40., 662., 420., 32.)) {
                    self.launch_enabled[1] = !self.launch_enabled[1];
                }
            }
            if self.visible[2] {
                if hit(rect(530., 510., 130., 36.)) {
                    let fresh = !self.watch.running() && self.watch.elapsed(now).is_zero();
                    self.watch.toggle(now);
                    if fresh {
                        self.launch(2);
                    }
                }
                if hit(rect(670., 510., 130., 36.)) {
                    self.watch.lap(now);
                }
                if hit(rect(810., 510., 130., 36.)) {
                    self.watch = Stopwatch::default();
                }
                if hit(rect(530., 662., 420., 32.)) {
                    self.launch_enabled[2] = !self.launch_enabled[2];
                }
            }
        }
        if hit(rect(840., 727., 140., 30.)) {
            self.launcher.target = None;
        }
        while let Some(c) = rl.get_char_pressed() {
            if let Some(index) = self.focus {
                let text = if index == 0 {
                    &mut self.alarm_text
                } else {
                    &mut self.timer_text
                };
                if (c.is_ascii_digit() || c == ':') && text.len() < 8 {
                    text.push(c);
                }
            }
        }
        if let Some(index) = self.focus {
            let text = if index == 0 {
                &mut self.alarm_text
            } else {
                &mut self.timer_text
            };
            if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
                && rl.is_key_pressed(KeyboardKey::KEY_A)
            {
                text.clear();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
                text.pop();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
                if index == 0 {
                    self.arm();
                } else {
                    self.timer_toggle(now);
                }
                self.focus = None;
            }
        }
    }
    pub fn draw(
        &self,
        d: &mut impl RaylibDraw,
        texture: &Texture2D,
        status: &str,
        db: f32,
        now: Instant,
    ) {
        text(d, "MIC FREQ", 20., 17., 28, INK);
        text(
            d,
            &Local::now().format("%a %d %b   %H:%M:%S").to_string(),
            590.,
            21.,
            23,
            ACCENT,
        );
        button(
            d,
            rect(20., 60., 200., 34.),
            if self.avatar_only {
                "F  Show dashboard"
            } else {
                "F  Avatar focus"
            },
            false,
        );
        for (i, label) in ["A  Alarm", "T  Timer", "S  Stopwatch"].iter().enumerate() {
            button(
                d,
                rect(240. + i as f32 * 165., 60., 155., 34.),
                label,
                self.visible[i],
            );
        }
        button(
            d,
            rect(750., 60., 230., 34.),
            if self.muted {
                "Sound: muted"
            } else {
                "Sound: on"
            },
            !self.muted,
        );
        if self.ringing() {
            d.draw_rectangle_rec(rect(20., 103., 960., 43.), Color::new(95, 57, 36, 255));
            let label = match (self.alarm.ringing, self.timer.ringing) {
                (true, true) => "ALARM + TIMER FINISHED",
                (true, false) => "ALARM",
                _ => "TIMER FINISHED",
            };
            text(d, label, 32., 115., 22, WARNING);
            button(d, rect(720., 107., 120., 34.), "Dismiss", true);
            if self.alarm.ringing {
                button(d, rect(850., 107., 130., 34.), "Snooze 5m", false);
            }
        } else {
            text(
                d,
                "Time tools keep running while hidden. Keep this app open.",
                20.,
                116.,
                18,
                MUTED,
            );
        }
        let avatar_panel = if self.avatar_only {
            rect(20., 158., 960., 552.)
        } else {
            rect(20., 158., 470., 266.)
        };
        panel(d, avatar_panel, "VOICE AVATAR");
        let area = rect(
            avatar_panel.x + 20.,
            avatar_panel.y + 48.,
            avatar_panel.width - 40.,
            avatar_panel.height - 100.,
        );
        d.draw_texture_pro(
            texture,
            rect(0., 0., texture.width() as f32, texture.height() as f32),
            fit_avatar(area, texture.width() as f32, texture.height() as f32),
            Vector2::zero(),
            0.,
            Color::WHITE,
        );
        text(
            d,
            status,
            avatar_panel.x + 20.,
            avatar_panel.y + avatar_panel.height - 43.,
            16,
            MUTED,
        );
        d.draw_rectangle_rec(
            rect(
                avatar_panel.x + 20.,
                avatar_panel.y + avatar_panel.height - 17.,
                avatar_panel.width - 40.,
                5.,
            ),
            Color::new(52, 63, 83, 255),
        );
        d.draw_rectangle_rec(
            rect(
                avatar_panel.x + 20.,
                avatar_panel.y + avatar_panel.height - 17.,
                (avatar_panel.width - 40.) * ((db + 90.) / 90.).clamp(0., 1.),
                5.,
            ),
            ACCENT,
        );
        if !self.avatar_only {
            panel(d, rect(510., 158., 470., 266.), "ALARM / LOCAL TIME");
            panel(d, rect(20., 440., 470., 270.), "COUNTDOWN TIMER");
            panel(d, rect(510., 440., 470., 270.), "STOPWATCH");
            if self.visible[0] {
                field(
                    d,
                    rect(530., 222., 200., 42.),
                    &self.alarm_text,
                    self.focus == Some(0),
                );
                text(d, "24-hour HH:MM", 750., 236., 18, MUTED);
                button(d, rect(530., 288., 130., 36.), "Arm", true);
                button(d, rect(670., 288., 130., 36.), "Cancel", false);
                let status = self
                    .alarm
                    .due
                    .map(|t| t.format("Armed: %a %d %b %H:%M").to_string())
                    .unwrap_or_else(|| {
                        if self.alarm.ringing {
                            "Ringing".into()
                        } else {
                            "Disarmed".into()
                        }
                    });
                text(d, &status, 530., 329., 16, ACCENT);
                launch_button(d, 530., 349., self.launch_enabled[0], "when alarm rings");
                text(
                    d,
                    "One-shot alarm  /  Snooze: 5 minutes",
                    530.,
                    392.,
                    16,
                    MUTED,
                );
            } else {
                text(d, "Hidden - press A to reopen", 530., 230., 22, MUTED);
            }
            if self.visible[1] {
                field(
                    d,
                    rect(40., 490., 200., 42.),
                    &self.timer_text,
                    self.focus == Some(1),
                );
                text(
                    d,
                    if self.timer_configured {
                        "Reset to edit"
                    } else {
                        "MM:SS / HH:MM:SS"
                    },
                    260.,
                    505.,
                    16,
                    MUTED,
                );
                let remaining = if self.timer_configured {
                    self.timer.remaining(now)
                } else {
                    parse_duration(&self.timer_text).unwrap_or_default()
                };
                // Round upward so a running timer never displays zero before it finishes.
                let seconds = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
                text(
                    d,
                    &display_duration(Duration::from_secs(seconds), false),
                    40.,
                    548.,
                    32,
                    ACCENT,
                );
                button(
                    d,
                    rect(40., 594., 130., 36.),
                    if self.timer.running() {
                        "Pause"
                    } else if self.timer_configured && !remaining.is_zero() {
                        "Resume"
                    } else {
                        "Start"
                    },
                    true,
                );
                button(d, rect(180., 594., 130., 36.), "Reset", false);
                text(
                    d,
                    if self.timer.ringing {
                        "Finished"
                    } else if self.timer.running() {
                        "Running"
                    } else {
                        "Ready / paused"
                    },
                    40.,
                    638.,
                    16,
                    MUTED,
                );
                launch_button(d, 40., 662., self.launch_enabled[1], "when timer finishes");
            } else {
                text(d, "Hidden - press T to reopen", 40., 500., 22, MUTED);
            }
            if self.visible[2] {
                text(
                    d,
                    &display_duration(self.watch.elapsed(now), true),
                    530.,
                    477.,
                    28,
                    ACCENT,
                );
                button(
                    d,
                    rect(530., 510., 130., 36.),
                    if self.watch.running() {
                        "Pause"
                    } else {
                        "Start"
                    },
                    true,
                );
                button(d, rect(670., 510., 130., 36.), "Lap", false);
                button(d, rect(810., 510., 130., 36.), "Reset", false);
                for (i, lap) in self.watch.laps.iter().enumerate() {
                    text(
                        d,
                        &format!(
                            "Lap {:02}    {}",
                            self.watch.lap_count - i,
                            display_duration(*lap, true)
                        ),
                        530.,
                        559. + i as f32 * 23.,
                        18,
                        MUTED,
                    );
                }
                launch_button(d, 530., 662., self.launch_enabled[2], "on a fresh start");
            } else {
                text(d, "Hidden - press S to reopen", 530., 500., 22, MUTED);
            }
        }
        let target = self
            .launcher
            .target
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| {
                "Drop a .json grid or .rb game here to select a launch target".into()
            });
        text(
            d,
            &format!("Grid: {}", clipped(&target, 76)),
            20.,
            733.,
            16,
            MUTED,
        );
        button(d, rect(840., 727., 140., 30.), "Clear target", false);
        text(d, &clipped(&self.message, 108), 20., 770., 16, WARNING);
        text(
            d,
            "Up/Down: mic gate   R: reload images   Ctrl+A: clear time field   Esc: quit",
            20.,
            800.,
            16,
            MUTED,
        );
    }
}

pub fn transform(w: f32, h: f32) -> (f32, Vector2) {
    let scale = (w.max(1.) / WIDTH).min(h.max(1.) / HEIGHT);
    (
        scale,
        Vector2::new((w - WIDTH * scale) / 2., (h - HEIGHT * scale) / 2.),
    )
}
fn rect(x: f32, y: f32, w: f32, h: f32) -> Rectangle {
    Rectangle::new(x, y, w, h)
}
fn text(d: &mut impl RaylibDraw, s: &str, x: f32, y: f32, size: i32, color: Color) {
    d.draw_text(s, x as i32, y as i32, size, color);
}
fn panel(d: &mut impl RaylibDraw, r: Rectangle, title: &str) {
    d.draw_rectangle_rec(r, Color::new(28, 35, 49, 255));
    text(d, title, r.x + 20., r.y + 17., 18, INK);
}
fn button(d: &mut impl RaylibDraw, r: Rectangle, label: &str, active: bool) {
    d.draw_rectangle_rec(
        r,
        if active {
            Color::new(37, 85, 83, 255)
        } else {
            Color::new(47, 59, 78, 255)
        },
    );
    text(
        d,
        label,
        r.x + 10.,
        r.y + 9.,
        16,
        if active { ACCENT } else { INK },
    );
}
fn field(d: &mut impl RaylibDraw, r: Rectangle, label: &str, focused: bool) {
    d.draw_rectangle_rec(r, Color::new(15, 22, 34, 255));
    d.draw_rectangle_lines_ex(r, 1., if focused { ACCENT } else { MUTED });
    text(
        d,
        &format!("{}{}", label, if focused { "_" } else { "" }),
        r.x + 12.,
        r.y + 10.,
        24,
        INK,
    );
}
fn launch_button(d: &mut impl RaylibDraw, x: f32, y: f32, active: bool, when: &str) {
    button(
        d,
        rect(x, y, 420., 32.),
        &format!("[{}] Launch grid {when}", if active { "x" } else { " " }),
        active,
    );
}
fn clipped(s: &str, limit: usize) -> String {
    let short: String = s.chars().take(limit).collect();
    if s.chars().count() > limit {
        format!("{short}...")
    } else {
        short
    }
}

fn fit_avatar(area: Rectangle, width: f32, height: f32) -> Rectangle {
    let factor = (area.width / width).min(area.height / height);
    let (w, h) = (width * factor, height * factor);
    rect(
        area.x + (area.width - w) / 2.,
        area.y + (area.height - h) / 2.,
        w,
        h,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dashboard_fits_and_hit_coordinates_round_trip_after_resize() {
        for (w, h) in [(1000., 820.), (720., 600.), (2400., 900.), (900., 2400.)] {
            let (scale, offset) = transform(w, h);
            assert!(offset.x >= 0. && offset.y >= 0.);
            assert!(offset.x + WIDTH * scale <= w + 0.01);
            assert!(offset.y + HEIGHT * scale <= h + 0.01);
            let button_center = Vector2::new(595., 306.);
            let screen = offset + button_center * scale;
            let restored = (screen - offset) * (1. / scale);
            assert!((restored.x - button_center.x).abs() < 0.001);
            assert!((restored.y - button_center.y).abs() < 0.001);
        }
    }
    #[test]
    fn avatar_fits_card_without_stretching() {
        let area = rect(40., 206., 430., 166.);
        for (w, h) in [(512., 512.), (1024., 256.), (256., 32000.)] {
            let avatar = fit_avatar(area, w, h);
            assert!(avatar.x >= area.x && avatar.y >= area.y);
            assert!(avatar.width <= area.width && avatar.height <= area.height);
            assert!((avatar.width / avatar.height - w / h).abs() < 0.001);
        }
    }
    #[test]
    fn hidden_tools_still_finish_and_reset_allows_a_new_duration() {
        let now = Instant::now();
        let mut app = Dashboard::new(PathBuf::from("unused"));
        app.timer_text = "00:01".into();
        app.timer_toggle(now);
        app.visible = [false; 3];
        app.avatar_only = true;
        app.tick(now + Duration::from_secs(2));
        assert!(app.ringing());
        assert!(!app.timer.running());
        app.timer = Timer::default();
        app.timer_configured = false;
        app.timer_text = "00:05".into();
        app.timer_toggle(now);
        assert_eq!(app.timer.remaining(now).as_secs(), 5);
        assert!(!app.ringing());
    }
}

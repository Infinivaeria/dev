use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
};

use raylib::prelude::*;

use crate::{
    project::{Kind, Project},
    settings::Settings,
    ui,
};

#[derive(Clone)]
enum Action {
    Organizer,
    Ide,
    Terminal,
    NewGame,
    NewApp,
    Audio,
    Composer,
    Graphics,
    Cas,
    Assets,
    #[cfg(target_os = "linux")]
    Host,
    Recent(PathBuf),
    DesktopMenu,
    Applications,
    ChangeIcon,
    Desktop {
        tool: &'static str,
        target: Option<PathBuf>,
        grid: Option<PathBuf>,
    },
    Back,
    Close,
}

#[derive(Default)]
pub(crate) struct Menu {
    pub open: bool,
    selected: usize,
    entries: Vec<(String, Action)>,
    message: String,
    children: Vec<Child>,
    query: String,
    shortcuts: bool,
    icon_requested: bool,
    icon_initialized: bool,
    icon_picker: Option<crate::icon::Picker>,
}

impl Menu {
    fn refresh(&mut self) {
        self.entries = vec![
            ("Organizer".into(), Action::Organizer),
            ("IDE".into(), Action::Ide),
            ("Terminal".into(), Action::Terminal),
            ("New Game Project".into(), Action::NewGame),
            ("New Application Project".into(), Action::NewApp),
            ("Microphone Suite".into(), Action::Audio),
            ("Music Composer".into(), Action::Composer),
            ("Graphics Studio".into(), Action::Graphics),
            ("CAS Notebook".into(), Action::Cas),
            ("Asset Manager".into(), Action::Assets),
            #[cfg(target_os = "linux")]
            ("Linux Host".into(), Action::Host),
            ("Desktop Shortcuts".into(), Action::DesktopMenu),
            ("Change Icon".into(), Action::ChangeIcon),
        ];
        let settings = Settings::load(&Settings::path_in(&crate::profiles::default_base_dir()));
        for path in settings
            .recent_projects
            .into_iter()
            .filter(|path| path.join(crate::project::MANIFEST).is_file())
            .take(4)
        {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            self.entries
                .push((format!("Recent: {name}"), Action::Recent(path)));
        }
        self.entries.push(("Close".into(), Action::Close));
        self.selected = 0;
        self.shortcuts = false;
        self.query.clear();
        self.message.clear();
    }

    fn matches(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        self.entries.iter().enumerate().filter(|(_, (label, action))| {
            let keywords = match action {
                Action::Organizer => "grid workspace files",
                Action::Ide => "code editor ruby programming",
                Action::Terminal => "terminal shell command console raylib gpu",
                Action::NewGame | Action::NewApp => "create project",
                Action::Audio => "audio music composer chiptune midi wav ogg microphone tuner pitch spectrum calibration clipping avatar voice presets",
                Action::Composer => "audio music composer chiptune midi wav ogg arrangement tracks",
                Action::Graphics => "graphics image photo video gif layer filter pngtuber ffmpeg",
                Action::Cas => "computer algebra cas symbolic math matrix determinant solve derivative integral notebook symbolica nalgebra",
                Action::Assets => "inventory files images search",
                #[cfg(target_os = "linux")]
                Action::Host => "linux operating system system host processes applications devices network services power appliance",
                Action::Recent(path) => return format!("{label} {}", path.display()).to_lowercase().contains(&query),
                Action::DesktopMenu | Action::Desktop { .. } => "create desktop shortcut launcher",
                Action::Applications => "gnome applications app menu install shortcuts",
                Action::ChangeIcon => "application icon image logo appearance customize",
                Action::Back => "back suite tools",
                Action::Close => "cancel dismiss",
            };
            query.split_whitespace().all(|word| label.to_lowercase().contains(word) || keywords.contains(word))
        }).map(|(index, _)| index).collect()
    }

    fn desktop_menu(&mut self, source: Option<&Path>, grid: Option<&Path>) {
        let project = source.and_then(|path| {
            if path.is_dir() {
                Project::load(path).ok()
            } else {
                Project::discover(path).ok().flatten()
            }
        });
        let grid = grid
            .map(Path::to_owned)
            .or_else(|| std::env::var_os("SELENITE_GRID").map(PathBuf::from))
            .or_else(|| project.as_ref().map(Project::grid))
            .filter(|path| path.is_file());
        self.entries = [
            ("All Suite Tools", "all"),
            ("Selenite", "organizer"),
            ("IDE", "ide"),
            ("Terminal", "terminal"),
            ("Microphone Suite", "audio"),
            ("Music Composer", "composer"),
            ("Graphics Studio", "graphics"),
            ("CAS Notebook", "cas"),
            ("Starter Game", "game"),
        ]
        .into_iter()
        .map(|(label, tool)| {
            (
                label.into(),
                Action::Desktop {
                    tool,
                    target: None,
                    grid: grid.clone(),
                },
            )
        })
        .collect();
        self.entries
            .insert(1, ("All Suite Apps (GNOME)".into(), Action::Applications));
        if let Some(path) = &grid {
            self.entries.push((
                "Current Grid".into(),
                Action::Desktop {
                    tool: "grid",
                    target: Some(path.clone()),
                    grid: None,
                },
            ));
            self.entries.push((
                "Asset Manager".into(),
                Action::Desktop {
                    tool: "assets",
                    target: Some(path.clone()),
                    grid: None,
                },
            ));
        }
        if let Some(path) = source.filter(|path| path.exists()) {
            self.entries.push((
                "Current File or Folder".into(),
                Action::Desktop {
                    tool: "file",
                    target: Some(path.to_owned()),
                    grid: None,
                },
            ));
            if path.is_file()
                && matches!(
                    path.extension().and_then(|extension| extension.to_str()),
                    Some("rb" | "rbc")
                )
            {
                self.entries.push((
                    "Run Current Game".into(),
                    Action::Desktop {
                        tool: "game",
                        target: Some(path.to_owned()),
                        grid: None,
                    },
                ));
            }
        }
        if let Some(project) = project {
            self.entries.push((
                "Run Current Project".into(),
                Action::Desktop {
                    tool: "project",
                    target: Some(project.root.clone()),
                    grid: None,
                },
            ));
            self.entries.push((
                "Current Project in IDE".into(),
                Action::Desktop {
                    tool: "project-ide",
                    target: Some(project.root),
                    grid: None,
                },
            ));
        }
        self.entries.push(("Back".into(), Action::Back));
        self.shortcuts = true;
        self.selected = 0;
        self.query.clear();
        self.message.clear();
    }

    fn geometry(width: i32, height: i32, count: usize) -> (Rectangle, Vec<Rectangle>, f32) {
        let row = ((height as f32 - 24.0) / (count + 3) as f32).clamp(7.0, 36.0);
        let menu_width = (width as f32 - 16.0).clamp(1.0, 500.0);
        let bounds = Rectangle::new(
            (width as f32 - menu_width) / 2.0,
            (height as f32 - row * (count + 3) as f32) / 2.0,
            menu_width,
            row * (count + 3) as f32,
        );
        let rows = (0..count)
            .map(|index| {
                Rectangle::new(
                    bounds.x + 8.0,
                    bounds.y + row * (index + 2) as f32,
                    bounds.width - 16.0,
                    row - 1.0,
                )
            })
            .collect();
        (bounds, rows, row)
    }

    fn launch(
        &mut self,
        action: Action,
        source: Option<&Path>,
        grid: Option<&Path>,
    ) -> Result<(), String> {
        match action {
            Action::ChangeIcon => {
                self.icon_requested = true;
                self.open = false;
                return Ok(());
            }
            Action::DesktopMenu => {
                self.desktop_menu(source, grid);
                return Ok(());
            }
            Action::Applications => {
                self.message = crate::desktop::create_application_shortcuts()?;
                return Ok(());
            }
            Action::Desktop { tool, target, grid } => {
                self.message = crate::desktop::create(tool, target.as_deref(), grid.as_deref())?;
                return Ok(());
            }
            Action::Back => {
                self.refresh();
                return Ok(());
            }
            _ => {}
        }
        if matches!(action, Action::Close) {
            self.open = false;
            return Ok(());
        }
        let program = std::env::current_exe().map_err(|error| error.to_string())?;
        let mut command = Command::new(&program);
        match action {
            Action::Organizer => {}
            Action::Terminal => {
                command.arg("--terminal-window");
                if let Some(directory) = source
                    .and_then(Path::parent)
                    .filter(|path| !path.as_os_str().is_empty())
                {
                    command.current_dir(directory);
                }
                if let Some(grid) = grid {
                    command.env("SELENITE_GRID", grid);
                }
            }
            Action::Audio => {
                let parent = program.parent().ok_or("Executable has no directory")?;
                let audio = [parent.join("mic_freq/mic_freq"), parent.join("mic_freq")]
                    .into_iter().find(|path| path.is_file()).ok_or("Audio Studio is not installed beside Selenite. Build the full suite package first.")?;
                command = Command::new(audio);
            }
            Action::Composer => {
                command.arg("--composer");
                if let Some(path) = source {
                    command.arg(path);
                }
            }
            Action::Graphics => {
                command.arg("--graphics-studio");
                if let Some(path) = source {
                    command.arg(path);
                }
            }
            Action::Cas => {
                command.arg("--cas");
            }
            Action::Assets => {
                let path = grid
                    .map(Path::to_owned)
                    .or_else(|| std::env::var_os("SELENITE_GRID").map(PathBuf::from))
                    .or_else(|| {
                        source.and_then(|path| {
                            Project::discover(path)
                                .ok()
                                .flatten()
                                .map(|project| project.grid())
                        })
                    })
                    .filter(|path| path.is_file())
                    .ok_or("Save or run the current grid/project before opening Asset Manager.")?;
                command.arg("--inventory").arg(path);
            }
            #[cfg(target_os = "linux")]
            Action::Host => {
                command.arg("--host");
            }
            action => {
                if !cfg!(feature = "scripting") {
                    return Err("IDE and project tools require a scripting-enabled build".into());
                }
                let path = match action {
                    Action::Recent(path) => path,
                    Action::Ide if source.is_some() => source.unwrap().to_owned(),
                    action => {
                        let kind = if matches!(action, Action::NewApp) {
                            Kind::App
                        } else {
                            Kind::Game
                        };
                        let directory = Project::suggest_directory(
                            &crate::profiles::default_base_dir().join("projects"),
                            kind,
                        );
                        Project::create(&directory, kind)?.root
                    }
                };
                command.arg("--ide").arg(path);
                if let Some(grid) = grid {
                    command.arg("--grid").arg(grid);
                }
            }
        }
        self.children.push(
            command
                .spawn()
                .map_err(|error| format!("Could not open suite tool: {error}"))?,
        );
        self.open = false;
        Ok(())
    }

    pub fn clear_resources(&mut self) {
        self.icon_picker = None;
    }

    pub fn update(
        &mut self,
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        source: Option<&Path>,
        grid: Option<&Path>,
    ) -> bool {
        if !self.icon_initialized {
            crate::icon::apply_saved(rl);
            self.icon_initialized = true;
        }
        if self.icon_requested {
            self.icon_picker = Some(crate::icon::Picker::new(rl, thread));
            self.icon_requested = false;
        }
        if let Some(picker) = &mut self.icon_picker {
            picker.update(rl, thread);
            if picker.closed {
                self.icon_picker = None;
            }
            return true;
        }
        self.children
            .retain_mut(|child| child.try_wait().ok().flatten().is_none());
        let ctrl = rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_CONTROL);
        let shift = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        if ctrl && shift && rl.is_key_pressed(KeyboardKey::KEY_P) {
            self.open = true;
            self.refresh();
        } else if rl.is_key_pressed(KeyboardKey::KEY_F8) {
            self.open = !self.open;
            if self.open {
                self.refresh();
            }
        }
        if !self.open {
            return false;
        }
        if self.entries.is_empty() {
            self.refresh();
        }
        if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
            self.open = false;
            self.entries.clear();
            return true;
        }
        let previous = self.query.clone();
        if ctrl && rl.is_key_pressed(KeyboardKey::KEY_A) {
            self.query.clear();
        }
        while let Some(character) = rl.get_char_pressed() {
            if !ctrl && !character.is_control() && self.query.len() < 256 {
                self.query.push(character);
            }
        }
        if rl.is_key_pressed(KeyboardKey::KEY_BACKSPACE) {
            self.query.pop();
        }
        if self.query != previous {
            self.selected = 0;
            self.message.clear();
        }
        let matches = self.matches();
        if !matches.is_empty() {
            self.selected = self.selected.min(matches.len() - 1);
            if rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
                self.selected = (self.selected + 1) % matches.len();
            }
            if rl.is_key_pressed(KeyboardKey::KEY_UP) {
                self.selected = (self.selected + matches.len() - 1) % matches.len();
            }
        }
        let mut activate = rl.is_key_pressed(KeyboardKey::KEY_ENTER);
        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            let (bounds, rows, _) = Self::geometry(
                rl.get_screen_width(),
                rl.get_screen_height(),
                matches.len().max(1),
            );
            let mouse = rl.get_mouse_position();
            if let Some(index) = rows
                .iter()
                .position(|row| row.check_collision_point_rec(mouse))
            {
                self.selected = index;
                activate = true;
            } else if !bounds.check_collision_point_rec(mouse) {
                self.open = false;
                self.entries.clear();
            }
        }
        if activate {
            if let Some(index) = matches.get(self.selected) {
                if let Err(error) = self.launch(self.entries[*index].1.clone(), source, grid) {
                    self.message = error;
                }
            }
            if !self.open {
                self.entries.clear();
            }
        }
        true
    }

    pub fn draw<D: RaylibDraw>(&self, d: &mut D, width: i32, height: i32) {
        if let Some(picker) = &self.icon_picker {
            picker.draw(d, width, height);
            return;
        }
        if !self.open {
            return;
        }
        let matches = self.matches();
        let (bounds, rows, row) = Self::geometry(width, height, matches.len().max(1));
        let font = (row * 0.48).clamp(8.0, 18.0);
        d.draw_rectangle(0, 0, width, height, Color::new(0, 0, 0, 170));
        d.draw_rectangle_rec(bounds, Color::new(28, 29, 35, 255));
        ui::draw_text_in(
            d,
            if self.shortcuts {
                "Create Desktop Shortcut"
            } else {
                "Selenite Suite"
            },
            Rectangle::new(bounds.x, bounds.y, bounds.width, row),
            12.0,
            font,
            Color::RAYWHITE,
        );
        let search = Rectangle::new(
            bounds.x + 8.0,
            bounds.y + row,
            bounds.width - 16.0,
            row - 1.0,
        );
        d.draw_rectangle_rec(search, Color::new(21, 23, 29, 255));
        let query = if self.query.is_empty() {
            "Search commands"
        } else {
            &self.query
        };
        ui::draw_text_in(
            d,
            &ui::fit(query, font, search.width - 24.0),
            search,
            12.0,
            font,
            Color::RAYWHITE,
        );
        for (index, (entry, rect)) in matches.iter().zip(&rows).enumerate() {
            let label = &self.entries[*entry].0;
            d.draw_rectangle_rec(
                *rect,
                if index == self.selected {
                    Color::new(50, 80, 95, 255)
                } else {
                    Color::new(38, 40, 48, 255)
                },
            );
            ui::draw_text_in(
                d,
                &ui::fit(label, font, rect.width - 24.0),
                *rect,
                12.0,
                font,
                Color::RAYWHITE,
            );
        }
        if matches.is_empty() {
            ui::draw_text_in(d, "No matching commands", rows[0], 12.0, font, Color::GRAY);
        }
        let footer = Rectangle::new(
            bounds.x,
            bounds.y + row * (matches.len().max(1) + 2) as f32,
            bounds.width,
            row,
        );
        ui::draw_text_in(
            d,
            &ui::fit(&self.message, font * 0.8, bounds.width - 24.0),
            footer,
            12.0,
            font * 0.8,
            Color::GOLD,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_search_matches_names_keywords_and_recent_project_paths() {
        let mut menu = Menu {
            entries: vec![
                ("IDE".into(), Action::Ide),
                ("Terminal".into(), Action::Terminal),
                (
                    "Recent: sample".into(),
                    Action::Recent("/projects/space-game".into()),
                ),
            ],
            ..Menu::default()
        };
        menu.query = "RUBY editor".into();
        assert_eq!(menu.matches(), vec![0]);
        menu.query = "shell".into();
        assert_eq!(menu.matches(), vec![1]);
        menu.query = "space-game".into();
        assert_eq!(menu.matches(), vec![2]);
        menu.query = "no such command".into();
        assert!(menu.matches().is_empty());
        menu.query.clear();
        assert_eq!(menu.matches(), vec![0, 1, 2]);
    }

    #[test]
    fn suite_menu_stays_inside_compact_and_large_windows() {
        for (width, height) in [(240, 140), (420, 300), (1100, 760), (3840, 2160)] {
            for count in [8, 12, 14] {
                let (bounds, rows, _) = Menu::geometry(width, height, count);
                assert!(bounds.x >= 0.0 && bounds.y >= 0.0);
                assert!(bounds.x + bounds.width <= width as f32);
                assert!(bounds.y + bounds.height <= height as f32);
                for row in rows {
                    assert!(row.y + row.height <= bounds.y + bounds.height);
                }
            }
        }
    }
}

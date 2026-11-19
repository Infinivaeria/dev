//! End-to-end tests of `selenite --edit`: real windows, real synthetic keystrokes.
//! They need an X11 display; without one (or without `cc` and libX11) they skip.
#![cfg(target_os = "linux")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use image::RgbImage;

static GUI: Mutex<()> = Mutex::new(());

struct TestWindow(Child);

impl std::ops::Deref for TestWindow {
    type Target = Child;
    fn deref(&self) -> &Child { &self.0 }
}

impl std::ops::DerefMut for TestWindow {
    fn deref_mut(&mut self) -> &mut Child { &mut self.0 }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn have_display() -> bool {
    std::env::var_os("DISPLAY").is_some_and(|display| !display.is_empty())
}

/// The key-sending helper, built once; `None` when it cannot be compiled here.
fn key_driver() -> Option<&'static Path> {
    static DRIVER: OnceLock<Option<PathBuf>> = OnceLock::new();
    DRIVER
        .get_or_init(|| {
            let output =
                std::env::temp_dir().join(format!("selenite-key-driver-{}", std::process::id()));
            let status = Command::new("cc")
                .args(["-Wall", "-Wextra", "-Werror"])
                .arg(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/tests/x11_key_driver.c"
                ))
                .args(["-lX11", "-o"])
                .arg(&output)
                .stderr(Stdio::null())
                .status()
                .ok()?;
            status.success().then_some(output)
        })
        .as_deref()
}

fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("selenite-editor-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}

/// Settings live beside the edited file, never in the real profile directory.
fn home_for(path: &Path) -> PathBuf {
    path.parent().unwrap().join("home")
}

fn editor_command(path: &Path) -> Command {
    window_command(path, "--edit")
}

fn window_command(path: &Path, mode: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_selenite"));
    command
        .arg(mode)
        .arg(path)
        .env("SELENITE_HOME", home_for(path))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Starts the editor and returns once its first frame is on screen.
fn editor(path: &Path) -> TestWindow {
    editor_with(path, None, &[])
}

/// Like `editor`, with a fixed window size and extra environment variables.
fn editor_with(path: &Path, size: Option<&str>, env: &[(&str, &str)]) -> TestWindow {
    window_with(path, "--edit", size, env)
}

fn window_with(path: &Path, mode: &str, size: Option<&str>, env: &[(&str, &str)]) -> TestWindow {
    let ready = path.parent().unwrap().join(format!(
        "ready-{}",
        path.file_name().unwrap().to_string_lossy()
    ));
    let _ = fs::remove_file(&ready);
    let mut command = window_command(path, mode);
    command
        .env("SELENITE_EDITOR_READY", &ready)
        .envs(env.iter().copied());
    if let Some(size) = size {
        command.env("SELENITE_EDITOR_SIZE", size);
    }
    let mut child = TestWindow(command.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(20);
    while !ready.exists() {
        assert!(Instant::now() < deadline, "the editor never drew a frame");
        assert!(
            child.try_wait().unwrap().is_none(),
            "the editor exited early"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(150));
    child
}

fn wait_for_exit(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("the editor did not exit");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn send(child: &Child, commands: &[&str]) {
    send_pid(child.id(), commands)
}

fn send_pid(pid: u32, commands: &[&str]) {
    let status = Command::new(key_driver().unwrap())
        .arg(pid.to_string())
        .args(commands)
        .status()
        .unwrap();
    assert!(status.success(), "key driver failed: {status}");
}

/// Runs the editor on `path`, sends `commands`, and waits for it to exit.
fn drive(path: &Path, commands: &[&str]) -> ExitStatus {
    let mut child = editor(path);
    send(&child, commands);
    wait_for_exit(&mut child)
}

macro_rules! gui_test {
    () => {
        if !have_display() || key_driver().is_none() {
            eprintln!("skipping: needs an X11 display, cc and libX11 headers");
            return;
        }
        let _guard = GUI.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    };
}

#[cfg(feature = "scripting")]
#[test]
fn ide_file_controls_terminal_help_and_resizable_workspace() {
    gui_test!();
    let directory = scratch("ide-suite");
    let source = directory.join("main.rb");
    let copy = directory.join("copy.rb");
    fs::write(&source, "puts 12").unwrap();
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[]);
    send(&child, &["key:ctrl+a", "key:ctrl+c", "key:ctrl+End", "key:Return", "key:ctrl+v", "key:ctrl+shift+s", "key:ctrl+a", &format!("type:{}", copy.display()), "key:Return", "wait:300"]);
    assert_eq!(fs::read_to_string(&copy).unwrap(), "puts 12\nputs 12");
    assert_eq!(fs::read_to_string(&source).unwrap(), "puts 12");
    send(&child, &["click:250,486", "click:300,620", "type:printf IDE_TERM_OK > term.txt", "key:Return", "wait:400"]);
    assert_eq!(fs::read_to_string(directory.join("term.txt")).unwrap(), "IDE_TERM_OK");
    send(&child, &["type:printf '%s' \"$SELENITE_GRID\" > grid-env.txt", "key:Return", "wait:200"]);
    assert_eq!(fs::read_to_string(directory.join("grid-env.txt")).unwrap(), source.with_extension("save.json").to_string_lossy());
    send(&child, &["key:F5", "wait:500", "key:F1", "wait:200", "click:150,518", "click:150,644", "type:DrawCube", "wait:400", &format!("capture:{}", directory.join("help.ppm").display()), "drag:800,470,800,400", "drag:253,150,300,150", "wait:300", "close"]);
    assert!(wait_for_exit(&mut child).success());
    let screenshot = image::open(directory.join("help.ppm")).unwrap().into_rgb8();
    assert_eq!(screenshot.dimensions(), (1100, 760));
    let bright = screenshot.enumerate_pixels().filter(|(_, row, pixel)| *row > 520 && pixel[0] > 170 && pixel[1] > 170 && pixel[2] > 170).count();
    assert!(bright > 100, "Help content did not render");
    let layout: serde_json::Value = serde_json::from_slice(&fs::read(home_for(&source).join("ide.json")).unwrap()).unwrap();
    assert!(layout["panel_fraction"].as_f64().unwrap() > 0.4);
    assert!(layout["sidebar_fraction"].as_f64().unwrap() > 0.26);
    assert_eq!(layout["tab"], "Help");
    assert!(!layout["workspaces"].as_array().unwrap().is_empty());
    let mut narrow = window_with(&source, "--ide", Some("420x300"), &[("SELENITE_EDITOR_FRAMES", "5")]);
    assert!(wait_for_exit(&mut narrow).success());
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn ide_confirm_and_cancel_buttons_work_over_every_pane() {
    gui_test!();
    let directory = scratch("ide-dialog-buttons");
    let source = directory.join("main.rb");
    fs::write(&source, "puts 'confirmed'\n").unwrap();
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[]);
    for (width, height) in [(1100, 760), (420, 300), (2200, 1520)] {
        send(&child, &[&format!("resize:{width},{height}"), "wait:200", "key:F1", "key:ctrl+shift+s"]);
        let copy = directory.join(format!("copy-{width}.rb"));
        let scale = (width as f32 / 960.0).min(height as f32 / 720.0).clamp(0.75, 6.0) * 1.25;
        let font = (14.0 * scale).round();
        let unit = (font / 14.0).min((width as f32 - 16.0) / 280.0).min((height as f32 - 16.0) / 180.0);
        let padding = (12.0 * unit).max(4.0);
        let dialog_width = (680.0 * unit).min(width as f32 - padding * 2.0);
        let left = (width as f32 - dialog_width) / 2.0;
        let button_y = (height as f32 + 180.0 * unit) / 2.0 - padding - (14.0 * unit).max(8.0);
        let confirm_x = left + padding + 45.0 * unit;
        send(&child, &["key:ctrl+a", &format!("type:{}", copy.display()), &format!("click:{confirm_x:.0},{button_y:.0}"), "wait:200"]);
        assert_eq!(fs::read_to_string(&copy).unwrap(), "puts 'confirmed'\n", "Confirm failed at {width}x{height}");
        send(&child, &["key:ctrl+shift+s", "key:ctrl+a", &format!("type:{}", directory.join("cancelled.rb").display()), &format!("click:{:.0},{button_y:.0}", confirm_x + 90.0 * unit + padding), "wait:100"]);
        assert!(!directory.join("cancelled.rb").exists());
    }
    send(&child, &["close"]);
    assert!(wait_for_exit(&mut child).success());
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn ide_quick_open_filters_project_files_and_edits_the_selected_source() {
    gui_test!();
    let directory = scratch("ide-quick-open");
    let source = directory.join("main.rb");
    let helper = directory.join("helper.rb");
    fs::write(&source, "puts 'main'\n").unwrap();
    fs::write(&helper, "puts 'helper'\n").unwrap();
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[]);
    send(&child, &["key:ctrl+p", "type:helper", "key:Return", "key:ctrl+End", "type:#quickopen", "key:ctrl+s", "wait:200", "close"]);
    assert!(wait_for_exit(&mut child).success());
    assert_eq!(fs::read_to_string(&source).unwrap(), "puts 'main'\n");
    assert_eq!(fs::read_to_string(&helper).unwrap(), "puts 'helper'\n#quickopen");
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn suite_menu_creates_application_projects_and_remembers_them() {
    gui_test!();
    let directory = scratch("suite-launcher");
    let source = directory.join("main.rb");
    fs::write(&source, "puts 'suite'\n").unwrap();
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[]);
    send(&child, &["key:F8", "key:Down", "key:Down", "key:Down", "key:Down", "key:Return", "wait:500"]);
    let project = home_for(&source).join("projects/app-project");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !home_for(&source).join("settings.json").exists() {
        assert!(Instant::now() < deadline, "Suite IDE did not record its project");
        std::thread::sleep(Duration::from_millis(20));
    }
    let children = fs::read_to_string(format!("/proc/{}/task/{}/children", child.id(), child.id())).unwrap();
    let ide = children.split_whitespace().map(|pid| pid.parse::<u32>().unwrap()).find(|pid| {
        fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|bytes| bytes.split(|byte| *byte == 0).any(|arg| arg == b"--ide"))
    }).expect("Suite did not launch an IDE");
    send_pid(ide, &["close"]);
    send(&child, &["wait:200", "close"]);
    assert!(wait_for_exit(&mut child).success());
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(project.join("selenite-project.json")).unwrap()).unwrap();
    assert_eq!(manifest["kind"], "app");
    let settings: serde_json::Value = serde_json::from_slice(&fs::read(home_for(&source).join("settings.json")).unwrap()).unwrap();
    assert_eq!(settings["recent_projects"][0], project.canonicalize().unwrap().to_string_lossy().as_ref());
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn ide_terminal_tabs_keep_independent_shells_and_open_scale_options() {
    gui_test!();
    let directory = scratch("ide-terminal-tabs");
    let source = directory.join("main.rb");
    fs::write(&source, "puts 'tabs'\n").unwrap();
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[]);
    send(&child, &["click:300,620", "type:export TAB_MARK=first", "key:Return", "key:ctrl+shift+t", "wait:200", "type:printf '%s' ${TAB_MARK:-second} > second.txt", "key:Return", "wait:200", "key:ctrl+Tab", "type:printf '%s' $TAB_MARK > first.txt", "key:Return", "wait:200"]);
    assert_eq!(fs::read_to_string(directory.join("first.txt")).unwrap(), "first");
    assert_eq!(fs::read_to_string(directory.join("second.txt")).unwrap(), "second");
    send(&child, &["click:1060,515", "wait:200", "click:550,360", "key:Escape", "close"]);
    assert!(wait_for_exit(&mut child).success());
    let settings: serde_json::Value = serde_json::from_slice(&fs::read(home_for(&source).join("settings.json")).unwrap()).unwrap();
    assert_eq!(settings["ui_scale"], 1.25);
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn ide_terminal_dock_moves_and_keeps_shell_input_working() {
    gui_test!();
    let directory = scratch("ide-movable-terminal");
    let source = directory.join("main.rb");
    fs::write(&source, "puts 'movable'\n").unwrap();
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[]);
    send(&child, &["drag:250,486,250,120", "wait:300", "click:300,180", "type:printf DOCK_OK > dock.txt", "key:Return", "wait:400"]);
    assert_eq!(fs::read_to_string(directory.join("dock.txt")).unwrap(), "DOCK_OK");
    send(&child, &["close"]);
    assert!(wait_for_exit(&mut child).success());
    let layout_path = home_for(&source).join("ide.json");
    let layout: serde_json::Value = serde_json::from_slice(&fs::read(&layout_path).unwrap()).unwrap();
    assert_eq!(layout["dock_top"], true);
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[]);
    send(&child, &["drag:250,54,250,650", "wait:300", "close"]);
    assert!(wait_for_exit(&mut child).success());
    let layout: serde_json::Value = serde_json::from_slice(&fs::read(&layout_path).unwrap()).unwrap();
    assert_eq!(layout["dock_top"], false);
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn ide_scales_with_live_window_resizing_but_not_pane_resizing() {
    gui_test!();
    let directory = scratch("ide-window-scaling");
    let source = directory.join("main.rb");
    let copy = directory.join("small.rb");
    fs::write(&source, "class Responsive\nend\n").unwrap();
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[]);
    let keyword_span = |image: &RgbImage| {
        let rows: Vec<_> = (0..image.height() / 4).filter(|row| {
            (image.width() / 4..image.width()).any(|column| {
                let pixel = image.get_pixel(column, *row);
                i32::from(pixel[0]) > i32::from(pixel[1]) + 35
                    && i32::from(pixel[2]) > i32::from(pixel[1]) + 40
            })
        }).collect();
        assert!(!rows.is_empty(), "Ruby keywords did not render");
        rows.last().unwrap() - rows[0] + 1
    };
    let initial = capture(&child, &directory.join("initial.ppm"));
    let initial_span = keyword_span(&initial);
    send(&child, &["drag:800,470,800,330", "wait:200"]);
    let pane = capture(&child, &directory.join("pane.ppm"));
    assert_eq!(keyword_span(&pane), initial_span, "Pane resizing changed editor text size");

    send(&child, &["key:ctrl+shift+s", "resize:2200,1520", "wait:300"]);
    let dialog = capture(&child, &directory.join("large-dialog.ppm"));
    assert_eq!(dialog.dimensions(), (2200, 1520));
    send(&child, &["key:Escape"]);
    let large = capture(&child, &directory.join("large.ppm"));
    assert!(keyword_span(&large) * 10 > initial_span * 17, "Editor text did not grow with the window");

    send(&child, &["key:F1", "key:ctrl+shift+s", "resize:420,300", "wait:200", "key:ctrl+a", &format!("type:{}", copy.display()), "click:60,209", "wait:200"]);
    assert_eq!(fs::read_to_string(&copy).unwrap(), "class Responsive\nend\n");
    let compact = capture(&child, &directory.join("compact.ppm"));
    assert_eq!(compact.dimensions(), (420, 300));
    assert!(close_to(compact.get_pixel(4, 145), [50, 64, 75], 1), "Wrapped Help control did not remain visible");

    send(&child, &["click:210,110", "click:210,240", "type:printf RESIZE_OK > resized-terminal.txt", "key:Return", "wait:200"]);
    assert_eq!(fs::read_to_string(directory.join("resized-terminal.txt")).unwrap(), "RESIZE_OK");
    send(&child, &["resize:1100,760", "wait:200"]);
    let restored = capture(&child, &directory.join("restored.ppm"));
    assert_eq!(restored.dimensions(), (1100, 760));
    assert_eq!(keyword_span(&restored), initial_span);
    send(&child, &["close"]);
    assert!(wait_for_exit(&mut child).success());
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn ide_creates_application_projects_and_runs_the_configured_entry() {
    gui_test!();
    let directory = scratch("v11-project-wizard");
    let source = directory.join("main.rb");
    fs::write(&source, "puts 'workspace'\n").unwrap();
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[]);
    send(&child, &["click:20,18", "click:150,162", "key:Return", "wait:300"]);
    let project = directory.join("app-project");
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(project.join("selenite-project.json")).unwrap()).unwrap();
    assert_eq!(manifest["kind"], "app");
    send(&child, &["key:F5", "wait:500"]);
    fs::create_dir(project.join("lib")).unwrap();
    let helper = project.join("lib/helper.rb");
    fs::write(&helper, "raise 'helper must not run directly'\n").unwrap();
    send(&child, &["key:ctrl+o", "key:ctrl+a", &format!("type:{}", helper.display()), "key:Return", "key:F5", "wait:500", "close"]);
    assert!(wait_for_exit(&mut child).success());
    let save: serde_json::Value = serde_json::from_slice(&fs::read(project.join("save.json")).unwrap()).unwrap();
    assert_eq!(save["root"]["inventory"]["runs"]["count"], 2);
    fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn organizer_opens_ide_and_reloads_its_shared_grid() {
    gui_test!();
    let directory = scratch("ide-integrated");
    let source = directory.join("main.rb");
    let ready = directory.join("ide-ready");
    let home = directory.join("home");
    let profile = home.join("profiles/default");
    fs::create_dir_all(&profile).unwrap();
    fs::write(&source, "puts 'Selenite project'\n").unwrap();
    fs::write(profile.join("init.rb"), format!("Selenite.ide({:?})\n", source.to_string_lossy())).unwrap();
    let mut organizer = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .args(["--profile", "default"])
        .env("SELENITE_HOME", &home)
        .env("SELENITE_EDITOR_READY", &ready)
        .env("SELENITE_EDITOR_SIZE", "1100x760")
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while !ready.exists() {
        assert!(organizer.try_wait().unwrap().is_none(), "Organizer exited before IDE launch");
        assert!(Instant::now() < deadline, "Selenite.ide did not open the IDE");
        std::thread::sleep(Duration::from_millis(20));
    }
    let children = fs::read_to_string(format!("/proc/{}/task/{}/children", organizer.id(), organizer.id())).unwrap();
    let ide = children.split_whitespace().map(|pid| pid.parse::<u32>().unwrap()).find(|pid| {
        fs::read(format!("/proc/{pid}/cmdline")).is_ok_and(|bytes| bytes.split(|byte| *byte == 0).any(|arg| arg == b"--ide"))
    }).expect("IDE child process missing");
    send_pid(ide, &["key:F6", "type:Selenite.inventory.add('ide', 3); Selenite.save", "key:Return", "wait:300", "close"]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while Path::new(&format!("/proc/{ide}")).exists() {
        assert!(organizer.try_wait().unwrap().is_none(), "Organizer exited before IDE reload");
        assert!(Instant::now() < deadline, "Organizer did not reap the IDE session");
        std::thread::sleep(Duration::from_millis(20));
    }
    send(&organizer, &["close"]);
    assert!(wait_for_exit(&mut organizer).success());
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(profile.join("selenite.json")).unwrap()).unwrap();
    assert_eq!(saved["root"]["inventory"]["ide"]["count"], 3);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(feature = "scripting")]
fn icon_picker_applies_previews_persists_rejects_bad_images_and_resets() {
    gui_test!();
    let directory = scratch("icon-picker");
    let source = directory.join("main.rb");
    let icon = directory.join("pink icon.png");
    let invalid = directory.join("invalid.png");
    let desktop = directory.join("Desktop");
    let data = directory.join("data");
    fs::create_dir_all(&desktop).unwrap();
    fs::write(&source, "puts 'icon'\n").unwrap();
    fs::write(&invalid, "not an image").unwrap();
    image::RgbaImage::from_pixel(100, 100, image::Rgba([200, 45, 90, 255])).save(&icon).unwrap();
    let shortcut = desktop.join("selenite.desktop");
    fs::write(&shortcut, "[Desktop Entry]\nType=Application\nName=Selenite\nExec=/bin/true\nIcon=applications-graphics\nX-Selenite-Managed=true\n").unwrap();
    let env = [("XDG_DESKTOP_DIR", desktop.to_str().unwrap()), ("XDG_DATA_HOME", data.to_str().unwrap())];
    let mut child = window_with(&source, "--ide", Some("1100x760"), &env);
    send(&child, &["key:ctrl+shift+p", "type:change icon", "key:Return", "wait:100", "key:ctrl+a", &format!("type:{}", icon.display()), "key:Return", "click:550,567", "wait:200", &format!("icon:{}", directory.join("live-icon.txt").display())]);
    assert_eq!(fs::read_to_string(directory.join("live-icon.txt")).unwrap().trim(), "256 256 ffc82d5a");
    let settings_path = home_for(&source).join("settings.json");
    let settings: serde_json::Value = serde_json::from_slice(&fs::read(&settings_path).unwrap()).unwrap();
    let saved = std::path::PathBuf::from(settings["application_icon"].as_str().unwrap());
    let saved_bytes = fs::read(&saved).unwrap();
    assert!(fs::read_to_string(&shortcut).unwrap().contains("selenite-custom.png"));
    assert!(fs::read_to_string(&shortcut).unwrap().contains("Exec=/bin/true"));
    let preview = capture(&child, &directory.join("preview.ppm"));
    assert!(count_where(&preview, |red, green, blue| red > 150 && green < 80 && blue > 60 && blue < 130) > 5000);
    send(&child, &["key:ctrl+a", &format!("type:{}", invalid.display()), "key:Return", "click:550,567", "wait:100"]);
    assert_eq!(fs::read(&saved).unwrap(), saved_bytes);
    let replacement = directory.join("replacement.png");
    image::RgbaImage::from_pixel(100, 100, image::Rgba([30, 180, 90, 255])).save(&replacement).unwrap();
    let personal = desktop.join("selenite-personal.desktop");
    fs::write(&personal, "[Desktop Entry]\nName=Personal\nExec=/bin/true\nIcon=personal\n").unwrap();
    send(&child, &["key:ctrl+a", &format!("type:{}", replacement.display()), "key:Return", "click:550,567", "wait:100"]);
    assert_eq!(fs::read(&saved).unwrap(), saved_bytes, "Refused shortcut update changed the saved application icon");
    assert!(fs::read_to_string(&personal).unwrap().contains("Icon=personal"));
    fs::remove_file(personal).unwrap();
    send(&child, &["key:Escape", "key:F8", "type:desktop shortcuts", "key:Return", "type:current file", "key:Return", "wait:100", "key:Escape", "close"]);
    assert!(wait_for_exit(&mut child).success());
    let entries: Vec<_> = fs::read_dir(&desktop).unwrap().map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap()).collect();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|entry| entry.contains("selenite-custom.png")), "New shortcuts did not inherit the application icon");
    fs::remove_file(&icon).unwrap();
    let mut settings: serde_json::Value = serde_json::from_slice(&fs::read(&settings_path).unwrap()).unwrap();
    settings["window_size"] = serde_json::json!([480, 320]);
    settings["window_maximized"] = serde_json::json!(false);
    fs::write(&settings_path, serde_json::to_vec(&settings).unwrap()).unwrap();
    let mut child = TestWindow(Command::new(env!("CARGO_BIN_EXE_selenite"))
        .arg("--grid").arg(directory.join("organizer.json"))
        .env("SELENITE_HOME", home_for(&source)).envs(env)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
    send(&child, &["wait:500", &format!("icon:{}", directory.join("restored-icon.txt").display())]);
    assert_eq!(fs::read_to_string(directory.join("restored-icon.txt")).unwrap().trim(), "256 256 ffc82d5a");
    send(&child, &["key:F8", "type:change icon", "key:Return", "wait:100", "click:326,260", "wait:100", &format!("icon:{}", directory.join("reset-icon.txt").display()), "key:Escape", "close"]);
    assert!(wait_for_exit(&mut child).success());
    let settings: serde_json::Value = serde_json::from_slice(&fs::read(&settings_path).unwrap()).unwrap();
    assert!(settings["application_icon"].is_null());
    assert!(fs::read_to_string(&shortcut).unwrap().contains("Icon=applications-graphics"));
    assert_eq!(fs::read_to_string(directory.join("reset-icon.txt")).unwrap().trim(), "none");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(feature = "scripting")]
fn desktop_shortcuts_are_created_from_the_suite_menu_without_vscode() {
    use std::os::unix::fs::PermissionsExt;
    gui_test!();
    let directory = scratch("native-desktop-shortcuts");
    let runtime = directory.join("runtime");
    let desktop = directory.join("Desktop");
    let data = directory.join("data");
    fs::create_dir_all(runtime.join("examples/games")).unwrap();
    fs::write(runtime.join("examples/games/starter.rb"), "puts 'game'\n").unwrap();
    for launcher in ["run.sh", "run-ide.sh", "run-game.sh", "run-mic-freq.sh"] {
        let path = runtime.join(launcher);
        fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let project = directory.join("project");
    for arguments in [vec!["--new-project", project.to_str().unwrap(), "--app"], vec!["--project", project.to_str().unwrap()]] {
        let output = Command::new(env!("CARGO_BIN_EXE_selenite")).args(arguments).env("SELENITE_HOME", directory.join("home")).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }
    let source = project.join("app.rb");
    let mut child = window_with(&source, "--ide", Some("1100x760"), &[("SELENITE_SHORTCUT_ROOT", runtime.to_str().unwrap()), ("XDG_DESKTOP_DIR", desktop.to_str().unwrap()), ("XDG_DATA_HOME", data.to_str().unwrap())]);
    send(&child, &["key:ctrl+shift+p", "type:desktop shortcuts", "key:Return", "type:gnome applications", "key:Return", "wait:200"]);
    send(&child, &["key:ctrl+a", "type:all suite", "key:Return", "wait:200"]);
    for query in ["current file", "current project in ide", "run current project", "current grid"] {
        send(&child, &["key:ctrl+a", &format!("type:{query}"), "key:Return", "wait:150"]);
    }
    send(&child, &["key:Escape", "close"]);
    assert!(wait_for_exit(&mut child).success());
    let entries: Vec<_> = fs::read_dir(&desktop).unwrap().map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap()).collect();
    assert_eq!(entries.len(), 11);
    for flag in ["--terminal-window", "--inventory", "--grid", "--edit", "--project", "--ide"] {
        assert!(entries.iter().any(|entry| entry.contains(flag)), "Missing shortcut action {flag}");
    }
    let applications: Vec<_> = fs::read_dir(data.join("applications")).unwrap().map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap()).collect();
    assert_eq!(applications.len(), 6);
    assert!(applications.iter().any(|entry| entry.contains("Name=Selenite Terminal")));
    assert!(applications.iter().any(|entry| entry.contains("Name=Selenite Asset Manager")));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(feature = "scripting")]
fn asset_manager_searches_filters_and_copies_selected_paths() {
    use std::os::unix::fs::PermissionsExt;
    gui_test!();
    let directory = scratch("asset-manager");
    let source = directory.join("assets.rb");
    let grid = directory.join("grid.json");
    let desktop = directory.join("Desktop");
    let runtime = directory.join("runtime");
    fs::create_dir_all(&runtime).unwrap();
    fs::write(runtime.join("run.sh"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(runtime.join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(directory.join("alpha.txt"), "alpha").unwrap();
    fs::write(directory.join("beta.rb"), "puts 'beta'").unwrap();
    fs::write(&source, "root = Selenite.grid\nroot.import(0, 0, File.join(__dir__, 'alpha.txt'))\nroot.import(1, 0, File.join(__dir__, 'beta.rb'))\nSelenite.save\n").unwrap();
    let prepared = Command::new(env!("CARGO_BIN_EXE_selenite")).arg("--script").arg(&source).arg("--grid").arg(&grid).env("SELENITE_HOME", home_for(&source)).output().unwrap();
    assert!(prepared.status.success(), "{}", String::from_utf8_lossy(&prepared.stderr));
    let mut child = TestWindow(Command::new(env!("CARGO_BIN_EXE_selenite")).arg("--inventory").arg(&grid).env("SELENITE_HOME", home_for(&source)).env("SELENITE_SHORTCUT_ROOT", &runtime).env("XDG_DESKTOP_DIR", &desktop).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
    send(&child, &["wait:1000", "key:ctrl+f", "type:ALPHA", "key:Return", "key:ctrl+c", "wait:100"]);
    let mut clipboard = arboard::Clipboard::new().unwrap();
    let copied = clipboard.get_text().unwrap();
    assert!(copied.ends_with("alpha.txt"), "Wrong filtered asset: {copied}");
    assert!(Path::new(&copied).is_file());
    send(&child, &["key:ctrl+f", "key:ctrl+a", "type:not-found", "key:Return", "key:ctrl+c", "wait:100"]);
    assert_eq!(clipboard.get_text().unwrap(), copied, "Empty result unexpectedly changed the clipboard");
    send(&child, &["key:ctrl+f", "key:ctrl+a", "key:Return", "resize:480,320", "wait:200", "click:300,63", "click:300,220", "wait:100", "key:Down", "key:ctrl+c"]);
    assert!(clipboard.get_text().unwrap().ends_with("beta.rb"));
    send(&child, &["click:420,102", "wait:150"]);
    let entries: Vec<_> = fs::read_dir(&desktop).unwrap().map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap()).collect();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].contains("beta.rb") && entries[0].contains("--edit"));
    send(&child, &["close"]);
    assert!(wait_for_exit(&mut child).success());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn organizer_toolbar_buttons_keep_visible_labels_across_scales() {
    gui_test!();
    let directory = scratch("toolbar-labels");
    let home = directory.join("home");
    fs::create_dir_all(&home).unwrap();
    for (width, height, zoom, buttons) in [(480, 320, 1.0, 0.85), (1100, 760, 0.75, 0.75), (1100, 760, 1.0, 0.85), (1100, 760, 1.5, 1.25)] {
        fs::write(home.join("settings.json"), serde_json::to_vec(&serde_json::json!({"window_size":[width,height],"ui_scale":zoom,"toolbar_scale":buttons,"toolbar_visible":true,"toolbar_enabled":true})).unwrap()).unwrap();
        let mut child = TestWindow(Command::new(env!("CARGO_BIN_EXE_selenite"))
            .arg("--grid").arg(directory.join("grid.json"))
            .env("SELENITE_HOME", &home)
            .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
        send(&child, &["wait:600", "move:450,290"]);
        let screenshot = capture(&child, &directory.join(format!("labels-{width}-{zoom}-{buttons}.ppm")));
        send(&child, &["close"]);
        assert!(wait_for_exit(&mut child).success());
        let background = |pixel: &image::Rgb<u8>| matches!(pixel.0, [60, 90, 160] | [80, 115, 200] | [45, 45, 54] | [170, 55, 55] | [205, 70, 70]);
        let image_width = screenshot.width();
        let image_height = screenshot.height();
        let mut visited = vec![false; (image_width * image_height) as usize];
        let mut checked = 0;
        for (column, row, pixel) in screenshot.enumerate_pixels() {
            if visited[(row * image_width + column) as usize] || !background(pixel) { continue; }
            let mut pending = vec![(column, row)];
            let (mut left, mut right, mut top, mut bottom) = (column, column, row, row);
            while let Some((x, y)) = pending.pop() {
                let offset = (y * image_width + x) as usize;
                if visited[offset] || !background(screenshot.get_pixel(x, y)) { continue; }
                visited[offset] = true;
                left = left.min(x); right = right.max(x); top = top.min(y); bottom = bottom.max(y);
                if x > 0 { pending.push((x - 1, y)); }
                if x + 1 < image_width { pending.push((x + 1, y)); }
                if y > 0 { pending.push((x, y - 1)); }
                if y + 1 < image_height { pending.push((x, y + 1)); }
            }
            if bottom - top < 8 || right - left < bottom - top { continue; }
            let text = (top..=bottom).any(|y| (left..=right).any(|x| {
                let pixel = screenshot.get_pixel(x, y);
                pixel[0] >= 80 && pixel[1] >= 80 && pixel[2] >= 80 && pixel[0].abs_diff(pixel[1]) < 12 && pixel[1].abs_diff(pixel[2]) < 20
            }));
            assert!(text, "Blank button at ({left},{top})-({right},{bottom}) in {width}x{height}, UI {zoom}, buttons {buttons}");
            checked += 1;
        }
        assert!(checked >= 30, "Only detected {checked} toolbar buttons");
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn organizer_scale_menu_resizes_buttons_and_toolbar_can_be_hidden() {
    gui_test!();
    let directory = scratch("organizer-toolbar-options");
    let home = directory.join("home");
    fs::create_dir_all(&home).unwrap();
    fs::write(home.join("settings.json"), r#"{"window_size":[1100,760],"ui_scale":1.0,"toolbar_scale":0.85}"#).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .arg("--grid").arg(directory.join("grid.json"))
        .env("SELENITE_HOME", &home)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    send(&child, &["wait:1500", "key:F9", "click:550,300", "click:550,500", "wait:200"]);
    let small = capture(&child, &directory.join("small.ppm"));
    let blue_pixels = |image: &RgbImage| image.pixels().filter(|pixel| **pixel == image::Rgb([60, 90, 160]) || **pixel == image::Rgb([80, 115, 200])).count();
    let small_buttons = blue_pixels(&small);
    assert!(small_buttons > 1000);
    send(&child, &["key:F9", "click:550,380", "click:550,500", "wait:200"]);
    let large = capture(&child, &directory.join("large.ppm"));
    assert!(blue_pixels(&large) > small_buttons * 3 / 2, "Menu selection did not enlarge rendered buttons");
    send(&child, &["key:F9", "click:650,260", "key:Escape", "wait:200"]);
    let disabled = capture(&child, &directory.join("disabled.ppm"));
    assert_eq!(blue_pixels(&disabled), 0, "Disabled buttons still rendered as active");
    let settings: serde_json::Value = serde_json::from_slice(&fs::read(home.join("settings.json")).unwrap()).unwrap();
    assert_eq!(settings["toolbar_enabled"], false);
    send(&child, &["click:25,15", "wait:200"]);
    let blocked = capture(&child, &directory.join("blocked.ppm"));
    assert_eq!(blue_pixels(&blocked), 0, "A disabled toolbar click opened an active panel");
    send(&child, &["key:F7", "wait:200"]);
    let enabled = capture(&child, &directory.join("enabled.ppm"));
    assert!(blue_pixels(&enabled) > 1000, "F7 did not restore enabled toolbar buttons");
    send(&child, &["key:F10", "wait:200"]);
    let hidden = capture(&child, &directory.join("hidden.ppm"));
    assert_eq!(blue_pixels(&hidden), 0, "Hidden toolbar still rendered buttons");
    send(&child, &["key:F10", "wait:200"]);
    let restored = capture(&child, &directory.join("restored.ppm"));
    assert!(blue_pixels(&restored) > 1000);
    send(&child, &["close"]);
    assert!(wait_for_exit(&mut child).success());
    let settings: serde_json::Value = serde_json::from_slice(&fs::read(home.join("settings.json")).unwrap()).unwrap();
    assert_eq!(settings["ui_scale"], 1.25);
    assert_eq!(settings["toolbar_visible"], true);
    assert_eq!(settings["toolbar_enabled"], true);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn rejects_unreadable_files_without_opening_a_window() {
    let directory = scratch("reject");
    let binary = directory.join("binary.txt");
    fs::write(&binary, [0xff, 0xfe, 0x00]).unwrap();
    for (path, message) in [(binary, "not UTF-8"), (directory.clone(), "not a file")] {
        let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
            .arg("--edit")
            .arg(path)
            .env_remove("DISPLAY")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains(message));
    }
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .arg("--edit")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn typing_ruby_indents_and_aligns_end_then_saves() {
    gui_test!();
    let directory = scratch("ruby");
    let file = directory.join("typed.rb");
    let status = drive(
        &file,
        &[
            "type:class Foo",
            "key:Return",
            "type:def bar",
            "key:Return",
            "type:puts 1",
            "key:Return",
            "type:end",
            "key:Return",
            "type:end",
            "key:ctrl+s",
            "wait:400",
            "close",
        ],
    );
    assert!(status.success());
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "class Foo\n  def bar\n    puts 1\n  end\nend"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn markdown_lists_continue_and_undo_restores_the_previous_state() {
    gui_test!();
    let directory = scratch("markdown");
    let file = directory.join("note.md");
    let status = drive(
        &file,
        &[
            "type:- one",
            "key:Return",
            "type:two",
            "key:Return",
            "key:Return",
            "type:done",
            "key:ctrl+z",
            "key:ctrl+s",
            "wait:400",
            "close",
        ],
    );
    assert!(status.success());
    assert_eq!(fs::read_to_string(&file).unwrap(), "- one\n- two\n");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn closing_the_window_keeps_edits_but_double_escape_discards_them() {
    gui_test!();
    let directory = scratch("close");
    let kept = directory.join("kept.txt");
    fs::write(&kept, "start\n").unwrap();
    assert!(drive(&kept, &["key:ctrl+End", "type:more", "close"]).success());
    assert_eq!(fs::read_to_string(&kept).unwrap(), "start\nmore");

    let discarded = directory.join("discarded.txt");
    fs::write(&discarded, "original\n").unwrap();
    assert!(drive(
        &discarded,
        &["type:changed", "key:Escape", "wait:200", "key:Escape"]
    )
    .success());
    assert_eq!(fs::read_to_string(&discarded).unwrap(), "original\n");

    let clean = directory.join("clean.txt");
    fs::write(&clean, "untouched").unwrap();
    assert!(drive(&clean, &["key:Escape"]).success());
    assert_eq!(fs::read_to_string(&clean).unwrap(), "untouched");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn saving_preserves_crlf_permissions_and_warns_about_outside_changes() {
    use std::os::unix::fs::PermissionsExt;

    gui_test!();
    let directory = scratch("save");
    let file = directory.join("windows.txt");
    fs::write(&file, "a\r\nb").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    assert!(drive(
        &file,
        &["key:ctrl+End", "type:!", "key:ctrl+s", "wait:300", "close"]
    )
    .success());
    assert_eq!(fs::read(&file).unwrap(), b"a\r\nb!");
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o640
    );

    // The file changes under the editor: the first save is refused, the second overwrites.
    let mut child = editor(&file);
    send(&child, &["key:ctrl+End", "type:?"]);
    std::thread::sleep(Duration::from_millis(1100));
    fs::write(&file, "changed elsewhere").unwrap();
    send(&child, &["key:ctrl+s", "wait:300"]);
    assert_eq!(fs::read_to_string(&file).unwrap(), "changed elsewhere");
    send(&child, &["key:ctrl+s", "wait:300", "close"]);
    assert!(wait_for_exit(&mut child).success());
    assert_eq!(fs::read(&file).unwrap(), b"a\r\nb!?");
    fs::remove_dir_all(directory).unwrap();
}

fn close_to(pixel: &image::Rgb<u8>, color: [u8; 3], tolerance: u8) -> bool {
    pixel
        .0
        .iter()
        .zip(color)
        .all(|(a, b)| a.abs_diff(b) <= tolerance)
}

fn count(image: &RgbImage, color: [u8; 3], tolerance: u8) -> usize {
    image
        .pixels()
        .filter(|pixel| close_to(pixel, color, tolerance))
        .count()
}

/// Counts pixels by hue, which anti-aliased grey text can never imitate.
fn count_where(image: &RgbImage, test: impl Fn(i32, i32, i32) -> bool) -> usize {
    image
        .pixels()
        .filter(|pixel| {
            test(
                i32::from(pixel[0]),
                i32::from(pixel[1]),
                i32::from(pixel[2]),
            )
        })
        .count()
}

const KEYWORD: fn(i32, i32, i32) -> bool = |r, g, b| r > g + 50 && b > g + 50;
const STRING: fn(i32, i32, i32) -> bool = |r, g, b| g > r + 25 && g > b + 45;
const COMMENT: fn(i32, i32, i32) -> bool =
    |r, _, b| (22..=36).contains(&(b - r)) && (90..=125).contains(&r);

/// Rows of flat bar colour at the top (or bottom) edge: the bar's pixel height.
fn bar_rows(image: &RgbImage, from_top: bool) -> u32 {
    let (width, height) = image.dimensions();
    let x = 3.min(width - 1);
    (0..height)
        .map(|offset| {
            if from_top {
                offset
            } else {
                height - 1 - offset
            }
        })
        .take_while(|y| close_to(image.get_pixel(x, *y), [18, 18, 24], 2))
        .count() as u32
}

fn screenshot(path: &Path, size: Option<&str>) -> RgbImage {
    let shot = path.parent().unwrap().join(format!(
        "{}-{}.png",
        path.file_name().unwrap().to_string_lossy(),
        size.unwrap_or("default")
    ));
    let mut command = editor_command(path);
    command
        .env("SELENITE_EDITOR_FRAMES", "6")
        .env("SELENITE_EDITOR_SCREENSHOT", &shot);
    if let Some(size) = size {
        command.env("SELENITE_EDITOR_SIZE", size);
    }
    assert!(command.status().unwrap().success());
    image::open(shot).unwrap().to_rgb8()
}

const PREVIEW_BG: [u8; 3] = [32, 33, 40];
const CODE_BG: [u8; 3] = [22, 23, 28];

#[test]
fn renders_ruby_highlighting_and_a_markdown_preview() {
    gui_test!();
    let directory = scratch("render");
    let code = "# comment\nclass Foo\n  def bar\n    puts \"text\"\n  end\n  def baz\n    \"more text\"\n  end\nend\n";
    let ruby = directory.join("demo.rb");
    let plain = directory.join("demo.txt");
    fs::write(&ruby, code).unwrap();
    fs::write(&plain, code).unwrap();
    let highlighted = screenshot(&ruby, None);
    let control = screenshot(&plain, None);
    for (name, test) in [
        ("keyword", KEYWORD),
        ("string", STRING),
        ("comment", COMMENT),
    ] {
        let (with, without) = (count_where(&highlighted, test), count_where(&control, test));
        assert!(
            with > 30 && without < 3,
            "{name}: {with} highlighted vs {without} plain"
        );
    }

    let markdown = directory.join("demo.md");
    fs::write(
        &markdown,
        "# Title\n\n```ruby\nclass Foo\nend\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n",
    )
    .unwrap();
    let image = screenshot(&markdown, None);
    assert!(
        count(&image, PREVIEW_BG, 0) > 50_000,
        "the preview pane is drawn"
    );
    assert!(
        count(&image, CODE_BG, 0) > 1_000,
        "the code block has its own background"
    );
    assert!(
        count(&image, [42, 46, 60], 0) > 200,
        "the table header is drawn"
    );
    assert!(
        count_where(&image, KEYWORD) > 60,
        "ruby keywords are highlighted"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn the_interface_scales_with_the_window() {
    gui_test!();
    let directory = scratch("scaling");
    let file = directory.join("scale.rb");
    fs::write(&file, "puts 'scale me'\n").unwrap();
    let sizes = ["640x480", "1280x800", "1920x1080", "2560x1440"];
    let mut bars = Vec::new();
    for size in sizes {
        let image = screenshot(&file, Some(size));
        let (width, height) = size.split_once('x').unwrap();
        assert_eq!(
            image.dimensions(),
            (width.parse().unwrap(), height.parse().unwrap()),
            "the window really is {size}"
        );
        let (top, bottom) = (bar_rows(&image, true), bar_rows(&image, false));
        assert!(top > 8 && bottom > 8, "{size}: bars {top}/{bottom}");
        bars.push((top, bottom));
    }
    for pair in bars.windows(2) {
        assert!(pair[1].0 > pair[0].0 && pair[1].1 > pair[0].1, "{bars:?}");
    }
    // 480p -> 1080p grows the UI about 2x; 1080p -> 1440p by the shorter side's ratio (4/3).
    let ratio = |a: u32, b: u32| f64::from(b) / f64::from(a);
    assert!(
        (1.7..2.3).contains(&ratio(bars[0].0, bars[2].0)),
        "{bars:?}"
    );
    assert!(
        (1.2..1.5).contains(&ratio(bars[2].0, bars[3].0)),
        "{bars:?}"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn markdown_split_collapses_to_the_source_when_the_window_is_narrow() {
    gui_test!();
    let directory = scratch("narrow");
    let file = directory.join("narrow.md");
    fs::write(&file, "# Title\n\nSome text\n").unwrap();
    let narrow = screenshot(&file, Some("520x400"));
    assert!(
        count(&narrow, PREVIEW_BG, 0) < 500,
        "no preview pane in a narrow window"
    );
    let wide = screenshot(&file, Some("1280x800"));
    assert!(
        count(&wide, PREVIEW_BG, 0) > 100_000,
        "the preview returns when there is room"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn zoom_applies_immediately_and_is_remembered_by_the_next_window() {
    gui_test!();
    let directory = scratch("zoom");
    let file = directory.join("zoom.rb");
    fs::write(&file, "puts 1\n").unwrap();
    let before = bar_rows(&screenshot(&file, Some("1280x800")), true);

    let status = drive(
        &file,
        &[
            "key:ctrl+equal",
            "wait:100",
            "key:ctrl+equal",
            "wait:100",
            "key:ctrl+equal",
            "wait:300",
            "close",
        ],
    );
    assert!(status.success());
    let settings = fs::read_to_string(home_for(&file).join("settings.json")).unwrap();
    let saved: serde_json::Value = serde_json::from_str(&settings).unwrap();
    let scale = saved["ui_scale"].as_f64().unwrap();
    assert!(
        (scale - 1.3).abs() < 0.01,
        "three zoom-in steps, got {scale}"
    );

    let after = bar_rows(&screenshot(&file, Some("1280x800")), true);
    assert!(
        f64::from(after) / f64::from(before) > 1.15,
        "the next window starts zoomed: {before} -> {after}"
    );

    let status = drive(&file, &["key:ctrl+0", "wait:300", "close"]);
    assert!(status.success());
    assert_eq!(
        bar_rows(&screenshot(&file, Some("1280x800")), true),
        before,
        "Ctrl+0 resets"
    );
    fs::remove_dir_all(directory).unwrap();
}

/// Runs the editor on `path` with `commands`, saves with Ctrl+S, and returns the file.
fn edit_and_read(path: &Path, content: &str, commands: &[&str]) -> String {
    fs::write(path, content).unwrap();
    let mut all = commands.to_vec();
    all.extend(["key:ctrl+s", "wait:400", "close"]);
    assert!(drive(path, &all).success());
    fs::read_to_string(path).unwrap()
}

#[test]
fn find_steps_through_matches_and_replaces_one_or_all() {
    gui_test!();
    let directory = scratch("find");
    let file = directory.join("find.txt");

    // Replace all is case-insensitive by default.
    assert_eq!(
        edit_and_read(
            &file,
            "one two\nTwo three\nnothing\ntwo\n",
            &["key:ctrl+h", "type:two", "key:Tab", "type:2", "key:alt+a",],
        ),
        "one 2\n2 three\nnothing\n2\n"
    );

    // Alt+C makes the search case-sensitive.
    assert_eq!(
        edit_and_read(
            &file,
            "one two\nTwo three\nnothing\ntwo\n",
            &[
                "key:ctrl+h",
                "type:two",
                "key:alt+c",
                "key:Tab",
                "type:2",
                "key:alt+a",
            ],
        ),
        "one 2\nTwo three\nnothing\n2\n"
    );

    // Enter in the replace field replaces one match at a time.
    assert_eq!(
        edit_and_read(
            &file,
            "a a a",
            &[
                "key:ctrl+h",
                "type:a",
                "key:Tab",
                "type:b",
                "key:Return",
                "key:Return",
            ],
        ),
        "b b a"
    );

    // Enter in the find field steps to the next match and leaves it selected.
    assert_eq!(
        edit_and_read(
            &file,
            "x1 x2 x3",
            &[
                "key:ctrl+f",
                "type:x",
                "key:Return",
                "key:Return",
                "key:Escape",
                "type:!",
            ],
        ),
        "x1 x2 !3"
    );

    // One Ctrl+Z undoes a whole replace-all.
    assert_eq!(
        edit_and_read(
            &file,
            "q q\nq\n",
            &[
                "key:ctrl+h",
                "type:q",
                "key:Tab",
                "type:zz",
                "key:alt+a",
                "key:Escape",
                "key:ctrl+z",
            ],
        ),
        "q q\nq\n"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn find_highlights_matches_in_the_source() {
    gui_test!();
    let directory = scratch("highlight");
    let file = directory.join("plain.txt");
    fs::write(&file, "needle in a haystack\nanother needle\n").unwrap();
    let mut child = editor_with(&file, Some("900x500"), &[]);
    let before = capture(&child, &directory.join("before.ppm"));
    send(&child, &["key:ctrl+f", "type:needle", "wait:200"]);
    let after = capture(&child, &directory.join("after.ppm"));
    let differing = (0..before.height())
        .flat_map(|y| (0..before.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| y > 90 && before.get_pixel(x, y) != after.get_pixel(x, y))
        .count();
    assert!(
        differing > 400,
        "matches should be painted behind the text: {differing} pixels changed"
    );
    send(&child, &["key:Escape", "key:Escape", "key:Escape"]);
    wait_for_exit(&mut child);
    fs::remove_dir_all(directory).unwrap();
}

fn capture(child: &Child, path: &Path) -> RgbImage {
    send(child, &[&format!("capture:{}", path.display())]);
    image::open(path).unwrap().to_rgb8()
}

/// First x (at row `y`) where the preview background begins.
fn preview_start(image: &RgbImage, y: u32) -> u32 {
    (0..image.width() - 2)
        .find(|&x| (0..3).all(|dx| close_to(image.get_pixel(x + dx, y), PREVIEW_BG, 0)))
        .expect("no preview background on the row")
}

#[test]
fn the_divider_drags_and_double_click_resets_it() {
    gui_test!();
    let directory = scratch("divider");
    let file = directory.join("split.md");
    fs::write(&file, "# Title\n\nSome text.\n").unwrap();
    let mut child = editor_with(&file, Some("1100x700"), &[]);
    let row = 600;
    let start = preview_start(&capture(&child, &directory.join("a.ppm")), row);
    assert!((530..=570).contains(&start), "centered by default: {start}");

    send(
        &child,
        &[&format!("drag:{},300,{},300", start - 2, 680), "wait:200"],
    );
    let wide = preview_start(&capture(&child, &directory.join("b.ppm")), row);
    assert!((660..=700).contains(&wide), "dragged right: {wide}");

    send(
        &child,
        &[&format!("drag:{},300,{},300", wide - 2, 400), "wait:200"],
    );
    let narrow = preview_start(&capture(&child, &directory.join("c.ppm")), row);
    assert!((360..=420).contains(&narrow), "dragged left: {narrow}");

    let x = narrow - 2;
    send(
        &child,
        &[
            &format!("click:{x},300"),
            &format!("click:{x},300"),
            "wait:200",
        ],
    );
    let reset = preview_start(&capture(&child, &directory.join("d.ppm")), row);
    assert!((530..=570).contains(&reset), "double-click resets: {reset}");

    send(&child, &["key:Escape", "key:Escape"]);
    wait_for_exit(&mut child);
    fs::remove_dir_all(directory).unwrap();
}

/// Bounding boxes of vertical clusters of link-coloured pixels in the preview.
fn link_boxes(image: &RgbImage) -> Vec<(u32, u32)> {
    let link = |p: &image::Rgb<u8>| {
        let [r, g, b] = p.0;
        r < 130 && (120..200).contains(&g) && b > 230
    };
    let mut rows: Vec<(u32, u32, u32)> = Vec::new(); // (y, min x, max x)
    for y in 60..image.height() - 40 {
        let xs: Vec<u32> = (560..image.width())
            .filter(|&x| link(image.get_pixel(x, y)))
            .collect();
        if let (Some(&first), Some(&last)) = (xs.first(), xs.last()) {
            rows.push((y, first, last));
        }
    }
    let mut boxes = Vec::new();
    let mut group: Vec<(u32, u32, u32)> = Vec::new();
    for row in rows {
        if group.last().is_some_and(|last| row.0 - last.0 > 8) {
            boxes.push(center(&group));
            group.clear();
        }
        group.push(row);
    }
    if !group.is_empty() {
        boxes.push(center(&group));
    }
    boxes
}

fn center(group: &[(u32, u32, u32)]) -> (u32, u32) {
    let x0 = group.iter().map(|row| row.1).min().unwrap();
    let x1 = group.iter().map(|row| row.2).max().unwrap();
    ((x0 + x1) / 2, (group[0].0 + group[group.len() - 1].0) / 2)
}

fn wait_for_file(path: &Path) -> Option<String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if let Ok(text) = fs::read_to_string(path) {
            if !text.is_empty() {
                return Some(text);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

#[test]
fn preview_links_open_safely_and_anchors_scroll() {
    gui_test!();
    let directory = scratch("links");
    let file = directory.join("links.md");
    let mut text = String::from(
        "# Links\n\n[web](https://example.com/page)\n\n[blocked](javascript:alert(1))\n\n\
         [data](data.bin)\n\n[gone](missing.rb)\n\n[jump](#second-section)\n\n",
    );
    for number in 1..=40 {
        text.push_str(&format!("Filler paragraph {number}.\n\n"));
    }
    text.push_str("## Second section\n\nArrived.\n");
    fs::write(&file, &text).unwrap();
    fs::write(directory.join("data.bin"), [0u8, 1, 2]).unwrap();

    let bin = directory.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let record = directory.join("opened.txt");
    let opener = bin.join("xdg-open");
    fs::write(
        &opener,
        format!("#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n", record.display()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&opener, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let mut child = editor_with(&file, Some("1100x700"), &[("PATH", &path)]);
    let before = capture(&child, &directory.join("a.ppm"));
    let links = link_boxes(&before);
    assert_eq!(links.len(), 5, "five links in the preview: {links:?}");
    let click = |index: usize| {
        let (x, y) = links[index];
        send(&child, &[&format!("click:{x},{y}"), "wait:200"]);
    };

    click(0);
    assert_eq!(
        wait_for_file(&record).as_deref(),
        Some("https://example.com/page")
    );
    fs::remove_file(&record).unwrap();

    click(1);
    click(3);
    assert!(
        wait_for_file(&record).is_none(),
        "blocked and missing links launch nothing"
    );

    click(2);
    assert_eq!(
        wait_for_file(&record).as_deref(),
        Some(directory.join("data.bin").to_str().unwrap())
    );

    click(4);
    let after = capture(&child, &directory.join("b.ppm"));
    let changed = (60..before.height() - 40)
        .flat_map(|y| (560..before.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| before.get_pixel(x, y) != after.get_pixel(x, y))
        .count();
    assert!(changed > 5_000, "the anchor scrolls the preview: {changed}");

    send(&child, &["key:Escape", "key:Escape"]);
    wait_for_exit(&mut child);
    fs::remove_dir_all(directory).unwrap();
}

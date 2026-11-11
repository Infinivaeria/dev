//! End-to-end tests of `selenite --terminal-window`: a real window, a real shell, real keystrokes.
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

fn have_display() -> bool {
    std::env::var_os("DISPLAY").is_some_and(|display| !display.is_empty())
}

fn key_driver() -> Option<&'static Path> {
    static DRIVER: OnceLock<Option<PathBuf>> = OnceLock::new();
    DRIVER
        .get_or_init(|| {
            let output = std::env::temp_dir().join(format!(
                "selenite-terminal-key-driver-{}",
                std::process::id()
            ));
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

macro_rules! gui_test {
    () => {
        if !have_display() || key_driver().is_none() {
            eprintln!("skipping: needs an X11 display, cc and libX11 headers");
            return;
        }
        let _guard = GUI.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    };
}

fn scratch(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("selenite-terminal-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}

struct Session {
    child: Child,
    dump: PathBuf,
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Starts a terminal window running `/bin/sh` and returns once its first frame is on screen.
fn session(directory: &Path, shell: &str, env: &[(&str, &str)]) -> Session {
    let ready = directory.join("ready");
    let dump = directory.join("screen.txt");
    let mut command = Command::new(env!("CARGO_BIN_EXE_selenite"));
    command
        .arg("--terminal-window")
        .env("SHELL", shell)
        .env("PS1", "$ ")
        .env("SELENITE_HOME", directory.join("home"))
        .env("SELENITE_TERMINAL_SIZE", "960x720")
        .env("SELENITE_TERMINAL_READY", &ready)
        .env("SELENITE_TERMINAL_DUMP", &dump)
        .current_dir(directory)
        .envs(env.iter().copied())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while !ready.exists() {
        assert!(Instant::now() < deadline, "the terminal never drew a frame");
        assert!(
            child.try_wait().unwrap().is_none(),
            "the terminal exited early"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(400));
    Session { child, dump }
}

impl Session {
    fn send(&self, commands: &[&str]) {
        let status = Command::new(key_driver().unwrap())
            .arg(self.child.id().to_string())
            .args(commands)
            .status()
            .unwrap();
        assert!(status.success(), "key driver failed: {status}");
    }

    fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            if Instant::now() > deadline {
                let _ = self.child.kill();
                panic!("the terminal did not exit");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn screen(&self) -> String {
        fs::read_to_string(&self.dump).unwrap_or_default()
    }

    fn capture(&self, path: &Path) -> RgbImage {
        self.send(&[&format!("capture:{}", path.display())]);
        image::open(path).unwrap().to_rgb8()
    }
}

#[test]
fn tabs_create_independent_shells_switch_and_close_with_buttons() {
    gui_test!();
    let directory = scratch("tabs");
    let mut terminal = session(&directory, "/bin/sh", &[]);
    terminal.send(&["type:export TAB_MARK=first", "key:Return", "key:ctrl+shift+t", "wait:300", "type:printf '%s' ${TAB_MARK:-second} > second.txt", "key:Return", "wait:300", "key:ctrl+Tab", "type:printf '%s' $TAB_MARK > first.txt", "key:Return", "wait:300"]);
    assert_eq!(fs::read_to_string(directory.join("first.txt")).unwrap(), "first");
    assert_eq!(fs::read_to_string(directory.join("second.txt")).unwrap(), "second");
    terminal.send(&["key:ctrl+shift+w", "click:872,17", "wait:300", "type:printf third > third.txt", "key:Return", "wait:300"]);
    assert_eq!(fs::read_to_string(directory.join("third.txt")).unwrap(), "third");
    terminal.send(&["click:942,17", "type:exit 0", "key:Return"]);
    assert!(terminal.wait().success());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn scale_menu_applies_and_saves_a_mouse_selected_preset() {
    gui_test!();
    let directory = scratch("scale-menu");
    let mut terminal = session(&directory, "/bin/sh", &[]);
    terminal.send(&["click:907,17", "wait:200", "click:480,400", "wait:200", "key:Escape"]);
    let settings: serde_json::Value = serde_json::from_slice(&fs::read(directory.join("home/settings.json")).unwrap()).unwrap();
    assert_eq!(settings["ui_scale"], 1.5);
    terminal.send(&["type:printf MENU_OK > menu.txt", "key:Return", "wait:200", "type:exit", "key:Return"]);
    assert!(terminal.wait().success());
    assert_eq!(fs::read_to_string(directory.join("menu.txt")).unwrap(), "MENU_OK");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn typed_commands_run_in_a_shell_and_the_exit_status_is_kept() {
    gui_test!();
    let directory = scratch("shell");
    let mut terminal = session(&directory, "/bin/sh", &[]);
    terminal.send(&[
        "type:printf 'v=%s\\n' $((6*7))",
        "key:Return",
        "wait:300",
        "type:echo $SELENITE_EXE | cat",
        "key:Return",
        "wait:300",
        "type:exit 5",
        "key:Return",
    ]);
    let status = terminal.wait();
    assert_eq!(status.code(), Some(5));
    let screen = terminal.screen();
    assert!(screen.contains("v=42"), "{screen}");
    assert!(screen.contains("selenite"), "{screen}");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn control_keys_reach_the_program() {
    gui_test!();
    let directory = scratch("ctrl");
    let mut terminal = session(&directory, "/bin/sh", &[]);
    terminal.send(&[
        "type:cat > got.txt",
        "key:Return",
        "wait:200",
        "type:line one",
        "key:Return",
        "key:ctrl+d",
        "wait:300",
        "type:sleep 30",
        "key:Return",
        "wait:300",
        "key:ctrl+c",
        "wait:300",
        "type:exit 0",
        "key:Return",
    ]);
    assert!(terminal.wait().success());
    assert_eq!(
        fs::read_to_string(directory.join("got.txt")).unwrap(),
        "line one\n"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn the_character_grid_follows_the_zoom() {
    gui_test!();
    let directory = scratch("zoom");
    let mut terminal = session(&directory, "/bin/sh", &[]);
    terminal.send(&[
        "type:stty size",
        "key:Return",
        "wait:300",
        "key:ctrl+equal",
        "wait:100",
        "key:ctrl+equal",
        "wait:100",
        "key:ctrl+equal",
        "wait:400",
        "type:stty size",
        "key:Return",
        "wait:300",
        "type:exit",
        "key:Return",
    ]);
    assert!(terminal.wait().success());
    let sizes: Vec<(u32, u32)> = terminal
        .screen()
        .lines()
        .filter_map(|line| {
            let mut numbers = line.split_whitespace().map(|word| word.parse::<u32>());
            match (numbers.next(), numbers.next(), numbers.next()) {
                (Some(Ok(rows)), Some(Ok(cols)), None) => Some((rows, cols)),
                _ => None,
            }
        })
        .collect();
    assert_eq!(sizes.len(), 2, "{sizes:?}");
    assert!(
        sizes[1].1 + 10 < sizes[0].1 && sizes[1].0 < sizes[0].0,
        "zooming in should leave fewer rows and columns: {sizes:?}"
    );
    let saved = fs::read_to_string(directory.join("home/settings.json")).unwrap();
    assert!(saved.contains("ui_scale"), "{saved}");
    fs::remove_dir_all(directory).unwrap();
}

fn grid_size_at(directory: &Path, window: &str) -> (u32, u32) {
    let mut terminal = session(directory, "/bin/sh", &[("SELENITE_TERMINAL_SIZE", window)]);
    terminal.send(&[
        "type:stty size",
        "key:Return",
        "wait:300",
        "type:exit",
        "key:Return",
    ]);
    assert!(terminal.wait().success());
    let screen = terminal.screen();
    screen
        .lines()
        .find_map(|line| {
            let mut numbers = line.split_whitespace().map(|word| word.parse::<u32>());
            match (numbers.next(), numbers.next(), numbers.next()) {
                (Some(Ok(rows)), Some(Ok(cols)), None) => Some((rows, cols)),
                _ => None,
            }
        })
        .unwrap_or_else(|| panic!("no size reported: {screen}"))
}

#[test]
fn the_text_scales_with_the_window_size() {
    gui_test!();
    let directory = scratch("window-scale");
    let (small_rows, small_cols) = grid_size_at(&directory, "640x480");
    let (large_rows, large_cols) = grid_size_at(&directory, "1600x900");
    // Fixed-size text would give about 2.5x the columns; scaled text grows the grid far less.
    assert!(large_cols > small_cols, "{small_cols} -> {large_cols}");
    assert!(
        f64::from(large_cols) < f64::from(small_cols) * 1.9,
        "{small_cols} -> {large_cols} columns"
    );
    assert!(
        f64::from(large_rows) < f64::from(small_rows) * 1.8,
        "{small_rows} -> {large_rows} rows"
    );
    fs::remove_dir_all(directory).unwrap();
}

const STRING: fn(i32, i32, i32) -> bool = |r, g, b| g > r + 25 && g > b + 45;
const KEYWORD: fn(i32, i32, i32) -> bool = |r, g, b| r > g + 50 && b > g + 50;

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

fn highlighted_run(directory: &Path, highlight: &str) -> RgbImage {
    let mut terminal = session(
        directory,
        "/bin/sh",
        &[("SELENITE_TERMINAL_HIGHLIGHT", highlight)],
    );
    terminal.send(&[
        "type:echo \"quoted text\" 42 --flag",
        "key:Return",
        "wait:400",
    ]);
    let image = terminal.capture(&directory.join(format!("shot-{highlight}.ppm")));
    terminal.send(&["type:exit", "key:Return"]);
    assert!(terminal.wait().success());
    image
}

#[test]
fn plain_output_is_syntax_highlighted_unless_switched_off() {
    gui_test!();
    let directory = scratch("highlight");
    let on = highlighted_run(&directory, "1");
    let off = highlighted_run(&directory, "0");
    for (name, test) in [("string", STRING), ("keyword", KEYWORD)] {
        let (with, without) = (count_where(&on, test), count_where(&off, test));
        assert!(
            with > 20 && without < 3,
            "{name}: {with} highlighted vs {without} plain"
        );
    }
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn programs_own_colours_are_left_alone() {
    gui_test!();
    let directory = scratch("ansi");
    let mut terminal = session(&directory, "/bin/sh", &[]);
    terminal.send(&[
        "type:printf '\\033[38;2;250;10;10m\"quoted\" 42 --flag\\033[0m\\n'",
        "key:Return",
        "wait:400",
    ]);
    let image = terminal.capture(&directory.join("ansi.ppm"));
    let red = count_where(&image, |r, g, b| r > 200 && g < 60 && b < 60);
    // The command line itself is highlighted; only the program's own output is pure red.
    assert!(red > 60, "the program's red text is drawn: {red}");
    terminal.send(&["type:exit", "key:Return"]);
    assert!(terminal.wait().success());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_failed_launch_stays_open_until_a_key_is_pressed() {
    gui_test!();
    let directory = scratch("failure");
    let mut terminal = session(&directory, "/no/such/selenite-shell", &[]);
    std::thread::sleep(Duration::from_millis(1200));
    assert!(
        terminal.child.try_wait().unwrap().is_none(),
        "the window should stay open to show the error"
    );
    terminal.send(&["key:Return"]);
    assert_eq!(terminal.wait().code(), Some(1));
    assert!(
        terminal.screen().contains("failed to start shell"),
        "{}",
        terminal.screen()
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn closing_the_window_hangs_up_the_shell() {
    gui_test!();
    let directory = scratch("close");
    let mut terminal = session(&directory, "/bin/sh", &[]);
    terminal.send(&["type:echo $$ > pid.txt", "key:Return", "wait:400", "close"]);
    assert!(terminal.wait().success());
    let pid: i32 = fs::read_to_string(directory.join("pid.txt"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Path::new(&format!("/proc/{pid}")).exists() {
        assert!(Instant::now() < deadline, "the shell outlived its terminal");
        std::thread::sleep(Duration::from_millis(50));
    }
    fs::remove_dir_all(directory).unwrap();
}

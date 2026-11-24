use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

#[cfg(target_os = "linux")]
static GUI_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("mic-freq-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn preset_commands_work_without_a_display_or_microphone() {
    let directory = scratch("preset-cli");
    let run = |arguments: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_mic_freq"))
            .args(arguments)
            .env("XDG_CONFIG_HOME", &directory)
            .env_remove("DISPLAY")
            .output()
            .unwrap()
    };
    let saved = run(&["--save-preset", "Quiet room"]);
    assert!(
        saved.status.success(),
        "{}",
        String::from_utf8_lossy(&saved.stderr)
    );
    let listed = run(&["--list-presets"]);
    assert!(listed.status.success());
    assert_eq!(String::from_utf8_lossy(&listed.stdout).trim(), "Quiet room");
    let version = run(&["--version"]);
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        env!("CARGO_PKG_VERSION")
    );
    let copied = run(&["--preset", "Quiet room", "--save-preset", "Broadcast"]);
    assert!(copied.status.success());
    assert!(directory.join("mic_freq/presets/Broadcast.json").is_file());
    assert!(!run(&["--preset", "Missing"]).status.success());
    assert!(!run(&["--save-preset", "../escape"]).status.success());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn scripts_validate_headlessly_and_report_invalid_programs() {
    let directory = scratch("script-cli");
    let valid = directory.join("valid.rhai");
    let invalid = directory.join("invalid.rhai");
    std::fs::write(
        &valid,
        r#"
            fn update(input) {
                let n = 0;
                while n < 25 { n += 1; }
                #{ pose: "auto", bounce: 1.5 }
            }
        "#,
    )
    .unwrap();
    std::fs::write(&invalid, "fn update(input) { #{ surprise: 1 } }").unwrap();
    let check = |path: &PathBuf| {
        Command::new(env!("CARGO_BIN_EXE_mic_freq"))
            .arg("--check-script")
            .arg(path)
            .env_remove("DISPLAY")
            .output()
            .unwrap()
    };
    let checked = check(&valid);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(String::from_utf8_lossy(&checked.stdout).contains("script OK"));
    let failed = check(&invalid);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("unknown field"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn every_distributed_script_example_checks_headlessly() {
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/mic_freq");
    let mut scripts = std::fs::read_dir(&examples)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "rhai")
        })
        .collect::<Vec<_>>();
    scripts.sort();
    assert!(scripts.len() >= 30);
    for script in scripts {
        let output = Command::new(env!("CARGO_BIN_EXE_mic_freq"))
            .arg("--check-script")
            .arg(&script)
            .env_remove("DISPLAY")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} failed: {}",
            script.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

struct Window(Child);

impl Drop for Window {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn preset_dialog_saves_and_loads_with_the_mouse() {
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    let _serial_gui = GUI_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let directory = scratch("preset-dialog");
    let driver = directory.join("keys");
    let compiled = Command::new("cc")
        .args(["-Wall", "-Wextra", "-Werror"])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/x11_key_driver.c"
        ))
        .args(["-lX11", "-o"])
        .arg(&driver)
        .status()
        .unwrap();
    assert!(compiled.success());
    let mut window = Window(
        Command::new(env!("CARGO_BIN_EXE_mic_freq"))
            .env("XDG_CONFIG_HOME", &directory)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let sent = Command::new(&driver)
        .arg(window.0.id().to_string())
        .args([
            "wait:1200",
            "click:1000,933",
            "click:1000,933",
            "key:F2",
            "type:GUI preset",
            "click:440,375",
            "wait:200",
            "click:636,375",
            "click:832,375",
            "close",
        ])
        .status()
        .unwrap();
    assert!(sent.success());
    let deadline = Instant::now() + Duration::from_secs(20);
    while window.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "Audio Studio did not close");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(directory.join("mic_freq/presets/GUI preset.json").is_file());
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn pngtuber_box_survives_repeated_capture_toggles() {
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    let _serial_gui = GUI_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let directory = scratch("capture-toggle");
    let driver = directory.join("keys");
    let compiled = Command::new("cc")
        .args(["-Wall", "-Wextra", "-Werror"])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/x11_key_driver.c"
        ))
        .args(["-lX11", "-o"])
        .arg(&driver)
        .status()
        .unwrap();
    assert!(compiled.success());
    let mut window = Window(
        Command::new(env!("CARGO_BIN_EXE_mic_freq"))
            .env("XDG_CONFIG_HOME", &directory)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let sent = Command::new(&driver)
        .arg(window.0.id().to_string())
        .args([
            "wait:900",
            "key:c",
            "wait:300",
            "key:c",
            "key:c",
            "wait:300",
            "key:c",
            "key:c",
            "wait:300",
            "key:c",
            "key:c",
            "key:c",
            "wait:400",
            "key:c",
            "wait:300",
            "key:Escape",
        ])
        .status()
        .unwrap();
    assert!(sent.success());
    let deadline = Instant::now() + Duration::from_secs(20);
    while window.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "Audio Studio did not close");
        std::thread::sleep(Duration::from_millis(20));
    }
    let status = window.0.try_wait().unwrap().unwrap();
    assert!(
        status.success(),
        "Audio Studio crashed during capture toggles"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn artwork_browser_scales_with_resized_window() {
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    let _serial_gui = GUI_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let directory = scratch("artwork-dialog");
    let driver = directory.join("keys");
    let compiled = Command::new("cc")
        .args(["-Wall", "-Wextra", "-Werror"])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/x11_key_driver.c"
        ))
        .args(["-lX11", "-o"])
        .arg(&driver)
        .status()
        .unwrap();
    assert!(compiled.success());

    let mut window = Window(
        Command::new(env!("CARGO_BIN_EXE_mic_freq"))
            .arg("--artwork")
            .arg(&directory)
            .env("XDG_CONFIG_HOME", &directory)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let sent = Command::new(&driver)
        .arg(window.0.id().to_string())
        .args([
            "wait:900",
            "resize:1200,900",
            "click:545,625",
            "wait:300",
            "click:916,108",
            "wait:200",
            "key:Escape",
        ])
        .status()
        .unwrap();
    assert!(sent.success());
    let deadline = Instant::now() + Duration::from_secs(20);
    while window.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "Artwork editor did not close");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::fs::remove_dir_all(directory).unwrap();
}

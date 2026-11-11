use std::{path::PathBuf, process::{Child, Command, Stdio}, time::{Duration, Instant}};

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("mic-freq-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn preset_commands_work_without_a_display_or_microphone() {
    let directory = scratch("preset-cli");
    let run = |arguments: &[&str]| Command::new(env!("CARGO_BIN_EXE_mic_freq")).args(arguments).env("XDG_CONFIG_HOME", &directory).env_remove("DISPLAY").output().unwrap();
    let saved = run(&["--save-preset", "Quiet room"]);
    assert!(saved.status.success(), "{}", String::from_utf8_lossy(&saved.stderr));
    let listed = run(&["--list-presets"]);
    assert!(listed.status.success());
    assert_eq!(String::from_utf8_lossy(&listed.stdout).trim(), "Quiet room");
    let copied = run(&["--preset", "Quiet room", "--save-preset", "Broadcast"]);
    assert!(copied.status.success());
    assert!(directory.join("mic_freq/presets/Broadcast.json").is_file());
    assert!(!run(&["--preset", "Missing"]).status.success());
    assert!(!run(&["--save-preset", "../escape"]).status.success());
    std::fs::remove_dir_all(directory).unwrap();
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
    if std::env::var_os("DISPLAY").is_none() { return; }
    let directory = scratch("preset-dialog");
    let driver = directory.join("keys");
    let compiled = Command::new("cc").args(["-Wall", "-Wextra", "-Werror"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/x11_key_driver.c"))
        .args(["-lX11", "-o"]).arg(&driver).status().unwrap();
    assert!(compiled.success());
    let mut window = Window(Command::new(env!("CARGO_BIN_EXE_mic_freq"))
        .env("XDG_CONFIG_HOME", &directory)
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
    let sent = Command::new(&driver).arg(window.0.id().to_string())
        .args(["wait:1200", "key:F2", "type:GUI preset", "click:250,345", "wait:200", "click:446,345", "click:642,345", "close"])
        .status().unwrap();
    assert!(sent.success());
    let deadline = Instant::now() + Duration::from_secs(20);
    while window.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < deadline, "Audio Studio did not close");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(directory.join("mic_freq/presets/GUI preset.json").is_file());
    std::fs::remove_dir_all(directory).unwrap();
}
use std::process::Command;

#[cfg(unix)]
#[test]
fn desktop_shortcuts_quote_paths_preserve_commands_and_protect_user_entries() {
    use std::os::unix::fs::PermissionsExt;
    let directory = std::env::temp_dir().join(format!("selenite-desktop-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let root = directory.join("suite with spaces %");
    let data = directory.join("data");
    let desktop = directory.join("Desktop");
    std::fs::create_dir_all(&root).unwrap();
    for name in ["run.sh", "run-ide.sh", "run-mic-freq.sh"] {
        let path = root.join(name);
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let helper = concat!(env!("CARGO_MANIFEST_DIR"), "/tools/desktop_shortcuts.py");
    let command = |arguments: &[&str]| Command::new("python3").arg(helper).arg("--root").arg(&root).args(arguments).env("XDG_DATA_HOME", &data).env("XDG_DESKTOP_DIR", &desktop).output().unwrap();
    let installed = command(&["--desktop"]);
    assert!(installed.status.success(), "{}", String::from_utf8_lossy(&installed.stderr));
    let shortcut = data.join("applications/selenite-consolegpu.desktop");
    let original = std::fs::read_to_string(&shortcut).unwrap();
    assert!(original.lines().any(|line| line == "Name=Selenite Terminal"));
    assert!(original.contains("suite with spaces %%/run.sh\" \"--terminal-window\""));
    assert!(original.contains("Actions=SetAsDefaultTerminal;RestoreDefaultTerminal;"));
    assert!(original.contains("Name=Use Selenite as Default Terminal"));
    assert!(original.contains("--set-gnome-terminal"));
    assert!(original.contains("--restore-gnome-terminal"));
    let assets = std::fs::read_to_string(data.join("applications/selenite-assets.desktop")).unwrap();
    assert!(assets.lines().any(|line| line == "Name=Selenite Asset Manager"));
    assert!(assets.contains("\"--inventory\""));
    assert!(desktop.join("selenite.desktop").is_file());
    let icon = directory.join("custom icon.png");
    image::RgbImage::new(32, 32).save(&icon).unwrap();
    let changed = command(&["--update-icon", "--icon", icon.to_str().unwrap()]);
    assert!(changed.status.success(), "{}", String::from_utf8_lossy(&changed.stderr));
    let updated = std::fs::read_to_string(&shortcut).unwrap();
    assert_eq!(original.lines().find(|line| line.starts_with("Exec=")), updated.lines().find(|line| line.starts_with("Exec=")));
    assert!(updated.contains("selenite-custom.png"));
    assert!(data.join("icons/selenite-custom.png").is_file());
    assert!(command(&[]).status.success());
    assert!(std::fs::read_to_string(&shortcut).unwrap().contains("selenite-custom.png"));
    let protected = data.join("applications/selenite.desktop");
    std::fs::write(&protected, "[Desktop Entry]\nName=Personal launcher\n").unwrap();
    assert!(!command(&[]).status.success());
    assert!(std::fs::read_to_string(protected).unwrap().contains("Personal launcher"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn gnome_default_terminal_can_be_enabled_and_restored() {
    use std::os::unix::fs::PermissionsExt;
    let directory = std::env::temp_dir().join(format!("selenite-gnome-terminal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let home = directory.join("selenite-home");
    let config = directory.join("config");
    let data = directory.join("data");
    let suite = directory.join("suite");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::create_dir_all(&suite).unwrap();
    let previous_terminals = "org.gnome.Ptyxis.desktop\n";
    std::fs::write(config.join("gnome-xdg-terminals.list"), previous_terminals).unwrap();
    let selenite = suite.join("selenite");
    std::fs::write(&selenite, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&selenite, std::fs::Permissions::from_mode(0o755)).unwrap();
    let installed = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tools/desktop_shortcuts.py"))
        .args(["--root", suite.to_str().unwrap(), "--tool", "terminal"])
        .env("XDG_DATA_HOME", &data)
        .output()
        .unwrap();
    assert!(installed.status.success(), "{}", String::from_utf8_lossy(&installed.stderr));
    let read_setting = |key: &str| {
        let output = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.default-applications.terminal", key])
            .env("GSETTINGS_BACKEND", "keyfile")
            .env("XDG_CONFIG_HOME", &config)
            .output()
            .ok()?;
        output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let Some(original_exec) = read_setting("exec") else {
        std::fs::remove_dir_all(directory).unwrap();
        return;
    };
    let Some(original_arg) = read_setting("exec-arg") else {
        std::fs::remove_dir_all(directory).unwrap();
        return;
    };
    let run = |argument: &str| {
        Command::new(env!("CARGO_BIN_EXE_selenite"))
            .arg(argument)
            .env("SELENITE_HOME", &home)
            .env("XDG_CONFIG_HOME", &config)
            .env("XDG_DATA_HOME", &data)
            .env("GSETTINGS_BACKEND", "keyfile")
            .output()
            .unwrap()
    };
    let enabled = run("--set-gnome-terminal");
    assert!(enabled.status.success(), "{}", String::from_utf8_lossy(&enabled.stderr));
    assert!(read_setting("exec").unwrap().contains("selenite"));
    assert!(read_setting("exec-arg").unwrap().contains("--terminal-window"));
    assert_eq!(std::fs::read_to_string(config.join("gnome-xdg-terminals.list")).unwrap(), format!("selenite-consolegpu.desktop\n{previous_terminals}"));
    if Command::new("xdg-terminal-exec").arg("--help").output().is_ok() {
        let dispatched = Command::new("xdg-terminal-exec")
            .args(["--print-cmd", "--dir=/tmp"])
            .env("XDG_CURRENT_DESKTOP", "GNOME")
            .env("XDG_CONFIG_HOME", &config)
            .env("XDG_DATA_HOME", &data)
            .env("XDG_CACHE_HOME", directory.join("cache"))
            .output()
            .unwrap();
        assert!(dispatched.status.success(), "{}", String::from_utf8_lossy(&dispatched.stderr));
        let command = String::from_utf8_lossy(&dispatched.stdout);
        assert!(command.contains(selenite.to_str().unwrap()), "{command}");
        assert!(command.contains("--terminal-window"), "{command}");
        assert!(command.contains("--working-directory\n/tmp"), "{command}");
    }
    let restored = run("--restore-gnome-terminal");
    assert!(restored.status.success(), "{}", String::from_utf8_lossy(&restored.stderr));
    assert_eq!(read_setting("exec").unwrap(), original_exec);
    assert_eq!(read_setting("exec-arg").unwrap(), original_arg);
    assert_eq!(std::fs::read_to_string(config.join("gnome-xdg-terminals.list")).unwrap(), previous_terminals);
    assert!(!home.join("gnome-terminal-previous.json").exists());
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn desktop_shortcuts_work_with_native_executables_and_no_shell_launchers() {
    use std::os::unix::fs::PermissionsExt;
    let directory = std::env::temp_dir().join(format!("selenite-native-shortcuts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let root = directory.join("runtime");
    let desktop = directory.join("Desktop");
    std::fs::create_dir_all(root.join("mic_freq")).unwrap();
    std::fs::create_dir_all(root.join("examples/games")).unwrap();
    std::fs::write(root.join("examples/games/starter.rb"), "puts 1").unwrap();
    for binary in ["selenite", "selenite-game", "mic_freq/mic_freq"] {
        let path = root.join(binary);
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let output = Command::new("python3").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tools/desktop_shortcuts.py"))
        .arg("--root").arg(&root).arg("--desktop-only").env("XDG_DESKTOP_DIR", &desktop).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let entries = std::fs::read_dir(&desktop).unwrap().map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap()).collect::<Vec<_>>();
    assert_eq!(entries.len(), 6);
    for name in ["Selenite", "Selenite IDE", "Selenite Terminal", "Selenite Audio Studio", "Selenite Asset Manager", "Selenite Game"] {
        assert!(entries.iter().any(|entry| entry.lines().any(|line| line == format!("Name={name}"))));
    }
    assert!(entries.iter().all(|entry| !entry.lines().any(|line| line.starts_with("Exec=") && line.contains(".sh"))));
    assert!(entries.iter().any(|entry| entry.contains("\"--ide\"")));
    assert!(entries.iter().any(|entry| entry.contains("mic_freq/mic_freq\"")));
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn desktop_shortcuts_support_files_folders_projects_games_and_grids() {
    use std::{os::unix::fs::PermissionsExt, path::PathBuf, time::{Duration, Instant}};
    let directory = std::env::temp_dir().join(format!("selenite-shortcut-targets-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let root = directory.join("suite with spaces %");
    let desktop = directory.join("Desktop");
    let data = directory.join("data");
    let recorded = directory.join("arguments.txt");
    std::fs::create_dir_all(&root).unwrap();
    for launcher in ["run.sh", "run-ide.sh", "run-game.sh", "run-mic-freq.sh"] {
        let path = root.join(launcher);
        std::fs::write(&path, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$SELENITE_SHORTCUT_TEST_ARGS\"\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let file = directory.join("notes $USER `echo test` \"quoted\" % \\.txt");
    std::fs::write(&file, "content").unwrap();
    let folder = directory.join("folder");
    std::fs::create_dir(&folder).unwrap();
    let project = directory.join("project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(project.join("selenite-project.json"), "{}").unwrap();
    let game = directory.join("game.rb");
    let grid = directory.join("grid.json");
    std::fs::write(&game, "puts 1").unwrap();
    std::fs::write(&grid, "{}").unwrap();
    let helper = concat!(env!("CARGO_MANIFEST_DIR"), "/tools/desktop_shortcuts.py");
    for (tool, target, flag) in [("file", &file, "--edit"), ("file", &folder, ""), ("project", &project, "--project"), ("project-ide", &project, "--ide"), ("game", &game, ""), ("grid", &grid, "--grid"), ("assets", &grid, "--inventory")] {
        let output = Command::new("python3").arg(helper).arg("--root").arg(&root)
            .args(["--desktop-only", "--json", "--tool", tool]).arg("--target").arg(target)
            .env("XDG_DATA_HOME", &data).env("XDG_DESKTOP_DIR", &desktop).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let shortcut = PathBuf::from(result["shortcuts"][0].as_str().unwrap());
        assert!(shortcut.starts_with(&desktop));
        assert!(shortcut.is_file());
        let entry = std::fs::read_to_string(&shortcut).unwrap();
        assert!(entry.contains(flag));
        if target == &file && Command::new("gio").arg("version").output().is_ok_and(|output| output.status.success()) {
            let launched = Command::new("gio").arg("launch").arg(&shortcut).env("SELENITE_SHORTCUT_TEST_ARGS", &recorded).output().unwrap();
            assert!(launched.status.success(), "{}", String::from_utf8_lossy(&launched.stderr));
            let deadline = Instant::now() + Duration::from_secs(5);
            while !recorded.exists() { assert!(Instant::now() < deadline); std::thread::sleep(Duration::from_millis(20)); }
            assert_eq!(std::fs::read_to_string(&recorded).unwrap(), format!("--edit\n{}\n", target.display()));
        }
    }
    assert!(!data.join("applications").exists(), "Desktop-only creation installed unrelated application entries");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn with_terminal_is_rejected_outside_the_grid_window() {
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .args(["--with-terminal", "--version"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("only applies to the grid window"));
}

#[test]
fn version_flags_print_the_package_version() {
    for flag in ["--version", "-V"] {
        let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
            .arg(flag)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            concat!("selenite ", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn ide_requires_one_ruby_source_path() {
        for args in [vec!["--ide"], vec!["--ide", "game.py"], vec!["--ide", "game.rb", "extra"]] {
                let output = Command::new(env!("CARGO_BIN_EXE_selenite")).args(args).output().unwrap();
                assert!(!output.status.success());
                assert!(String::from_utf8_lossy(&output.stderr).contains("--ide"));
        }
}

#[cfg(feature = "scripting")]
#[test]
fn project_checks_validate_ruby_without_executing_or_writing_saves() {
    let directory = std::env::temp_dir().join(format!("selenite-project-check-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let created = Command::new(env!("CARGO_BIN_EXE_selenite")).arg("--new-project").arg(&directory).arg("--app").output().unwrap();
    assert!(created.status.success());
    std::fs::write(directory.join("app.rb"), "File.write(File.join(__dir__, 'executed.txt'), 'must not run')\n").unwrap();
    for binary in [env!("CARGO_BIN_EXE_selenite"), env!("CARGO_BIN_EXE_selenite-game")] {
        std::fs::write(directory.join("helper.rb"), "raise 'must not execute'\n").unwrap();
        let output = Command::new(binary).arg("--check-project").arg(&directory).env_remove("DISPLAY").output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["ruby_files_checked"], 2);
        assert!(!directory.join("executed.txt").exists());
        assert!(!directory.join("save.json").exists());
        std::fs::write(directory.join("helper.rb"), "def broken(\n").unwrap();
        let rejected = Command::new(binary).arg("--check-project").arg(&directory).output().unwrap();
        assert!(!rejected.status.success());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("helper.rb"));
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn project_commands_create_run_and_preserve_existing_files() {
    let directory = std::env::temp_dir().join(format!("selenite-v11-project with spaces-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let output = Command::new(env!("CARGO_BIN_EXE_selenite")).arg("--new-project").arg(&directory).arg("--app").output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(directory.join("selenite-project.json")).unwrap()).unwrap();
    assert_eq!(manifest["kind"], "app");
    let original = std::fs::read(directory.join("app.rb")).unwrap();
    std::fs::write(directory.join("app.rb"), [original.as_slice(), b"\nraise 'wrong project directory' unless Dir.pwd == File.expand_path(__dir__)\n"].concat()).unwrap();
    let original = std::fs::read(directory.join("app.rb")).unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_selenite")).arg("--new-project").arg(&directory).output().unwrap();
    assert!(!rejected.status.success());
    assert_eq!(std::fs::read(directory.join("app.rb")).unwrap(), original);
    for (binary, count) in [(env!("CARGO_BIN_EXE_selenite"), 1), (env!("CARGO_BIN_EXE_selenite"), 2), (env!("CARGO_BIN_EXE_selenite-game"), 3)] {
        let output = Command::new(binary).arg("--project").arg(&directory).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains(&format!("application run: {count}")));
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn portable_ide_launcher_preserves_project_arguments_and_help_flags() {
    use std::os::unix::fs::PermissionsExt;
    let directory = std::env::temp_dir().join(format!("selenite-ide-launcher-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let launcher = directory.join("run-ide.sh");
    std::fs::write(&launcher, include_str!("../bin/portable-run.sh")).unwrap();
    let binary = directory.join("selenite");
    std::fs::write(&binary, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    for path in [&launcher, &binary] { std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap(); }
    let default = Command::new(&launcher).output().unwrap();
    assert!(default.status.success());
    assert_eq!(String::from_utf8_lossy(&default.stdout).lines().collect::<Vec<_>>(), vec!["--ide".to_owned(), directory.join("examples/projects/starter").to_string_lossy().into_owned()]);
    let arguments = ["project with spaces", "--grid", "custom save.json"];
    let project = Command::new(&launcher).args(arguments).output().unwrap();
    assert!(project.status.success());
    assert_eq!(String::from_utf8_lossy(&project.stdout).lines().collect::<Vec<_>>(), vec!["--ide", arguments[0], arguments[1], arguments[2]]);
    let help = Command::new(&launcher).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert_eq!(String::from_utf8_lossy(&help.stdout).trim(), "--help");
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn portable_mic_freq_launcher_preserves_companion_arguments() {
    use std::os::unix::fs::PermissionsExt;
    let directory = std::env::temp_dir().join(format!("selenite-mic launcher-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("mic_freq/assets")).unwrap();
    let launcher = directory.join("run-mic-freq.sh");
    std::fs::write(&launcher, include_str!("../bin/portable-run.sh")).unwrap();
    let binary = directory.join("mic_freq/mic_freq");
    std::fs::write(&binary, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    for path in [&launcher, &binary] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    for arguments in [vec!["--help"], vec!["my avatar folder", "--device", "USB microphone", "--capture"]] {
        let output = Command::new(&launcher).args(&arguments).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(String::from_utf8_lossy(&output.stdout).lines().collect::<Vec<_>>(), arguments);
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn distributor_installs_the_full_suite_and_rejects_unsafe_archives() {
    use std::os::unix::fs::PermissionsExt;
    if !Command::new("python3").args(["-c", "import sys, tarfile; sys.exit(0 if sys.version_info >= (3,11) and hasattr(tarfile, 'data_filter') else 1)"]).status().is_ok_and(|status| status.success()) {
        eprintln!("skipping distributor test: Python with safe tar extraction is required");
        return;
    }
    let directory = std::env::temp_dir().join(format!("selenite-distribute-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let fixture = directory.join("complete suite");
    let home = directory.join("home");
    for folder in ["lib/ruby", "lib/ruby-arch", "mic_freq/assets", "docs", "examples", "partitioned_array/lib"] {
        std::fs::create_dir_all(fixture.join(folder)).unwrap();
    }
    std::fs::create_dir_all(&home).unwrap();
    let binaries = ["selenite", "selenite-game", "mic_freq/mic_freq"];
    for name in binaries.into_iter().chain(["run.sh", "run-ide.sh", "run-game.sh", "run-mic-freq.sh"]) {
        let file = fixture.join(name);
        std::fs::write(&file, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    for name in ["mic_freq/README.md", "mic_freq/MANUAL.md", "partitioned_array/lib/managed_partitioned_array.rb"] {
        std::fs::write(fixture.join(name), "bundled\n").unwrap();
    }
    let mic_freq_manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/mic_freq/Cargo.toml");
    let mic_freq_version = Command::new("python3").args(["-c", "import sys, tomllib; print(tomllib.load(open(sys.argv[1], 'rb'))['package']['version'])", mic_freq_manifest]).output().unwrap();
    assert!(mic_freq_version.status.success());
    let mic_freq_version = String::from_utf8(mic_freq_version.stdout).unwrap().trim().to_owned();
    let metadata = serde_json::json!({"schema":1, "name":"selenite", "version":env!("CARGO_PKG_VERSION"), "binaries":binaries, "components":{"mic_freq":{"version":mic_freq_version}}, "capabilities":{"mic_freq":true}});
    std::fs::write(fixture.join("package-info.json"), serde_json::to_vec(&metadata).unwrap()).unwrap();
    let archive = directory.join("suite.tar.gz");
    let pack = |archive: &std::path::Path| {
        assert!(Command::new("tar").arg("-czf").arg(archive).arg("-C").arg(&fixture).arg(".").status().unwrap().success());
    };
    pack(&archive);
    let destination = home.join("Selenite installation");
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/distribute.sh");
    let distribute = |archive: &std::path::Path, destination: &std::path::Path, dry: bool| {
        let mut command = Command::new("bash");
        command.arg(script).arg("--archive").arg(archive).arg("--destination").arg(destination);
        if dry { command.arg("--dry-run"); }
        command.output().unwrap()
    };
    let mut stale_metadata = metadata.clone();
    stale_metadata["components"]["mic_freq"]["version"] = serde_json::json!("0.0.0");
    std::fs::write(fixture.join("package-info.json"), serde_json::to_vec(&stale_metadata).unwrap()).unwrap();
    let stale_archive = directory.join("stale-mic-freq.tar.gz");
    pack(&stale_archive);
    let stale = distribute(&stale_archive, &home.join("stale install"), true);
    assert!(!stale.status.success());
    assert!(String::from_utf8_lossy(&stale.stderr).contains("mic_freq"));
    assert!(!home.join("stale install").exists());
    std::fs::write(fixture.join("package-info.json"), serde_json::to_vec(&metadata).unwrap()).unwrap();
    let dry = distribute(&archive, &destination, true);
    assert!(dry.status.success(), "{}", String::from_utf8_lossy(&dry.stderr));
    assert!(!destination.exists() && !home.join("suite.tar.gz").exists());
    let default_dry = Command::new("bash").arg(script).arg("--archive").arg(&archive).arg("--dry-run").env("HOME", &home).output().unwrap();
    assert!(default_dry.status.success(), "{}", String::from_utf8_lossy(&default_dry.stderr));
    assert!(String::from_utf8_lossy(&default_dry.stdout).contains(&format!("Extract to: {}", home.join("selenite").display())));
    assert!(String::from_utf8_lossy(&default_dry.stdout).contains(&format!("Copy to: {}", home.join("suite.tar.gz").display())));
    let installed = distribute(&archive, &destination, false);
    assert!(installed.status.success(), "{}", String::from_utf8_lossy(&installed.stderr));
    assert!(destination.join("mic_freq/mic_freq").is_file());
    assert!(destination.join("mic_freq/assets").is_dir());
    assert!(destination.join("distribution-info.json").is_file());
    assert!(home.join("suite.tar.gz").is_file());
    assert!(Command::new("sha256sum").arg("-c").arg("suite.tar.gz.sha256").current_dir(&home).status().unwrap().success());
    std::fs::write(destination.join("personal.txt"), "keep me").unwrap();
    std::fs::write(destination.join("run.sh"), "old launcher\n").unwrap();
    std::fs::create_dir_all(destination.join("user-project/nested")).unwrap();
    std::fs::write(destination.join("user-project/nested/save.json"), "personal data\n").unwrap();
    let refreshed = distribute(&archive, &destination, false);
    assert!(refreshed.status.success(), "{}", String::from_utf8_lossy(&refreshed.stderr));
    assert_eq!(std::fs::read(destination.join("run.sh")).unwrap(), std::fs::read(fixture.join("run.sh")).unwrap());
    assert_eq!(std::fs::read_to_string(destination.join("user-project/nested/save.json")).unwrap(), "personal data\n");
    assert_eq!(std::fs::read_to_string(destination.join("personal.txt")).unwrap(), "keep me");
    let default_install = || Command::new("bash").arg(script).arg("--archive").arg(&archive).env("HOME", &home).output().unwrap();
    let first_default = default_install();
    assert!(first_default.status.success(), "{}", String::from_utf8_lossy(&first_default.stderr));
    let default_destination = home.join("selenite");
    std::fs::write(default_destination.join("personal.txt"), "preserve default install").unwrap();
    let repeated = default_install();
    assert!(repeated.status.success(), "{}", String::from_utf8_lossy(&repeated.stderr));
    assert!(String::from_utf8_lossy(&repeated.stdout).contains("Updated Selenite"));
    std::fs::write(fixture.join("docs/update.md"), "new archive\n").unwrap();
    pack(&archive);
    let updated = default_install();
    assert!(updated.status.success(), "{}", String::from_utf8_lossy(&updated.stderr));
    assert_eq!(std::fs::read_to_string(default_destination.join("docs/update.md")).unwrap(), "new archive\n");
    assert!(Command::new("sha256sum").arg("-c").arg("suite.tar.gz.sha256").current_dir(&home).status().unwrap().success());
    assert_eq!(std::fs::read_to_string(default_destination.join("personal.txt")).unwrap(), "preserve default install");
    let installs = std::fs::read_dir(&home).unwrap().filter_map(Result::ok).filter(|entry| entry.path().is_dir() && entry.file_name() == "selenite").collect::<Vec<_>>();
    assert_eq!(installs.len(), 1);
    std::fs::remove_file(destination.join("run.sh")).unwrap();
    std::fs::create_dir(destination.join("run.sh")).unwrap();
    std::fs::write(destination.join("selenite"), "must survive conflict\n").unwrap();
    let conflict = distribute(&archive, &destination, false);
    assert!(conflict.status.success(), "{}", String::from_utf8_lossy(&conflict.stderr));
    assert!(destination.join("run.sh").is_file());
    assert_eq!(std::fs::read(destination.join("selenite")).unwrap(), std::fs::read(fixture.join("selenite")).unwrap());
    let unrelated = home.join("unrelated-directory");
    std::fs::create_dir(&unrelated).unwrap();
    std::fs::write(unrelated.join("personal.txt"), "must not delete\n").unwrap();
    let refused = distribute(&archive, &unrelated, false);
    assert!(!refused.status.success());
    assert_eq!(std::fs::read_to_string(unrelated.join("personal.txt")).unwrap(), "must not delete\n");
    let refused = Command::new("bash").arg(script).arg("--archive").arg(&archive).arg("--destination").arg(&home).env("HOME", &home).output().unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("unsafe installation destination"));
    assert!(destination.join("selenite").is_file());
    std::fs::remove_file(fixture.join("mic_freq/mic_freq")).unwrap();
    let incomplete = directory.join("incomplete.tar.gz");
    pack(&incomplete);
    let refused = distribute(&incomplete, &home.join("incomplete install"), false);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("Incomplete suite"));
    assert!(!home.join("incomplete install").exists());
    std::fs::write(destination.join("personal.txt"), "keep if validation fails\n").unwrap();
    let refused = distribute(&incomplete, &destination, false);
    assert!(!refused.status.success());
    assert_eq!(std::fs::read_to_string(destination.join("personal.txt")).unwrap(), "keep if validation fails\n");
    let malicious = directory.join("unsafe.tar.gz");
    assert!(Command::new("python3").arg("-c").arg("import tarfile,io,sys; archive=tarfile.open(sys.argv[1],'w:gz'); member=tarfile.TarInfo('../escape.txt'); member.size=4; archive.addfile(member,io.BytesIO(b'bad!')); archive.close()").arg(&malicious).status().unwrap().success());
    let refused = distribute(&malicious, &home.join("unsafe install"), false);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("Unsafe archive path"));
    assert!(!home.join("unsafe install").exists() && !home.join("escape.txt").exists());
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
#[test]
fn documentation_collector_includes_nested_product_guides_not_generated_copies() {
    if !Command::new("ruby").arg("--version").output().is_ok_and(|output| output.status.success()) {
        eprintln!("skipping documentation collector test: Ruby is required");
        return;
    }
    let directory = std::env::temp_dir().join(format!("selenite-documentation-{}", std::process::id()));
    let source = directory.join("source");
    let destination = source.join("custom-release");
    let included = ["ROOT-NOTES.md", "mic_freq/README.md", "mic_freq/guides/VOICE.MD", "mic_freq/guides/voice guide.md", "docs/ide.md", "src/module/architecture.md", "assets/avatar/USAGE.md", "examples/game/README.md", "partitioned_array/guides/storage.md"];
    let excluded = ["target/generated.md", "mic_freq/target/README.md", ".git/private.md", "dist-previous/README.md", "tmp/scratch.md", "test-artifacts/results.md", "custom-release/stale.md"];
    for name in included.into_iter().chain(excluded) {
        let file = source.join(name);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, format!("# Guide\n{name}\n")).unwrap();
    }
    let output = Command::new("ruby").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tools/package_documentation.rb")).arg(&source).arg(&destination).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(destination.join("documentation-manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema"], 1);
    let mut actual = manifest["documents"].as_array().unwrap().iter().map(|entry| {
        assert_eq!(entry["sha256"].as_str().unwrap().len(), 64);
        entry["path"].as_str().unwrap().to_owned()
    }).collect::<Vec<_>>();
    actual.sort();
    let mut expected = included.map(str::to_owned).to_vec();
    expected.sort();
    assert_eq!(actual, expected);
    for name in included {
        assert_eq!(std::fs::read(source.join(name)).unwrap(), std::fs::read(destination.join(name)).unwrap());
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(feature = "scripting")]
#[test]
fn complete_native_raylib_api_and_console_guard() {
        let directory = std::env::temp_dir().join(format!("selenite-native-api-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("native.rb");
        std::fs::write(&source, r#"
api = Selenite.raylib_native
raise "incomplete API" unless api::API["functions"].size == 600
raise "missing methods" unless api::API["functions"].keys.all? { |name| api.respond_to?(name) }
if ENV["NATIVE_GUARD"]
    begin
        api.enable!
        raise "unsafe console access"
    rescue RuntimeError => error
        raise unless error.message.include?("--game")
    end
else
    api.enable!
    color = api.ColorFromHSV(120, 1, 1)
    raise "bad record" unless color.to_a == [0, 255, 0, 255]
    hsv = api.ColorToHSV(color)
    raise "bad vector" unless (hsv.x - 120).abs < 0.1
    raise "bad alias" unless api.color_to_int(color) == 0x00ff00ff
    raise "bad formatting" unless api.TextFormat("%s %d", "native", 42) == "native 42"
    image = api.GenImageColor(4, 3, color)
    raise "bad image" unless image.width == 4 && image.height == 3 && !image.data.null?
    api.UnloadImage(image)
    collision = api.CheckCollisionBoxes(api::BoundingBox.new(api::Vector3.new(0,0,0), api::Vector3.new(2,2,2)), api::BoundingBox.new(api::Vector3.new(1,1,1), api::Vector3.new(3,3,3)))
    raise "bad 3D conversion" unless collision
    begin
        api.GetColor
        raise "missing arity check"
    rescue ArgumentError
    end
end
puts "NATIVE_API_OK"
"#).unwrap();
        for guard in [false, true] {
                let mut command = Command::new(env!("CARGO_BIN_EXE_selenite"));
                command.arg(if guard { "--script" } else { "--game" }).arg(&source).arg("--grid").arg(directory.join("save.json"));
                if guard { command.env("NATIVE_GUARD", "1"); }
                let output = command.output().unwrap();
                assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
                assert!(String::from_utf8_lossy(&output.stdout).contains("NATIVE_API_OK"));
        }
        std::fs::remove_dir_all(directory).unwrap();
}

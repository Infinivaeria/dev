#![cfg(unix)]

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn terminal_runs_shell_commands_with_inherited_context() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .env("SHELL", "/bin/sh")
        .env("SELENITE_TERMINAL_TEST", "inherited")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "--terminal",
            "-c",
            "read line; printf '%s|%s|%s|%s' \"$line\" \"$SELENITE_TERMINAL_TEST\" \"$PWD\" \"$1\"; exit 37",
            "shell-name",
            "argument with spaces",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"input\n").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(37));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "input|inherited|{}|argument with spaces",
            env!("CARGO_MANIFEST_DIR")
        )
    );
}

#[test]
fn terminal_runs_ruby_and_propagates_its_status() {
    if Command::new("ruby").arg("--version").output().is_err() {
        return;
    }
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .env("SHELL", "/bin/sh")
        .args([
            "--terminal",
            "-c",
            "ruby -e 'puts ARGV.fetch(0).upcase; exit 23' 'ruby argument' | cat; exit 17",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(17));
    assert_eq!(output.stdout, b"RUBY ARGUMENT\n");

    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .env("SHELL", "/bin/sh")
        .args(["--terminal", "-c", "ruby -e 'exit 23'"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(23));
}

#[test]
fn terminal_reports_a_missing_shell() {
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .env("SHELL", "/no-such-selenite-shell")
        .arg("--terminal")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("failed to start shell"));
}

#[test]
fn terminal_runs_command_file_without_interpreting_its_path() {
    let directory =
        std::env::temp_dir().join(format!("selenite-command-file-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let script = directory.join("command with spaces;literal.sh");
    std::fs::write(
        &script,
        "printf '%s\\n' \"${BASH_VERSION:+bash}\" \"$@\"; exit 29\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .arg("--terminal-run")
        .arg(&script)
        .args(["argument with spaces", "--literal", ";$(not-a-command)"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(29));
    assert_eq!(
        output.stdout,
        b"bash\nargument with spaces\n--literal\n;$(not-a-command)\n"
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn terminal_runs_ruby_command_file() {
    if Command::new("ruby").arg("--version").output().is_err() {
        return;
    }
    let directory = std::env::temp_dir().join(format!("selenite-ruby-file-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let script = directory.join("command file.rb");
    std::fs::write(&script, "puts 'ruby file', ARGV; exit 31\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .arg("--terminal-run")
        .arg(&script)
        .args(["ruby argument", "--literal"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(31));
    assert_eq!(output.stdout, b"ruby file\nruby argument\n--literal\n");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn terminal_rejects_missing_files_and_ruby_bytecode() {
    let directory =
        std::env::temp_dir().join(format!("selenite-invalid-file-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let bytecode = directory.join("script.rbc");
    std::fs::write(&bytecode, "not executable").unwrap();
    for path in [directory.join("missing.sh"), bytecode, directory.clone()] {
        let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
            .arg("--terminal-run")
            .arg(path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(!output.stderr.is_empty());
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn terminal_run_preserves_non_utf8_arguments_and_signal_status() {
    use std::os::unix::ffi::OsStringExt;

    let directory =
        std::env::temp_dir().join(format!("selenite-command-bytes-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("command.sh");
    std::fs::write(&path, "printf '%s' \"$1\"; kill -TERM $$\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .arg("--terminal-run")
        .arg(&path)
        .arg(std::ffi::OsString::from_vec(vec![b'a', 0xff, b'b']))
        .output()
        .unwrap();
    assert_eq!(output.stdout, [b'a', 0xff, b'b']);
    assert_eq!(output.status.code(), Some(143));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn terminal_keeps_a_shell_after_command_file_finishes() {
    if Command::new("script").arg("--version").output().is_err() {
        return;
    }
    let directory =
        std::env::temp_dir().join(format!("selenite-command-tty-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("command.sh");
    std::fs::write(&path, "printf 'command finished\\n'\n").unwrap();
    let quote =
        |value: &std::path::Path| format!("'{}'", value.to_string_lossy().replace('\'', "'\\''"));
    let command = format!(
        "{} --terminal-run {}",
        quote(std::path::Path::new(env!("CARGO_BIN_EXE_selenite"))),
        quote(&path)
    );
    let mut child = Command::new("script")
        .env("SHELL", "/bin/sh")
        .args(["-qec", &command, "/dev/null"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"printf 'shell-%s\\n' ready; exit\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("command finished"));
    assert!(stdout.contains("Starting shell."));
    assert!(stdout.contains("shell-ready"));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn terminal_exports_selenite_and_finds_it_on_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .env("SHELL", "/bin/sh")
        .env("PATH", "/usr/bin:/bin")
        .args([
            "--terminal",
            "-c",
            "printf '%s\\n%s' \"$SELENITE_EXE\" \"$(command -v selenite)\"",
        ])
        .output()
        .unwrap();
    let expected = std::fs::canonicalize(env!("CARGO_BIN_EXE_selenite")).unwrap();
    let expected = expected.to_str().unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{expected}\n{expected}")
    );
}

#[test]
fn terminal_run_uses_shebang_interpreter_with_one_argument() {
    use std::os::unix::fs::PermissionsExt;

    let directory = std::env::temp_dir().join(format!("selenite-shebang-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let interpreter = directory.join("interp");
    std::fs::write(&interpreter, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
    std::fs::set_permissions(&interpreter, std::fs::Permissions::from_mode(0o700)).unwrap();
    let script = directory.join("script.custom");
    std::fs::write(
        &script,
        format!("#!{} --flag value\nignored\n", interpreter.display()),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .arg("--terminal-run")
        .arg(&script)
        .arg("extra")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "--flag value\n{}\nextra\n",
            std::fs::canonicalize(&script).unwrap().display()
        )
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn terminal_run_maps_python_extension() {
    if Command::new("python3").arg("--version").output().is_err() {
        return;
    }
    let directory = std::env::temp_dir().join(format!("selenite-python-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let script = directory.join("script.py");
    std::fs::write(&script, "import sys\nprint(sys.argv[1])\nsys.exit(5)\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .arg("--terminal-run")
        .arg(&script)
        .arg("python argument")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(output.stdout, b"python argument\n");
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(all(feature = "scripting", target_os = "linux"))]
#[test]
fn ruby_terminal_api_launches_an_emulator_with_context() {
    use std::os::unix::fs::PermissionsExt;

    let directory =
        std::env::temp_dir().join(format!("selenite-ruby-terminal-{}", std::process::id()));
    let bin = directory.join("bin");
    let work = directory.join("work folder");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    let record = directory.join("record");
    let emulator = bin.join("x-terminal-emulator");
    std::fs::write(
        &emulator,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{0}.args'\npwd -P > '{0}.cwd'\n",
            record.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&emulator, std::fs::Permissions::from_mode(0o700)).unwrap();
    let command_file = directory.join("command.sh");
    std::fs::write(&command_file, "true\n").unwrap();
    let script = directory.join("launch.rb");
    std::fs::write(
        &script,
        format!(
            "Selenite.terminal({:?}, 'two words', dir: {:?})\n",
            command_file.display().to_string(),
            work.display().to_string()
        ),
    )
    .unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .env("PATH", path)
        .env("SELENITE_HOME", directory.join("home"))
        .arg("--script")
        .arg(&script)
        .arg("--grid")
        .arg(directory.join("save.json"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let args_file = directory.join("record.args");
    for _ in 0..100 {
        if args_file.is_file() && directory.join("record.cwd").is_file() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let args = std::fs::read_to_string(&args_file).unwrap();
    let lines: Vec<&str> = args.lines().collect();
    assert_eq!(lines[..2], ["-e", "env"]);
    assert!(lines[2].starts_with("SELENITE_GRID="));
    assert!(lines[3].ends_with("selenite"));
    assert_eq!(
        lines[4..],
        [
            "--terminal-run",
            std::fs::canonicalize(&command_file)
                .unwrap()
                .to_str()
                .unwrap(),
            "two words"
        ]
    );
    assert_eq!(
        std::fs::read_to_string(directory.join("record.cwd"))
            .unwrap()
            .trim(),
        std::fs::canonicalize(&work).unwrap().to_str().unwrap()
    );
    std::fs::remove_dir_all(directory).unwrap();
}

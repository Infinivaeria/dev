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
    let mut command = Command::new(env!("CARGO_BIN_EXE_selenite"));
    command
        .arg("--edit")
        .arg(path)
        .env("SELENITE_HOME", home_for(path))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Starts the editor and returns once its first frame is on screen.
fn editor(path: &Path) -> Child {
    editor_with(path, None, &[])
}

/// Like `editor`, with a fixed window size and extra environment variables.
fn editor_with(path: &Path, size: Option<&str>, env: &[(&str, &str)]) -> Child {
    let ready = path.parent().unwrap().join(format!(
        "ready-{}",
        path.file_name().unwrap().to_string_lossy()
    ));
    let _ = fs::remove_file(&ready);
    let mut command = editor_command(path);
    command
        .env("SELENITE_EDITOR_READY", &ready)
        .envs(env.iter().copied());
    if let Some(size) = size {
        command.env("SELENITE_EDITOR_SIZE", size);
    }
    let mut child = command.spawn().unwrap();
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
    let status = Command::new(key_driver().unwrap())
        .arg(child.id().to_string())
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

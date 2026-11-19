#[path = "../src/cas.rs"]
mod cas;

#[test]
fn notebook_json_roundtrips() {
    let path = std::env::current_dir()
        .unwrap()
        .join(format!(".cas-notebook-test-{}.json", std::process::id()));
    let notebook = cas::Notebook {
        cells: vec![cas::NotebookCell {
            id: "cell-1".into(),
            input: "diff x^2; x".into(),
            result: cas::execute_command("diff x^2; x"),
        }],
    };
    notebook.save(&path).unwrap();
    let loaded = cas::Notebook::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(loaded, notebook);
}

#[test]
fn command_dsl_supports_symbolic_and_matrix_results() {
    assert!(cas::backend_capabilities()
        .iter()
        .any(|capability| capability.contains("license key")));
    let solved = cas::execute_command("solve x+y=3,x-y=1; x,y");
    assert_eq!(solved.error, None);
    assert!(solved.output.contains("x = 2"));

    let inverse = cas::execute_command("inverse [[1,2],[3,4]]");
    assert_eq!(inverse.error, None);
    assert!(inverse.output.starts_with("[["));
}

#[cfg(target_os = "linux")]
#[test]
fn cas_notebook_gui_runs_and_saves_a_cell() {
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    let directory = std::env::temp_dir().join(format!("selenite-cas-gui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let driver: PathBuf = directory.join("keys");
    let compiled = Command::new("cc")
        .args(["-Wall", "-Wextra", "-Werror"])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/x11_key_driver.c"
        ))
        .args(["-lX11", "-o"])
        .arg(&driver)
        .status()
        .unwrap();
    assert!(compiled.success());
    let notebook = directory.join("workbook.json");
    let mut child = Command::new(env!("CARGO_BIN_EXE_selenite"))
        .args(["--cas", notebook.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let sent = Command::new(driver)
        .arg(child.id().to_string())
        .args([
            "wait:900",
            "click:360,132",
            "type:diff x^3+2*x; x",
            "click:55,68",
            "wait:300",
            "key:ctrl+s",
            "wait:200",
            "key:Escape",
        ])
        .status()
        .unwrap();
    assert!(sent.success());
    assert!(child.wait().unwrap().success());
    let saved = cas::Notebook::load(notebook).unwrap();
    assert_eq!(saved.cells.len(), 1);
    assert_eq!(saved.cells[0].result.error, None);
    assert_eq!(saved.cells[0].result.output, "2+3*x^2");
    std::fs::remove_dir_all(directory).unwrap();
}

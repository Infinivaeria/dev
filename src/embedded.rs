pub fn run() -> Result<i32, String> {
    let payload = include_bytes!(env!("SELENITE_EMBEDDED_PAYLOAD"));
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let root = executable.parent().ok_or("Executable has no directory")?;
    let data = root.join("data").join(env!("SELENITE_EMBEDDED_DATA"));
    if !data.is_dir() { return Err("Keep the executable's data and lib directories beside it".into()); }
    let grid_path = std::env::var_os("SELENITE_GRID").map(std::path::PathBuf::from).unwrap_or_else(|| root.join("save.json"));
    let grid = if grid_path.exists() { crate::persistence::SavedGrid::load(&grid_path).map_err(|error| error.to_string())?.unwrap_or_else(crate::persistence::SavedGrid::new) } else { crate::persistence::SavedGrid::new() };
    let grid = std::sync::Arc::new(std::sync::Mutex::new(grid));
    let engine = crate::scripting::ScriptEngine::new()?;
    engine.set_context(crate::scripting::ScriptContext::new(grid.clone(), grid, grid_path));
    let game = env!("SELENITE_EMBEDDED_MODE") == "game";
    if game { crate::game::enable(); }
    let arguments = std::env::args_os().skip(1).map(|argument| argument.into_string().map_err(|_| "Ruby arguments must be valid UTF-8".to_owned())).collect::<Result<Vec<_>, _>>()?;
    let (output, result) = engine.execute_binary(payload, &data, &arguments);
    crate::game::shutdown();
    print!("{output}");
    result
}
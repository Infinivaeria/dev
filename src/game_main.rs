#![allow(dead_code)]

mod download;
mod game;
mod game_cli;
mod grid_inventory;
mod music;
mod persistence;
mod plugins;
mod profiles;
mod scripting;
mod search;
mod ui;

use std::{
    env, fs, process,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

fn timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn main() {
    if let Err(error) = run() {
        eprintln!("selenite-game: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match game_cli::parse(&env::args_os().collect::<Vec<_>>())? {
        game_cli::GameCommand::Help => println!("{}", game_cli::usage()),
        game_cli::GameCommand::Version => println!("selenite-game {}", env!("CARGO_PKG_VERSION")),
        game_cli::GameCommand::Compile {
            source,
            output,
            force,
        } => {
            let engine = scripting::ScriptEngine::new()?;
            engine.compile_file(&source, &output, force)?;
            println!("Compiled {} -> {}", source.display(), output.display());
        }
        game_cli::GameCommand::Run { script, grid } => {
            if !script.is_file() {
                return Err(format!(
                    "game not found: {} (run --help for usage)",
                    script.display()
                ));
            }
            let root = if grid.exists() {
                persistence::SavedGrid::load(&grid)
                    .map_err(|error| error.to_string())?
                    .unwrap_or_else(persistence::SavedGrid::new)
            } else {
                persistence::SavedGrid::new()
            };
            if let Some(parent) = grid
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let root = Arc::new(Mutex::new(root));
            let engine = scripting::ScriptEngine::new()?;
            engine.set_context(scripting::ScriptContext::new(Arc::clone(&root), root, grid));
            game::enable();
            let (printed, result) = engine.eval_file_captured(&script);
            game::shutdown();
            print!("{printed}");
            for request in engine.drain_requests() {
                if let scripting::AppRequest::Status(text) = request {
                    println!("{text}");
                }
            }
            if result.starts_with("error") {
                return Err(result);
            }
        }
    }
    Ok(())
}

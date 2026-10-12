use std::{ffi::OsString, path::PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum GameCommand {
    Help,
    Version,
    Compile {
        source: PathBuf,
        output: PathBuf,
        force: bool,
    },
    Run {
        script: PathBuf,
        grid: PathBuf,
    },
}

pub fn usage() -> String {
    format!(
        "selenite-game {} — Ruby-only game runtime\n\nUsage:\n  selenite-game GAME.rb|GAME.rbc [--grid SAVE.json]\n  selenite-game --game GAME.rb|GAME.rbc [--grid SAVE.json]\n  selenite-game --compile GAME.rb [--output GAME.rbc] [--force]\n  selenite-game --help\n  selenite-game --version\n\nWithout arguments, prefers ./game.rbc, then ./game.rb. Default grid save: save.json beside the script.\nCompilation does not execute the source. Bytecode requires the same Ruby runtime; it is not encryption.\nEmbedded Ruby, Raylib, and PartitionedArray are included in the portable package.",
        env!("CARGO_PKG_VERSION")
    )
}

pub fn validate_ruby_path(path: &std::path::Path) -> Result<(), String> {
    if !matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("rb" | "rbc")
    ) {
        return Err(format!(
            "Ruby-only runtime: {} must have a .rb or .rbc extension",
            path.display()
        ));
    }
    Ok(())
}

pub fn parse_compile(values: &[&str]) -> Result<GameCommand, String> {
    let Some(source) = values.first().filter(|value| !value.starts_with('-')) else {
        return Err("--compile needs a Ruby .rb source file".to_owned());
    };
    let source = PathBuf::from(source);
    if source.extension().and_then(|value| value.to_str()) != Some("rb") {
        return Err("compilation input must be a .rb file".to_owned());
    }
    let mut output = None;
    let mut force = false;
    let mut index = 1;
    while index < values.len() {
        match values[index] {
            "--output" if output.is_none() => {
                index += 1;
                let path = values
                    .get(index)
                    .filter(|value| !value.starts_with('-'))
                    .ok_or("--output needs a .rbc path")?;
                output = Some(PathBuf::from(path));
            }
            "--force" if !force => force = true,
            other => return Err(format!("invalid compile argument: {other}")),
        }
        index += 1;
    }
    let output = output.unwrap_or_else(|| source.with_extension("rbc"));
    if output.extension().and_then(|value| value.to_str()) != Some("rbc") {
        return Err("compilation output must be a .rbc file".to_owned());
    }
    Ok(GameCommand::Compile {
        source,
        output,
        force,
    })
}

pub fn parse(args: &[OsString]) -> Result<GameCommand, String> {
    let values = args
        .iter()
        .skip(1)
        .map(|arg| {
            arg.to_str()
                .ok_or_else(|| "arguments must be valid UTF-8".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    match values.as_slice() {
        ["--help" | "-h"] => return Ok(GameCommand::Help),
        ["--version" | "-V"] => return Ok(GameCommand::Version),
        _ => {}
    }
    if values.first() == Some(&"--compile") {
        return parse_compile(&values[1..]);
    }
    let values = if values.first() == Some(&"--game") {
        &values[1..]
    } else {
        &values[..]
    };
    let (script, grid) = match values {
        [] if args.len() <= 1 => (
            PathBuf::from(if std::path::Path::new("game.rbc").is_file() {
                "game.rbc"
            } else {
                "game.rb"
            }),
            None,
        ),
        [script] if !script.starts_with('-') => (PathBuf::from(script), None),
        [script, "--grid", grid] if !script.starts_with('-') && !grid.starts_with('-') => {
            (PathBuf::from(script), Some(PathBuf::from(grid)))
        }
        _ => return Err(format!("invalid game arguments\n\n{}", usage())),
    };
    validate_ruby_path(&script)?;
    let grid = grid.unwrap_or_else(|| {
        script
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("save.json")
    });
    if grid.extension().and_then(|value| value.to_str()) != Some("json") {
        return Err("grid saves must have a .json extension".to_owned());
    }
    Ok(GameCommand::Run { script, grid })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(arguments: &[&str]) -> Result<GameCommand, String> {
        parse(
            &std::iter::once("selenite-game")
                .chain(arguments.iter().copied())
                .map(OsString::from)
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn local_default_and_explicit_saves() {
        assert_eq!(
            command(&[]).unwrap(),
            GameCommand::Run {
                script: "game.rb".into(),
                grid: "save.json".into()
            }
        );
        assert_eq!(
            command(&["levels/play.rb"]).unwrap(),
            GameCommand::Run {
                script: "levels/play.rb".into(),
                grid: "levels/save.json".into()
            }
        );
        assert_eq!(
            command(&["--game", "play.rb", "--grid", "custom.json"]).unwrap(),
            GameCommand::Run {
                script: "play.rb".into(),
                grid: "custom.json".into()
            }
        );
        assert_eq!(command(&["--version"]).unwrap(), GameCommand::Version);
        assert_eq!(command(&["--help"]).unwrap(), GameCommand::Help);
    }

    #[test]
    fn rejects_other_languages_and_organizer_modes() {
        for arguments in [
            vec!["play.py"],
            vec!["play.js"],
            vec!["play.RB"],
            vec!["--script", "play.rb"],
            vec!["--profile", "work"],
            vec!["--game"],
            vec!["play.rb", "--grid"],
            vec!["play.rb", "--grid", "a.json", "extra"],
            vec!["--help", "extra"],
            vec!["play.rb", "--grid", "play.rb"],
        ] {
            assert!(command(&arguments).is_err(), "{arguments:?}");
        }
    }

    #[test]
    fn bytecode_and_compile_commands() {
        assert_eq!(
            command(&["levels/play.rbc"]).unwrap(),
            GameCommand::Run {
                script: "levels/play.rbc".into(),
                grid: "levels/save.json".into()
            }
        );
        assert_eq!(
            command(&["--compile", "levels/play.rb"]).unwrap(),
            GameCommand::Compile {
                source: "levels/play.rb".into(),
                output: "levels/play.rbc".into(),
                force: false
            }
        );
        assert_eq!(
            command(&[
                "--compile",
                "play.rb",
                "--force",
                "--output",
                "release/game.rbc"
            ])
            .unwrap(),
            GameCommand::Compile {
                source: "play.rb".into(),
                output: "release/game.rbc".into(),
                force: true
            }
        );
        for arguments in [
            vec!["--compile"],
            vec!["--compile", "play.rbc"],
            vec!["--compile", "play.rb", "--output", "play.rb"],
            vec!["--compile", "play.rb", "--output"],
            vec!["--compile", "play.rb", "--force", "--force"],
            vec!["--compile", "play.rb", "--grid", "save.json"],
        ] {
            assert!(command(&arguments).is_err(), "{arguments:?}");
        }
    }
}

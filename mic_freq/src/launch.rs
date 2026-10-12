use std::path::{Path, PathBuf};
use std::process::{Child, Command};

pub struct Launcher {
    pub executable: PathBuf,
    pub target: Option<PathBuf>,
    children: Vec<Child>,
}

impl Launcher {
    pub fn new(executable: PathBuf) -> Self {
        Self {
            executable,
            target: None,
            children: Vec::new(),
        }
    }
    pub fn select(&mut self, path: PathBuf) -> Result<(), String> {
        launch_flag(&path)?;
        if !path.is_file() {
            return Err("The launch target must be an existing file".into());
        }
        self.target = Some(path.canonicalize().map_err(|e| e.to_string())?);
        Ok(())
    }
    pub fn launch(&mut self) -> Result<(), String> {
        let Some(path) = &self.target else {
            return Ok(());
        };
        if !path.is_file() {
            return Err(format!("Target no longer exists: {}", path.display()));
        }
        if self.children.len() >= 8 {
            return Err("Eight launched instances are still open; close one first".into());
        }
        let child = Command::new(&self.executable)
            .arg(launch_flag(path)?)
            .arg(path)
            .spawn()
            .map_err(|e| format!("Could not launch {}: {e}", self.executable.display()))?;
        self.children.push(child);
        Ok(())
    }
    pub fn poll(&mut self) -> Option<String> {
        let mut error = None;
        self.children.retain_mut(|child| match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    error = Some(format!("Selenite exited with {status}"));
                }
                false
            }
            Ok(None) => true,
            Err(e) => {
                error = Some(format!("Could not check Selenite: {e}"));
                false
            }
        });
        error
    }
}

fn launch_flag(path: &Path) -> Result<&'static str, String> {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json") => Ok("--grid"),
        Some("rb") => Ok("--game"),
        _ => Err("Drop a Selenite grid (.json) or game script (.rb)".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn targets_use_explicit_selenite_modes() {
        assert_eq!(launch_flag(Path::new("a grid.json")).unwrap(), "--grid");
        assert_eq!(launch_flag(Path::new("game.RB")).unwrap(), "--game");
        assert!(launch_flag(Path::new("command.sh")).is_err());
    }
}

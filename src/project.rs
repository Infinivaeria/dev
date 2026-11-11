use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

pub const MANIFEST: &str = "selenite-project.json";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Game,
    App,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub name: String,
    pub kind: Kind,
    pub entry: PathBuf,
    pub grid: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
}

impl Manifest {
    fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err(format!(
                "Unsupported project schema {}; expected 1",
                self.schema
            ));
        }
        if self.name.trim().is_empty() {
            return Err("Project name cannot be empty".to_owned());
        }
        for (name, path, extension) in [("entry", &self.entry, "rb"), ("grid", &self.grid, "json")]
        {
            if path.as_os_str().is_empty()
                || path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
                || path.extension().and_then(|value| value.to_str()) != Some(extension)
            {
                return Err(format!(
                    "Project {name} must be a relative .{extension} path inside the project"
                ));
            }
        }
        Ok(())
    }
}

impl Project {
    pub fn suggest_directory(parent: &Path, kind: Kind) -> PathBuf {
        let stem = if kind == Kind::Game {
            "game-project"
        } else {
            "app-project"
        };
        let mut path = parent.join(stem);
        let mut number = 2;
        while path.exists() {
            path = parent.join(format!("{stem}-{number}"));
            number += 1;
        }
        path
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let directory = if path.file_name().is_some_and(|name| name == MANIFEST) {
            path.parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
        } else {
            path
        };
        let root = directory
            .canonicalize()
            .map_err(|error| format!("Could not open project {}: {error}", directory.display()))?;
        let file = root.join(MANIFEST);
        let metadata = fs::metadata(&file).map_err(|error| {
            format!(
                "Could not read {}: {error}. Create a project with --new-project DIR",
                file.display()
            )
        })?;
        if metadata.len() > 64 * 1024 {
            return Err("Project manifest exceeds 64 KiB".to_owned());
        }
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(&file).map_err(|error| error.to_string())?)
                .map_err(|error| format!("Invalid project manifest {}: {error}", file.display()))?;
        manifest.validate()?;
        let project = Self { root, manifest };
        if !project.entry().is_file() {
            return Err(format!(
                "Project entry does not exist: {}",
                project.entry().display()
            ));
        }
        Ok(project)
    }

    pub fn discover(source: &Path) -> Result<Option<Self>, String> {
        let source = std::path::absolute(source).map_err(|error| error.to_string())?;
        for parent in source.parent().into_iter().flat_map(Path::ancestors) {
            if parent.join(MANIFEST).is_file() {
                return Self::load(parent).map(Some);
            }
        }
        Ok(None)
    }

    pub fn entry(&self) -> PathBuf {
        self.root.join(&self.manifest.entry)
    }
    pub fn grid(&self) -> PathBuf {
        self.root.join(&self.manifest.grid)
    }

    pub fn create(directory: &Path, kind: Kind) -> Result<Self, String> {
        let root = std::path::absolute(directory).map_err(|error| error.to_string())?;
        let name = root
            .file_name()
            .ok_or("Choose a project directory below the filesystem root")?
            .to_string_lossy()
            .into_owned();
        if root.exists() {
            if !root.is_dir()
                || fs::read_dir(&root)
                    .map_err(|error| error.to_string())?
                    .next()
                    .is_some()
            {
                return Err(format!(
                    "Project directory must be new or empty: {}",
                    root.display()
                ));
            }
        }
        let parent = root.parent().ok_or("Project directory needs a parent")?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let staging = parent.join(format!(".selenite-new-{}-{unique}", std::process::id()));
        fs::create_dir(&staging).map_err(|error| error.to_string())?;
        let entry = if kind == Kind::Game {
            "game.rb"
        } else {
            "app.rb"
        };
        let manifest = Manifest {
            schema: 1,
            name,
            kind,
            entry: entry.into(),
            grid: "save.json".into(),
        };
        let result: Result<(), String> = (|| {
            fs::write(
                staging.join(MANIFEST),
                serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())? + "\n",
            )
            .map_err(|error| error.to_string())?;
            fs::write(
                staging.join(entry),
                if kind == Kind::Game {
                    GAME_SOURCE
                } else {
                    APP_SOURCE
                },
            )
            .map_err(|error| error.to_string())?;
            fs::write(staging.join("README.md"), PROJECT_README)
                .map_err(|error| error.to_string())?;
            fs::rename(&staging, &root).map_err(|error| {
                format!("Could not install project {}: {error}", root.display())
            })?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result?;
        Self::load(&root)
    }
}

pub fn is_project_path(path: &Path) -> bool {
    path.is_dir() || path.file_name().is_some_and(|name| name == MANIFEST)
}

pub fn source_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![directory.to_owned()];
    let mut files = Vec::new();
    let mut visited = 0;
    while let Some(directory) = pending.pop() {
        visited += 1;
        if visited > 1000 || files.len() >= 10_000 { break; }
        for entry in fs::read_dir(&directory).map_err(|error| format!("Could not list {}: {error}", directory.display()))? {
            let entry = entry.map_err(|error| error.to_string())?;
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            let path = entry.path();
            if kind.is_dir() {
                if !matches!(entry.file_name().to_str(), Some(".git" | "target" | "node_modules" | "vendor" | ".bundle" | "dist" | "dist-game")) { pending.push(path); }
            } else if kind.is_file() && matches!(path.extension().and_then(|value| value.to_str()), Some("rb" | "md" | "txt" | "json" | "toml" | "sh" | "py" | "js" | "rs" | "yaml" | "yml" | "html" | "css")) {
                files.push(path);
                if files.len() >= 10_000 { break; }
            }
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(feature = "scripting")]
pub fn check_project(directory: &Path) -> Result<serde_json::Value, String> {
    let project = Project::load(directory)?;
    let files = source_files(&project.root)?;
    let engine = crate::scripting::ScriptEngine::new()?;
    let mut checked = 0;
    for path in files.iter().filter(|path| path.extension().is_some_and(|extension| extension == "rb")) {
        if fs::metadata(path).map_err(|error| error.to_string())?.len() > 4 * 1024 * 1024 { return Err(format!("Ruby source exceeds 4 MiB: {}", path.display())); }
        engine.check_file(path)?;
        checked += 1;
    }
    let mut warnings = Vec::new();
    if project.grid().exists() {
        if let Some(grid) = crate::persistence::SavedGrid::load(&project.grid()).map_err(|error| error.to_string())? {
            for (_, _, _, _, path) in grid.collect_all_items("") {
                if let Some(path) = path {
                    let path = if path.is_absolute() { path } else { project.root.join(path) };
                    if !path.exists() { warnings.push(format!("Missing asset: {}", path.display())); }
                }
            }
        }
    }
    Ok(serde_json::json!({"schema":1,"project":project.manifest.name,"kind":project.manifest.kind,"root":project.root,"entry":project.manifest.entry,"source_files":files.len(),"ruby_files_checked":checked,"warnings":warnings}))
}

const GAME_SOURCE: &str = r#"Selenite::Game.run(title: "Selenite project", width: 960, height: 600,
                   audio: false, background: Raylib::RAYWHITE) do |game|
  position = Raylib::Vector2.create(120, 160)

  game.update do |elapsed|
    position += game.axis.normalize * (180 * elapsed)
  end

  game.draw { game.circle(position.x, position.y, 22, Raylib::BLUE) }
  game.draw_ui { game.text("Selenite", 24, 24, size: 28) }
end
"#;

const APP_SOURCE: &str = r#"inventory = Selenite.inventory
inventory.add("runs", 1)
Selenite.save
puts ["Selenite application run", inventory["runs"]].join(": ")
"#;

const PROJECT_README: &str = "# Selenite Project\n\nOpen this folder with `selenite --ide .`. Run it with `selenite --project .`.\n\nThe project manifest selects the Ruby entry point, game/app mode and JSON grid save.\nUse the IDE's Help tab for Ruby, raylib, storage, networking and game tutorials.\n\nGame projects use WASD or arrow keys to move the blue player; Escape exits.\nApplication projects run without a graphics window and persist their run count.\n\nKeep project entry/grid paths relative. Generated saves are local application data.\n";

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("selenite-project-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn project_file_scan_skips_dependencies_binaries_and_symlinks() {
        let directory = scratch("file-scan");
        fs::create_dir_all(directory.join("lib")).unwrap();
        fs::create_dir_all(directory.join("target")).unwrap();
        fs::write(directory.join("main.rb"), "puts 1").unwrap();
        fs::write(directory.join("lib/helper.rb"), "puts 2").unwrap();
        fs::write(directory.join("target/generated.rb"), "not source").unwrap();
        fs::write(directory.join("picture.png"), "not text").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&directory, directory.join("lib/cycle")).unwrap();
        let files = source_files(&directory).unwrap();
        assert_eq!(files, vec![directory.join("lib/helper.rb"), directory.join("main.rb")]);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn creates_game_and_application_projects_and_discovers_helpers() {
        for kind in [Kind::Game, Kind::App] {
            let directory = scratch(if kind == Kind::Game {
                "game with spaces"
            } else {
                "app"
            });
            let project = Project::create(&directory, kind).unwrap();
            assert_eq!(project.manifest.kind, kind);
            assert_eq!(project.manifest.schema, 1);
            assert!(project.entry().is_file());
            assert!(!project.grid().exists());
            assert!(directory.join("README.md").is_file());
            fs::create_dir(directory.join("lib")).unwrap();
            assert_eq!(
                Project::discover(&directory.join("lib/helper.rb"))
                    .unwrap()
                    .unwrap()
                    .root,
                project.root
            );
            assert_eq!(
                Project::load(&directory.join(MANIFEST)).unwrap().entry(),
                project.entry()
            );
            fs::remove_dir_all(directory).unwrap();
        }
    }

    #[test]
    fn refuses_to_overwrite_an_existing_directory() {
        let directory = scratch("existing");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("important.txt"), "keep me").unwrap();
        assert!(Project::create(&directory, Kind::Game).is_err());
        assert_eq!(
            fs::read_to_string(directory.join("important.txt")).unwrap(),
            "keep me"
        );
        assert!(!directory.join(MANIFEST).exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn accepts_an_empty_directory_and_rejects_bad_manifests() {
        let directory = scratch("validation");
        fs::create_dir_all(&directory).unwrap();
        let project = Project::create(&directory, Kind::App).unwrap();
        for entry in ["../outside.rb", "/tmp/outside.rb", "game.rbc", ""] {
            let mut manifest = project.manifest.clone();
            manifest.entry = entry.into();
            assert!(manifest.validate().is_err());
        }
        let mut manifest = project.manifest.clone();
        manifest.grid = "../save.json".into();
        assert!(manifest.validate().is_err());
        manifest = project.manifest.clone();
        manifest.schema = 99;
        assert!(manifest.validate().is_err());
        fs::remove_file(project.entry()).unwrap();
        assert!(Project::load(&directory).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
}

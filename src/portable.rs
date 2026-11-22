use std::{collections::HashSet, env, path::{Path, PathBuf}};

pub fn package_root(executable: &Path) -> PathBuf {
    let directory = executable.parent().unwrap_or(Path::new("."));
    if directory.file_name().is_some_and(|name| name == "mic_freq") {
        if let Some(parent) = directory.parent().filter(|parent| parent.join("lib").is_dir()) {
            return parent.to_owned();
        }
    }
    directory.to_owned()
}

fn ruby_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let library = root.join("lib/ruby");
    if !library.is_dir() { return Ok(Vec::new()); }
    let mut versions = std::fs::read_dir(&library).map_err(|error| error.to_string())?
        .filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.is_dir()).collect::<Vec<_>>();
    versions.sort();
    let mut paths = versions.into_iter().take(1).collect::<Vec<_>>();
    if root.join("lib/ruby-arch").is_dir() { paths.push(root.join("lib/ruby-arch")); }
    Ok(paths)
}

fn prepend_paths(name: &str, paths: Vec<PathBuf>) -> Result<(), String> {
    if paths.is_empty() { return Ok(()); }
    let inherited = env::var_os(name).unwrap_or_default();
    let mut seen = HashSet::new();
    let paths = paths.into_iter().chain(env::split_paths(&inherited).filter(|path| !path.as_os_str().is_empty())).filter(|path| seen.insert(path.clone()));
    let joined = env::join_paths(paths).map_err(|error| format!("Invalid {name}: {error}"))?;
    // SAFETY: initialize() is called at process startup before any worker threads exist.
    unsafe { env::set_var(name, joined) };
    Ok(())
}

pub fn initialize() -> Result<(), String> {
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let root = package_root(&executable);
    // SAFETY: Both executable entry points call initialize() before creating threads.
    unsafe {
        env::set_var("SELENITE_ROOT", &root);
        env::set_var("SELENITE_EXE", &executable);
    }
    if root.join("lib").is_dir() {
        prepend_paths(if cfg!(target_os = "macos") { "DYLD_LIBRARY_PATH" } else { "LD_LIBRARY_PATH" }, vec![root.join("lib")])?;
    }
    prepend_paths("RUBYLIB", ruby_paths(&root)?)?;
    let mut command_paths = vec![root.clone(), root.join("ffmpeg/bin")];
    if let Some(bin) = executable.parent() {
        if package_root(bin) != root {
            command_paths.push(bin.to_path_buf());
        }
    }
    prepend_paths("PATH", command_paths)?;
    if env::var_os("RUBY_YJIT_ENABLE").is_none() {
        // SAFETY: This startup-time mutation happens before application threads exist.
        unsafe { env::set_var("RUBY_YJIT_ENABLE", "1") };
    }
    if env::var_os("SELENITE_PARTITIONED_ARRAY_LIB").is_none() && root.join("partitioned_array/lib").is_dir() {
        // SAFETY: This startup-time mutation happens before application threads exist.
        unsafe { env::set_var("SELENITE_PARTITIONED_ARRAY_LIB", root.join("partitioned_array/lib")) };
    }
    Ok(())
}

pub fn game_entry(executable: &Path, working_directory: &Path) -> PathBuf {
    for name in ["game.rbc", "game.rb"] {
        if working_directory.join(name).is_file() { return PathBuf::from(name); }
    }
    let root = package_root(executable);
    for name in ["game.rbc", "game.rb", "examples/games/starter.rb"] {
        let path = root.join(name);
        if path.is_file() { return path; }
    }
    PathBuf::from("game.rb")
}

pub fn default_game() -> PathBuf {
    game_entry(&env::current_exe().unwrap_or_else(|_| PathBuf::from("selenite-game")), &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

pub fn default_project() -> Option<PathBuf> {
    let working = env::current_dir().ok()?;
    if working.join("selenite-project.json").is_file() { return Some(working); }
    let root = package_root(&env::current_exe().ok()?);
    let starter = root.join("examples/projects/starter");
    starter.join("selenite-project.json").is_file().then_some(starter)
}

pub fn source_root(executable: &Path) -> Option<PathBuf> {
    let mut current = package_root(executable);
    loop {
        if current.join("Cargo.toml").is_file() && current.join("package.sh").is_file() {
            return Some(current);
        }
        let parent = current.parent()?;
        if parent == current {
            return None;
        }
        current = parent.to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_resources_and_game_entry_follow_the_executable() {
        let root = env::temp_dir().join(format!("selenite-portable-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for directory in ["lib/ruby/4.0.0", "lib/ruby-arch", "mic_freq", "other"] { std::fs::create_dir_all(root.join(directory)).unwrap(); }
        assert_eq!(ruby_paths(&root).unwrap(), vec![root.join("lib/ruby/4.0.0"), root.join("lib/ruby-arch")]);
        assert_eq!(package_root(&root.join("mic_freq/mic_freq")), root);
        std::fs::write(root.join("game.rb"), "source").unwrap();
        assert_eq!(game_entry(&root.join("selenite-game"), &root.join("other")), root.join("game.rb"));
        std::fs::write(root.join("game.rbc"), "bytecode").unwrap();
        assert_eq!(game_entry(&root.join("selenite-game"), &root.join("other")), root.join("game.rbc"));
        std::fs::write(root.join("other/game.rb"), "local").unwrap();
        assert_eq!(game_entry(&root.join("selenite-game"), &root.join("other")), PathBuf::from("game.rb"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_root_finds_checkout_and_rejects_installed_runtime() {
        let root = env::temp_dir().join(format!("selenite-source-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("target/debug")).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(root.join("package.sh"), "#!/bin/sh").unwrap();
        assert_eq!(
            source_root(&root.join("target/debug/selenite")),
            Some(root.clone())
        );
        let runtime =
            env::temp_dir().join(format!("selenite-installed-{}/selenite", std::process::id()));
        assert_eq!(source_root(&runtime), None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
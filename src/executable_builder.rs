use std::{ffi::OsString, path::PathBuf, process::Command};

pub fn run(arguments: &[OsString]) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let packaged = executable.parent().unwrap().join("sdk");
    let sdk = std::env::var_os("SELENITE_BUILD_SDK").map(PathBuf::from)
        .unwrap_or_else(|| if packaged.join("Cargo.toml").is_file() { packaged } else { PathBuf::from(env!("CARGO_MANIFEST_DIR")) });
    let ruby = std::env::var_os("RUBY").unwrap_or_else(|| "ruby".into());
    let mut command = Command::new(ruby);
    command.args(["-e", concat!("__dummy = nil\n", include_str!("../tools/build_executable.rb"), "\nrequire 'pathname'\nbegin\nSeleniteExecutable.build(ARGV)\nrescue StandardError => error\nwarn error.message\nexit 1\nend\n"), "--", "--sdk"])
        .arg(sdk).args(arguments)
        .env_remove("RUBYLIB").env_remove("RUBYOPT").env_remove("LD_LIBRARY_PATH").env_remove("DYLD_LIBRARY_PATH");
    let status = command.status().map_err(|error| format!("Executable builds require Ruby, Cargo and native build tools: {error}"))?;
    if status.success() { Ok(()) } else { Err(format!("Executable build failed ({status})")) }
}
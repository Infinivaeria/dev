use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let status = Command::new(std::env::var_os("RUBY").unwrap_or_else(|| "ruby".into()))
        .args(["-e", include_str!("../bootstrap.rb"), "--"])
        .args(std::env::args_os().skip(1))
        .env("RUSTBY_SDK", env!("CARGO_MANIFEST_DIR"))
        .status();
    match status {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8),
        Err(error) => {
            eprintln!("rustby: could not start Ruby: {error}");
            ExitCode::FAILURE
        }
    }
}

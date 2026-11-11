use std::{env, process::Command};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(selenite_embedded)");
    println!("cargo:rerun-if-env-changed=SELENITE_EMBEDDED_PAYLOAD");
    println!("cargo:rerun-if-env-changed=SELENITE_EMBEDDED_MODE");
    println!("cargo:rerun-if-env-changed=SELENITE_EMBEDDED_DATA");
    if let Ok(payload) = env::var("SELENITE_EMBEDDED_PAYLOAD") {
        println!("cargo:rustc-cfg=selenite_embedded");
        println!("cargo:rustc-env=SELENITE_EMBEDDED_PAYLOAD={payload}");
        println!("cargo:rustc-env=SELENITE_EMBEDDED_MODE={}", env::var("SELENITE_EMBEDDED_MODE").unwrap_or_else(|_| "game".into()));
        println!("cargo:rustc-env=SELENITE_EMBEDDED_DATA={}", env::var("SELENITE_EMBEDDED_DATA").unwrap_or_else(|_| ".".into()));
        println!("cargo:rerun-if-changed={payload}");
    }
    println!("cargo:rerun-if-env-changed=RUBY");
    let target = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target == "linux" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/lib");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib");
    } else if target == "macos" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/lib");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../lib");
    } else {
        return;
    }
    if env::var_os("CARGO_FEATURE_SCRIPTING").is_some() {
        let ruby = env::var_os("RUBY").unwrap_or_else(|| "ruby".into());
        if let Ok(output) = Command::new(ruby).args(["-rrbconfig", "-e", "print RbConfig::CONFIG['libdir']"]).output() {
            if output.status.success() {
                let directory = String::from_utf8_lossy(&output.stdout);
                if !directory.is_empty() && !directory.contains(['\n', '\r', ',']) {
                    println!("cargo:rustc-link-arg=-Wl,-rpath,{directory}");
                }
            }
        }
    }
}
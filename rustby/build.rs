use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=RUBY");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/lib");
        let output = Command::new(env::var_os("RUBY").unwrap_or_else(|| "ruby".into()))
            .args(["-rrbconfig", "-e", "print RbConfig::CONFIG.fetch('libdir')"])
            .output()
            .expect("Ruby is required; set RUBY to the MRI executable");
        assert!(
            output.status.success(),
            "Could not query Ruby's library directory"
        );
        let directory = String::from_utf8(output.stdout).expect("Ruby library path must be UTF-8");
        assert!(
            !directory.contains(['\n', '\r', ',']),
            "Unsupported Ruby library path"
        );
        println!("cargo:rustc-link-arg=-Wl,-rpath,{directory}");
    }
}

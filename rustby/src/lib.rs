use magnus::{prelude::*, RModule};
use std::sync::atomic::{AtomicBool, Ordering};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Configure bundled Ruby load paths before starting any worker threads.
pub fn configure_runtime() -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let root = executable
        .parent()
        .ok_or("Executable has no parent directory")?;
    let paths = [root.join("lib/ruby"), root.join("lib/ruby-arch")];
    if paths.iter().all(|path| path.is_dir()) {
        let inherited = std::env::var_os("RUBYLIB").unwrap_or_default();
        let mut directories = paths.to_vec();
        directories.extend(std::env::split_paths(&inherited).filter(|path| !paths.contains(path)));
        let value = std::env::join_paths(directories).map_err(|error| error.to_string())?;
        // SAFETY: configure_runtime must run at startup before worker threads.
        unsafe { std::env::set_var("RUBYLIB", value) };
    }
    Ok(())
}

/// Execute trusted Rustby bytecode once, on the process's main thread.
///
/// This function owns the Ruby VM for its lifetime. Do not mix it with another
/// embedded Ruby runtime or call it from a worker thread. Ruby cannot restart
/// after cleanup. Ruby exceptions become strings and SystemExit becomes a code.
///
/// # Safety
/// Call only on the process's main thread, before any other Ruby initialization.
/// No other thread may interact with Ruby during this call.
pub unsafe fn run(
    bytes: &[u8],
    arguments: impl IntoIterator<Item = String>,
) -> Result<i32, String> {
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err("The Ruby VM can only be initialized once per process".into());
    }
    let ruby = unsafe { magnus::embed::init() };
    let result = (|| {
        ruby.eval::<magnus::Value>(include_str!("../lib/bytecode.rb"))?;
        let runtime: RModule = ruby.eval("Rustby::Bytecode")?;
        let args = ruby.ary_new();
        for argument in arguments {
            args.push(ruby.str_new(&argument))?;
        }
        let payload = ruby.str_from_slice(bytes);
        runtime.funcall::<_, _, i32>("execute", (payload, args))
    })();
    result.map_err(|error| error.to_string())
}

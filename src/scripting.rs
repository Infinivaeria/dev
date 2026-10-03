//! Embedded Ruby (via Magnus) scripting for Selenite.
//!
//! Only compiled with `--features scripting`. The Ruby VM is started once and
//! kept alive for the process lifetime. `set_context` points the API at the
//! grid / profile the app is currently showing. Ruby code gets:
//!
//! * classic `grid_*` global functions,
//! * a `Selenite` module with a wrapped `Selenite::Grid` class (indexing,
//!   nesting, moving, enumeration), event hooks (`Selenite.on(:activate)`),
//!   and app actions (select/open/goto/download/status/profile switching)
//!   that are queued as [`AppRequest`]s for the UI to perform,
//! * the bundled partitioned_array library through `grid_pa_*`.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, OnceLock},
};

use magnus::{embed::Cleanup, function, method, prelude::*, Error, RHash, RModule, Ruby, Value};

use crate::{
    grid_inventory::GridInventory,
    music::{Repeat, Snapshot},
    persistence::{CellContent, CellKind, SavedGrid},
    plugins::{ItemKind, PluginInfo, PluginItem},
    profiles::ProfileStore,
};

/// Something a script asked the app to do; drained after each evaluation.
#[derive(Clone, Debug, PartialEq)]
pub enum AppRequest {
    Status(String),
    Select(i64, i64),
    Open(i64, i64),
    Goto(i64, i64),
    Enter(i64, i64),
    Back,
    Download {
        url: String,
        col: i64,
        row: i64,
        /// Push onto the cell's stack instead of fanning out / replacing.
        stack: bool,
    },
    /// Replaces the multi-selection (the first cell becomes primary).
    SelectCells(Vec<(i64, i64)>),
    SwitchProfile(String),
    View3d(bool),
    Orbit {
        yaw: f64,
        pitch: f64,
        distance: f64,
    },
    Undo,
    Redo,
    /// Opens the Find bar with this query and jumps to the first match.
    Find(String),
    /// Shows or hides cell labels.
    Labels(bool),
    Music(MusicCommand),
    CopyText(String),
    ReloadPlugins,
}

/// A music-player command queued from Ruby (`Selenite.music.*`).
#[derive(Clone, Debug, PartialEq)]
pub enum MusicCommand {
    /// Play a file (with its grid's audio as the playlist), or resume.
    Play(Option<String>),
    PlayList(Vec<String>, usize),
    Queue(String),
    Toggle,
    Pause,
    Resume,
    Stop,
    Next,
    Previous,
    Clear,
    Show(bool),
    Seek(f64),
    Volume(f64),
    Shuffle(bool),
    Repeat(Repeat),
}

impl MusicCommand {
    pub fn parse(command: &str, arg: Option<String>) -> Result<Self, String> {
        let number = |arg: &Option<String>| {
            arg.as_deref()
                .and_then(|text| text.trim().parse::<f64>().ok())
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("music {command} needs a number"))
        };
        Ok(match command {
            "play" => Self::Play(arg.filter(|path| !path.is_empty())),
            "queue" => Self::Queue(arg.ok_or("music queue needs a path")?),
            "toggle" => Self::Toggle,
            "pause" => Self::Pause,
            "resume" => Self::Resume,
            "stop" => Self::Stop,
            "next" => Self::Next,
            "previous" => Self::Previous,
            "clear" => Self::Clear,
            "show" => Self::Show(true),
            "hide" => Self::Show(false),
            "seek" => Self::Seek(number(&arg)?.max(0.0)),
            "volume" => Self::Volume(number(&arg)?.clamp(0.0, 1.0)),
            "shuffle" => Self::Shuffle(arg.as_deref() == Some("true")),
            "repeat" => Self::Repeat(
                arg.as_deref()
                    .and_then(Repeat::parse)
                    .ok_or("music repeat expects :off, :all or :one")?,
            ),
            other => return Err(format!("unknown music command {other:?}")),
        })
    }
}

/// What the scripts can see of the app's current state.
pub struct ScriptContext {
    /// The grid currently being viewed -- may be a nested sub-grid.
    pub grid: Arc<Mutex<SavedGrid>>,
    /// The profile's top-level grid; saves always go through the root.
    pub root: Arc<Mutex<SavedGrid>>,
    pub save_path: PathBuf,
    pub selected: Option<(i64, i64)>,
    /// Every selected cell (the primary one included).
    pub selection: Vec<(i64, i64)>,
    pub depth: usize,
    pub profile: Option<String>,
    pub profiles: Option<ProfileStore>,
    /// Labels of the next undo / redo steps.
    pub history: (Option<String>, Option<String>),
    pub labels: bool,
    pub music: Snapshot,
}

impl ScriptContext {
    pub fn new(
        grid: Arc<Mutex<SavedGrid>>,
        root: Arc<Mutex<SavedGrid>>,
        save_path: PathBuf,
    ) -> Self {
        Self {
            grid,
            root,
            save_path,
            selected: None,
            selection: Vec::new(),
            depth: 0,
            profile: None,
            profiles: None,
            history: (None, None),
            labels: true,
            music: Snapshot::default(),
        }
    }
}

/// Argument passed to a Ruby event hook.
pub enum HookArg {
    Int(i64),
    Str(String),
    List(Vec<String>),
    Nil,
}

static CONTEXT: OnceLock<Mutex<Option<ScriptContext>>> = OnceLock::new();
static REQUESTS: Mutex<Vec<AppRequest>> = Mutex::new(Vec::new());

fn context_slot() -> MutexGuard<'static, Option<ScriptContext>> {
    CONTEXT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_grid(grid: &Mutex<SavedGrid>) -> MutexGuard<'_, SavedGrid> {
    grid.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn push_request(request: AppRequest) {
    REQUESTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .push(request);
}

pub fn kind_name(kind: CellKind) -> &'static str {
    kind.name()
}

/// An embedded Ruby VM exposing the Selenite API.
pub struct ScriptEngine {
    // Keeps the Ruby VM alive / runs cleanup on drop; never read directly.
    _cleanup: Cleanup,
}

impl ScriptEngine {
    pub fn new() -> Result<Self, String> {
        // `init` (unlike `setup`) runs Ruby's option processing, which
        // populates `$LOAD_PATH` with the standard library and RubyGems.
        let cleanup = unsafe { magnus::embed::init() };
        register_api(&cleanup).map_err(|error| error.to_string())?;
        install_partitioned_array(&cleanup).map_err(|error| error.to_string())?;
        Ok(Self { _cleanup: cleanup })
    }

    /// Snapshot the current grid into a `ManagedPartitionedArray` at `db_path`.
    pub fn pa_export(&self, db_path: &Path) -> Result<i64, String> {
        call_bridge("grid_pa_export", db_path)
    }

    /// Restore a `ManagedPartitionedArray` snapshot into the current grid.
    pub fn pa_import(&self, db_path: &Path) -> Result<i64, String> {
        call_bridge("grid_pa_import", db_path)
    }

    /// Point the API at what the app is currently showing. Call before
    /// every evaluation so scripts see the latest grid/selection/profile.
    pub fn set_context(&self, context: ScriptContext) {
        *context_slot() = Some(context);
    }

    /// Updates just the selection/depth of the existing context.
    #[allow(dead_code)]
    pub fn set_selection(&self, selected: Option<(i64, i64)>) {
        if let Some(context) = context_slot().as_mut() {
            context.selected = selected;
        }
    }

    /// Evaluate Ruby and return `inspect` of the result (or `error: ...`).
    pub fn eval(&self, code: &str) -> String {
        let ruby = match Ruby::get() {
            Ok(ruby) => ruby,
            Err(_) => return "error: Ruby VM not initialized".to_owned(),
        };
        match ruby.eval::<Value>(code) {
            Ok(value) => value.inspect(),
            Err(error) => format!("error: {error}"),
        }
    }

    /// Like [`eval`](Self::eval) but also captures anything the code wrote
    /// to `$stdout`/`$stderr`. Returns `(printed output, result)`.
    pub fn eval_captured(&self, code: &str) -> (String, String) {
        let Ok(ruby) = Ruby::get() else {
            return (String::new(), "error: Ruby VM not initialized".to_owned());
        };
        capture(&ruby, || self.eval(code))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn eval_file(&self, path: &Path) -> String {
        match fs::read_to_string(path) {
            Ok(code) => self.eval_named(&code, path),
            Err(error) => format!("error reading {}: {error}", path.display()),
        }
    }

    pub fn eval_file_captured(&self, path: &Path) -> (String, String) {
        let Ok(ruby) = Ruby::get() else {
            return (String::new(), "error: Ruby VM not initialized".to_owned());
        };
        match fs::read_to_string(path) {
            Ok(code) => capture(&ruby, || self.eval_named(&code, path)),
            Err(error) => (
                String::new(),
                format!("error reading {}: {error}", path.display()),
            ),
        }
    }

    /// Evaluates a script's source at top level under its real filename so
    /// `__FILE__`, `__dir__` and backtrace line numbers are correct.
    fn eval_named(&self, code: &str, path: &Path) -> String {
        let Ok(ruby) = Ruby::get() else {
            return "error: Ruby VM not initialized".to_owned();
        };
        let file = fs::canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
            .display()
            .to_string();
        let result = ruby.eval::<Value>("TOPLEVEL_BINDING").and_then(|binding| {
            ruby.module_kernel()
                .funcall::<_, _, Value>("eval", (code, binding, file, 1))
        });
        match result {
            Ok(value) => value.inspect(),
            Err(error) => format!("error: {error}"),
        }
    }

    /// Whether any Ruby hook is registered for `event`.
    pub fn has_hooks(&self, event: &str) -> bool {
        let Ok(ruby) = Ruby::get() else { return false };
        selenite_module(&ruby)
            .and_then(|module| module.funcall::<_, _, bool>("hooks?", (event,)))
            .unwrap_or(false)
    }

    /// Runs the hooks registered for `event`. Returns whether a hook asked
    /// to suppress the default action (by returning `:handled`) plus any
    /// printed output.
    pub fn emit(&self, event: &str, args: Vec<HookArg>) -> (bool, String) {
        let Ok(ruby) = Ruby::get() else {
            return (false, String::new());
        };
        if !self.has_hooks(event) {
            return (false, String::new());
        }
        let (output, handled) = capture(&ruby, || {
            let array = ruby.ary_new();
            for arg in args {
                let pushed = match arg {
                    HookArg::Int(value) => array.push(value),
                    HookArg::Str(value) => array.push(value),
                    HookArg::List(values) => array.push(values),
                    HookArg::Nil => array.push(ruby.qnil()),
                };
                if let Err(error) = pushed {
                    return Err(error.to_string());
                }
            }
            selenite_module(&ruby)
                .and_then(|module| module.funcall::<_, _, bool>("emit_list", (event, array)))
                .map_err(|error| error.to_string())
        });
        match handled {
            Ok(handled) => (handled, output),
            Err(error) => (false, format!("{output}hook {event} error: {error}")),
        }
    }

    /// Takes the app actions queued by scripts since the last call.
    pub fn drain_requests(&self) -> Vec<AppRequest> {
        std::mem::take(
            &mut *REQUESTS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }

    /// Forgets every loaded plugin (removing their hooks), then evaluates
    /// each enabled file in order. Disabled files are listed but not run.
    /// Returns one entry per plugin (or per file that failed / defined
    /// none) plus everything the files printed.
    pub fn load_plugins(&self, files: &[(PathBuf, bool)]) -> (Vec<PluginInfo>, String) {
        let Ok(ruby) = Ruby::get() else {
            return (Vec::new(), "Ruby VM not initialized".to_owned());
        };
        let plugins = match plugins_module(&ruby) {
            Ok(module) => module,
            Err(error) => return (Vec::new(), error.to_string()),
        };
        let mut output = String::new();
        if let Err(error) = plugins.funcall::<_, _, Value>("reset!", ()) {
            output.push_str(&format!("plugin reset failed: {error}\n"));
        }
        let mut errors: Vec<Option<String>> = Vec::with_capacity(files.len());
        for (file, enabled) in files {
            if !enabled {
                errors.push(None);
                continue;
            }
            let file_text = file.display().to_string();
            let _ = plugins.funcall::<_, _, Value>("current_file=", (file_text,));
            let (printed, result) = self.eval_file_captured(file);
            let _ = plugins.funcall::<_, _, Value>("current_file=", (ruby.qnil(),));
            output.push_str(&printed);
            errors.push(
                result
                    .starts_with("error")
                    .then(|| result.trim_start_matches("error: ").to_owned()),
            );
        }

        let manifest: Vec<RawPlugin> = plugins.funcall("manifest", ()).unwrap_or_else(|error| {
            output.push_str(&format!("plugin manifest failed: {error}\n"));
            Vec::new()
        });
        let mut infos = Vec::new();
        for ((file, enabled), error) in files.iter().zip(errors) {
            if !enabled {
                infos.push(PluginInfo::for_file(file, false, None));
                continue;
            }
            let file_text = file.display().to_string();
            let mut defined: Vec<PluginInfo> = manifest
                .iter()
                .filter(|raw| raw.0 == file_text)
                .map(|(_, name, description, version, items)| PluginInfo {
                    file: file.clone(),
                    name: name.clone(),
                    description: description.clone(),
                    version: version.clone(),
                    enabled: true,
                    error: None,
                    items: items
                        .iter()
                        .filter_map(|(id, kind, label, key, kinds, interval)| {
                            Some(PluginItem {
                                id: *id,
                                kind: ItemKind::parse(kind)?,
                                label: label.clone(),
                                key: key.clone(),
                                kinds: kinds.clone(),
                                interval: *interval,
                            })
                        })
                        .collect(),
                })
                .collect();
            match (defined.first_mut(), error) {
                (Some(first), error) => first.error = error,
                (None, Some(error)) => defined.push(PluginInfo::for_file(file, true, Some(error))),
                (None, None) => defined.push(PluginInfo::for_file(
                    file,
                    true,
                    Some(
                        "defines no plugin; wrap it in Selenite.plugin(\"Name\") { |p| ... }"
                            .to_owned(),
                    ),
                )),
            }
            infos.extend(defined);
        }
        (infos, output)
    }

    /// Runs a plugin action (button, menu item or timer). `cell` is passed
    /// to the block as a `Selenite::Cell`. Returns printed output and the
    /// error message if the block raised.
    pub fn invoke_plugin(&self, id: i64, cell: Option<(i64, i64)>) -> (String, Option<String>) {
        let Ok(ruby) = Ruby::get() else {
            return (String::new(), Some("Ruby VM not initialized".to_owned()));
        };
        let (col, row) = match cell {
            Some((col, row)) => (Some(col), Some(row)),
            None => (None, None),
        };
        let (output, result) = capture(&ruby, || {
            plugins_module(&ruby)?.funcall::<_, _, Option<String>>("invoke", (id, col, row))
        });
        let error = match result {
            Ok(error) => error,
            Err(error) => Some(error.to_string()),
        };
        (output, error)
    }
}

type RawItem = (i64, String, String, Option<String>, Vec<String>, f64);
type RawPlugin = (String, String, String, String, Vec<RawItem>);

fn plugins_module(ruby: &Ruby) -> Result<RModule, Error> {
    selenite_module(ruby)?.const_get("Plugins")
}

fn capture<R>(ruby: &Ruby, f: impl FnOnce() -> R) -> (String, R) {
    let redirected = ruby
        .eval::<Value>(
            "require 'stringio'; $__selenite_out = StringIO.new; \
             $stdout = $__selenite_out; $stderr = $__selenite_out; nil",
        )
        .is_ok();
    let result = f();
    let output = if redirected {
        ruby.eval::<String>(
            "$stdout = STDOUT; $stderr = STDERR; \
             (__s = $__selenite_out.string; $__selenite_out = nil; __s)",
        )
        .unwrap_or_default()
    } else {
        String::new()
    };
    (output, result)
}

fn selenite_module(ruby: &Ruby) -> Result<RModule, Error> {
    ruby.class_object().const_get("Selenite")
}

const PRELUDE: &str = include_str!("ruby/selenite_prelude.rb");

fn register_api(ruby: &Ruby) -> Result<(), Error> {
    ruby.define_global_function("grid_set", function!(grid_set, 3))?;
    ruby.define_global_function("grid_set_file", function!(grid_set_file, 3))?;
    ruby.define_global_function("grid_get", function!(grid_get, 2))?;
    ruby.define_global_function("grid_new_grid", function!(grid_new_grid, 2))?;
    ruby.define_global_function("grid_remove", function!(grid_remove, 2))?;
    ruby.define_global_function("grid_move", function!(grid_move, 4))?;
    ruby.define_global_function("grid_swap", function!(grid_swap, 4))?;
    ruby.define_global_function("grid_clear", function!(grid_clear, 0))?;
    ruby.define_global_function("grid_count", function!(grid_count, 0))?;
    ruby.define_global_function("grid_list", function!(grid_list, 0))?;
    ruby.define_global_function("grid_save", function!(grid_save, 0))?;
    ruby.define_global_function("grid_eval_file", function!(grid_eval_file, 1))?;
    ruby.define_global_function("grid_status", function!(app_status, 1))?;
    ruby.define_global_function("grid_select", function!(app_select, 2))?;
    ruby.define_global_function("grid_download", function!(grid_download, 3))?;
    ruby.define_global_function("grid_list_stacks", function!(grid_list_stacks, 0))?;
    ruby.define_global_function("grid_push_file", function!(grid_push_file, 3))?;
    ruby.define_global_function("grid_push_grid", function!(grid_push_grid, 2))?;

    let module = ruby.define_module("Selenite")?;
    module.const_set("VERSION", env!("CARGO_PKG_VERSION"))?;
    module.define_singleton_method("grid", function!(sel_grid, 0))?;
    module.define_singleton_method("root", function!(sel_root, 0))?;
    module.define_singleton_method("save", function!(grid_save, 0))?;
    module.define_singleton_method("selected", function!(sel_selected, 0))?;
    module.define_singleton_method("depth", function!(sel_depth, 0))?;
    module.define_singleton_method("save_path", function!(sel_save_path, 0))?;
    module.define_singleton_method("assets_dir", function!(sel_assets_dir, 0))?;
    module.define_singleton_method("grid_folder", function!(sel_grid_folder, 0))?;
    module.define_singleton_method("import", function!(sel_import, 3))?;
    module.define_singleton_method("grids_dir", function!(sel_grids_dir, 0))?;
    module.define_singleton_method("classify", function!(sel_classify, 1))?;
    module.define_singleton_method("status", function!(app_status, 1))?;
    module.define_singleton_method("select", function!(app_select, 2))?;
    module.define_singleton_method("open", function!(app_open, 2))?;
    module.define_singleton_method("goto", function!(app_goto, 2))?;
    module.define_singleton_method("enter", function!(app_enter, 2))?;
    module.define_singleton_method("back", function!(app_back, 0))?;
    module.define_singleton_method("view_3d", function!(app_view_3d, 1))?;
    module.define_singleton_method("orbit", function!(app_orbit, 3))?;
    module.define_singleton_method("undo", function!(app_undo, 0))?;
    module.define_singleton_method("redo", function!(app_redo, 0))?;
    module.define_singleton_method("history", function!(sel_history, 0))?;
    module.define_singleton_method("find", function!(app_find, 1))?;
    module.define_singleton_method("search", function!(sel_search, 1))?;
    module.define_singleton_method("__download", function!(app_download, 4))?;
    module.define_singleton_method("selection", function!(sel_selection, 0))?;
    module.define_singleton_method("__select_cells", function!(app_select_cells, 1))?;
    module.define_singleton_method("profile", function!(sel_profile, 0))?;
    module.define_singleton_method("profiles", function!(sel_profiles, 0))?;
    module.define_singleton_method("create_profile", function!(sel_create_profile, 1))?;
    module.define_singleton_method("switch_profile", function!(sel_switch_profile, 1))?;
    module.define_singleton_method("__labels", function!(app_labels, 1))?;
    module.define_singleton_method("labels?", function!(sel_labels, 0))?;
    module.define_singleton_method("copy_text", function!(app_copy_text, 1))?;
    module.define_singleton_method("reload_plugins", function!(app_reload_plugins, 0))?;
    module.define_singleton_method("file_info", function!(sel_file_info, 1))?;
    module.define_singleton_method("image_size", function!(sel_image_size, 1))?;
    module.define_singleton_method("checksum", function!(sel_checksum, 1))?;
    module.define_singleton_method("playable?", function!(sel_playable, 1))?;
    module.define_singleton_method("__music", function!(app_music, 2))?;
    module.define_singleton_method("__music_list", function!(app_music_list, 2))?;
    module.define_singleton_method("__music_state", function!(sel_music_state, 0))?;

    let class = module.define_class("Grid", ruby.class_object())?;
    class.undef_default_alloc_func();
    class.define_method("raw_get", method!(RbGrid::raw_get, 2))?;
    class.define_method("raw_cells", method!(RbGrid::raw_cells, 0))?;
    class.define_method("set", method!(RbGrid::set, 3))?;
    class.define_method("[]=", method!(RbGrid::replace, 3))?;
    class.define_method("replace", method!(RbGrid::replace, 3))?;
    class.define_method("remove", method!(RbGrid::remove, 2))?;
    class.define_method("subgrid", method!(RbGrid::subgrid, 2))?;
    class.define_method("new_grid", method!(RbGrid::new_grid, 2))?;
    class.define_method("move", method!(RbGrid::move_cell, 4))?;
    class.define_method("swap", method!(RbGrid::swap, 4))?;
    class.define_method("fill", method!(RbGrid::fill, 3))?;
    class.define_method("next_free", method!(RbGrid::next_free, 2))?;
    class.define_method("size", method!(RbGrid::size, 0))?;
    class.define_method("clear", method!(RbGrid::clear, 0))?;
    class.define_method("same?", method!(RbGrid::same, 1))?;
    class.define_method("items_raw", method!(RbGrid::items_raw, 2))?;
    class.define_method("stacks_raw", method!(RbGrid::stacks_raw, 0))?;
    class.define_method("push", method!(RbGrid::push, 3))?;
    class.define_method("push_grid", method!(RbGrid::push_grid, 2))?;
    class.define_method("__pop", method!(RbGrid::pop, 3))?;
    class.define_method("cycle", method!(RbGrid::cycle, 3))?;
    class.define_method("raise_item", method!(RbGrid::raise, 3))?;
    class.define_method("merge", method!(RbGrid::merge, 4))?;
    class.define_method("unstack", method!(RbGrid::unstack, 2))?;
    class.define_method("stack_size", method!(RbGrid::stack_size, 2))?;
    class.define_method("item_count", method!(RbGrid::item_count, 0))?;
    class.define_method("inv_add", method!(RbGrid::inv_add, 2))?;
    class.define_method("inv_remove", method!(RbGrid::inv_remove, 2))?;
    class.define_method("inv_set", method!(RbGrid::inv_set, 2))?;
    class.define_method("inv_delete", method!(RbGrid::inv_delete, 1))?;
    class.define_method("inv_rename", method!(RbGrid::inv_rename, 2))?;
    class.define_method("inv_count", method!(RbGrid::inv_count, 1))?;
    class.define_method("inv_has", method!(RbGrid::inv_has, 2))?;
    class.define_method("inv_json", method!(RbGrid::inv_json, 0))?;
    class.define_method("inv_set_meta", method!(RbGrid::inv_set_meta, 3))?;
    class.define_method("inv_transfer", method!(RbGrid::inv_transfer, 3))?;
    class.define_method("inv_clear", method!(RbGrid::inv_clear, 0))?;
    class.define_method("inv_total", method!(RbGrid::inv_total, 0))?;
    class.define_method("inv_len", method!(RbGrid::inv_len, 0))?;
    class.define_method("folder", method!(RbGrid::folder, 0))?;
    class.define_method("assets_dir", method!(RbGrid::assets_dir, 0))?;
    class.define_method("import", method!(RbGrid::import, 3))?;
    class.define_method("folder_id", method!(RbGrid::folder_id, 0))?;

    let _: Value = ruby.eval(PRELUDE)?;
    crate::game::register(ruby)?;
    Ok(())
}

const PARTITIONED_ARRAY_BRIDGE: &str = include_str!("ruby/partitioned_array_bridge.rb");
const PARTITIONED_ARRAY_ENTRY: &str = "managed_partitioned_array.rb";

/// Locates the partitioned_array `lib/` directory: an explicit env override,
/// then next to the executable (portable bundle), then the source checkout.
pub fn partitioned_array_lib_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = std::env::var_os("SELENITE_PARTITIONED_ARRAY_LIB") {
        candidates.push(PathBuf::from(dir));
    }
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        candidates.push(exe_dir.join("partitioned_array").join("lib"));
    }
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("partitioned_array/lib"));
    candidates
        .into_iter()
        .find(|dir| dir.join(PARTITIONED_ARRAY_ENTRY).is_file())
}

fn install_partitioned_array(ruby: &Ruby) -> Result<(), Error> {
    if let Some(dir) = partitioned_array_lib_dir() {
        let load_path: Value = ruby.eval("$LOAD_PATH")?;
        let _: Value = load_path.funcall("unshift", (dir.display().to_string(),))?;
    }
    let _: Value = ruby.eval(PARTITIONED_ARRAY_BRIDGE)?;
    Ok(())
}

fn call_bridge(function: &str, db_path: &Path) -> Result<i64, String> {
    let ruby = Ruby::get().map_err(|_| "Ruby VM not initialized".to_owned())?;
    let main: Value = ruby.eval("self").map_err(|error| error.to_string())?;
    main.funcall(function, (db_path.display().to_string(),))
        .map_err(|error| error.to_string())
}

fn runtime_error(message: impl Into<String>) -> Error {
    let ruby = Ruby::get().expect("only called from Ruby");
    Error::new(ruby.exception_runtime_error(), message.into())
}

fn with_context<R>(f: impl FnOnce(&ScriptContext) -> R) -> Result<R, Error> {
    let guard = context_slot();
    let context = guard
        .as_ref()
        .ok_or_else(|| runtime_error("no active grid"))?;
    Ok(f(context))
}

fn current_grid() -> Result<Arc<Mutex<SavedGrid>>, Error> {
    with_context(|context| Arc::clone(&context.grid))
}

fn with_grid<R>(f: impl FnOnce(&mut SavedGrid) -> R) -> Result<R, Error> {
    let grid = current_grid()?;
    let mut guard = lock_grid(&grid);
    Ok(f(&mut guard))
}

fn cell_info(grid: &SavedGrid, col: i64, row: i64) -> Option<(String, Option<String>)> {
    let kind = grid.kind_at(col, row)?;
    let path = grid
        .file_at(col, row)
        .map(|path| path.display().to_string());
    Some((kind_name(kind).to_owned(), path))
}

fn list_cells(grid: &SavedGrid) -> Vec<(i64, i64, String, Option<String>)> {
    grid.entries()
        .map(|(col, row, kind)| {
            let path = grid
                .file_at(col, row)
                .map(|path| path.display().to_string());
            (col, row, kind_name(kind).to_owned(), path)
        })
        .collect()
}

/// Ruby-visible handle to a grid (`Selenite::Grid`).
#[magnus::wrap(class = "Selenite::Grid", free_immediately, size)]
struct RbGrid(Arc<Mutex<SavedGrid>>);

impl RbGrid {
    fn raw_get(&self, col: i64, row: i64) -> Option<(String, Option<String>)> {
        cell_info(&lock_grid(&self.0), col, row)
    }

    fn raw_cells(&self) -> Vec<(i64, i64, String, Option<String>)> {
        list_cells(&lock_grid(&self.0))
    }

    fn set(&self, col: i64, row: i64, path: String) -> bool {
        lock_grid(&self.0)
            .insert_file(col, row, PathBuf::from(path))
            .is_ok()
    }

    /// Places a file, replacing any file already there (never a grid).
    fn replace(&self, col: i64, row: i64, path: String) -> Result<bool, Error> {
        let mut grid = lock_grid(&self.0);
        if grid.kind_at(col, row) == Some(CellKind::Grid) {
            return Err(runtime_error(format!(
                "cell ({col}, {row}) holds a nested grid; remove it first"
            )));
        }
        grid.remove(col, row);
        Ok(grid.insert_file(col, row, PathBuf::from(path)).is_ok())
    }

    fn remove(&self, col: i64, row: i64) -> bool {
        lock_grid(&self.0).remove(col, row).is_some()
    }

    fn subgrid(&self, col: i64, row: i64) -> Option<RbGrid> {
        lock_grid(&self.0).grid_at(col, row).map(RbGrid)
    }

    fn new_grid(&self, col: i64, row: i64) -> Option<RbGrid> {
        lock_grid(&self.0).create_grid(col, row).ok().map(RbGrid)
    }

    fn move_cell(&self, col: i64, row: i64, to_col: i64, to_row: i64) -> bool {
        let mut grid = lock_grid(&self.0);
        grid.occupied(col, row) && grid.move_cell((col, row), (to_col, to_row)).is_ok()
    }

    fn swap(&self, col: i64, row: i64, to_col: i64, to_row: i64) {
        lock_grid(&self.0).swap_cells((col, row), (to_col, to_row));
    }

    fn fill(&self, col: i64, row: i64, paths: Vec<String>) -> usize {
        lock_grid(&self.0).insert_files_fanout(col, row, paths)
    }

    fn next_free(&self, col: i64, row: i64) -> i64 {
        lock_grid(&self.0).next_free_in_row(col, row)
    }

    fn size(&self) -> usize {
        lock_grid(&self.0).len()
    }

    fn clear(&self) {
        lock_grid(&self.0).clear();
    }

    fn same(&self, other: &RbGrid) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    // ---- stacks (index 0 is the top / visible item) ----

    fn items_raw(&self, col: i64, row: i64) -> Vec<(String, Option<String>)> {
        lock_grid(&self.0)
            .items_at(col, row)
            .iter()
            .map(item_info)
            .collect()
    }

    fn stacks_raw(&self) -> Vec<(i64, i64, usize, String, Option<String>)> {
        list_stack_items(&lock_grid(&self.0))
    }

    fn push(&self, col: i64, row: i64, path: String) -> usize {
        lock_grid(&self.0).push_file(col, row, PathBuf::from(path))
    }

    fn push_grid(&self, col: i64, row: i64) -> RbGrid {
        RbGrid(lock_grid(&self.0).push_grid(col, row))
    }

    fn pop(&self, col: i64, row: i64, index: usize) -> Option<(String, Option<String>)> {
        lock_grid(&self.0)
            .pop_item(col, row, index)
            .map(|content| item_info(&content))
    }

    fn cycle(&self, col: i64, row: i64, delta: i64) -> bool {
        lock_grid(&self.0).cycle_stack(col, row, delta)
    }

    fn raise(&self, col: i64, row: i64, index: usize) -> bool {
        lock_grid(&self.0).raise_item(col, row, index)
    }

    fn merge(&self, col: i64, row: i64, to_col: i64, to_row: i64) -> usize {
        lock_grid(&self.0).merge_cells((col, row), (to_col, to_row))
    }

    fn unstack(&self, col: i64, row: i64) -> Vec<(i64, i64)> {
        lock_grid(&self.0).unstack(col, row)
    }

    fn stack_size(&self, col: i64, row: i64) -> usize {
        lock_grid(&self.0).stack_len(col, row)
    }

    fn item_count(&self) -> usize {
        lock_grid(&self.0).item_count()
    }

    // ---- per-grid game inventory ----

    fn with_inventory<R>(
        &self,
        f: impl FnOnce(&mut GridInventory) -> Result<R, String>,
    ) -> Result<R, Error> {
        f(lock_grid(&self.0).inventory_mut()).map_err(runtime_error)
    }

    fn inv_add(&self, name: String, amount: i64) -> Result<i64, Error> {
        self.with_inventory(|inventory| inventory.add(&name, amount))
    }

    fn inv_remove(&self, name: String, amount: i64) -> Result<i64, Error> {
        self.with_inventory(|inventory| inventory.remove(&name, amount))
    }

    fn inv_set(&self, name: String, count: i64) -> Result<i64, Error> {
        self.with_inventory(|inventory| inventory.set(&name, count).map(|()| count.max(0)))
    }

    fn inv_delete(&self, name: String) -> bool {
        lock_grid(&self.0).inventory_mut().delete(&name)
    }

    fn inv_rename(&self, from: String, to: String) -> Result<bool, Error> {
        self.with_inventory(|inventory| inventory.rename(&from, &to).map(|()| true))
    }

    fn inv_count(&self, name: String) -> i64 {
        lock_grid(&self.0).inventory().count(&name)
    }

    fn inv_has(&self, name: String, amount: i64) -> bool {
        lock_grid(&self.0).inventory().has(&name, amount)
    }

    fn inv_json(&self) -> Result<String, Error> {
        serde_json::to_string(lock_grid(&self.0).inventory())
            .map_err(|error| runtime_error(error.to_string()))
    }

    /// `value_json` is JSON text; `"null"` clears the key.
    fn inv_set_meta(&self, name: String, key: String, value_json: String) -> Result<bool, Error> {
        let value: serde_json::Value = serde_json::from_str(&value_json)
            .map_err(|error| runtime_error(format!("bad metadata JSON: {error}")))?;
        self.with_inventory(|inventory| inventory.set_meta(&name, &key, value).map(|()| true))
    }

    fn inv_transfer(&self, other: &RbGrid, name: String, amount: i64) -> Result<bool, Error> {
        if Arc::ptr_eq(&self.0, &other.0) {
            // Same inventory: just validate there's enough.
            return if self.inv_has(name.clone(), amount) && amount > 0 {
                Ok(true)
            } else {
                Err(runtime_error(format!("not enough {}", name.trim())))
            };
        }
        let mut from = lock_grid(&self.0);
        let mut to = lock_grid(&other.0);
        from.inventory_mut()
            .transfer(to.inventory_mut(), &name, amount)
            .map(|()| true)
            .map_err(runtime_error)
    }

    fn inv_clear(&self) {
        lock_grid(&self.0).inventory_mut().clear();
    }

    fn inv_total(&self) -> i64 {
        lock_grid(&self.0).inventory().total()
    }

    fn inv_len(&self) -> usize {
        lock_grid(&self.0).inventory().len()
    }

    /// This grid's own folder (created on first use).
    fn folder(&self) -> Result<String, Error> {
        folder_for(&self.0)
    }

    /// This grid's `assets/` folder (created on first use).
    fn assets_dir(&self) -> Result<String, Error> {
        assets_for(&self.0).map(|path| path.display().to_string())
    }

    /// Copies a file into this grid's assets folder and stacks it on a cell.
    fn import(&self, col: i64, row: i64, path: String) -> Result<String, Error> {
        import_into(&self.0, col, row, &path)
    }

    /// The folder id, or nil if the folder has never been used.
    fn folder_id(&self) -> Option<String> {
        lock_grid(&self.0).id().map(str::to_owned)
    }
}

/// `grid_set(col, row, path)` -- place an image at `(col, row)`.
fn grid_set(col: i64, row: i64, path: String) -> Result<bool, Error> {
    with_grid(|grid| grid.insert_image(col, row, PathBuf::from(path)).is_ok())
}

/// `grid_set_file(col, row, path)` -- place any file at `(col, row)`.
fn grid_set_file(col: i64, row: i64, path: String) -> Result<bool, Error> {
    with_grid(|grid| grid.insert_file(col, row, PathBuf::from(path)).is_ok())
}

/// `grid_get(col, row)` -- `[kind, path]` or `nil`.
fn grid_get(col: i64, row: i64) -> Result<Option<(String, Option<String>)>, Error> {
    with_grid(|grid| cell_info(grid, col, row))
}

/// `grid_new_grid(col, row)` -- create a nested grid at `(col, row)`.
fn grid_new_grid(col: i64, row: i64) -> Result<bool, Error> {
    with_grid(|grid| grid.create_grid(col, row).is_ok())
}

/// `grid_remove(col, row)` -- remove whatever occupies `(col, row)`.
fn grid_remove(col: i64, row: i64) -> Result<bool, Error> {
    with_grid(|grid| grid.remove(col, row).is_some())
}

fn grid_move(col: i64, row: i64, to_col: i64, to_row: i64) -> Result<bool, Error> {
    with_grid(|grid| {
        grid.occupied(col, row) && grid.move_cell((col, row), (to_col, to_row)).is_ok()
    })
}

fn grid_swap(col: i64, row: i64, to_col: i64, to_row: i64) -> Result<(), Error> {
    with_grid(|grid| grid.swap_cells((col, row), (to_col, to_row)))
}

fn grid_clear() -> Result<(), Error> {
    with_grid(SavedGrid::clear)
}

fn grid_count() -> Result<usize, Error> {
    with_grid(|grid| grid.len())
}

/// `grid_list` -- all occupied cells as `[col, row, kind, path]` tuples
/// (`path` is `nil` for nested grids).
fn grid_list() -> Result<Vec<(i64, i64, String, Option<String>)>, Error> {
    with_grid(|grid| list_cells(grid))
}

/// `grid_save` -- persist the whole document (from the root) to disk.
fn grid_save() -> Result<bool, Error> {
    let (root, path) =
        with_context(|context| (Arc::clone(&context.root), context.save_path.clone()))?;
    let saved = lock_grid(&root).save(&path).is_ok();
    Ok(saved)
}

/// `grid_eval_file(path)` -- evaluate another Ruby source file in this VM.
fn grid_eval_file(ruby: &Ruby, path: String) -> Result<Value, Error> {
    let code = fs::read_to_string(&path)
        .map_err(|error| runtime_error(format!("could not read {path}: {error}")))?;
    ruby.eval(&code)
}

fn sel_grid() -> Result<RbGrid, Error> {
    current_grid().map(RbGrid)
}

fn sel_root() -> Result<RbGrid, Error> {
    with_context(|context| RbGrid(Arc::clone(&context.root)))
}

fn sel_selected() -> Result<Option<(i64, i64)>, Error> {
    with_context(|context| context.selected)
}

fn sel_depth() -> Result<usize, Error> {
    with_context(|context| context.depth)
}

fn sel_save_path() -> Result<String, Error> {
    with_context(|context| context.save_path.display().to_string())
}

fn sel_assets_dir() -> Result<String, Error> {
    let grid = current_grid()?;
    assets_for(&grid).map(|path| path.display().to_string())
}

fn sel_grid_folder() -> Result<String, Error> {
    let grid = current_grid()?;
    folder_for(&grid)
}

fn sel_import(col: i64, row: i64, path: String) -> Result<String, Error> {
    let grid = current_grid()?;
    import_into(&grid, col, row, &path)
}

/// Creates (on first use) and returns `grid`'s `assets/` folder, saving the
/// profile if the grid was just given its folder id.
fn assets_for(grid: &Arc<Mutex<SavedGrid>>) -> Result<PathBuf, Error> {
    let (root, path) =
        with_context(|context| (Arc::clone(&context.root), context.save_path.clone()))?;
    let (assets, new) =
        crate::persistence::grid_assets_folder(&path, grid, &root).map_err(|error| {
            runtime_error(format!(
                "could not create the grid's assets folder: {error}"
            ))
        })?;
    if new {
        lock_grid(&root)
            .save(&path)
            .map_err(|error| runtime_error(error.to_string()))?;
    }
    Ok(assets)
}

/// Copies `source` into `grid`'s assets folder (unless it is already there),
/// stacks the copy on (`col`, `row`) and saves. Returns the copy's path.
fn import_into(
    grid: &Arc<Mutex<SavedGrid>>,
    col: i64,
    row: i64,
    source: &str,
) -> Result<String, Error> {
    let assets = assets_for(grid)?;
    let copied = crate::download::import_file(Path::new(source), &assets).map_err(runtime_error)?;
    lock_grid(grid).push_file(col, row, copied.clone());
    let (root, path) =
        with_context(|context| (Arc::clone(&context.root), context.save_path.clone()))?;
    lock_grid(&root)
        .save(&path)
        .map_err(|error| runtime_error(error.to_string()))?;
    Ok(copied.display().to_string())
}

/// Creates (on first use) and returns `grid`'s own folder, saving the
/// profile if the grid was just given its folder id.
fn folder_for(grid: &Arc<Mutex<SavedGrid>>) -> Result<String, Error> {
    let (root, path) =
        with_context(|context| (Arc::clone(&context.root), context.save_path.clone()))?;
    let (folder, new) = crate::persistence::grid_folder(&path, grid, &root)
        .map_err(|error| runtime_error(format!("could not create the grid folder: {error}")))?;
    if new {
        lock_grid(&root)
            .save(&path)
            .map_err(|error| runtime_error(error.to_string()))?;
    }
    Ok(folder.display().to_string())
}

fn sel_grids_dir() -> Result<String, Error> {
    with_context(|context| {
        crate::persistence::grids_root(&context.save_path)
            .display()
            .to_string()
    })
}

fn sel_classify(path: String) -> &'static str {
    kind_name(SavedGrid::classify_path(Path::new(&path)))
}

fn sel_profile() -> Result<Option<String>, Error> {
    with_context(|context| context.profile.clone())
}

fn profile_store() -> Result<ProfileStore, Error> {
    with_context(|context| context.profiles.clone())?
        .ok_or_else(|| runtime_error("profiles are unavailable (opened with --grid)"))
}

fn sel_profiles() -> Result<Vec<String>, Error> {
    Ok(profile_store()?.list())
}

fn sel_create_profile(name: String) -> Result<String, Error> {
    profile_store()?
        .create(&name)
        .map(|profile| profile.name)
        .map_err(runtime_error)
}

fn sel_switch_profile(name: String) -> Result<bool, Error> {
    let store = profile_store()?;
    let name = crate::profiles::validate_name(&name).map_err(runtime_error)?;
    if !store.exists(&name) {
        return Err(runtime_error(format!(
            "profile '{name}' does not exist; use Selenite.create_profile first"
        )));
    }
    push_request(AppRequest::SwitchProfile(name));
    Ok(true)
}

fn app_status(message: String) {
    push_request(AppRequest::Status(message));
}

fn app_select(col: i64, row: i64) {
    push_request(AppRequest::Select(col, row));
}

fn app_open(col: i64, row: i64) {
    push_request(AppRequest::Open(col, row));
}

fn app_goto(col: i64, row: i64) {
    push_request(AppRequest::Goto(col, row));
}

fn app_enter(col: i64, row: i64) {
    push_request(AppRequest::Enter(col, row));
}

fn app_back() {
    push_request(AppRequest::Back);
}

fn app_view_3d(enabled: bool) {
    push_request(AppRequest::View3d(enabled));
}

fn app_orbit(yaw: f64, pitch: f64, distance: f64) {
    push_request(AppRequest::Orbit {
        yaw,
        pitch,
        distance,
    });
}

fn app_undo() {
    push_request(AppRequest::Undo);
}

fn app_redo() {
    push_request(AppRequest::Redo);
}

fn app_find(query: String) {
    push_request(AppRequest::Find(query));
}

fn app_labels(on: bool) {
    push_request(AppRequest::Labels(on));
}

fn sel_labels() -> Result<bool, Error> {
    with_context(|context| context.labels)
}

fn app_copy_text(text: String) {
    push_request(AppRequest::CopyText(text));
}

fn app_reload_plugins() {
    push_request(AppRequest::ReloadPlugins);
}

fn app_music(command: String, arg: Option<String>) -> Result<bool, Error> {
    let command = MusicCommand::parse(&command, arg).map_err(runtime_error)?;
    push_request(AppRequest::Music(command));
    Ok(true)
}

fn app_music_list(paths: Vec<String>, start: i64) -> Result<bool, Error> {
    if paths.is_empty() {
        return Err(runtime_error("music play_list needs at least one path"));
    }
    let start = (start.max(0) as usize).min(paths.len() - 1);
    push_request(AppRequest::Music(MusicCommand::PlayList(paths, start)));
    Ok(true)
}

fn sel_music_state(ruby: &Ruby) -> Result<RHash, Error> {
    let music = with_context(|context| context.music.clone())?;
    let hash = ruby.hash_new();
    hash.aset(ruby.to_symbol("state"), music.state)?;
    hash.aset(ruby.to_symbol("track"), music.track)?;
    hash.aset(ruby.to_symbol("position"), music.position)?;
    hash.aset(ruby.to_symbol("length"), music.length)?;
    hash.aset(ruby.to_symbol("volume"), music.volume)?;
    hash.aset(ruby.to_symbol("shuffle"), music.shuffle)?;
    hash.aset(ruby.to_symbol("repeat"), music.repeat)?;
    hash.aset(ruby.to_symbol("tracks"), music.tracks)?;
    hash.aset(ruby.to_symbol("index"), music.index)?;
    Ok(hash)
}

fn sel_playable(path: String) -> bool {
    crate::music::is_playable(Path::new(&path))
}

/// `{path:, exists:, dir:, size:, modified:, kind:, ext:}` for any path.
fn sel_file_info(ruby: &Ruby, path: String) -> Result<RHash, Error> {
    let file = Path::new(&path);
    let metadata = fs::metadata(file).ok();
    let absolute = fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let modified = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|age| age.as_secs_f64());
    let ext = file
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let hash = ruby.hash_new();
    hash.aset(ruby.to_symbol("path"), absolute.display().to_string())?;
    hash.aset(ruby.to_symbol("exists"), metadata.is_some())?;
    hash.aset(
        ruby.to_symbol("dir"),
        metadata.as_ref().is_some_and(|metadata| metadata.is_dir()),
    )?;
    hash.aset(
        ruby.to_symbol("size"),
        metadata.as_ref().map_or(0, |metadata| metadata.len()),
    )?;
    hash.aset(ruby.to_symbol("modified"), modified)?;
    hash.aset(ruby.to_symbol("kind"), sel_classify(path.clone()))?;
    hash.aset(ruby.to_symbol("ext"), ext)?;
    Ok(hash)
}

/// Image dimensions read from the header only, so any size is cheap.
fn sel_image_size(path: String) -> Option<(u32, u32)> {
    image::image_dimensions(&path).ok()
}

fn sel_checksum(path: String) -> Result<String, Error> {
    checksum_file(Path::new(&path)).map_err(|error| runtime_error(format!("{path}: {error}")))
}

/// Streaming 64-bit FNV-1a of a file: constant memory for any size. Fast
/// and good for spotting duplicates; not a cryptographic hash.
pub fn checksum_file(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0u8; 1 << 20];
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for byte in &buffer[..read] {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    Ok(format!("fnv1a64:{hash:016x}"))
}

/// `[next undo label, next redo label]` (each may be nil).
fn sel_history() -> Result<(Option<String>, Option<String>), Error> {
    with_context(|context| context.history.clone())
}

type SearchRow = (Vec<(i64, i64)>, i64, i64, String);

/// Every cell in the profile whose name, path or kind matches `query`, as
/// `[path_of_nested_grids, col, row, name]`.
fn sel_search(query: String) -> Result<Vec<SearchRow>, Error> {
    let root = with_context(|context| Arc::clone(&context.root))?;
    let hits = crate::search::search(&lock_grid(&root), &query);
    Ok(hits
        .into_iter()
        .map(|hit| (hit.path, hit.cell.0, hit.cell.1, hit.name))
        .collect())
}

fn app_download(url: String, col: i64, row: i64, stack: bool) -> Result<bool, Error> {
    if !crate::download::is_url(&url) {
        return Err(runtime_error(format!("not an http(s) URL: {url}")));
    }
    push_request(AppRequest::Download {
        url,
        col,
        row,
        stack,
    });
    Ok(true)
}

/// `grid_download(url, col, row)` -- the classic three-argument form.
fn grid_download(url: String, col: i64, row: i64) -> Result<bool, Error> {
    app_download(url, col, row, false)
}

fn sel_selection() -> Result<Vec<(i64, i64)>, Error> {
    with_context(|context| {
        if context.selection.is_empty() {
            context.selected.into_iter().collect()
        } else {
            context.selection.clone()
        }
    })
}

fn app_select_cells(cells: Vec<(i64, i64)>) {
    push_request(AppRequest::SelectCells(cells));
}

/// `grid_list_stacks` -- every item of every stack as
/// `[col, row, index, kind, path]` (index 0 is the top).
fn grid_list_stacks() -> Result<Vec<(i64, i64, usize, String, Option<String>)>, Error> {
    with_grid(|grid| list_stack_items(grid))
}

fn grid_push_file(col: i64, row: i64, path: String) -> Result<usize, Error> {
    with_grid(|grid| grid.push_file(col, row, PathBuf::from(path)))
}

fn grid_push_grid(col: i64, row: i64) -> Result<bool, Error> {
    with_grid(|grid| {
        grid.push_grid(col, row);
        true
    })
}

fn list_stack_items(grid: &SavedGrid) -> Vec<(i64, i64, usize, String, Option<String>)> {
    let mut items: Vec<_> = grid
        .stacks()
        .flat_map(|((col, row), stack)| {
            stack.iter().enumerate().map(move |(index, content)| {
                (
                    col,
                    row,
                    index,
                    kind_name(content.kind()).to_owned(),
                    content.path().map(|path| path.display().to_string()),
                )
            })
        })
        .collect();
    items.sort_by_key(|(col, row, index, ..)| (*row, *col, *index));
    items
}

fn item_info(content: &CellContent) -> (String, Option<String>) {
    (
        kind_name(content.kind()).to_owned(),
        content.path().map(|path| path.display().to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The embedded Ruby VM is a process-wide singleton (it can only be
    // initialised once), so this crate only ever creates one
    // `ScriptEngine` -- exercise the whole API through it in a single test.
    #[test]
    fn grid_functions_round_trip_through_ruby() {
        let engine = ScriptEngine::new().expect("ruby VM should start");
        let grid = Arc::new(Mutex::new(SavedGrid::new()));
        let save_path = std::env::temp_dir().join(format!(
            "selenite_scripting_test_{}.json",
            std::process::id()
        ));
        engine.set_context(ScriptContext::new(
            Arc::clone(&grid),
            Arc::clone(&grid),
            save_path.clone(),
        ));

        assert_eq!(engine.eval("grid_set(0, 0, \"cat.png\")"), "true");
        assert_eq!(engine.eval("grid_new_grid(1, 0)"), "true");
        assert_eq!(engine.eval("grid_set_file(2, 0, \"song.mp3\")"), "true");
        // Occupied cell: insert must fail.
        assert_eq!(engine.eval("grid_set(0, 0, \"dog.png\")"), "false");

        assert!(grid.lock().unwrap().occupied(0, 0));
        assert!(grid.lock().unwrap().occupied(1, 0));
        assert_eq!(grid.lock().unwrap().kind_at(2, 0), Some(CellKind::Audio));
        assert_eq!(engine.eval("grid_get(2, 0)"), "[\"audio\", \"song.mp3\"]");
        assert_eq!(engine.eval("grid_get(9, 9)"), "nil");
        assert_eq!(engine.eval("grid_count"), "3");

        let listing = engine.eval("grid_list.sort");
        assert!(
            listing.contains("\"cat.png\""),
            "unexpected listing: {listing}"
        );
        assert!(
            listing.contains("\"grid\""),
            "unexpected listing: {listing}"
        );
        assert!(
            listing.contains("\"audio\""),
            "unexpected listing: {listing}"
        );

        let script_path = save_path.with_extension("rb");
        fs::write(&script_path, "grid_set_file(3, 0, 'movie.mp4')").unwrap();
        assert_eq!(engine.eval_file(&script_path), "true");
        assert_eq!(grid.lock().unwrap().kind_at(3, 0), Some(CellKind::Video));

        assert_eq!(engine.eval("grid_remove(1, 0)"), "true");
        assert!(!grid.lock().unwrap().occupied(1, 0));

        assert_eq!(engine.eval("grid_save"), "true");
        assert!(
            save_path.exists(),
            "grid_save should have written the save file"
        );
        let _ = std::fs::remove_file(&save_path);
        let _ = std::fs::remove_file(script_path);

        // Object API.
        assert_eq!(engine.eval("Selenite.grid.size"), "3");
        assert_eq!(engine.eval("Selenite.grid[0, 0].kind"), "\"image\"");
        assert_eq!(engine.eval("Selenite.grid[0, 0].path"), "\"cat.png\"");
        assert_eq!(engine.eval("Selenite.grid.map(&:col)"), "[0, 2, 3]");
        assert_eq!(
            engine.eval("Selenite.grid.new_grid(5, 5).set(1, 1, 'x.rb')"),
            "true"
        );
        assert_eq!(
            engine.eval("Selenite.grid.subgrid(5, 5)[1, 1].kind"),
            "\"ruby\""
        );
        assert_eq!(engine.eval("Selenite.grid.subgrid(0, 0)"), "nil");
        assert_eq!(engine.eval("Selenite.grid.move(3, 0, 4, 0)"), "true");
        assert_eq!(grid.lock().unwrap().kind_at(4, 0), Some(CellKind::Video));
        assert_eq!(
            engine.eval("Selenite.grid.swap(4, 0, 0, 0); Selenite.grid[0,0].kind"),
            "\"video\""
        );
        assert_eq!(
            engine.eval("g = Selenite.grid; g[0, 0] = 'new.png'; g[0, 0].kind"),
            "\"image\""
        );
        assert!(engine
            .eval("Selenite.grid[5, 5] = 'x.png'")
            .starts_with("error:"));
        assert_eq!(
            engine.eval("Selenite.grid.fill(10, 0, %w[a.png b.mp3])"),
            "2"
        );
        assert_eq!(engine.eval("Selenite.grid.next_free(10, 0)"), "12");
        assert_eq!(engine.eval("Selenite.grid.same?(Selenite.root)"), "true");
        assert_eq!(engine.eval("Selenite.classify('a.MP4')"), "\"video\"");
        assert_eq!(
            engine.eval("Selenite::VERSION"),
            format!("{:?}", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(engine.eval("Selenite.profile"), "nil");
        assert!(engine.eval("Selenite.profiles").starts_with("error:"));

        // Captured output.
        let (printed, result) = engine.eval_captured("puts 'hello'; warn 'careful'; 42");
        assert_eq!(printed, "hello\ncareful\n");
        assert_eq!(result, "42");
        assert_eq!(engine.eval("$stdout.equal?(STDOUT)"), "true");

        // App requests.
        assert!(engine.drain_requests().is_empty());
        engine.eval("Selenite.status('hi'); Selenite.select(1, 2); Selenite.goto(3, 4); Selenite.download('https://e.x/a.png', 7, 8)");
        assert_eq!(
            engine.drain_requests(),
            vec![
                AppRequest::Status("hi".to_owned()),
                AppRequest::Select(1, 2),
                AppRequest::Goto(3, 4),
                AppRequest::Download {
                    url: "https://e.x/a.png".to_owned(),
                    col: 7,
                    row: 8,
                    stack: false,
                },
            ]
        );
        assert!(engine
            .eval("Selenite.download('ftp://nope')")
            .starts_with("error:"));

        // Undo/redo/find requests, history labels and synchronous search.
        engine.eval("Selenite.undo; Selenite.redo; Selenite.find('b.mp3')");
        assert_eq!(
            engine.drain_requests(),
            vec![
                AppRequest::Undo,
                AppRequest::Redo,
                AppRequest::Find("b.mp3".to_owned()),
            ]
        );
        assert_eq!(engine.eval("Selenite.history"), "[nil, nil]");
        assert_eq!(
            engine.eval("Selenite.search('B.MP3')"),
            "[[[], 11, 0, \"b.mp3\"]]"
        );
        assert_eq!(engine.eval("Selenite.search('zzz')"), "[]");
        engine.drain_requests();

        // Event hooks.
        assert!(!engine.has_hooks("activate"));
        assert_eq!(engine.emit("activate", vec![]), (false, String::new()));
        engine.eval("$seen = []; Selenite.on(:activate) { |c, r, k, p| $seen << [c, r, k, p]; k == 'ruby' ? :handled : nil }");
        assert!(engine.has_hooks("activate"));
        let (handled, _) = engine.emit(
            "activate",
            vec![
                HookArg::Int(1),
                HookArg::Int(2),
                HookArg::Str("image".into()),
                HookArg::Nil,
            ],
        );
        assert!(!handled);
        let (handled, _) = engine.emit(
            "activate",
            vec![
                HookArg::Int(3),
                HookArg::Int(4),
                HookArg::Str("ruby".into()),
                HookArg::Str("x.rb".into()),
            ],
        );
        assert!(handled);
        assert_eq!(
            engine.eval("$seen"),
            "[[1, 2, \"image\", nil], [3, 4, \"ruby\", \"x.rb\"]]"
        );
        engine.eval("Selenite.on(:save) { raise 'boom' }");
        let (_, output) = engine.emit("save", vec![HookArg::Str("p".into())]);
        assert!(output.contains("boom"), "unexpected: {output}");
        assert!(engine.eval("Selenite.on(:nope) {}").starts_with("error:"));
        engine.eval("Selenite.off");
        assert!(!engine.has_hooks("save"));

        // Profiles through a real store.
        let store = ProfileStore::at(std::env::temp_dir().join(format!(
            "selenite_scripting_profiles_{}",
            std::process::id()
        )));
        store.ensure("default").unwrap();
        let mut context = ScriptContext::new(Arc::clone(&grid), Arc::clone(&grid), save_path);
        context.profile = Some("default".to_owned());
        context.profiles = Some(store.clone());
        context.selected = Some((4, 5));
        engine.set_context(context);
        assert_eq!(engine.eval("Selenite.selected"), "[4, 5]");
        assert_eq!(engine.eval("Selenite.create_profile('Art')"), "\"Art\"");
        assert_eq!(engine.eval("Selenite.profiles"), "[\"Art\", \"default\"]");
        assert_eq!(engine.eval("Selenite.switch_profile('Art')"), "true");
        assert!(engine
            .eval("Selenite.switch_profile('Missing')")
            .starts_with("error:"));
        engine.eval("Selenite.download('https://e.x/b.mp4')");
        assert_eq!(
            engine.drain_requests(),
            vec![
                AppRequest::SwitchProfile("Art".to_owned()),
                AppRequest::Download {
                    url: "https://e.x/b.mp4".to_owned(),
                    col: 4,
                    row: 5,
                    stack: false,
                },
            ]
        );
        let _ = fs::remove_dir_all(store.base());

        // partitioned_array round trip on a fresh grid.
        let pa_grid = Arc::new(Mutex::new(SavedGrid::new()));
        engine.set_context(ScriptContext::new(
            Arc::clone(&pa_grid),
            Arc::clone(&pa_grid),
            std::env::temp_dir().join("selenite_pa_unused.json"),
        ));
        engine.eval(
            "grid_set(0, 0, 'cat.png'); grid_new_grid(1, 0); grid_set_file(2, 0, 'song.mp3'); Selenite.grid.push(0, 0, 'dog.png')",
        );
        let pa_dir = std::env::temp_dir().join(format!("selenite_pa_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&pa_dir);
        assert_eq!(
            engine.eval("grid_pa_available"),
            "true",
            "load error: {} / $LOAD_PATH: {}",
            engine.eval("SelenitePartitionedArray.load_error"),
            engine.eval("$LOAD_PATH")
        );
        let exported = engine.pa_export(&pa_dir).expect("export should succeed");
        assert_eq!(exported, pa_grid.lock().unwrap().item_count() as i64);
        assert_eq!(exported, 4);
        let records = engine.eval(&format!(
            "grid_pa_records({:?}).size",
            pa_dir.display().to_string()
        ));
        assert_eq!(records, exported.to_string());

        let original: Vec<_> = pa_grid.lock().unwrap().entries().collect();
        for (col, row, _) in &original {
            pa_grid.lock().unwrap().remove(*col, *row);
        }
        assert_eq!(engine.pa_import(&pa_dir), Ok(exported));
        for (col, row, kind) in original {
            assert_eq!(pa_grid.lock().unwrap().kind_at(col, row), Some(kind));
        }
        assert_eq!(
            engine.eval("Selenite.grid.items(0, 0).map { |c| File.basename(c.path) }"),
            "[\"dog.png\", \"cat.png\"]"
        );
        let _ = fs::remove_dir_all(&pa_dir);

        // v3: stacks, inventory and multi-selection.
        let v3 = Arc::new(Mutex::new(SavedGrid::new()));
        let other = Arc::new(Mutex::new(SavedGrid::new()));
        let mut context = ScriptContext::new(
            Arc::clone(&v3),
            Arc::clone(&v3),
            std::env::temp_dir().join("selenite_v3_unused.json"),
        );
        context.selected = Some((0, 0));
        context.selection = vec![(0, 0), (1, 0)];
        engine.set_context(context);
        assert_eq!(engine.eval("Selenite.selection"), "[[0, 0], [1, 0]]");
        assert_eq!(engine.eval("Selenite.select_cells([[2, 2], [3, 3]])"), "2");
        assert_eq!(
            engine.drain_requests(),
            vec![AppRequest::SelectCells(vec![(2, 2), (3, 3)])]
        );
        engine.eval("Selenite.download('https://e.x/c.png', 1, 1, stack: true)");
        assert_eq!(
            engine.drain_requests(),
            vec![AppRequest::Download {
                url: "https://e.x/c.png".to_owned(),
                col: 1,
                row: 1,
                stack: true,
            }]
        );

        engine.eval("$g = Selenite.grid; $g.push(0, 0, 'a.png'); $g.push(0, 0, 'b.mp3'); $g.push(0, 0, 'c.rb')");
        assert_eq!(engine.eval("$g.stack_size(0, 0)"), "3");
        assert_eq!(engine.eval("$g.stacked?(0, 0)"), "true");
        assert_eq!(engine.eval("$g[0, 0].kind"), "\"ruby\"");
        assert_eq!(
            engine.eval("$g.items(0, 0).map(&:kind)"),
            "[\"ruby\", \"audio\", \"image\"]"
        );
        assert_eq!(engine.eval("$g.cycle(0, 0, 1)"), "true");
        assert_eq!(engine.eval("$g[0, 0].kind"), "\"audio\"");
        assert_eq!(engine.eval("$g.raise_item(0, 0, 2)"), "true");
        assert_eq!(engine.eval("$g.pop(0, 0).kind"), "\"ruby\"");
        assert_eq!(engine.eval("$g.item_count"), "2");
        engine.eval("$g.set(5, 0, 'd.png')");
        assert_eq!(engine.eval("$g.merge(5, 0, 0, 0)"), "1");
        assert_eq!(engine.eval("$g[5, 0]"), "nil");
        assert_eq!(engine.eval("$g.unstack(0, 0).size"), "2");
        assert_eq!(engine.eval("$g.stack_size(0, 0)"), "1");
        assert_eq!(engine.eval("$g.size"), "3");
        assert_eq!(engine.eval("$g.push_grid(9, 9).class"), "Selenite::Grid");
        assert!(engine.eval("$g.inspect").contains("items"));

        engine.eval("$inv = Selenite.inventory");
        assert_eq!(engine.eval("$inv.add('Gold', 50)"), "50");
        assert_eq!(engine.eval("$inv.add('Potion')"), "1");
        assert_eq!(engine.eval("$inv.remove('Gold', 20)"), "30");
        assert!(engine.eval("$inv.remove('Gold', 99)").starts_with("error:"));
        assert!(engine.eval("$inv.add('', 1)").starts_with("error:"));
        assert_eq!(engine.eval("$inv['Gold']"), "30");
        assert_eq!(engine.eval("$inv['Missing']"), "0");
        assert_eq!(engine.eval("$inv.has?('Gold', 30)"), "true");
        assert_eq!(engine.eval("$inv.has?('Gold', 31)"), "false");
        engine.eval("$inv['Arrow'] = 12");
        assert_eq!(
            engine.eval("$inv.to_h"),
            "{\"Arrow\" => 12, \"Gold\" => 30, \"Potion\" => 1}"
        );
        assert_eq!(engine.eval("$inv.set_meta('Potion', 'heal', 25)"), "true");
        assert_eq!(
            engine.eval("$inv.set_meta('Potion', 'tags', ['red'])"),
            "true"
        );
        assert_eq!(
            engine.eval("$inv.meta('Potion')"),
            "{\"heal\" => 25, \"tags\" => [\"red\"]}"
        );
        assert!(engine
            .eval("$inv.set_meta('Nope', 'x', 1)")
            .starts_with("error:"));
        assert_eq!(engine.eval("$inv.rename('Arrow', 'Bolt')"), "true");
        assert_eq!(
            engine.eval("$inv.names"),
            "[\"Bolt\", \"Gold\", \"Potion\"]"
        );
        assert_eq!(engine.eval("$inv.total"), "43");
        assert_eq!(engine.eval("$inv.size"), "3");
        assert_eq!(
            engine.eval("$inv.map { |n, c, m| [n, c, m.size] }"),
            "[[\"Bolt\", 12, 0], [\"Gold\", 30, 0], [\"Potion\", 1, 2]]"
        );
        assert_eq!(engine.eval("$inv.delete('Bolt')"), "true");
        assert_eq!(engine.eval("$inv.transfer($inv, 'Gold', 5)"), "true");
        assert_eq!(engine.eval("$inv['Gold']"), "30");
        assert_eq!(v3.lock().unwrap().inventory().count("Gold"), 30);

        // Transfer into another grid's inventory (metadata travels too).
        let mut context = ScriptContext::new(
            Arc::clone(&other),
            Arc::clone(&other),
            std::env::temp_dir().join("selenite_v3_unused.json"),
        );
        context.selected = None;
        engine.set_context(context);
        assert_eq!(
            engine.eval("$inv.transfer(Selenite.grid, 'Potion', 1)"),
            "true"
        );
        assert_eq!(engine.eval("Selenite.inventory['Potion']"), "1");
        assert_eq!(
            engine.eval("Selenite.inventory.meta('Potion')['heal']"),
            "25"
        );
        assert_eq!(engine.eval("$inv.has?('Potion')"), "false");
        assert!(engine
            .eval("$inv.transfer(Selenite.grid, 'Gold', 999)")
            .starts_with("error:"));
        assert_eq!(engine.eval("Selenite.selection"), "[]");
        assert_eq!(engine.eval("$inv.clear.empty?"), "true");

        // Per-grid folders.
        let folder_root = Arc::new(Mutex::new(SavedGrid::new()));
        let folder_save =
            std::env::temp_dir().join(format!("selenite_folders_{}.json", std::process::id()));
        engine.set_context(ScriptContext::new(
            Arc::clone(&folder_root),
            Arc::clone(&folder_root),
            folder_save.clone(),
        ));
        assert_eq!(engine.eval("Selenite.grid.folder_id"), "nil");
        let root_dir = engine.eval("Selenite.grid_folder");
        assert!(root_dir.ends_with("-grids/root\""), "{root_dir}");
        assert_eq!(
            engine.eval("Selenite.assets_dir == File.join(Selenite.grid_folder, 'assets') && File.directory?(Selenite.assets_dir)"),
            "true"
        );
        assert_eq!(engine.eval("Selenite.grid.folder"), root_dir);
        assert_eq!(engine.eval("Selenite.grid.folder_id"), "\"root\"");
        assert_eq!(engine.eval("File.directory?(Selenite.grid_folder)"), "true");
        // Using a folder saves the new id.
        assert!(fs::read_to_string(&folder_save)
            .unwrap()
            .contains("\"id\": \"root\""));
        assert_eq!(
            engine.eval("s = Selenite.grid.push_grid(1, 1); f = s.folder; [File.directory?(f), f != Selenite.grid_folder, File.dirname(f) == Selenite.grids_dir, s.assets_dir == File.join(f, 'assets')]"),
            "[true, true, true, true]"
        );
        // Importing copies into assets/ and stacks the copy on the cell.
        let outside =
            std::env::temp_dir().join(format!("selenite_import_{}.txt", std::process::id()));
        fs::write(&outside, "import me").unwrap();
        let outside_text = format!("{:?}", outside.display().to_string());
        let copied = engine.eval(&format!("$c = Selenite.import(3, 0, {outside_text})"));
        assert!(
            copied.ends_with(&format!(
                "/assets/selenite_import_{}.txt\"",
                std::process::id()
            )),
            "{copied}"
        );
        assert_eq!(engine.eval("File.read($c)"), "\"import me\"");
        assert_eq!(
            folder_root
                .lock()
                .unwrap()
                .file_at(3, 0)
                .map(|path| format!("{:?}", path.display().to_string())),
            Some(copied.clone())
        );
        assert_eq!(
            engine.eval(&format!(
                "Selenite.grid.import(3, 0, {outside_text}) != $c && Selenite.grid.stack_size(3, 0)"
            )),
            "2"
        );
        assert_eq!(engine.eval("Selenite.import(4, 0, $c) == $c"), "true");
        assert!(engine
            .eval("Selenite.import(5, 0, '/no/such/file.bin')")
            .starts_with("error:"));
        assert!(fs::read_to_string(&folder_save)
            .unwrap()
            .contains("selenite_import_"));
        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir_all(crate::persistence::grids_root(&folder_save));
        let _ = fs::remove_file(&folder_save);

        // Labels, clipboard, music requests and state.
        engine.drain_requests();
        assert_eq!(engine.eval("Selenite.labels?"), "true");
        engine.eval("Selenite.labels(false); Selenite.copy_text('hi'); Selenite.reload_plugins");
        engine.eval("m = Selenite.music; m.play('a.mp3'); m.toggle; m.seek(12.5); m.volume(2); m.shuffle(true); m.repeat(:one); m.queue('b.ogg'); m.next; m.hide");
        assert!(engine
            .eval("Selenite.music.repeat(:sometimes)")
            .starts_with("error:"));
        assert!(engine
            .eval("Selenite.music.play_list([])")
            .starts_with("error:"));
        engine.eval("Selenite.music.play_list(%w[x.mp3 y.mp3], 9)");
        assert_eq!(
            engine.drain_requests(),
            vec![
                AppRequest::Labels(false),
                AppRequest::CopyText("hi".to_owned()),
                AppRequest::ReloadPlugins,
                AppRequest::Music(MusicCommand::Play(Some("a.mp3".to_owned()))),
                AppRequest::Music(MusicCommand::Toggle),
                AppRequest::Music(MusicCommand::Seek(12.5)),
                AppRequest::Music(MusicCommand::Volume(1.0)),
                AppRequest::Music(MusicCommand::Shuffle(true)),
                AppRequest::Music(MusicCommand::Repeat(Repeat::One)),
                AppRequest::Music(MusicCommand::Queue("b.ogg".to_owned())),
                AppRequest::Music(MusicCommand::Next),
                AppRequest::Music(MusicCommand::Show(false)),
                AppRequest::Music(MusicCommand::PlayList(
                    vec!["x.mp3".to_owned(), "y.mp3".to_owned()],
                    1
                )),
            ]
        );
        assert_eq!(engine.eval("Selenite.music.state[:state]"), "\"stopped\"");
        assert_eq!(engine.eval("Selenite.music.playing?"), "false");
        assert_eq!(engine.eval("Selenite.playable?('x.FLAC')"), "true");

        // Rust helpers.
        let sample =
            std::env::temp_dir().join(format!("selenite_helper_{}.txt", std::process::id()));
        fs::write(&sample, "hello").unwrap();
        let sample_text = format!("{:?}", sample.display().to_string());
        assert_eq!(
            engine.eval(&format!("Selenite.checksum({sample_text})")),
            "\"fnv1a64:a430d84680aabd0b\""
        );
        assert_eq!(
            engine.eval(&format!("Selenite.file_info({sample_text})[:size]")),
            "5"
        );
        assert_eq!(
            engine.eval(&format!("Selenite.file_info({sample_text})[:ext]")),
            "\"txt\""
        );
        assert_eq!(
            engine.eval(&format!("Selenite.image_size({sample_text})")),
            "nil"
        );
        assert_eq!(
            engine.eval("Selenite.file_info('/no/such/file')[:exists]"),
            "false"
        );
        assert!(engine
            .eval("Selenite.checksum('/no/such/file')")
            .starts_with("error:"));
        let png = sample.with_extension("png");
        image::RgbaImage::new(3, 2).save(&png).unwrap();
        assert_eq!(
            engine.eval(&format!(
                "Selenite.image_size({:?})",
                png.display().to_string()
            )),
            "[3, 2]"
        );
        let _ = fs::remove_file(&sample);
        let _ = fs::remove_file(&png);

        // Plugins: load, list, invoke, reload (hooks removed), errors.
        let plugin_dir =
            std::env::temp_dir().join(format!("selenite_plugin_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&plugin_dir);
        fs::create_dir_all(&plugin_dir).unwrap();
        let good = plugin_dir.join("good.rb");
        fs::write(
            &good,
            r#"
            Selenite.plugin "Good" do |p|
              p.description "test plugin"
              p.version "2.0"
              p.button("Hi", key: "F6") { |cell| $plugin_hits << [:button, cell&.col] }
              p.menu("Look", kinds: %w[image empty]) { |cell| $plugin_hits << [:menu, cell.col, cell.row, cell.kind] }
              p.command("double") { |x| x * 2 }
              p.on(:save) { |*| $plugin_hits << :saved }
              p.every(5) { raise "tick failed" }
            end
            puts "loaded good"
            "#,
        )
        .unwrap();
        let block_style = plugin_dir.join("block_style.rb");
        fs::write(
            &block_style,
            "Selenite.plugin('Bare') { description 'no |p|'; button('B') { } }",
        )
        .unwrap();
        let broken = plugin_dir.join("broken.rb");
        fs::write(
            &broken,
            "Selenite.plugin('Broken') do |p|\n  p.button('x')\nend",
        )
        .unwrap();
        let empty = plugin_dir.join("empty.rb");
        fs::write(&empty, "x = 1").unwrap();
        let disabled = plugin_dir.join("off.rb");
        fs::write(&disabled, "raise 'must not run'").unwrap();

        engine.eval("$plugin_hits = []");
        let files = vec![
            (good.clone(), true),
            (block_style, true),
            (broken, true),
            (empty, true),
            (disabled, false),
        ];
        let (infos, output) = engine.load_plugins(&files);
        assert!(output.contains("loaded good"), "{output}");
        assert_eq!(infos.len(), 5, "{infos:#?}");
        let good_info = &infos[0];
        assert_eq!(
            (
                good_info.name.as_str(),
                good_info.version.as_str(),
                good_info.error.as_ref()
            ),
            ("Good", "2.0", None)
        );
        let kinds: Vec<ItemKind> = good_info.items.iter().map(|item| item.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ItemKind::Button,
                ItemKind::Menu,
                ItemKind::Command,
                ItemKind::Hook,
                ItemKind::Timer
            ]
        );
        assert_eq!(good_info.items[0].key.as_deref(), Some("F6"));
        assert_eq!(
            good_info.items[1].kinds,
            vec!["image".to_owned(), "empty".to_owned()]
        );
        assert_eq!(good_info.items[4].interval, 5.0);
        assert_eq!(infos[1].name, "Bare");
        assert_eq!(infos[1].description, "no |p|");
        assert!(
            infos[2]
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("needs a block"),
            "{:?}",
            infos[2]
        );
        assert!(infos[3]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("defines no plugin"));
        assert!(!infos[4].enabled && infos[4].error.is_none());
        assert_eq!(engine.eval("Selenite.plugins"), "[\"Good\", \"Bare\"]");
        assert_eq!(engine.eval("Selenite.run('double', 21)"), "42");
        assert!(engine.eval("Selenite.run('missing')").starts_with("error:"));
        assert!(engine.has_hooks("save"));

        let ids: Vec<i64> = good_info.items.iter().map(|item| item.id).collect();
        assert_eq!(engine.invoke_plugin(ids[0], None), (String::new(), None));
        engine.eval("Selenite.grid.set(30, 31, 'pic.png')");
        assert_eq!(engine.invoke_plugin(ids[1], Some((30, 31))).1, None);
        assert_eq!(engine.invoke_plugin(ids[1], Some((99, 99))).1, None);
        let (_, error) = engine.invoke_plugin(ids[4], None);
        assert_eq!(error.as_deref(), Some("RuntimeError: tick failed"));
        assert!(engine
            .invoke_plugin(9999, None)
            .1
            .unwrap()
            .contains("unknown"));
        engine.emit("save", vec![HookArg::Str("x".into())]);
        assert_eq!(
            engine.eval("$plugin_hits"),
            "[[:button, nil], [:menu, 30, 31, \"image\"], [:menu, 99, 99, nil], :saved]"
        );

        // Reloading only the disabled file removes the old plugin's hook.
        let (infos, reload_output) = engine.load_plugins(&[(good.clone(), false)]);
        assert_eq!(infos.len(), 1);
        assert!(!engine.has_hooks("save"), "{reload_output}");
        assert_eq!(engine.eval("Selenite.plugins"), "[]");
        // Bundled examples all load cleanly.
        crate::plugins::install_examples(&plugin_dir.join("examples")).unwrap();
        let examples: Vec<(PathBuf, bool)> =
            crate::plugins::discover(&[plugin_dir.join("examples")])
                .into_iter()
                .map(|path| (path, true))
                .collect();
        let (infos, output) = engine.load_plugins(&examples);
        assert_eq!(infos.len(), crate::plugins::EXAMPLES.len());
        for info in &infos {
            assert_eq!(info.error, None, "{} failed to load: {output}", info.name);
            assert!(!info.items.is_empty());
        }
        engine.load_plugins(&[]);
        let _ = fs::remove_dir_all(&plugin_dir);

        // v3 game API: raylib-bindings style module, usable headless for
        // everything that doesn't need a window.
        assert_eq!(engine.eval("Raylib.GameMode?"), "false");
        let error = engine.eval("Raylib.InitWindow(10, 10, 'x')");
        assert!(error.contains("--game"), "unexpected: {error}");
        assert_eq!(engine.eval("require 'raylib'"), "false");
        assert_eq!(engine.eval("require 'raylib-bindings'"), "false");
        assert_eq!(
            engine.eval("include Raylib; [KEY_A, KEY_SPACE, KEY_ESCAPE, KEY_F1, MOUSE_BUTTON_RIGHT, FLAG_WINDOW_RESIZABLE]"),
            "[65, 32, 256, 290, 1, 4]"
        );
        assert_eq!(engine.eval("Raylib::RED.to_a"), "[230, 41, 55, 255]");
        assert_eq!(engine.eval("Raylib::BLANK.a"), "0");
        assert_eq!(
            engine.eval(
                "Raylib.CheckCollisionRecs([0,0,10,10], Raylib::Rectangle.create(5,5,10,10))"
            ),
            "true"
        );
        assert_eq!(
            engine.eval("Raylib.check_collision_recs([0,0,10,10], [20,20,5,5])"),
            "false"
        );
        assert_eq!(
            engine.eval("Raylib.GetCollisionRec([0,0,10,10], [5,5,10,10]).to_a"),
            "[5.0, 5.0, 5.0, 5.0]"
        );
        assert_eq!(
            engine.eval("Raylib.CheckCollisionCircleRec([0,0], 2.0, [1,1,5,5])"),
            "true"
        );
        assert_eq!(
            engine.eval("Raylib.CheckCollisionPointCircle(Raylib::Vector2.create(3,4), [0,0], 5)"),
            "true"
        );
        assert_eq!(
            engine.eval("(Raylib::Vector2.create(3,4) + [1,1]).to_a"),
            "[4.0, 5.0]"
        );
        assert_eq!(engine.eval("Raylib.vector2_length([3,4])"), "5.0");
        assert_eq!(engine.eval("Raylib.Fade(Raylib::WHITE, 0.5).a"), "128");
        assert_eq!(engine.eval("Raylib.Lerp(0.0, 10.0, 0.25)"), "2.5");
        assert_eq!(engine.eval("Raylib.respond_to?(:draw_text)"), "true");
        assert_eq!(engine.eval("Raylib.respond_to?(:begin_mode_2d)"), "true");
        assert_eq!(engine.eval("Raylib.respond_to?(:draw_fps)"), "true");
        assert_eq!(engine.eval("Raylib.GetRandomValue(3, 3)"), "3");
        let error = engine.eval("Raylib.DrawText('x', 0, 0, 10, Raylib::RED)");
        assert!(error.starts_with("error"), "unexpected: {error}");
        assert_eq!(
            engine.eval("g = Selenite::Game.new(title: 'T', width: 64, height: 32); [g.title, g.width, g.height, g.frame]"),
            "[\"T\", 64, 32, 0]"
        );
        assert_eq!(
            engine.eval("g = Selenite::Game.new; n = 0; g.every(0.5) { n += 1 }; g.send(:tick_timers, 1.1); n"),
            "2"
        );
        let error = engine.eval("Selenite::Game.run(width: 8, height: 8) { }");
        assert!(error.contains("--game"), "unexpected: {error}");

        // Bogus Ruby surfaces as a readable error string, not a panic.
        let error = engine.eval("this is not ruby (((");
        assert!(error.starts_with("error:"), "unexpected: {error}");
    }
}

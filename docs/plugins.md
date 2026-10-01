# Selenite plugins (Ruby ↔ Rust)

Plugins are plain Ruby files that Selenite evaluates inside its embedded Ruby VM
([Magnus](https://github.com/matsadler/magnus)). A plugin can add:

- buttons to the **Plugins** panel, optionally bound to a function key;
- entries to the right-click cell menu;
- named commands for the Ruby console and other plugins;
- event hooks, such as "a new track started" or "a download finished";
- repeating timers.

Plugins call back into Rust through the `Selenite` module. That covers grid
editing, the music player, labels, the clipboard, and native helpers such as
streaming checksums and image-header parsing.

Plugins need a build with the `scripting` feature, which `./package.sh`
always enables. Without it, the Plugins panel only explains how to turn
scripting on.

- [Quick start](#quick-start)
- [Where plugins live](#where-plugins-live)
- [The plugin DSL](#the-plugin-dsl)
- [Calling Rust from Ruby](#calling-rust-from-ruby)
- [The music API](#the-music-api)
- [Events](#events)
- [Errors, undo and reloading](#errors-undo-and-reloading)
- [How it works](#how-it-works)
- [Bundled examples](#bundled-examples)
- [Recipes](#recipes)

## Quick start

1. Click **Plugins** in the toolbar, then **Install examples**. This copies
   five example plugins into the global plugins folder and loads them.
2. Press **F6** (Hello), or right-click a file and choose
   **» Inspect file** (File Inspector).
3. Create `hello_world.rb` in the plugins folder (**Open folder** in the
   panel shows it):

   ```ruby
   Selenite.plugin "Hello World" do |p|
     p.description "My first plugin"
     p.version "0.1"

     p.button("Greet", key: "F5") do |cell|
       Selenite.status(cell ? "Hello, #{cell}" : "Hello, nothing selected")
     end
   end
   ```

4. Click **Reload** in the panel. The new button appears and **F5** triggers it.

## Where plugins live

Selenite loads every `*.rb` file in these folders. Files are sorted by name
within each folder:

| Folder | Scope |
|---|---|
| `<data>/plugins/` | every profile |
| `<data>/profiles/<name>/plugins/` | only that profile |

`<data>` is `~/.local/share/selenite` unless `SELENITE_HOME` is set.

Plugins load:

- at startup, after the profile's `init.rb`;
- on every profile switch;
- whenever you click **Reload** or call `Selenite.reload_plugins`.

The **Plugins** panel lists each file with its name, version, description and
items. **On/Off** disables a file without deleting it; the choice is saved in
`<data>/settings.json`. A file that fails to load is shown in red with the
Ruby error.

## The plugin DSL

```ruby
Selenite.plugin "Name" do |p|          # or: Selenite.plugin("Name") { description "..." }
  p.description "Shown in the Plugins panel"
  p.version "1.0"

  p.button(label, key: "F6") { |cell| ... }          # Plugins panel + optional hotkey
  p.menu(label, kinds: %w[image audio]) { |cell| ... } # right-click menu entry
  p.command(name) { |*args| ... }                     # Selenite.run(name, *args)
  p.on(:event) { |*args| ... }                        # event hook
  p.every(seconds) { ... }                            # repeating timer (>= 0.25 s)
end
```

A single file may declare several plugins. If the block takes no argument, it
is `instance_eval`ed, so you can write `button(...)` instead of `p.button(...)`.

### `button(label, key: nil) { |cell| }`

- Appears in the Plugins panel under its plugin.
- `cell` is the selected `Selenite::Cell`, or `nil` when nothing is selected.
- `key:` may be `F2` or any key from `F4` to `F12`. F1 (Help) and F3 (3D) are
  reserved. When two buttons share a key, the first one loaded wins.

### `menu(label, kinds: nil) { |cell| }`

- Adds `» label` to the right-click menu.
- `cell` is always a `Selenite::Cell`. For an empty cell, `kind` and `path`
  are `nil`.
- `kinds:` limits where the entry appears. Leave it out to show the entry on
  every cell.

| kind | matches |
|---|---|
| `image`, `audio`, `video`, `ruby` | files of that type |
| `file` | any file cell (not grids, not empty) |
| `grid` | nested grids |
| `empty` | empty cells |

### `command(name) { |*args| }`

Registers a named function:

```ruby
Selenite.run("greet", "world")   # => "Hello, world!"
Selenite.commands                # => ["downloads", "greet", "largest", "np"]
```

Commands are the way plugins share functionality with each other and with
the console.

### `on(event) { |*args| }`

Works like `Selenite.on`, except the hook is removed when plugins reload, so
reloading never duplicates it. See [Events](#events).

### `every(seconds) { }`

- Runs on the UI thread every `seconds` (minimum 0.25).
- Keep timer blocks short: the UI waits while Ruby runs.
- A timer that raises is stopped, and the error is shown in the status bar.

### `Selenite::Cell`

`Struct.new(:col, :row, :kind, :path)` plus two helpers:

- `grid?`
- `to_s`, for example `(1, 0) audio /music/a.mp3`

## Calling Rust from Ruby

All of the following are native Rust functions exposed via Magnus.

### Grid

```ruby
g = Selenite.grid              # grid being viewed; Selenite.root = profile root
g[0, 0]                        # => Selenite::Cell or nil
g[1, 0] = "/music/song.flac"   # place/replace any file
g.set(col, row, path)          # false if occupied
g.new_grid(col, row); g.subgrid(col, row)
g.move(c1, r1, c2, r2); g.swap(c1, r1, c2, r2); g.remove(col, row)
g.fill(col, row, paths); g.next_free(col, row); g.size; g.clear
g.each { |cell| }              # Enumerable (map, select, count, ...)
```

### App

```ruby
Selenite.status("text")                 # status bar message
Selenite.select(c, r); Selenite.open(c, r); Selenite.goto(c, r)
Selenite.enter(c, r); Selenite.back; Selenite.depth; Selenite.selected
Selenite.labels(false); Selenite.labels?  # grid labels on/off
Selenite.view_3d(true); Selenite.orbit(yaw, pitch, distance)
Selenite.copy_text("text")              # system clipboard
Selenite.download(url, col, row)        # background, any size
Selenite.search("q"); Selenite.find("q")
Selenite.undo; Selenite.redo; Selenite.save
Selenite.profile; Selenite.profiles; Selenite.switch_profile("work")
Selenite.reload_plugins
```

### Native helpers

These run in Rust, so they are fast and safe on huge files:

| Helper | Returns |
|---|---|
| `Selenite.file_info(path)` | `{path:, exists:, dir:, size:, modified:, kind:, ext:}` (`modified` is a Unix time) |
| `Selenite.image_size(path)` | `[width, height]` from the image header only (any resolution), or `nil` |
| `Selenite.checksum(path)` | `"fnv1a64:<hex>"`, streamed in 1 MiB chunks |
| `Selenite.playable?(path)` | whether the built-in player can decode the file |
| `Selenite.classify(path)` | `"image"`, `"audio"`, `"video"`, `"ruby"` or `"file"` |

App-changing calls such as `status`, `labels`, `music`, `select`, `open`
and `copy_text` are queued. They take effect as soon as the current Ruby
block returns, in the order you made them. Grid edits apply immediately.

## The music API

`Selenite.music` (alias `Selenite::Music`) controls the built-in player.
It supports wav, ogg, mp3, flac, qoa, xm and mod.

```ruby
m = Selenite.music
m.play("/music/a.mp3")         # play one file (path = nil resumes)
m.play_grid                    # every playable audio cell here, row by row; returns count
m.play_list(paths, start = 0)
m.queue(path)                  # append to the playlist
m.toggle; m.pause; m.resume; m.stop; m.next; m.previous; m.clear
m.seek(90)                     # seconds
m.volume(0.5)                  # 0.0..1.0; m.volume => current
m.shuffle(true); m.repeat(:all)  # :off, :all, :one
m.show; m.hide                 # player bar
m.state   # => {state: "playing", track: "/music/a.mp3", index: 0, tracks: 12,
          #     position: 12.3, length: 201.0, volume: 0.8, shuffle: false, repeat: "all"}
m.playing?; m.track
```

`state` is a snapshot taken when the current Ruby evaluation started.
Commands issued during the same block are not reflected until the next one.

## Events

| Event | Arguments | Notes |
|---|---|---|
| `:activate` | `col, row, kind, path` | Return `:handled` to skip the default action |
| `:paste` | `col, row, path` | Once per pasted item |
| `:drop` | `col, row, [paths]` | Files dropped onto the window |
| `:download` | `col, row, path` | A background download finished |
| `:save` | `json_path` | After the profile is saved |
| `:profile` | `name` | After switching profile |
| `:track` | `path, index, count` | The music player started a new track |

## Errors, undo and reloading

- **Errors never crash Selenite.**
  - An exception in a button, menu or command is shown in the status bar.
  - An exception in a hook is printed to the Ruby console.
  - An exception in a timer stops that timer.
  - A syntax error marks the file red in the panel.
- **Undo:** every button, menu or hotkey action is recorded as one undo step
  named `Plugin: Name › Label`. **Ctrl+Z** reverts all grid edits the action
  made, even across nested grids. The profile is saved automatically when an
  action changes it.
- **Reloading:**
  - Everything plugins registered is forgotten first: actions, commands,
    `p.on` hooks and timers. Then the enabled files are evaluated again.
  - Global variables and constants live on, because the VM is shared. Prefer
    locals captured by blocks (see `clock.rb`).
  - Hooks added with plain `Selenite.on` (for example in `init.rb`) are *not*
    removed.

## How it works

```
 plugin.rb ──eval──▶ Ruby prelude (Selenite.plugin / PluginBuilder)
                       │ every block gets an action id
                       ▼
             Selenite::Plugins.manifest ──Magnus──▶ Rust PluginInfo list
                                                     │
   Plugins panel / F-keys / right-click / timers ◀───┘
                       │ invoke(id, col, row)
                       ▼
             Ruby block runs ──▶ Selenite.* natives (Rust)
                       │           grid edits apply now;
                       │           app requests are queued
                       ▼
             Rust applies the queued requests, records the undo step, saves
```

- `src/plugins.rs`: discovery, manifest types, the panel UI and the bundled
  examples.
- `src/scripting.rs`: the Magnus bindings (`load_plugins`, `invoke_plugin`,
  native helpers).
- `src/ruby/selenite_prelude.rb`: the Ruby half of the DSL.

## Bundled examples

The examples live in [`examples/plugins/`](../examples/plugins). They are also
built into the binary, so **Install examples** works from a packaged build.

| File | Shows |
|---|---|
| [`hello.rb`](../examples/plugins/hello.rb) | A button with a hotkey (F6), a command, reading app state |
| [`file_inspector.rb`](../examples/plugins/file_inspector.rb) | Kind-filtered menu entries; Rust helpers `file_info`, `image_size`, `checksum`; `copy_text`; walking nested grids |
| [`now_playing.rb`](../examples/plugins/now_playing.rb) | The `:track` event, `Selenite.music`, play/shuffle the grid (F7/F8), "Queue track" on audio cells |
| [`grid_tools.rb`](../examples/plugins/grid_tools.rb) | Rearranging cells (undoable), toggling labels (F9), building sub-grids |
| [`clock.rb`](../examples/plugins/clock.rb) | `every` timers, state shared by a hook, a timer and a command |

## Recipes

### Open videos in mpv instead of the desktop default

```ruby
Selenite.plugin "mpv" do |p|
  p.on(:activate) do |_col, _row, kind, path|
    next unless kind == "video"
    spawn("mpv", path)
    :handled
  end
end
```

### Auto-tag downloads by checksum

```ruby
Selenite.plugin "Dedupe" do |p|
  seen = {}
  p.on(:download) do |col, row, path|
    sum = Selenite.checksum(path)
    if seen[sum]
      Selenite.status("#{File.basename(path)} duplicates #{seen[sum]}")
    else
      seen[sum] = File.basename(path)
    end
  end
end
```

### A playlist from a folder

```ruby
Selenite.plugin "Folder Radio" do |p|
  p.button("Play ~/Music", key: "F10") do
    files = Dir[File.expand_path("~/Music/**/*")].select { |f| Selenite.playable?(f) }.sort
    Selenite.music.shuffle(true)
    Selenite.music.play_list(files)
    Selenite.status("#{files.size} tracks")
  end
end
```

### Lay out a folder of images as a sub-grid

```ruby
Selenite.plugin "Import Folder" do |p|
  p.menu("Import ~/Pictures here", kinds: %w[empty]) do |cell|
    sub = Selenite.grid.new_grid(cell.col, cell.row)
    Dir[File.expand_path("~/Pictures/*")].sort.each_with_index do |path, i|
      sub.set(i % 10, i / 10, path)
    end
  end
end
```

This is one undo step: Ctrl+Z removes the whole sub-grid.

### Hide labels while zoomed into the 3D view

```ruby
Selenite.plugin "Clean 3D" do |p|
  p.button("Cinematic 3D", key: "F11") do
    Selenite.labels(false)
    Selenite.view_3d(true)
    Selenite.orbit(30, 25, 18)
  end
end
```

# Selenite

**Version 3.0.0-rc.2** (v3.0 release candidate). See [CHANGELOG.md](CHANGELOG.md).

Selenite is now a **pure Rust** desktop application built with
[`raylib-rs`](https://crates.io/crates/raylib). It provides an infinite,
pannable, zoomable multimedia file grid with recursively nestable sub-grids, all packed
into a single native executable.

## Features

New in 3.0:

- **More than one thing per cell.**
  - Stacked cells: drop onto a file, use Shift+drop, Ctrl+Shift+V or
    Shift+drag. **Tab** cycles the top item.
  - Multi-select: Ctrl/Shift+click, drag a box from an empty cell, or
    Ctrl+A. Then move, cut (Ctrl+X), copy, delete or stack the whole
    selection.
  - **Z** zooms to fit the grid. See [docs/inventory.md](docs/inventory.md).
- **A game inventory for every grid** (**B** / **Items**).
  - Named items with counts and JSON metadata, edited with buttons or
    commands such as `gold x50`, `gold = 12`, `arrow -> bolt` and
    `gem @color=red`.
  - Saved with the grid, undoable and fully scriptable.
- **Game development in Ruby.**
  - A native, raylib-bindings-compatible `Raylib` module (`include Raylib`,
    `InitWindow`, `DrawText`, textures, input, audio, `Camera2D`,
    collisions) built into the binary.
  - `Selenite::Game` is a game-loop helper with access to grids and
    inventories.
  - Run games with `selenite --game file.rb`, or right-click a script →
    **Run as game** / **Export standalone game**.
  - See [docs/game.md](docs/game.md) and [examples/games](examples/games).

Everything else:

- Infinite sparse grid of signed `(col, row)` cells, recursively nestable.
- **Local profiles** ("accounts"): each profile is a completely separate set
  of grids, pasted/downloaded assets, partitioned_array exports and an
  optional `init.rb`. Switch, create, rename and delete them from the
  **Profile** toolbar button. Profiles are local and have no passwords.
- Middle-mouse drag to pan, wheel zoom toward the cursor, **Home** to reset.
- **3D view** (**3D View** button or **F3**): the grid becomes a tilted
  landscape of blocks. Each block's height and colour show its type, images lie
  on top of their tiles, and nested grids are tall blocks with a lattice on top.
  - Rotate: left-drag, **Q**/**E**, **PgUp**/**PgDn**.
  - Pan: right/middle or Shift+drag. Zoom: wheel.
  - Click selects and double-click opens. Arrow keys glide the camera to the selection.
  - Each nested grid remembers its own 3D camera.
  - **W**/**A**/**S**/**D** fly across the grid; right-click (without
    dragging) opens the cell menu.
- **Undo / Redo** (Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z): paste, drop, move, delete
  (including whole nested grids), new grid, finished downloads, PA Import and
  Ruby changes. The last 200 steps are kept for the session.
- **Find** (Ctrl+F): searches every grid of the profile by file name, path
  or kind (`image`, `audio`, `video`, `ruby`, `file`, `grid`). Matches are
  outlined in cyan (2D, 3D and the minimap). Enter/↓ and Shift+Enter/↑ jump
  between them, entering nested grids as needed.
- **Right-click menu** on any cell: Open, Info, Copy file, Copy path,
  Show in folder, Paste here, New grid, Delete, Open grid folder; nested
  grid cells also get **Open its grid folder**.
- **A folder for every grid.** The first time a grid is used (a drop, a
  paste, a download, or **Grid Folder** / **O**) it gets its own folder next
  to the save file. **Grid Folder** opens it in your file manager. Folder ids
  are saved with the grid, so the same grid always maps to the same folder.
- **Added files are copied into the grid's `assets/` folder.** Dropped
  files, pasted files and file paths, pasted images and URL downloads are all
  stored in `<grid folder>/assets/`. Copies of any size run in the
  background with progress on the cell (Esc cancels). Folders are linked,
  never copied. **Copy In: On/Off** turns copying off so files are linked in
  place instead.
- **Info panel** (**I**): kind, full path, size, modification age and image
  pixel size; nested grids show their direct and total cell counts.
- **Minimap** (**M**, on by default): an overview of the grid with your view
  outlined. Click or drag it to jump.
- Inside nested grids the toolbar shows a breadcrumb such as `Root › (2,0) › (1,1)`.
- Status messages fade after 8 seconds.
- Click to select, arrow keys to move the selection, and drag a cell onto
  another cell to move or swap it.
- Double-click / **Enter** to activate a cell:
  - image cells open in a separate tiled image viewer (any resolution)
  - audio cells play in the built-in music player (formats it can't decode,
    and video, open in the desktop media player)
  - Ruby cells run inside the embedded Magnus VM
  - other files open in their registered desktop application
  - nested-grid cells drill into that sub-grid
- Drag-and-drop placement for any file type with left-to-right fan-out.
- **Paste** (Ctrl+V) accepts:
  - files copied in a file manager
  - raw clipboard images of any size, saved as PNG
  - text with one entry per line: `http(s)://` URLs, `file://` URLs,
    or local paths (`~` is expanded)
- URLs download **in the background** and stream to disk with no size limit or
  timeout. Progress bars appear on the target cells and in the status bar;
  **Cancel DL** or **Esc** cancels them.
- **Copy** (Ctrl+C) puts the selected cell's file on the system clipboard.
- Images of any pixel size: thumbnails are decoded on worker threads and
  downscaled to at most 1024 px. The viewer splits full-resolution images into
  4096 px GPU tiles.
- Hover any toolbar button for a tooltip. **Help** (F1) lists every button,
  shortcut, and the Ruby API.
- Inventory list view for browsing and opening every attached file.
- JSON persistence with `SAVE_VERSION = 3` (version 2 saves still load).
- Embedded Ruby (Magnus) with a console, a `Selenite` API, event hooks and
  per-profile `init.rb` (`scripting` Cargo feature, on in packaged builds).
- **Grid labels toggle** (**Labels: On/Off** button or **L**): hide the
  coordinates, file names and type badges (Image, Audio, Video, etc.) for a
  clean view in 2D and 3D. Right-click an item → **Hide item labels** /
  **Show item labels** to toggle it individually. In a stack, this affects
  the top item; use **Tab** to reach another item. Individual choices are
  saved, follow moves and stacking, and support undo/redo. **Labels: Off**
  hides all item labels without resetting those choices. The selected cell
  still shows its coordinates.
- **Built-in music player**:
  - Formats: wav, ogg, mp3, flac, qoa, xm and mod.
  - Double-clicking an audio cell plays it and queues the rest of that grid's
    audio.
  - The bottom bar has previous, play/pause, next, stop, a seek bar, volume,
    shuffle, repeat (off/all/one), a clickable playlist (**P**) and a live
    level visualizer.
  - The playing cell is outlined in purple.
  - Right-click → **Add to playlist** queues more tracks.
  - Volume, shuffle and repeat are remembered.
- **Ruby ↔ Rust plugins** (**Plugins** button):
  - Plugins are `.rb` files that add buttons with F-key hotkeys, right-click
    menu entries, commands, event hooks and timers.
  - From Ruby they call native Rust helpers, the grid API and the music player.
  - Every plugin action is undoable.
  - Five documented examples are included. See [docs/plugins.md](docs/plugins.md).

## Architecture

The app is a single Rust crate:

- `src/camera.rs` — reusable infinite camera math (`Camera`), including
  screen/world conversion, visible ranges, cell picking, and double-click
  detection.
- `src/view3d.rs` — 3D orbit camera (`Orbit`): rotate, pan, zoom, glide-to-cell,
  ray picking against raised blocks, and textured image tiles.
- `src/history.rs` — undo/redo stack of shallow grid snapshots (nested grids
  are shared, so undoing a delete restores their whole contents).
- `src/search.rs` — recursive, case-insensitive search across nested grids.
- `src/persistence.rs` — reusable infinite sparse saved-grid model
  (`SavedGrid`) with nested sub-grids stored via `Arc<Mutex<_>>`, plus JSON
  load/save and recursive item collection.
- `src/inventory.rs` — dedicated inventory list application showing all
  attached items across root and nested grids, complete with thumbnails,
  hierarchy path, coordinates, and double-click to view.
- `src/main.rs` — the raylib application loop, rendering, input, toolbar,
  drag-and-drop, paste/copy, downloads, profiles, navigation, process
  spawning, and CLI mode dispatch.
- `src/profiles.rs` — local profile store (per-profile directories,
  last-used tracking, create/rename/delete).
- `src/download.rs` — cancellable background streaming downloads and
  clipboard text parsing.
- `src/imaging.rs` — unlimited-size image decoding, the async thumbnail
  cache, and tiled full-resolution textures.
- `src/panels.rs`, `src/help.rs` — profile manager, Help overlay, and
  tooltips.
- `src/scripting.rs`, `src/ruby/selenite_prelude.rb` — optional,
  feature-gated (`scripting`) embedded Ruby powered by
  [`magnus`](https://crates.io/crates/magnus).
- `src/ui.rs` — embedded-font text rendering, text fitting, and window/zoom
  aware UI scaling.
- `src/selection.rs` — multi-cell selection (toggle, rectangles, group-move
  planning).
- `src/grid_inventory.rs`, `src/item_panel.rs` — per-grid game inventory
  model and its panel / command parser.
- `src/game.rs`, `src/ruby/raylib_game.rb` — the native raylib-bindings
  style `Raylib` Ruby module and `Selenite::Game` (scripting feature).

Nested grids are shared with `Arc<Mutex<SavedGrid>>`, so edits inside drilled-in
sub-grids mutate the same persisted root graph directly.

## Build

Prerequisites on Linux include a working C/C++ toolchain, CMake, and the usual
raylib desktop dependencies (X11/OpenGL/ALSA headers). Once those are installed:

```sh
cargo build --release
```

This produces:

```sh
./target/release/selenite
```

You can also run directly with Cargo:

```sh
cargo run --release
```

### Scripting console (optional)

The embedded Ruby scripting console is opt-in and excluded from the default
build to keep the default binary dependency-free (no Ruby toolchain needed).
To enable it, build with the `scripting` feature:

```sh
cargo build --release --features scripting
```

This requires a Ruby development environment (headers + `libruby`) available
on the system, since [`magnus`](https://crates.io/crates/magnus) embeds a Ruby
VM directly into the process via its `embed` feature.

## Run modes

### Main grid (profiles)

```sh
./target/release/selenite                    # last-used profile ("default" on first run)
./target/release/selenite --profile work     # open or create the profile "work"
./target/release/selenite --list-profiles
./target/release/selenite --grid my.json     # edit a grid file directly, outside profiles
```

Profiles are stored under:

- `$SELENITE_HOME` if set
- otherwise `%APPDATA%\Selenite` on Windows
- `~/Library/Application Support/Selenite` on macOS
- `$XDG_DATA_HOME/selenite` or `~/.local/share/selenite` on Linux

Each profile directory `profiles/<name>/` holds:

- `selenite.json`, the grids
- `selenite-grids/`, one folder per grid, created the first time a grid is
  used (`root/` for the top grid, `grid-…/` for nested ones). Each holds an
  `assets/` folder with the copies of every file added to that grid (drops,
  pastes, clipboard images, URL downloads).
- `.selenite-assets/`, pasted images and downloads from versions before
  3.0.0-rc.2 (still referenced, never moved)
- `selenite_partitioned_array/`, PA exports
- an optional `init.rb`

Switching profiles saves the current one first, so you get a whole new
series of grids.

### Games

```sh
./target/release/selenite --game examples/games/pong.rb
./target/release/selenite --game examples/games/collector.rb --profile work
```

This runs a Ruby game (raylib-bindings style or `Selenite::Game`) in its own
window, against a profile's grids or a `--grid FILE`. Inside the app,
right-click a `.rb` cell and choose **Run as game** or **Export standalone
game**. See [docs/game.md](docs/game.md).

### Image viewer

```sh
./target/release/selenite --image /path/to/image.png
```

Opens a single image in a fully resizable, pannable, and zoomable viewer window.
The header is read first, so even gigapixel images open instantly with a
"Decoding W×H…" placeholder while the pixels decode in the background. Images
larger than the GPU texture limit are split into 4096 px tiles. Images bigger
than the window start fitted.

Controls in image viewer:
- **Mouse wheel** / **+ -**: zoom
- **Any mouse button drag**: pan image
- **F**: fit image to current window size
- **R** / **1**: reset zoom to 100% (1:1 full pixel size)
- **Escape**: close window

### Inventory viewer

```sh
./target/release/selenite --inventory /path/to/selenite.json
```

Opens an itemized, browsable inventory list view displaying all files and
sub-grids attached anywhere across the grid hierarchy:
- Displays thumbnails, hierarchy path (e.g. `root / Subgrid A / ...`), grid coordinates, and item type.
- **Mouse wheel / Up / Down / PageUp / PageDown**: scroll through the inventory list.
- **Double click / Enter**: opens the selected file using its appropriate viewer/player.
- **R**: reload the JSON file from disk.
- **Escape**: close the inventory window.

You can also launch this window directly from the main application using the **Inventory** button in the toolbar.

## Controls

- **Middle mouse drag**: pan
- **Mouse wheel**: zoom toward cursor
- **Home**: reset pan and zoom
- **Left click**: select cell; **arrow keys** move the selection
- **Double left click** / **Enter**: activate cell
- **Left drag a cell onto another**: move it (swaps if the target is occupied);
  **Shift+drag** stacks it onto the target instead
- **Ctrl+click / Shift+click / drag from an empty cell**: multi-select;
  **Ctrl+A** selects all, **Ctrl+X** cuts. Dragging any selected cell moves
  the whole selection.
- **Tab / Shift+Tab**: cycle a stacked cell; **Shift+Delete**: remove its top item
- **Ctrl+Shift+V**: paste as a stack onto the selected cell
- **B**: grid inventory panel; **Z**: zoom to fit the grid
- **Ctrl+V / Ctrl+C / Ctrl+S**: paste, copy, save
- **Ctrl+Z / Ctrl+Y** (or **Ctrl+Shift+Z**): undo / redo
- **Ctrl+F**: find (Enter/↓ next, Shift+Enter/↑ previous, Esc close)
- **Right click** a cell: context menu
- **I**: cell info panel; **M**: minimap; **L**: grid labels on/off
- **Space**: play/pause music (plays this grid's audio when idle); **P**: playlist
- **[ / ]**: previous / next track; **, / .**: seek −5 s / +5 s; **9 / 0**: volume −5% / +5%
- **F2, F4–F12**: plugin buttons that declare a hotkey
- **Backspace**: go back to parent grid when inside a nested grid
- **Delete**: delete the selected cell (same as the **Delete** toolbar button)
- **Esc**: close dialogs / cancel running downloads / clear selection
- **F1**: Help overlay
- **F3**: toggle the 3D view. In 3D:
  - Rotate: left-drag, **Q**/**E** (turn 15°), **PgUp**/**PgDn** (tilt).
  - Pan: right/middle or Shift+left drag, or **W**/**A**/**S**/**D**. Zoom: wheel.
  - Click selects and double-click opens; right-click opens the cell menu.
    **Home** resets the 3D camera.
- **Backtick ( \` )**: toggle the Ruby console
- **Ctrl + = / Ctrl + -**: make UI text (toolbar, console, dialogs, status
  bar, inventory) larger or smaller; **Ctrl + 0** resets
- **Drag and drop files**: place any files beginning at the target cell and
  fan them out left-to-right

### Text scaling

- UI text and buttons scale with the window size (and the Ctrl +/- multiplier).
  When the window is too narrow, toolbar buttons wrap onto additional rows.
- Text inside grid cells (coordinates, file names, type badges) is sized from
  each cell's on-screen size, so it grows and shrinks with the camera zoom.
  Long names are shortened with "…" to fit.
- Text is rendered with the embedded DejaVu Sans font
  (`assets/fonts/DejaVuSans.ttf`, license in `assets/fonts/LICENSE-DejaVu.txt`)
  through a mipmapped atlas, so it stays smooth at every size.

## Toolbar actions

### Main mode

Hover any button for a tooltip that explains it.

- **Profile: NAME** — open the profile manager to switch, create (**N**),
  rename, or delete profiles. You cannot delete the active profile, and
  deleting asks for confirmation.
- **Save** — save the profile's grids to JSON (also automatic after edits)
- **New Grid** — create a nested sub-grid at the selected empty cell
  (disabled unless an empty cell is selected)
- **Delete** — remove the selected cell's contents (disabled unless an
  occupied cell is selected; deleting a nested grid that still has contents
  asks for confirmation — **Undo** brings it back)
- **Paste** — paste files, an image of any size, paths, or URLs (one per
  line). The first item replaces the selected cell; the rest fill the next
  free cells to the right.
- **Copy** — copy the selected file to the system clipboard
- **Open / Run** — view, play, open, or run the selected cell
- **PA Export** — save every cell of the grid you're viewing into a
  [partitioned_array](https://github.com/Infinivaeria/partitioned_array)
  database in the profile's `selenite_partitioned_array/` folder. This
  replaces the previous export.
- **PA Import** — load that export back into the grid you're viewing.
  Cells that are already occupied are left unchanged.
- **Inventory** — spawn a separate inventory window
- **Items** — this grid's game inventory panel (B)
- **Grid Folder** — open this grid's own folder in the file manager (O)
- **Undo / Redo** — step through the edit history (Ctrl+Z / Ctrl+Y)
- **Find** — search the whole profile (Ctrl+F)
- **Ruby** — toggle the Ruby console
- **Home** — reset pan/zoom and the 3D camera
- **3D View / 2D View** — switch between the flat grid and the rotatable 3D view (F3)
- **Labels: On / Off** — show or hide cell coordinates, file names and type badges (L)
- **Copy In: On / Off** — copy dropped/pasted files into the grid's `assets/` folder, or link them in place
- **Music / Hide Music** — show or hide the music player bar
- **Plugins (n)** — the plugin panel:
  - enable or disable each plugin file;
  - run plugin buttons;
  - **Reload**, **Open folder**, **Install examples**.

  `n` is the number of enabled plugins that loaded cleanly.
- **Help** — every button, shortcut, and the Ruby API
- **Cancel DL (n)** — only shown while downloads run
- **Back** — return to the parent grid when inside a nested grid

### Inventory mode

- **Refresh** — reload the JSON file from disk
- **Find** — search the inventory's grids
- **3D View / 2D View** — the same 3D view as the main window
- **Labels** / **Music** — same as in the main window
- **Back** — return to the parent grid when inside a nested grid

## Persistence format

Selenite stores data in JSON using `SAVE_VERSION = 3`.

- Each grid is `{ "cells": [{ "col", "row", "items": [...] }], "inventory": {...} }`.
  `items` is the cell's stack, top first.
- Each item is one of:
  - `Image(PathBuf)`
  - `File(PathBuf)` (audio, video, Ruby, documents, archives, and other files)
  - `Grid(Arc<Mutex<SavedGrid>>)`
- The `inventory` holds the grid's named items (`count` + `meta`) and is
  left out when empty.
- Items may also contain `"labels_hidden": true` to hide their labels.
  Missing values default to visible, so existing saves retain their labels.
- Version 2 saves (single-item cells) load unchanged and are written as
  version 3 on the next save.

Image files are referenced by path; keep them accessible if you want the saved
grid to continue displaying them.

## Scripting (Ruby via Magnus)

When built with `--features scripting` (the packaged build), pressing
backtick (`` ` ``) or the **Ruby** button opens a console. The Ruby VM starts
lazily, or at launch when the profile has an `init.rb`.

Console keys:

- **Enter**: run the line
- **Up/Down**: history
- **Ctrl+V**: paste (multi-line pastes are joined with `;`)
- **PageUp/PageDown** or the wheel: scroll back
- **Ctrl+L**: clear
- **Ctrl+U**: clear the input line
- **Esc**: close

`puts`/`p` output is captured into the console, and errors never crash the app.

### `Selenite` module

```ruby
g = Selenite.grid             # grid you're viewing (Selenite.root = top level)
g[0, 0]                       # => #<Selenite::Cell col=0 row=0 kind=image path=...> or nil
g[1, 0] = "/music/song.mp3"   # place any file (also g.set(col, row, path))
sub = g.new_grid(2, 0)        # nested grid; g.subgrid(2, 0) fetches it
g.move(1, 0, 5, 5); g.swap(0, 0, 5, 5)   # (from_col, from_row, to_col, to_row)
g.fill(0, 3, Dir["/pics/*.png"])   # fan out files from (0,3)
g.each { |cell| puts cell }   # Enumerable: g.map, g.select, g.count…
g.remove(5, 5); g.next_free(0, 0); g.size; g.clear

Selenite.select(1, 0); Selenite.goto(40, -12); Selenite.open(1, 0)
Selenite.enter(2, 0); Selenite.back
Selenite.view_3d(true); Selenite.orbit(45, 35, 20)   # yaw°, pitch°, distance (cells)
Selenite.search("song")       # => [[[2, 0], 1, 1, "song.mp3"], ...]  ([nested path, col, row, name])
Selenite.find("song")         # open the Find bar and jump to the first match
Selenite.undo; Selenite.redo; Selenite.history   # => ["Paste", nil] (next undo / redo)
Selenite.download("https://example.com/video.mp4", 3, 0)  # background, no size limit
Selenite.status("hello")
Selenite.selected; Selenite.depth; Selenite.save; Selenite.save_path; Selenite.assets_dir
Selenite.grid_folder          # this grid's own folder (created on first use)
Selenite.assets_dir           # <grid folder>/assets — where files added to the grid are copied
Selenite.import(3, 0, "/music/song.mp3")   # copy into assets/ and stack it on (3, 0); returns the copy's path
g.subgrid(2, 0).import(0, 0, "notes.txt")  # same for any grid; g.assets_dir is that grid's assets/
g.subgrid(2, 0).folder        # any grid's folder; g.folder_id is nil until first used
Selenite.grids_dir            # the folder that holds every grid folder
Selenite.classify("x.flac")   # => "audio"
Selenite.profile; Selenite.profiles; Selenite.create_profile("work"); Selenite.switch_profile("work")
Selenite.help
```

### Stacks, selection and inventories

```ruby
g.push(0, 0, "/music/b.ogg"); g.items(0, 0)    # stack on top; list top-first
g.cycle(0, 0, 1); g.pop(0, 0); g.merge(0, 0, 1, 0); g.unstack(1, 0)
Selenite.selection; Selenite.select_cells([[0, 0], [1, 0]])
inv = Selenite.inventory                       # this grid's (g.inventory for any grid)
inv.add("gold", 50); inv.remove("gold", 5); inv["key"] = 1
inv.set_meta("sword", "damage", 7); inv.each { |name, count, meta| p [name, count, meta] }
inv.transfer(Selenite.root.inventory, "gold", 10)
```

The full reference is in [docs/inventory.md](docs/inventory.md).

### Games (`Raylib` + `Selenite::Game`)

```ruby
require "raylib"   # optional; built in
include Raylib
InitWindow(800, 450, "Hello"); SetTargetFPS(60)
until WindowShouldClose()
  BeginDrawing(); ClearBackground(RAYWHITE)
  DrawText("Hello from Selenite!", 190, 200, 20, LIGHTGRAY)
  EndDrawing()
end
CloseWindow()
```

Run it with `selenite --game hello.rb` or with **Run as game** in the app.
`Selenite::Game.run(title:) { |g| g.update { |dt| }; g.draw { } }` adds a
game loop with timers, a camera, cached textures and access to
`g.grid` / `g.inventory`. See [docs/game.md](docs/game.md).

### Event hooks

```ruby
Selenite.on(:activate) do |col, row, kind, path|
  if kind == "audio"
    system("mpv", path)
    :handled                  # skip the default open action
  end
end
Selenite.on(:download) { |col, row, path| puts "got #{path}" }
```

Supported events:

- `:activate`, with `(col, row, kind, path)`
- `:paste`, with `(col, row, path)`
- `:drop`, with `(col, row, [paths])`
- `:download`, with `(col, row, path)`
- `:save`, with `(json_path)`
- `:profile`, with `(name)`
- `:track`, with `(path, index, count)`; fired when the music player starts a track
- `:inventory`, with `(message)`; fired when the inventory panel changes an item

Use `Selenite.off(:event)` to remove hooks and `Selenite.hooks` to list them.
Put hooks in `profiles/<name>/init.rb` so they load automatically.

### Labels, music and Rust helpers

```ruby
Selenite.labels(false); Selenite.labels?        # grid labels
m = Selenite.music
m.play_grid; m.play("/music/a.flac"); m.queue(path); m.toggle; m.next
m.seek(30); m.volume(0.6); m.shuffle(true); m.repeat(:all); m.state
Selenite.file_info(path)    # {path:, exists:, dir:, size:, modified:, kind:, ext:}
Selenite.image_size(path)   # [w, h] from the header, any resolution
Selenite.checksum(path)     # "fnv1a64:..." streamed in Rust
Selenite.playable?(path); Selenite.copy_text("text")
```

### Plugins

```ruby
# ~/.local/share/selenite/plugins/hello.rb
Selenite.plugin "Hello" do |p|
  p.description "Says hello"
  p.button("Say hello", key: "F6") { |cell| Selenite.status("Hello #{cell}") }
  p.menu("Inspect", kinds: %w[file]) { |cell| puts Selenite.file_info(cell.path) }
  p.command("greet") { |name = "you"| "Hello, #{name}!" }   # Selenite.run("greet")
  p.on(:track) { |path, i, n| Selenite.status("#{i + 1}/#{n} #{File.basename(path)}") }
  p.every(60) { Selenite.status(Time.now.strftime("%H:%M")) }
end
```

The full guide, with recipes and a walkthrough of the five bundled examples
in [`examples/plugins/`](examples/plugins), is in
[docs/plugins.md](docs/plugins.md).

### Classic global helpers

These still work:

- `grid_set`, `grid_set_file`, `grid_get`, `grid_new_grid`
- `grid_remove`, `grid_move`, `grid_swap`, `grid_clear`
- `grid_count`, `grid_list`, `grid_save`, `grid_eval_file`
- `grid_status`, `grid_select`, `grid_download`

### partitioned_array integration

Selenite embeds the [partitioned_array](https://github.com/Infinivaeria/partitioned_array)
Ruby library (`ManagedPartitionedArray`, `LineDB`, etc.) into the Magnus VM.
Its `lib/` directory is added to `$LOAD_PATH`, resolved in this order:

1. `SELENITE_PARTITIONED_ARRAY_LIB` environment variable
2. `partitioned_array/lib` next to the executable (portable bundle)
3. `partitioned_array/lib` in the source checkout

Each cell of the currently viewed grid is stored as one hash record
(`col`, `row`, `kind`, `path`). Nested grids are recorded as empty `grid`
containers.

- `grid_pa_available()` — whether the library could be loaded
- `grid_pa_export(db_path, db_name = "selenite_cells")` — overwrite a snapshot
- `grid_pa_records(db_path, db_name = "selenite_cells")` — read stored records
- `grid_pa_import(db_path, db_name = "selenite_cells")` — restore cells;
  occupied cells are skipped

The **PA Export** / **PA Import** toolbar buttons use
`selenite_partitioned_array/` next to the profile's `selenite.json`, so each
profile has its own PA database. Any other library class
is usable from the console or scripts, e.g. `require "line_db"`.

Ruby scripts can also run headlessly:

```sh
./target/release/selenite --script script.rb                  # last-used profile
./target/release/selenite --script script.rb --profile work
./target/release/selenite --script script.rb --grid my.json   # or just my.json
```

`__FILE__`/`__dir__` refer to the script, output is printed, the grid is saved
afterwards, and an uncaught Ruby error exits with status 1. (`cargo build --release`) if you don't need
scripting — it keeps the dependency tree and build time minimal, since
`magnus`/`rb-sys` are declared as optional dependencies.

## Tests

The reusable engine modules contain regular Rust unit tests:

```sh
cargo test
```

These cover:

- camera math and double-click detection
- 3D orbit clamping/panning/gliding and ray picking (blocks vs. ground)
- undo/redo (including restoring deleted nested grids) and recursive search
- context menu items/layout and human-readable file ages
- sparse persistence, nested-grid sharing, and save/load round-trips
- profiles (create, rename, delete, last-used)
- music playlist order (shuffle laps, repeat modes), player bar layout
- plugin discovery, example installation, panel buttons and menu filtering
- settings round-trips
- streaming downloads against a local HTTP server
- clipboard parsing, tile layout, text wrapping, and CLI parsing

Run with `--features scripting` to also exercise the Ruby console round-trip
test:

```sh
cargo test --features scripting
```

## Packaging & Distribution

To create a self-contained, portable distribution of Selenite with Ruby runtime libraries bootstrapped and bundled alongside:

```sh
./package.sh
```

This script:
1. Compiles the binary in release mode with embedded scripting: `cargo build --release --features scripting`.
2. Packages the output into `dist/`.
3. Bundles `libruby.so*` and standard library assets (`dist/lib/ruby/`) so the app runs without requiring Ruby to be pre-installed on the host system.
4. Generates a portable launcher script `dist/run.sh` that sets up `LD_LIBRARY_PATH` and `RUBYLIB`.
5. Copies `docs/`, `examples/`, `README.md` and `CHANGELOG.md` into the bundle.
6. Compresses the release bundle into `selenite-<version>-portable.tar.gz`
   (for example `selenite-3.0.0-rc.2-portable.tar.gz`).

To test or run the standalone package on any Linux machine:
```sh
mkdir selenite && tar -xzf selenite-3.0.0-rc.2-portable.tar.gz -C selenite
cd selenite
./run.sh [--profile NAME | --grid FILE.json | --script FILE.rb | --game GAME.rb ...]
./run.sh --game examples/games/pong.rb
```

"Export standalone game" in a packaged build copies this runtime into the
exported folder, so `./run-game.sh` works on machines without Selenite.

Profiles go to `~/.local/share/selenite` unless `SELENITE_HOME` is set.
For a fully self-contained setup, run `SELENITE_HOME=./data ./run.sh`.

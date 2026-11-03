# Changelog

## 7.0.0-rc.1

### Added

- Text editor (`selenite --edit FILE`) with Ruby and Markdown syntax
  highlighting, a live Markdown preview, undo/redo, selection and clipboard,
  Ruby block-aware indentation, Markdown list continuation, atomic saves that
  keep line endings and permissions, and a warning before overwriting outside
  changes. Open it from a cell's menu (Edit text, New note here), by activating
  `.md`/`.txt` cells, or with `Selenite.edit(path)`.
- Editor find and replace (Ctrl+F / Ctrl+H) with match highlighting, case
  toggle, stepping, and a replace-all that undoes in one step.
- Draggable divider between the editor and the Markdown preview
  (double-click resets it) and clickable preview links: web and `mailto` links,
  heading anchors and relative files open; other schemes are blocked.
- Per-grid Markdown notes (`notes.md` in each grid's folder) from the Notes
  toolbar button and the cell menu.
- Terminal support: a Terminal toolbar button, `selenite --terminal` and
  `--terminal-run FILE [ARGS]` (shebang, Ruby, Python, JS or Bash), Run in
  terminal and Terminal in grid folder cell actions, `Selenite.terminal`, and
  `SELENITE_*` variables inside terminals.
- Scaling: windows keep scaling up to 6x when maximized, new windows follow the
  display DPI and are kept on the monitor, the Ctrl +/- zoom is saved as
  `ui_scale`, Ctrl+wheel zooms the editor, and the Ruby console grows with the
  window instead of stopping at 14 lines.
- Live Asset Inventory management for each grid, including every stacked file
  and nested-grid container, with search and keyboard navigation.
- Multi-file drag-and-drop into the inventory: drop on a row to extend its
  stack, or on blank panel space to create a new stacked asset cell.
- Inventory open, promote, extract, remove, paste and unstack commands,
  in-panel undo/redo, transfer progress, and automatic saving.
- Ruby `Grid#assets`, `Selenite.assets`, and `Selenite.asset_inventory` for
  per-grid enumeration and organizer/plugin automation.

### Fixed

- Reserve blank-drop targets while background transfers are pending.
- Prevent confirmed removal from deleting the wrong item if a background
  import changed its stack while the prompt was open.
- Recheck filename availability after acquiring a copy's temporary file so
  concurrent same-named imports do not replace completed copies.

### Changed

- Organizer and dedicated game runtime version is 7.0.0-rc.1, a release
  candidate rather than a final release. Existing save formats remain compatible.
- The separate File List remains read-only, avoiding competing save-file
  writers. The editable Asset Inventory is a live modal in the organizer.
- Ruby remains the Turing-complete scripting language; no new language engine
  or sandbox is introduced. Long-running scripts still require care.

## 6.0.0

### Added

- Stack Items toolbar button and cell-menu manager with case-insensitive search,
  focused rows, keyboard navigation, clipboard stacking, and in-panel undo/redo.
- Extract a single stacked file or grid into a free cell, preserving the other
  items, with undo and persistent saves.
- Ruby `Grid#extract_item(col, row, index)` and
  `Selenite.stack_items(col, row)` for organizer automation and plugins.

### Changed

- Bumped the organizer and dedicated game runtime to 6.0.0; existing saves remain
  compatible. Ruby remains the general-purpose scripting language.
- Bringing an item to the top preserves the relative order of all other items.
- Music and other files without visible thumbnails retain labels even when
  global or individual labels are disabled.

### Fixed

- Stack-manager removal of non-empty nested grids requires confirmation;
  removal, extraction, promotion, and unstacking support undo.

## 5.0.0

### Changed

- Bumped the Selenite organizer and dedicated game runtime to 5.0.0.
- Dimensional grids have child-grid portals at every empty coordinate, with
  the same coordinate space at every level and no fixed child-count preset.
  Entered branches are materialized and saved on demand, with undo/redo support.

## 3.0.0-rc.4

### Added

- Ruby multiplayer networking in both runtimes: nonblocking TCP peers/meshes,
  shared-key handshake/message authentication, bounded queues, host-authoritative
  state and late-join snapshots, IPv4 LAN discovery, and logical-asset grid rooms.
- Cooperative Gems multiplayer example, Shared Grid organizer plugin,
  plugin resource-cleanup callbacks, and networking documentation/tests.
- Ruby `.rbc` compilation and execution in both executables, with runtime
  compatibility metadata, corruption checks, explicit replacement, compiled
  helper loading, relocated game assets, and bytecode-aware exports/launchers.
- Dedicated `selenite-game` executable, built only with the `scripting`
  feature. Runs Ruby games without opening the organizer or selecting a profile.
- `bootstrap-game.sh` and `package.sh --game` produce a game-only portable
  distribution with Ruby libraries, PartitionedArray, documentation,
  examples, and a playable starter. Full packages include both executables.
- Persistent game tables, stable IDs, lazy loading, active / dormant / revenant
  partition labels, metadata, checkpoints, and safe commits.
- Deterministic BSP trees and persistent worlds with region streaming.
- Complete game-development tutorial and partitioned-storage instructions.

### Changed

- Game/script entry files must use `.rb` or `.rbc`. Games execute through embedded Ruby;
  no Python, JavaScript, or other language runners are added.
- Standalone exports prefer the dedicated game executable and copy sibling
  `game-data`, along with assets and the array library.
- Portable launchers configure Ruby standard-library and architecture extension
  paths, and include the actual versioned libruby shared libraries.

### Fixed

- Fixed maximized windows retaining an old-sized drawing area by refreshing
  the framebuffer viewport and projection every frame.
- Added full-scene 4-sample supersampling to the grid, 3D view, image viewer,
  and inventory, including correctly scaled help-panel clipping.
- Keep grid saving and GPU cleanup on rendering-error exits.

## 3.0.0-rc.3

### Added

- **Added files are copied into a per-grid `assets/` folder**
  - Files dropped from the file manager or pasted (copied files / file
    paths) onto a grid are copied into `<grid folder>/assets/`, and the cell
    points at the copy. The grid keeps working if the originals are moved or
    deleted.
  - Copies run in the background with no size limit. Progress shows on the
    target cell and in the status bar, and Esc / **Cancel DL** cancels them.
    Name clashes get `name (2).ext`, `name (3).ext`, …
  - Folders and files that are already in the grid's `assets/` are linked,
    not copied.
  - Pasted clipboard images and URL downloads now go into `assets/` too.
  - **Copy In: On / Off** toolbar toggle (saved in `settings.json`, on by
    default). Turn it off to link files in place as before.
  - Ruby: `Selenite.assets_dir` / `Grid#assets_dir` return the grid's
    `assets/` folder. `Selenite.import(col, row, path)` / `Grid#import` copy
    a file in and stack it on a cell, returning the copy's path. `:drop` and
    `:paste` hooks fire when a copy lands, with the copied path.

### Changed

- `Selenite.assets_dir` is now `<grid folder>/assets` (it was the grid
  folder itself in rc.2). Existing references are left untouched.

## 3.0.0-rc.2

### Added

- **Per-grid folders**
  - Every grid (the root and each nested grid) gets its own folder the first
    time it is used: `<save dir>/<save name>-grids/root/` and
    `…/grid-<id>/`. The folder id is stored in the save file, so a grid
    keeps its folder across sessions, moves and undo.
  - Pasted clipboard images and URL downloads are written into the folder of
    the grid they land in (previously one shared `.selenite-assets/`).
    Files dropped from the file manager are still referenced in place.
  - **Grid Folder** toolbar button, **O** key and right-click → **Open grid
    folder** open the current grid's folder; right-click a nested grid →
    **Open its grid folder** opens that grid's folder.
  - Ruby: `Selenite.grid_folder` (also `Selenite.assets_dir`), `Grid#folder`,
    `Grid#folder_id` and `Selenite.grids_dir`.

### Changed

- Older saves load unchanged; existing `.selenite-assets/` files stay where
  they are.

## 3.0.0-rc.1 (v3.0 release candidate)

### Added

- **More than one thing per cell**
  - Cells hold a stack of items: files and/or nested grids.
  - Ways to stack: drop onto a file cell, Shift+drop, **Ctrl+Shift+V**,
    **Shift+drag** a cell onto another, or right-click → **Stack selection
    here**.
  - **Tab** / **Shift+Tab** cycles the top item. **Shift+Delete** removes
    only the top item. **Unstack into free cells** spreads a stack out.
  - Stacked cells show a `×N` badge.
- **Multi-select**
  - Ctrl+click toggles a cell, Shift+click selects a rectangle, and dragging
    from an empty cell draws a selection box. **Ctrl+A** selects all.
  - Group move by dragging any selected cell: blockers are detected and
    nothing is lost.
  - Group copy, **Ctrl+X** cut, and delete.
- **Per-grid game inventory**
  - Named items with counts and JSON metadata, saved with each grid.
  - The **Items** panel (**B**) has +/−/delete buttons and an entry field
    that accepts `name xN`, `N name`, `name -N`, `name = N`, `old -> new`
    and `name @key=value`.
  - Changes are undoable and fire a new `:inventory` Ruby event.
- **Game development API**
  - A native `Raylib` Ruby module, implemented in Rust via Magnus and
    raylib-rs, in the style of the raylib-bindings gem (`include Raylib`).
    It covers window/timing, shapes, text, textures, keyboard/mouse,
    sounds/music streams and `Camera2D`, plus Ruby-side `Vector2`,
    `Rectangle`, `Color`, colour/key constants, collisions and vector math.
    Every function also has a snake_case alias.
  - `require "raylib"` works without a gem.
  - `Selenite::Game`: a game loop with update/draw/draw_ui callbacks, timers,
    camera, input helpers, cached textures, `cell_texture`, grid and
    inventory access, and autosave.
  - `selenite --game FILE.rb [--profile NAME | --grid FILE]`.
  - Right-click a `.rb` cell → **Run as game**. Selenite stays responsive
    and reloads the grid when the game saved changes.
  - Right-click a `.rb` cell → **Export standalone game**. This writes a
    folder with the script, a save snapshot, assets, the runtime (the full
    portable bundle when available) and `run-game.sh`.
  - `SELENITE_GAME_FRAMES` / `SELENITE_GAME_SCREENSHOT` for unattended
    runs.
  - Example games: `examples/games/pong.rb` and `collector.rb`.
- **Ruby API**
  - `Grid#items/push/push_grid/pop/cycle/raise_item/merge/unstack/stacked?/stack_size`.
  - `Grid#inventory` and `Selenite.inventory` (`add`, `remove`, `[]`,
    `[]=`, `has?`, `rename`, `meta`, `set_meta`, `transfer`, `each`, …).
  - `Selenite.selection` / `select_cells`.
  - `grid_push_file`, `grid_push_grid` and `grid_list_stacks`.
- **Z** zooms to fit the whole grid.
- **Docs**: [docs/game.md](docs/game.md) and
  [docs/inventory.md](docs/inventory.md). The Help overlay has a new Games
  section and the new shortcuts.

### Changed

- The save format is now `SAVE_VERSION = 3` (stacked cells and
  inventories). Version 2 saves load unchanged.
- The right-click menu adds stack actions, Grid inventory, Run as game and
  Export standalone game.

### Tests

- New tests cover inventory, the panel command parser, stacks, selection,
  group moves, v2→v3 migration, the Ruby game API, the `--game` CLI parsing,
  context menus per cell kind and standalone game export.

## 2.0.0-rc.1 ("v2.0f" release candidate)

### Added

- **Grid labels toggle**
  - New **Labels: On/Off** toolbar button (main and inventory windows), the
    **L** key, and `Selenite.labels(bool)` / `Selenite.labels?` from Ruby.
  - Turning labels off hides coordinates and file names in the 2D and 3D
    views. The selected cell keeps its coordinates.
  - The setting persists in `settings.json`.
- **Built-in music player**
  - Formats: wav, ogg, mp3, flac, qoa, xm and mod.
  - Activating an audio cell plays it, with the rest of that grid's audio
    queued in row order.
  - The player bar has previous, play/pause, next, stop, a seek bar, volume,
    shuffle, repeat (off/all/one), a playlist panel and a live level
    visualizer.
  - The playing cell is outlined in purple.
  - Keys: Space, `[` `]`, `,` `.`, `9` `0`, P.
  - The right-click menu gains **Add to playlist**.
  - Volume, shuffle and repeat persist.
  - Fully scriptable through `Selenite.music`, plus a new `:track` event.
- **Ruby ↔ Rust plugins**
  - `*.rb` files in `<data>/plugins` and `<profile>/plugins`.
  - The DSL covers `button` (with F-key hotkeys), `menu` (filtered by cell
    kind), `command`, `on` and `every`.
  - A **Plugins** panel to enable/disable, reload, open the folder and
    install examples.
  - Each plugin action is one undo step.
  - Errors are isolated per plugin.
- **New Rust helpers for Ruby**: `file_info`, `image_size` (header-only),
  `checksum` (streamed), `playable?`, `copy_text`, `reload_plugins`,
  `run` / `commands`.
- **Five example plugins**, also built into the binary: hello,
  file_inspector, now_playing, grid_tools and clock.
- **Documentation**: [docs/plugins.md](docs/plugins.md).
- **Global settings file**: `<data>/settings.json` stores labels, minimap,
  volume, shuffle, repeat and disabled plugins.

### Changed

- The version is shown in the window title and in Help.
- raylib is built with FLAC support.
- The status bar and minimap move up while the player bar is visible.
- Packaged builds include `docs/` and `examples/`. The tarball name now
  contains the version.

### Fixed

- Reloading plugins now removes hooks registered with `p.on` reliably.

# Changelog

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

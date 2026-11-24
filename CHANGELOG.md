# Changelog

## mic_freq 6.0.0-rc.1 (2026-11-23)

- Embedded a sandboxed Rhai runtime, making the microphone suite computationally
  Turing complete through per-frame scripts loaded with `--script FILE`.
- Added live script inputs for frame/time, level, pitch, band, pose, mute state
  and thresholds; scripts can override pose, band, gains, thresholds, bounce,
  scale, mirror and blink.
- Added bounded execution, strict variables, operation/call/memory limits and
  no host I/O API. Invalid scripts disable themselves with an explicit UI error.
- Added `--check-script FILE`, headless script validation in `--self-test`, and
  a complete example script for the packaged documentation/examples.

## mic_freq 5.0.0-rc.1 (2026-11-23)

- mic_freq is now a complete microphone suite with its existing
  voice-reactive PNGTuber studio plus a dedicated live tools panel.
- Added a 50–2,000 Hz pitch tuner with nearest note, frequency and cents from
  concert A 440, an eight-band spectrum, input-level/clipping display, and
  input-device/sample-rate diagnostics.
- Added guided, opt-in two-stage calibration: measure room noise, measure voice,
  review the recommendation, then explicitly apply the gate and loud offset.
- Microphone pitch uses a time-based 80 ms window with downsampling for stable
  low/mid/high band switching at common and high input sample rates.
- Extended gate and loud sensitivity ranges, aligned all frequency gain
  controls, and retained band artwork through short pitch dropouts.
- Frequency artwork slots are isolated by band and pose, including when source
  files have identical names. Quiet, muted and unpitched states return to
  global artwork.
- System microphone gain is never changed automatically. Clipping is diagnosed
  explicitly so input gain can be corrected at the host audio layer.

## mic_freq 4.0.0

### Improved

- Frequency animation windows use a fully scaled 900 x 760 logical layout,
  keeping High voice instructions clear of the gain controls at every size.
- Global and per-frequency artwork can be replaced through an in-app browser.
- Blinks remain visible for about 220 ms with a randomized 2–5 second interval.
- Music Composer is now owned by the global Selenite suite; Audio Studio remains
  focused on microphone-driven PNGTuber presentation.

## 19.0.0-rc.1 (2026-11-20)

### Added

- Audio Studio now classifies live microphone pitch into Low, Mid and High
  voice bands and provides independently adjustable -18 to +18 dB sensitivity
  for each band before the noise-gate and talking/loud pose thresholds.
- The Audio Studio status area shows the active pitch band, detected pitch and
  applied gain so frequency response can be tuned while speaking.

### Improved

- Audio Studio now opens at 1280 x 960 with a 960 x 720 minimum and a wider,
  uniformly scaled sidebar, giving the additional controls larger sliders and
  touch targets across window sizes.
- Low/Mid/High gain settings clamp safely, persist with normal settings and
  named presets, and leave unpitched input at its original microphone level.

This is a release candidate. Existing Audio Studio settings remain backward
compatible and use zero dB frequency gains by default.

## 18.0.0-rc.1 (2026-11-19)

### Finalized

- Packaged executables now initialize their own package root, Ruby library paths,
  native libraries, portable FFmpeg path, partitioned array path, `PATH`,
  `SELENITE_EXE`, and `SELENITE_ROOT`; `run.sh`, `run-ide.sh`, `run-game.sh`, and
  `run-mic-freq.sh` are compatibility shims rather than startup requirements.
- `selenite` directly exposes source-checkout package, distribution, bootstrap,
  game-bootstrap and appliance installation controls through
  `--package-suite`, `--distribute-suite`, `--bootstrap-suite`,
  `--bootstrap-game-suite`, and `--install-appliance`.
- Audio Studio can open Graphics Studio directly with `--graphics-studio`, and
  the organizer toolbar now provides dedicated Audio, Graphics and CAS buttons.
- Package and relocated smoke tests delete shell launchers before verification,
  then verify direct Selenite, game, Audio Studio, Graphics/CAS headless paths,
  FFmpeg tools and package metadata.

## 17.0.0-rc.1 (2026-11-19)

### Added

- Selenite CAS Notebook, backed by Symbolica 3.0.2 and nalgebra 0.33, with exact
  parse/simplify/expand/factor/together/cancel/apart, differentiation, supported
  rational/polynomial integration, Taylor series, solving and substitutions.
- nalgebra determinant, inverse, matrix solve, rank and complex eigenvalue
  commands using explicit JSON matrix/vector input and finite/error validation.
- A scaled CAS notebook GUI with cell CRUD, command templates, results/steps,
  persistent JSON notebooks and keyboard workflows; organizer/Suite/shortcut and
  direct executable entry points.
- Direct `--cas`, `--cas-run` and `--cas-notebook` modes for GUI, one-shot and
  batch notebook execution.
- Complete CAS documentation, command reference and Symbolica non-commercial /
  one-thread licensing disclosure.
- Package and distribution capability checks for symbolic mathematics,
  numerical linear algebra, CAS notebook and direct CAS execution.

This is a release candidate. Symbolica remains a restricted non-commercial
backend unless the operator supplies their own compliant Symbolica licensing.
Existing profiles, grids and studio settings remain backward compatible.

## 16.0.0-rc.1 (2026-11-19)

### Added

- Selenite Graphics Studio with scale-safe layered image editing, transforms,
  crop/resize, brightness/contrast/grayscale/invert/blur, undo/redo, animated
  GIF playback, JSON graphics projects and flattened PNG export.
- Explicit ffmpeg/ffprobe-backed video probing, representative frame extraction
  and safe allow-listed transcoding, all launched as direct processes without a
  shell.
- PNGTuber visual creation directly from source images, including idle/talking/
  loud and blink pose output compatible with mic_freq; Audio Studio's Graphics
  button opens this workflow.
- Direct `selenite --graphics-studio` and `--audio-studio` suite launchers,
  headless graphics export/PNGTuber/frame-extract/transcode modes, toolbar and
  Suite entries, and application-menu Graphics Studio shortcuts.
- Image processing now uses the `imageproc` Rust crate for production Gaussian
  filtering; the rest remains native Rust `image` processing.

### Finalized

- Graphics Studio and Audio Studio can be launched directly from their
  executables; packaged `run*.sh` scripts remain compatibility launchers rather
  than runtime requirements.
- Existing `package.sh` and `distribute.sh` workflows now advertise and verify
  Graphics Studio, Audio Studio, host/appliance and media capabilities.

This is a release candidate. Existing profiles, grids, Audio Studio settings and
Composer projects remain backward compatible.

## 15.0.0-rc.1 (2026-11-19)

### Added

- Release-candidate package metadata now identifies the `release_candidate`
  channel and asserts the complete Audio Studio, Linux Host and music-analysis
  capability set during packaging and relocated smoke tests.
- Audio Studio gains `--self-test`, a headless deterministic diagnostic covering
  project JSON, MIDI pitch/theory helpers, WAV round-trip, synthesis rendering and
  pitch/rhythm analysis without opening audio or graphics devices.

### Finalized

- The Linux Host appliance session, XDG application launcher, process/service,
  network/mount, notification and power integrations.
- Audio Studio's PNGTuber, chiptune Composer and live pitch/harmonic/rhythm
  analyzer surfaces, including scale-safe mouse and keyboard controls.
- Portable suite packaging, distribution, documentation manifests, SDK,
  launcher, game-runtime and relocated integration verification for the v15 RC.

This is a release candidate. Existing grid, project and Audio Studio settings
schemas remain backward compatible.

## 14.0.0 (2026-11-19)

### Added

- Audio Studio 2.0 live signal analysis measures level, fundamental pitch,
  MIDI note and cents, FFT harmonics, spectral centroid/brightness, harmonic
  confidence, zero crossings, tonal/percussive balance, onsets, pulse regularity
  and estimated BPM.
- The PNGTuber analyzer can be shown or hidden with its own scaled sidebar
  button and reads only newly arrived microphone samples for rhythm tracking.
- Composer's **Analyze Mix** button measures the deterministic rendered mix used
  for playback and WAV export, showing pitch, harmonic/percussive balance and BPM.
- A reusable `mic_freq::analysis` API supports mono buffers, rendered audio,
  bounded FFT windows and sequential rhythm analysis.

## 13.0.0 (2026-11-19)

### Added

- Selenite Audio Studio 1.0 combines the PNGTuber workflow with a dedicated
  four-track chiptune composer. The scale-responsive Composer uses a 16-step,
  12-pitch piano roll, explicit output-device selection, transport controls,
  waveform/envelope instruments, track mute/volume/pan, and stereo playback.
- Chiptune oscillators for sine, square, triangle, saw and deterministic noise,
  with pluck/pad/default envelopes and 44.1 kHz deterministic rendering.
- Scale selection, chord qualities and inversions, I-IV-V-I progression
  generation, five arpeggiator patterns, key detection and chord analysis.
- JSON composer projects, WAV and OGG Vorbis sample import, standard MIDI file
  import, and 32-bit floating-point stereo WAV export.
- Scaled **Composer** controls are available in Audio Studio and through
  `mic_freq --composer [PROJECT.json]`.

### Changed

- Audio Studio was versioned independently as 1.0.0 and is packaged as a
  first-class music and streaming suite component.
- Selenite remains Linux-native while adding an optional appliance-style host
  layer backed by XDG, DBus, systemd, logind, `/proc` and `/sys`.

## 12.0.0-rc.1 (2026-11-10)

### Added

- Rust/Magnus executable builds with embedded, runtime-verified Ruby bytecode,
  Ruby/Bash bootstrappers, a bundled build SDK, supporting assets and reproducible
  Cargo projects. Supports app/game/project/.rbc inputs and preserves ARGV/exit codes.

- In-app Suite > Change Icon image browser with drag/drop, validated preview,
  Apply/Reset, saved window icons and managed shortcut updates. New shortcuts
  inherit the selected icon; invalid images and refused updates preserve it.

- Direct executable startup with executable-relative library lookup and native
  Ruby/PartitionedArray setup. Shell launchers are optional; shortcuts and Run
  tasks prefer native executables. Relocated tests remove wrappers entirely.

- In-app desktop shortcut creation through Suite > Desktop Shortcuts, organizer
  context menus and Asset Manager, for tools, files/folders, projects and grids.
  Launchers preserve custom icons and protect unmanaged files; Linux desktop
  directory detection and special-character quoting are covered by tests.

- Shared F8 suite navigation and recent project folders.
- F9 scale presets with exact 75%/125% values, smaller default organizer
  buttons, saved toolbar visibility (F10), and click disabling (F7).
- Independent terminal tabs in standalone and IDE terminals, with new/close
  controls, keyboard switching, background output and session cleanup.
- Movable top/bottom IDE dock with persisted position; Ctrl+P project Quick Open.
- Non-executing `--check-project` validation in both runtimes, with Ruby syntax
  checks and missing-asset diagnostics. File List gains a Missing filter.
- Asset Manager search, composable type/missing filters, name/type/location
  sorting, path copying and containing-folder access, with an F8 suite entry.
- Audio Studio 0.2.0 named voice presets through F2 and headless preset commands.

### Fixed

- IDE path replacement and mouse Confirm/Cancel at compact and large sizes.
- Narrow toolbar labels, disabled hit targets and terminal Clear behavior.
- GUI regression process cleanup and focus/input handling on slow displays.
- Distributor regressions now verify preservation of user projects and assets.

This is a release candidate. Existing grid and project schemas are unchanged.

## 11.0.0 (2026-11-04)

### Added

- Automatic inclusion of all source Markdown documentation for Selenite and its
  mic_freq product, including nested guides, with preserved paths and a verified
  SHA-256 documentation manifest. Generated output and temporary copies are excluded.

- The full `package.sh` extraction bundle now builds and ships the `mic_freq`
  PNGTuber Studio executable, adjacent avatar assets, `run-mic-freq.sh`, companion
  version metadata and relocated startup/rendering verification.
- `distribute.sh` copies and extracts the full suite under the current user's
  home directory, with staged extraction, checksums, component validation and
  dry-run support. Existing Selenite installations are replaced completely,
  including old projects and stale files; unrelated directories are protected.

- Game/application projects with validated `selenite-project.json` manifests,
  safe scaffolding (`--new-project DIR [--app]`), project opening/running, File
  menu wizards, configured entry-point execution and project-local save paths.
- Portable `run-ide.sh` launcher with a bundled starter project, package capability
  metadata, and `package.sh --output DIR` for versioned distribution folders.

- Integrated Selenite IDE (`--ide FILE.rb [--grid FILE.json]`, organizer IDE
  button, Ruby-cell menu, and `Selenite.ide(path)`), with File/Edit menus,
  workspace/file browsers, embedded PTY terminal, Ruby console, program output,
  run/stop controls, resizable persisted layout and bundled tutorial Help.
- Complete generated native raylib 6 public API: 600 functions, 35 records and
  305 enum constants through `Selenite.raylib_native`. Managed game bindings
  remain available. Native calls require explicit opt-in and C ownership rules.
- Shared GPU framebuffer rendering for the text editor and IDE, including
  framebuffer-aware clipping and responsive sidebar/dock panes.

### Changed

- The organizer, integrated IDE and dedicated game runtime identify as 11.0.0.
- IDE fonts, controls, dialogs, Help, terminal and pane layouts scale with the
  whole window. Compact dock controls wrap; resizing a pane does not resize text.
- Existing grid saves and profiles remain compatible; no data migration is needed.

## 8.0.0

### Added

- `selenite --version` / `-V`.
- `selenite --with-terminal` starts the grid window and Selenite's terminal
  window together, with an optional profile or grid file.
- A **Run in Terminal** toolbar button runs the selected script cell in
  Selenite's terminal window; the cell menu entry is now **Run in Selenite
  terminal**.
- The organizer remembers its window size and maximized state between sessions
  (`window_size`, `window_maximized` in `settings.json`).

### Fixed

- A large saved Ctrl +/- zoom could make the toolbar fill the whole window, and
  maximizing multiplied it further. The organizer and File List now stop
  zooming once the wrapped toolbar would take over 25% of the window.

### Changed

- First final release of the 7.0.0 release-candidate feature set (text editor,
  native terminal window, Asset Inventory, scaling) as version 8.0.0. Save
  formats are unchanged and remain compatible.

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
- A native raylib terminal window (`selenite --terminal-window`, Linux and
  macOS) replaces launching an external terminal emulator: a pseudo-terminal
  with an xterm-compatible emulator (colours, alternate screen, mouse reporting,
  bracketed paste, scrollback, selection and clipboard, clickable links),
  Ctrl +/- and Ctrl+wheel zoom with the character grid following zoom and window
  size, and syntax highlighting of plain output (Ctrl+Shift+H toggles it).
- New text file here / New text file inside cell actions create an empty
  `untitled.txt` in a cell, on top of a stack, or in a nested grid and edit it.
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

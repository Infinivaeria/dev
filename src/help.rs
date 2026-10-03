//! Tooltip and Help-panel text.

pub const PA_EXPORT_TIP: &str = "PA Export: saves every cell of the grid you're viewing into a \
partitioned_array database (Infinivaeria's Ruby library) in this profile's \
selenite_partitioned_array folder. Replaces the previous export.";
pub const VIEW_3D_TIP: &str = "Toggle the 3D view (F3): the grid becomes a tilted landscape of \
blocks you can rotate (left-drag, Q/E, PgUp/PgDn), pan (right/middle or Shift+drag) and zoom (wheel). \
Click selects, double-click opens, images lie on top of their tiles.";
pub const PA_IMPORT_TIP: &str = "PA Import: loads the cells from this profile's partitioned_array \
export back into the grid you're viewing. Cells that are already occupied are left unchanged.";
pub const LABELS_TIP: &str = "Labels: show or hide coordinates, file names and type badges \
(Image, Audio, Video, etc.) in 2D and 3D (L). Right-click an item to hide or show its labels \
individually; this choice is saved. In stacks, Tab selects the next item. The selected cell \
keeps its coordinates. Labels: Off hides all item labels without resetting individual choices.";
pub const COPY_FILES_TIP: &str =
    "Copy In: when on, files dropped or pasted onto a grid are copied \
into that grid's own assets/ folder in the background (any size, with progress, Esc cancels), so \
the grid keeps working if the originals move. Folders and files already in assets/ are linked, \
never copied. When off, files are linked where they are.";
pub const MUSIC_TIP: &str = "Music: show or hide the built-in music player (wav, ogg, mp3, flac, \
qoa, xm, mod). Double-click an audio cell to play it with the rest of this grid's audio as the \
playlist. Space play/pause, [ ] previous/next, , . seek, 9 0 volume, P playlist.";
pub const PLUGINS_TIP: &str = "Plugins: Ruby plugins that add toolbar-style actions, right-click \
menu entries, hotkeys, timers and hooks. Enable/disable, reload, install the bundled examples or \
open the plugin folder. See docs/plugins.md.";
pub const VERSION_LABEL: &str = concat!("Selenite v", env!("CARGO_PKG_VERSION"));

/// `(heading, [(keys / button, description)])`
pub const HELP_SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Toolbar buttons",
        &[
            ("Profile: NAME", "Switch, create, rename or delete local profiles. Each profile has its own separate set of grids, assets and PA exports."),
            ("Save", "Write this profile's grids to disk (also happens automatically after edits)."),
            ("New Grid", "Create a nested grid in the selected empty cell. Double-click it to go inside."),
            ("Delete", "Remove the selected cell. Non-empty nested grids ask for confirmation."),
            ("Paste", "Paste the clipboard into the selected cell: an image of any size, copied files, file paths, or one URL per line. URLs download in the background with no size limit."),
            ("Copy", "Copy the selected cell's file (and its path) to the system clipboard."),
            ("Open / Run", "Open the selected cell: images in the viewer, Ruby scripts in the embedded VM, music in the built-in player, video and other files in the system player."),
            ("PA Export", PA_EXPORT_TIP),
            ("PA Import", PA_IMPORT_TIP),
            ("Inventory", "Open a list window of every file across all nested grids."),
            ("Grid Folder", "Open this grid's own folder in your file manager (O). Each grid gets a folder the first time it is used; files dropped or pasted onto the grid are copied into its assets/ subfolder, as are pasted images and URL downloads. Right-click a nested grid -> Open its grid folder."),
            ("Undo / Redo", "Step back or forward through edits: paste, drop, move, delete, new grid, downloads and Ruby changes (Ctrl+Z / Ctrl+Y). Keeps the last 200 steps per session."),
            ("Find", "Search every grid of this profile by file name, path or kind (image, audio, video, ruby, file, grid). Enter/Down = next, Shift+Enter/Up = previous, Esc closes (Ctrl+F)."),
            ("Ruby", "Toggle the embedded Ruby console (same as the ` key)."),
            ("Home", "Reset pan/zoom and the 3D camera to the origin (same as the Home key)."),
            ("3D View / 2D View", VIEW_3D_TIP),
            ("Labels: On / Off", LABELS_TIP),
            ("Copy In: On / Off", COPY_FILES_TIP),
            ("Music / Hide Music", MUSIC_TIP),
            ("Plugins (n)", PLUGINS_TIP),
            ("Help", "Show this panel (F1)."),
            ("Cancel DL (n)", "Appears while downloads run; cancels all of them (same as Esc). Progress shows on the target cells and in the status bar."),
            ("Back", "Leave the current nested grid (same as Backspace)."),
        ],
    ),
    (
        "Mouse & keyboard",
        &[
            ("Click / double-click", "Select a cell / open it."),
            ("Right-click a cell", "Context menu: open, run as game / export standalone game (Ruby), add to playlist (audio), stack actions, info, copy file or path, show in folder, paste, new grid, grid inventory, delete, plus any plugin menu entries (marked »)."),
            ("Drag a cell", "Move (or swap) its contents to another cell."),
            ("Middle drag, wheel", "Pan and zoom the grid."),
            ("Home", "Reset pan and zoom."),
            ("F3", "Toggle the 3D view."),
            ("3D: left-drag / Q, E / PgUp, PgDn", "Rotate around the grid, turn 15°, tilt."),
            ("3D: right or middle drag, Shift+drag", "Pan across the grid. Wheel zooms; arrow keys glide the camera to the selection."),
            ("3D: W A S D", "Fly across the grid. Right-click without dragging opens the cell menu."),
            ("Ctrl+Z / Ctrl+Y (Ctrl+Shift+Z)", "Undo / redo."),
            ("Ctrl+F", "Find cells anywhere in the profile; matches are outlined in cyan."),
            ("I / M", "Toggle the cell info panel / the minimap (click or drag the minimap to jump)."),
            ("L", "Toggle grid labels (coordinates, file names and type badges)."),
            ("Space / P", "Music: play-pause (plays this grid's audio when idle) / show the playlist."),
            ("[ ] , . 9 0", "Music: previous / next track, seek -5 s / +5 s, volume -5% / +5%."),
            ("F2, F4-F12", "Plugin buttons that declare a key: run on the selected cell."),
            ("Arrow keys", "Move the selection."),
            ("Enter", "Open / run the selected cell."),
            ("Ctrl+V / Ctrl+C", "Paste into / copy from the selected cell."),
            ("Ctrl+Shift+V", "Paste onto the selected cell as a stack (adds to what is already there)."),
            ("Ctrl+A / Ctrl+X", "Select every occupied cell / cut the selection (delete it and copy its files)."),
            ("Ctrl+click, Shift+click", "Add or remove one cell from the multi-selection / select a rectangle. Dragging from an empty cell draws a selection box."),
            ("Drag a selected cell", "Moves the whole multi-selection together (blocked cells are reported, nothing is lost)."),
            ("Shift+drag a cell", "Stack its items on top of the target cell instead of swapping."),
            ("Drop on a file / Shift+drop", "Dropping onto an occupied file cell stacks the files there; Shift forces a stack anywhere."),
            ("Tab / Shift+Tab", "Cycle which item of a stacked cell is on top."),
            ("Shift+Delete", "Remove only the top item of a stack."),
            ("B", "Open this grid's inventory panel (game-style items with counts and metadata)."),
            ("O", "Open this grid's folder (created on first use) in the file manager."),
            ("Z", "Zoom to fit every cell of the grid (2D)."),
            ("Ctrl+S", "Save."),
            ("Delete, Backspace", "Delete selection, go back out of a nested grid."),
            ("Drop files", "Drop any files from your file manager onto a cell. With Copy In on they are copied into the grid's assets/ folder (progress on the cell)."),
            ("Ctrl + / - / 0", "Scale all text and buttons."),
            ("F1, Esc", "Toggle this help, close dialogs. Esc also cancels downloads."),
            ("`", "Ruby console: Up/Down history, Ctrl+V paste, PageUp/PageDown scroll, Ctrl+L clear."),
        ],
    ),
    (
        "Ruby (Magnus)",
        &[
            ("Selenite.grid", "Current grid object: g[col,row], g.set(col,row,path), g.remove, g.subgrid, g.new_grid, g.move, g.swap, g.each, g.cells, g.size, g.clear."),
            ("Selenite.root", "Top-level grid of the current profile."),
            ("Selenite.on(:event) { }", "Hooks: :activate (col,row,kind,path; return :handled to skip the default), :paste, :drop, :download, :save, :profile, :track (path, index, count)."),
            ("Selenite.labels(bool) / labels?", "Turn grid labels on or off from Ruby."),
            ("Selenite.music", "play(path), play_grid, play_list(paths), queue(path), toggle, pause, resume, stop, next, previous, seek(s), volume(0..1), shuffle(bool), repeat(:off/:all/:one), state, playing?, track, show, hide, clear."),
            ("Rust helpers", "file_info(path), image_size(path), checksum(path) (fast, streaming), playable?(path), copy_text(text) — implemented in Rust, called from Ruby."),
            ("Selenite.select / open / goto", "Move the selection, open a cell, or center the camera on a cell."),
            ("Selenite.view_3d(bool) / orbit(yaw, pitch, dist)", "Switch the 3D view on/off and set its camera angles in degrees."),
            ("Selenite.undo / redo / history", "Undo or redo edits; history returns [next undo label, next redo label]."),
            ("Selenite.search(q) / find(q)", "search returns [[nested_path, col, row, name], ...] for the whole profile; find opens the Find bar on q."),
            ("Selenite.download(url, col, row)", "Background download into a cell (no size limit)."),
            ("Selenite.grid_folder / g.folder / Selenite.grids_dir", "This grid's (or any grid's) own folder, created on first use; grids_dir is the folder holding every grid folder. g.folder_id is nil until used."),
            ("Selenite.assets_dir / g.assets_dir", "The grid's assets/ subfolder, where added files are copied."),
            ("Selenite.import(col,row,path) / g.import(col,row,path)", "Copy a file into the grid's assets/ folder and put it on the cell (stacks onto existing items). Returns the copy's path."),
            ("Selenite.status(msg)", "Show a message in the status bar."),
            ("Selenite.profile / profiles / switch_profile / create_profile", "Inspect and change local profiles."),
            ("grid_set, grid_set_file, grid_get, grid_list, grid_remove, grid_new_grid, grid_save", "Classic global helpers."),
            ("init.rb", "A profile's init.rb runs automatically when the profile opens."),
            ("selenite --script FILE.rb", "Run a script headlessly against the current profile."),
            ("Stacks", "g.items(c,r) lists every item of a cell, g.push(c,r,path) / g.push_grid add on top, g.pop, g.cycle, g.raise_item, g.merge, g.unstack, g.stacked?, g.stack_size; Selenite.selection / select_cells for multi-select."),
            ("g.inventory", "Game inventory of a grid: add(name, n), remove, [], []=, has?, rename, meta, set_meta, transfer(other, name, n), total, names, each, to_h. Selenite.inventory is the current grid's."),
        ],
    ),
    (
        "Games (raylib-bindings style)",
        &[
            ("selenite --game FILE.rb", "Run a Ruby game in its own window with the current profile's grids available. Add --profile NAME or --grid FILE."),
            ("Run as game / Export standalone game", "Right-click a .rb cell. Export writes <name>-game/ with the script, a save snapshot, assets/, the runtime and run-game.sh."),
            ("include Raylib", "require \"raylib\" works as in the raylib-bindings gem: InitWindow, BeginDrawing, DrawRectangle, DrawText, IsKeyDown, LoadTexture, PlaySound, BeginMode2D, CheckCollisionRecs... (snake_case aliases too)."),
            ("Selenite::Game.new(title:, width:, height:) { |g| }", "Game loop helper: g.update { |dt| }, g.draw { }, g.draw_ui { }, g.every(s) { }, g.axis, g.key?, g.pressed?, g.texture, g.cell_texture(c,r), g.grid, g.inventory, g.run."),
            ("Docs", "docs/game.md and docs/inventory.md; examples/games has pong.rb and collector.rb."),
        ],
    ),
    (
        "Plugins (Ruby <-> Rust)",
        &[
            ("Where", "Every *.rb in <data>/plugins (all profiles) and <profile>/plugins loads at startup and on profile switch. Use the Plugins button to enable/disable, reload or install the examples."),
            ("Selenite.plugin(\"Name\") { |p| ... }", "Declare a plugin. p.description, p.version set the info shown in the Plugins panel."),
            ("p.button(label, key: \"F6\") { |cell| }", "An action listed in the Plugins panel (and on its hotkey); receives the selected cell or nil."),
            ("p.menu(label, kinds: %w[audio]) { |cell| }", "A right-click menu entry; kinds filters by cell kind (image, audio, video, ruby, file, grid, empty)."),
            ("p.command(name) { |*args| }", "Callable from the console or other plugins: Selenite.run(name, *args); Selenite.commands lists them."),
            ("p.on(:event) { }, p.every(seconds) { }", "Hooks and repeating timers that are removed when plugins reload. Errors stop the timer and show in the status bar."),
            ("Undo", "Every plugin action is one undoable step named after the plugin."),
            ("Docs", "docs/plugins.md has the full API with worked examples; examples/plugins has five ready-made plugins."),
        ],
    ),
    (
        "partitioned_array (PA)",
        &[
            ("What", "partitioned_array is a Ruby database library by Infinivaeria, bundled with Selenite and loaded inside the embedded Ruby VM."),
            ("PA Export", PA_EXPORT_TIP),
            ("PA Import", PA_IMPORT_TIP),
            ("From Ruby", "grid_pa_export(dir), grid_pa_import(dir), grid_pa_records(dir), grid_pa_available."),
        ],
    ),
];

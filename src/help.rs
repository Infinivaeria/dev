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
keeps its coordinates. Files without a visible thumbnail (including music) always keep their \
labels so they cannot disappear. Labels: Off does not reset individual choices.";
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
        "Keyboard: General",
        &[
            ("F1", "Open Help from the grid; F1 or Esc closes Help. Modal panels capture their own keys."),
            ("Ctrl+= / Ctrl+- / Ctrl+0", "Increase, decrease or reset interface text and button scale. Keypad +, - and 0 work too; scaling remains available in panels. The zoom is remembered between launches, and windows also scale with their size when maximized."),
            ("Ctrl / Super; Shift", "Grid command shortcuts accept left/right Ctrl or Super (Windows/Command). Text scaling, Ruby console paste/clear and image-viewer zoom modifiers use Ctrl. Either Shift key works."),
            ("Text entry / Backspace", "Type into an active search or input field; Backspace deletes the last character. Context-specific bindings below replace normal grid commands."),
            ("Games and custom plugins", "Game windows and plugin scripts may define additional controls. Those bindings belong to the running game/plugin; the sections below list Selenite's built-in shortcuts and assignable plugin keys."),
        ],
    ),
    (
        "Keyboard: Grid",
        &[
            ("Arrow keys", "Move the selected cell, repeating while held, and keep it visible. With no selection, select (0,0)."),
            ("Enter / Keypad Enter", "Open/run the selected file or enter its child grid. Empty cells create a child grid on demand."),
            ("Backspace", "Return to the parent nested grid when no text field or modal panel is active."),
            ("Home", "Reset the grid view and 3D camera to the origin."),
            ("Z", "In 2D, fit the multi-selection; otherwise fit every occupied cell."),
            ("F3", "Switch between 2D and 3D views."),
            ("I / M", "Toggle cell information / the minimap."),
            ("L", "Toggle labels. Items without visible thumbnails still retain identifying labels."),
            ("B", "Open the current grid's named game-item inventory, separate from Stack Items."),
            ("O", "Open the current grid's folder in the file manager."),
            ("Ctrl+S", "Save the current profile/grid."),
            ("Ctrl+Z", "Undo the last grid edit."),
            ("Ctrl+Y / Ctrl+Shift+Z", "Redo the last undone grid edit."),
            ("Ctrl+C / Ctrl+X", "Copy selected files / cut the selected cells and copy their files."),
            ("Ctrl+V", "Paste clipboard files, image, paths or URLs at the selected cell."),
            ("Ctrl+Shift+V", "Add clipboard items to the selected cell's stack."),
            ("Ctrl+A", "Select every occupied cell in the current grid."),
            ("Ctrl+F", "Open or close Find for this profile's grid hierarchy."),
            ("Tab / Shift+Tab", "Bring the next / previous item of the selected stack to the top."),
            ("Delete", "Remove selected cells and their stacks; non-empty child grids require confirmation."),
            ("Shift+Delete", "Remove only the top item of the selected stack; a single item uses normal deletion."),
            ("Esc", "Cancel active downloads; otherwise clear the selection. If a menu or modal is active, cancel/close that instead."),
            ("` (Backtick)", "Toggle the Ruby console when another modal panel is not active."),
        ],
    ),
    (
        "Keyboard: 3D Camera",
        &[
            ("Q / E", "Rotate the camera left / right in 15-degree steps; repeats while held."),
            ("PageUp / PageDown", "Tilt the camera up / down; repeats while held."),
            ("W / A / S / D", "Move the camera across the ground while held, outside text-entry mode."),
            ("Shift + drag", "Pan the 3D camera rather than rotating it. Right/middle drag also pans."),
            ("Arrow keys / Home / F3", "Grid selection follows the camera; Home resets it; F3 returns to 2D."),
        ],
    ),
    (
        "Keyboard: Find",
        &[
            ("Text / Backspace", "Edit the search query; names, paths and item kinds are searched across nested grids."),
            ("Ctrl+V", "Paste clipboard text into the query."),
            ("Enter / Keypad Enter / Down", "Visit the next search match."),
            ("Shift+Enter / Shift+Keypad Enter / Up", "Visit the previous search match."),
            ("Esc / Ctrl+F", "Close Find."),
        ],
    ),
    (
        "Keyboard: Stack Items",
        &[
            ("Text / Backspace", "Filter item names case-insensitively / edit the filter."),
            ("Up / Down", "Focus the previous / next matching item and scroll it into view."),
            ("Home / End", "Focus the first / last matching item."),
            ("Enter", "Open the focused item. During nested-grid removal confirmation, confirm removal instead."),
            ("Delete", "Remove the focused item; non-empty nested grids require confirmation."),
            ("Ctrl+V / Ctrl+Shift+V", "Paste clipboard items into this cell's stack."),
            ("Ctrl+Z", "Undo without closing the panel; cancels any pending removal prompt."),
            ("Ctrl+Y / Ctrl+Shift+Z", "Redo without closing the panel; cancels any pending removal prompt."),
            ("Esc", "Cancel a removal prompt; otherwise clear the filter; if already clear, close the panel."),
        ],
    ),
    (
        "Keyboard: Asset Inventory",
        &[
            ("Text / Backspace", "Search this grid's asset names and coordinates / edit the query."),
            ("Up / Down / Home / End", "Focus the previous, next, first or last matching asset."),
            ("Enter / Delete", "Open / remove the focused asset. Non-empty child-grid removal opens a confirmation prompt."),
            ("Ctrl+V / Ctrl+Shift+V", "Add clipboard items to the focused cell's stack, or a new asset cell when no row is focused."),
            ("Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z", "Undo / redo without closing the inventory."),
            ("Esc", "Cancel active transfers; otherwise clear the query; otherwise close the inventory."),
            ("F1", "Open Help without closing the inventory."),
        ],
    ),
    (
        "Keyboard: Game-Item Inventory",
        &[
            ("Text / Backspace", "Edit the inventory command: name, name x5, name = 5, old -> new or name @key=value."),
            ("Enter / Keypad Enter", "Apply the typed inventory command."),
            ("Ctrl+Z", "Undo the last edit and reset the inventory panel's input/selection."),
            ("Esc", "Close the inventory panel. Other quantity, selection and deletion controls are mouse buttons."),
        ],
    ),
    (
        "Keyboard: Music",
        &[
            ("Space", "Toggle play/pause; when idle, start this grid's audio playlist."),
            ("P", "Show/hide the music playlist."),
            ("[ / ]", "Play the previous / next track."),
            (", / .", "Seek backward / forward five seconds; repeats while held."),
            ("9 / 0", "Decrease / increase volume by five percentage points; repeats while held. Ctrl+0 resets UI scale instead."),
        ],
    ),
    (
        "Keyboard: Ruby Console",
        &[
            ("Text / Backspace", "Edit the Ruby expression."),
            ("Enter / Keypad Enter", "Evaluate the expression."),
            ("Up / Down", "Recall older / newer submitted commands; Down past the newest clears input."),
            ("Ctrl+V", "Paste text; non-empty lines are joined with semicolons."),
            ("Ctrl+U", "Clear the current input."),
            ("Ctrl+L", "Clear console output and reset its scroll position."),
            ("PageUp / PageDown", "Scroll console output up / down one page."),
            ("Esc / ` (Backtick)", "Close the console."),
        ],
    ),
    (
        "Keyboard: Profiles",
        &[
            ("N", "Start creating a profile when no input or deletion prompt is active."),
            ("Text / Backspace", "Edit the profile name during creation or rename."),
            ("Enter / Keypad Enter", "Submit a create/rename name."),
            ("Esc", "Cancel create/rename input; otherwise close the profile panel."),
            ("Enter / Y", "Confirm profile deletion while its confirmation prompt is active."),
            ("Esc / N", "Cancel profile deletion while its confirmation prompt is active."),
        ],
    ),
    (
        "Keyboard: Menus and Plugins",
        &[
            ("Esc", "Close the cell context menu or Plugins panel."),
            ("Up / Down", "Scroll the Plugins panel; repeats while held. Cell-menu actions are selected with the mouse."),
            ("F2, F4, F5, F6, F7, F8, F9, F10, F11, F12", "Run enabled plugin actions assigned to these keys from the grid. F1 and F3 are reserved. Unassigned keys have no action; plugin behavior depends on installed plugins."),
        ],
    ),
    (
        "Keyboard: Grid Deletion Prompt",
        &[
            ("Enter / Y", "Confirm deletion of the selected non-empty child grids."),
            ("Esc / N", "Cancel deletion."),
        ],
    ),
    (
        "Keyboard: Image Viewer",
        &[
            ("= / + / Keypad +", "Zoom in around the window center without Ctrl."),
            ("- / Keypad -", "Zoom out around the window center without Ctrl."),
            ("R / 1", "Reset pan and zoom to 100 percent (one image pixel per screen pixel)."),
            ("F", "Fit the image to the window."),
            ("Ctrl+= / Ctrl+- / Ctrl+0", "Adjust/reset interface scale; keypad equivalents work too."),
            ("Esc", "Close the image viewer."),
        ],
    ),
    (
        "Keyboard: File List Window",
        &[
            ("Up / Down", "Select the previous / next list item. Keyboard selection does not automatically scroll the list."),
            ("Enter", "Open the selected file using its viewer or system association."),
            ("Ctrl+= / Ctrl+- / Ctrl+0", "Adjust/reset interface scale; keypad equivalents work too."),
            ("Esc", "Close this window. Refresh and scrolling use the mouse; the organizer's editing shortcuts do not apply."),
        ],
    ),
    (
        "Keyboard: Help",
        &[
            ("Up / Down", "Scroll this reference; repeats while held."),
            ("PageUp / PageDown", "Scroll almost one page up / down."),
            ("Home / End", "Jump to the beginning / end."),
            ("F1 / Esc", "Close Help."),
        ],
    ),
    (
        "Toolbar buttons",
        &[
            ("Profile: NAME", "Switch, create, rename or delete local profiles. Each profile has its own separate set of grids, assets and PA exports."),
            ("Save", "Write this profile's grids to disk (also happens automatically after edits)."),
            ("New Grid", "Create a nested grid in the selected empty cell. Double-click it to go inside."),
            ("Delete", "Remove the selected cell. Non-empty nested grids ask for confirmation."),
            ("Paste", "Paste the clipboard into the selected cell: an image of any size, copied files, file paths, or one URL per line. URLs download in the background with no size limit."),
            ("Copy", "Copy the selected cell's file (and its path) to the system clipboard."),
            ("Open / Run", "Enter the selected child grid, including any empty cell's implicit grid. Files open in their viewer/player; Ruby scripts run in the embedded VM."),
            ("PA Export", PA_EXPORT_TIP),
            ("PA Import", PA_IMPORT_TIP),
            ("Inventory", "Open a list window of every file across all nested grids."),
            ("Asset Inventory", "Live asset management for the current grid. Drop multiple files on a row to add to that cell's stack, or on blank panel space to create a new stacked asset cell. Search, open, promote, extract, remove, paste and unstack; transfers show progress and changes save with undo/redo."),
            ("Stack Items", "Search this cell's files and grids. Arrows/Home/End focus; Enter opens; Delete removes; Ctrl+V stacks clipboard items; Ctrl+Z/Ctrl+Y undo/redo. Row controls bring to top, open, extract into a free cell or remove. Non-empty nested grids require confirmation."),
            ("Grid Folder", "Open this grid's own folder in your file manager (O). Each grid gets a folder the first time it is used; files dropped or pasted onto the grid are copied into its assets/ subfolder, as are pasted images and URL downloads. Right-click a nested grid -> Open its grid folder."),
            ("Undo / Redo", "Step back or forward through edits: paste, drop, move, delete, new grid, downloads and Ruby changes (Ctrl+Z / Ctrl+Y). Keeps the last 200 steps per session."),
            ("Find", "Search every grid of this profile by file name, path or kind (image, audio, video, ruby, file, grid). Enter/Down = next, Shift+Enter/Up = previous, Esc closes (Ctrl+F)."),
            ("Ruby", "Toggle the embedded Ruby console (same as the ` key)."),
            ("Terminal", "Open a separate system terminal running your shell."),
            ("Notes", "Edit this grid's Markdown notes (notes.md in its grid folder) in the text editor, which highlights Ruby and previews Markdown. Also: right-click a cell for Edit text, New note here, or Grid notes; Enter on a .md or .txt cell opens it too."),
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
        "Mouse and Modifiers",
        &[
            ("Click / double-click", "Select a cell / open it."),
            ("Right-click a cell", "Context menu: open, run file or Ruby commands in a terminal, open a terminal in the grid folder, run as game / export standalone game (Ruby), add to playlist (audio), stack actions, info, copy file or path, show in folder, paste, new grid, grid inventory, delete, plus any plugin menu entries (marked »)."),
            ("Drag a cell", "Move (or swap) its contents to another cell."),
            ("Middle drag, wheel", "Pan and zoom the grid."),
            ("3D: left-drag", "Rotate around the grid."),
            ("3D: right or middle drag, Shift+drag", "Pan across the grid. Wheel zooms; arrow keys glide the camera to the selection."),
            ("Ctrl+click, Shift+click", "Add or remove one cell from the multi-selection / select a rectangle. Dragging from an empty cell draws a selection box."),
            ("Drag a selected cell", "Moves the whole multi-selection together (blocked cells are reported, nothing is lost)."),
            ("Shift+drag a cell", "Stack its items on top of the target cell instead of swapping."),
            ("Drop on a file / Shift+drop", "Dropping onto an occupied file cell stacks the files there; Shift forces a stack anywhere."),
            ("Drop files", "Drop any files from your file manager onto a cell. With Copy In on they are copied into the grid's assets/ folder (progress on the cell)."),
            ("Image viewer: drag / wheel", "Drag with any mouse button to pan; wheel zooms around the pointer."),
            ("Panels: wheel", "Scroll Help, Plugins, profiles, Stack Items, game-item inventory or the separate file list."),
        ],
    ),
    (
        "Ruby (Magnus)",
        &[
            ("Selenite.grid", "Current grid object: g[col,row], g.set(col,row,path), g.remove, g.subgrid, g.new_grid, g.move, g.swap, g.each, g.cells, g.size, g.clear."),
            ("Selenite.root", "Top-level grid of the current profile."),
            ("Selenite.assets / g.assets / Selenite.asset_inventory", "Enumerate all assets in the current grid or a specific grid, including stack contents; asset_inventory opens the current grid's live organizer manager."),
            ("Selenite.on(:event) { }", "Hooks: :activate (col,row,kind,path; return :handled to skip the default), :paste, :drop, :download, :save, :profile, :track (path, index, count)."),
            ("Selenite.labels(bool) / labels?", "Turn grid labels on or off from Ruby."),
            ("Selenite.terminal(file = nil, *args, dir: nil)", "Open a terminal window; with a file it runs that command file with the arguments, then leaves a shell open. dir sets the starting folder. The terminal gets SELENITE_GRID, SELENITE_PROFILE and SELENITE_EXE."),
            ("Selenite.edit(path)", "Open a file in the text editor window (Ruby and Markdown aware; F11 maximizes, Ctrl+P toggles the Markdown preview, Ctrl+F / Ctrl+H find and replace, preview links are clickable)."),
            ("Selenite.music", "play(path), play_grid, play_list(paths), queue(path), toggle, pause, resume, stop, next, previous, seek(s), volume(0..1), shuffle(bool), repeat(:off/:all/:one), state, playing?, track, show, hide, clear."),
            ("Rust helpers", "file_info(path), image_size(path), checksum(path) (fast, streaming), playable?(path), copy_text(text) — implemented in Rust, called from Ruby."),
            ("Selenite.select / open / goto", "Move the selection, open a cell (empty cells enter their child grid), or center the camera on a cell."),
            ("Selenite.enter(col,row) / back", "Enter a child grid at any coordinate, creating it on demand if empty; return to its parent. Every child has the same unbounded coordinate space."),
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
            ("selenite --script FILE.rb|FILE.rbc", "Run a Ruby source or bytecode script headlessly against the current profile."),
            ("selenite --compile FILE.rb [--output FILE.rbc] [--force]", "Compile without executing the source. Use the same Ruby runtime for compilation and distribution. Existing outputs require --force; bytecode is not encryption."),
            ("Selenite::Bytecode.load / require", "Execute a compiled helper each time / once per resolved path. grid_eval_file accepts both .rb and .rbc."),
            ("Selenite::Network", "Direct TCP peers/meshes, host/join sessions, authenticated JSON messages, replicated state, LAN discovery and shared grids. See docs/networking.md."),
            ("Stacks", "g.items(c,r) lists every item of a cell, g.push(c,r,path) / g.push_grid add on top, g.pop, g.cycle, g.raise_item, g.merge, g.unstack, g.stacked?, g.stack_size; Selenite.selection / select_cells for multi-select."),
            ("g.inventory", "Game inventory of a grid: add(name, n), remove, [], []=, has?, rename, meta, set_meta, transfer(other, name, n), total, names, each, to_h. Selenite.inventory is the current grid's."),
        ],
    ),
    (
        "Games (raylib-bindings style)",
        &[
            ("selenite --game FILE.rb|FILE.rbc", "Run a Ruby game in its own window with the current profile's grids available. Add --profile NAME or --grid FILE."),
            ("Run as game / Export standalone game", "Right-click a .rb or .rbc cell. Export writes <name>-game/ with the entry, a save snapshot, assets/, game-data/, the runtime and run-game.sh."),
            ("include Raylib", "require \"raylib\" works as in the raylib-bindings gem: InitWindow, BeginDrawing, DrawRectangle, DrawText, IsKeyDown, LoadTexture, PlaySound, BeginMode2D, CheckCollisionRecs... (snake_case aliases too)."),
            ("Selenite::Game.new(title:, width:, height:) { |g| }", "Game loop helper: g.update { |dt| }, g.draw { }, g.draw_ui { }, g.every(s) { }, g.axis, g.key?, g.pressed?, g.texture, g.cell_texture(c,r), g.grid, g.inventory, g.run."),
            ("game.host / join / peer", "Game-owned multiplayer sessions or direct peers; polled before updates and closed on exit. Cooperative Gems: examples/games/multiplayer.rb."),
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
            ("p.on_unload { }", "Release sockets and other resources on plugin reset/reload/disable or profile switch."),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_reference_covers_every_native_context() {
        let expected = [
            ("Keyboard: General", "Ctrl+= / Ctrl+- / Ctrl+0"),
            ("Keyboard: Grid", "Shift+Delete"),
            ("Keyboard: 3D Camera", "W / A / S / D"),
            ("Keyboard: Find", "Shift+Enter / Shift+Keypad Enter / Up"),
            ("Keyboard: Stack Items", "Ctrl+Y / Ctrl+Shift+Z"),
            ("Keyboard: Game-Item Inventory", "Enter / Keypad Enter"),
            ("Keyboard: Asset Inventory", "F1"),
            ("Keyboard: Music", "9 / 0"),
            ("Keyboard: Ruby Console", "Ctrl+U"),
            ("Keyboard: Profiles", "Esc / N"),
            (
                "Keyboard: Menus and Plugins",
                "F2, F4, F5, F6, F7, F8, F9, F10, F11, F12",
            ),
            ("Keyboard: Grid Deletion Prompt", "Enter / Y"),
            ("Keyboard: Image Viewer", "R / 1"),
            ("Keyboard: File List Window", "Enter"),
            ("Keyboard: Help", "Home / End"),
        ];
        assert_eq!(HELP_SECTIONS[0].0, "Keyboard: General");
        for (heading, shortcut) in expected {
            let (_, items) = HELP_SECTIONS
                .iter()
                .find(|(name, _)| *name == heading)
                .unwrap_or_else(|| panic!("missing keyboard context: {heading}"));
            assert!(
                items.iter().any(|(keys, _)| *keys == shortcut),
                "missing {shortcut} in {heading}"
            );
        }
        let mut headings = std::collections::HashSet::new();
        for (heading, items) in HELP_SECTIONS {
            assert!(headings.insert(heading), "duplicate heading: {heading}");
            assert!(!items.is_empty());
            for (keys, description) in *items {
                assert!(!keys.trim().is_empty() && !description.trim().is_empty());
            }
        }
    }

    #[test]
    fn complete_help_reference_is_scrollable_at_supported_window_sizes() {
        for (width, height) in [(320, 240), (640, 480), (1280, 720), (3840, 2160)] {
            let layout = crate::panels::help_layout(width, height);
            assert!(layout.body.width > 0.0 && layout.body.height > 0.0);
            assert!(layout.frame.x >= 0.0 && layout.frame.y >= 0.0);
            assert!(layout.frame.x + layout.frame.width <= width as f32);
            assert!(layout.frame.y + layout.frame.height <= height as f32);
            let max_scroll = crate::panels::help_max_scroll(&layout);
            assert!(max_scroll.is_finite() && max_scroll > 0.0);
        }
    }
}

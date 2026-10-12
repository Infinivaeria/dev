# Selenite instruction manual

For **Selenite 3.0.0-rc.4**, as implemented in this checkout. This manual covers the desktop application, its command-line modes, local data, Ruby automation, plugins, games, and the optional mic_freq companion. Some features require a build with Ruby scripting enabled.

## Contents

1. [What Selenite stores](#1-what-selenite-stores)
2. [Install and start](#2-install-and-start)
3. [First session walkthrough](#3-first-session-walkthrough)
4. [Window layout and navigation](#4-window-layout-and-navigation)
5. [Add files, images, and downloads](#5-add-files-images-and-downloads)
6. [Select, move, copy, and delete](#6-select-move-copy-and-delete)
7. [Stacks](#7-stacks)
8. [Nested grids](#8-nested-grids)
9. [Find, information, labels, and minimap](#9-find-information-labels-and-minimap)
10. [3D view](#10-3d-view)
11. [Images and the image viewer](#11-images-and-the-image-viewer)
12. [Music, video, and other files](#12-music-video-and-other-files)
13. [File Inventory window](#13-file-inventory-window)
14. [Game-item inventories](#14-game-item-inventories)
15. [Profiles and preferences](#15-profiles-and-preferences)
16. [Saving, backups, and moving data](#16-saving-backups-and-moving-data)
17. [Toolbar and context-menu reference](#17-toolbar-and-context-menu-reference)
18. [Keyboard reference](#18-keyboard-reference)
19. [Command-line and environment reference](#19-command-line-and-environment-reference)
20. [Ruby console and automation](#20-ruby-console-and-automation)
21. [Plugins](#21-plugins)
22. [Games and standalone exports](#22-games-and-standalone-exports)
23. [Partitioned array import and export](#23-partitioned-array-import-and-export)
24. [mic_freq alarms, timers, and stopwatch](#24-mic_freq-alarms-timers-and-stopwatch)
25. [Troubleshooting and limits](#25-troubleshooting-and-limits)
26. [Save-format reference](#26-save-format-reference)
27. [Maintenance and further references](#27-maintenance-and-further-references)

## 1. What Selenite stores

Selenite is a desktop organizer built around an effectively unbounded, sparse grid. A position is written **(column, row)**, for example `(3, -2)`. Columns increase to the right and rows downward in 2D. Negative coordinates are valid. Only occupied cells need storage.

| Term | Meaning |
| --- | --- |
| Cell | One coordinate in a grid. It can be empty or hold a stack. |
| Item | One file reference or nested grid inside a cell's stack. |
| Stack | Multiple items at one coordinate. The top item is displayed and activated. |
| Nested grid | A grid held as an item in another grid. Nesting can continue recursively. |
| Root | The outermost grid of the current save or profile. |
| Profile | A local collection of grids and associated files, separate from other profiles. |
| Grid folder | A filesystem folder assigned to a grid, with an `assets/` subfolder. |
| Inventory window | A file browser listing attached items throughout the saved hierarchy. |
| Items panel | The current grid's game inventory: names, quantities, and metadata. |

Files remain ordinary files on disk. By default, adding a file copies it into the grid's assets folder; turning **Copy In** off stores a link to its existing location. A nested grid is Selenite data, not a recursively imported filesystem folder. Adding a directory links that directory; it does not automatically turn its contents into grid cells.

Profiles are local organizational units, without passwords or cloud synchronization. Opening a Ruby item, game, or plugin executes Ruby code with the application's filesystem and process permissions. Use scripts you intend to run.

## 2. Install and start

### Portable Linux distribution

Extract the portable archive into a folder you can write to, open a terminal in that folder, and run:

```sh
./run.sh
```

Use the launcher so bundled Ruby libraries are found. Keep the accompanying `lib/`, `partitioned_array/`, and other runtime folders beside the executable. The bundle still relies on a compatible Linux system, display server, graphics driver, and system libraries; it is not an operating-system-independent executable.

For portable application data as well as a portable runtime:

```sh
SELENITE_HOME="$PWD/data" ./run.sh
```

Use the same data location on later launches. Changing `SELENITE_HOME` changes which profiles and preferences you see.

### Build from this repository

Requirements: Rust **1.88 or newer**, a C/C++ build toolchain, CMake, and raylib's native dependencies. On Debian/Ubuntu, a typical dependency installation is:

```sh
sudo apt install build-essential cmake clang pkg-config libasound2-dev libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev
cargo build --release
./target/release/selenite
```

The default build supports the organizer and media features. Ruby console execution, scripts, plugins, games, and PA operations require the `scripting` feature and a Ruby development environment with headers and `libruby` (Ruby 3.1 or newer):

```sh
cargo build --release --features scripting
./target/release/selenite
```

On distributions providing a compatible Ruby, `ruby-dev` supplies the development package. The Magnus dependency is pinned to a Git revision; the first scripting build needs access to GitHub as well as the crate registry.

`./package.sh` builds a scripting-enabled Linux distribution and copies this manual under `dist/docs/`. It **replaces the existing `dist/` directory**; keep personal work outside that build-output folder. Windows and macOS have platform-specific data paths, but the supplied packaging script and launchers are Linux-oriented.

## 3. First session walkthrough

1. Start Selenite. The first run opens the `default` profile; later launches reopen the last-used profile.
2. Click **Profile: default**, then create a profile such as `Practice`. Alternatively, launch `selenite --profile Practice`.
3. Drop a few images onto empty cells below the toolbar. Wait for any copy progress to finish. Leave **Copy In: On** enabled to keep managed copies.
4. Click a cell to select it. Double-click it, press **Enter**, or click **Open / Run** to open its top item.
5. Drag with the middle mouse button to pan. Scroll to zoom. Press **Z** in 2D to fit the occupied grid, or **Home** to reset the camera to the origin.
6. Select an empty cell and click **New Grid**. Open that cell, add another file inside, and press **Backspace** to return.
7. Drop two files onto one occupied file cell. Its stack badge shows the number of items. Select it and press **Tab** to change the top item.
8. Press **B**, enter `gold x10`, and press **Enter**. This adds a game-inventory quantity, not a file. Close the panel.
9. Press **Ctrl+F** and search for a filename. Press **Enter** to visit matches; **Esc** closes Find.
10. Press **Ctrl+S**. Edits are normally saved automatically as well. Close the main window using its window-close button, then reopen the profile to see your saved grids.

The main grid's **Esc** key cancels or closes the current interaction; it is not the main-window quit command.

## 4. Window layout and navigation

The toolbar contains profile, editing, viewing, media, and scripting actions. Buttons wrap onto additional rows in narrower windows. Hover a button for its tooltip. Buttons requiring a selection may be disabled until a suitable cell is selected.

The grid occupies the area below the toolbar. Nested-grid navigation shows a breadcrumb such as `Root > (2,0) > (1,1)`. The status area reports actions, errors, and transfer progress; normal status messages fade after about eight seconds. Music controls appear at the bottom when shown.

| Action | Control |
| --- | --- |
| Select a cell | Left-click, including an empty cell |
| Move the selected coordinate | Arrow keys |
| Activate the top item | Double-click, Enter, or Open / Run |
| Pan in 2D | Middle-button drag |
| Zoom in 2D | Mouse wheel, toward the cursor |
| Reset camera | Home key or Home button |
| Fit all occupied cells | Z, in 2D |
| Return to parent grid | Backspace or Back |
| Open the cell menu | Right-click |
| Show help | F1 or Help |

**Ctrl+= / Ctrl+-** adjusts interface text and button scaling; **Ctrl+0** resets it. This is separate from grid zoom. Cell labels scale with the cell's on-screen size and long names are shortened to fit. UI scaling is a session adjustment, not a field in the saved preferences.

Dialogs, the console, and search consume relevant keys while open. Close them before using shortcuts intended for the grid.

## 5. Add files, images, and downloads

### Drag and drop

Drop files onto the grid area, not the toolbar. On an empty area, multiple files are placed horizontally. On an occupied **file** cell, they are stacked there. Hold **Shift** to explicitly stack the dropped items at the target, including a nested-grid cell. In 3D, drop on the grid surface rather than the sky.

Use an empty stretch of cells for predictable horizontal placement: a horizontal drop can replace top files at later occupied destinations; it does not always skip occupied cells. Drop and paste have different replacement rules; use **Shift+drop** or **Ctrl+Shift+V** when preserving the target as a stack is your intention.

### Paste

Select the destination cell, then press **Ctrl+V** or click **Paste**. Selenite checks for:

1. Files copied in a file manager.
2. Raw clipboard image data, saved as a new PNG.
3. Text containing one local path, `file://` URL, or HTTP(S) URL per line.

Local text paths must exist; `~` expands to your home directory. Invalid lines are reported or skipped. Ordinary prose is not converted into a note item.

A normal paste replaces the **top file item** at the first destination and places additional entries in free cells to its right. Other items below that top item remain in the stack. A nested-grid top item is protected from normal paste replacement; use an empty cell or stack paste. **Ctrl+Shift+V** stacks all pasted items onto the selected cell.

### Copy In and grid folders

With **Copy In: On**, dropped and pasted files are copied into the destination grid's `assets/` directory. Copies run in the background with progress. Directories and files already in the destination assets directory are linked instead of copied again.

With **Copy In: Off**, existing files are referenced in place. Moving or deleting those originals breaks their references. Clipboard images and HTTP(S) downloads still need on-disk storage and are written to the grid's assets folder.

Press **O** or click **Grid Folder** to open the current grid's folder. Right-click a nested grid and choose **Open its grid folder** for that grid's folder. **Show in folder** instead opens the parent directory of the selected file.

Putting a file into a grid folder using a file manager does not automatically add a cell. Drop, paste, or import it into Selenite as well.

### Downloads and background copies

HTTP(S) URLs download to disk in the background. Progress appears on target cells and in the status area. The implementation imposes no fixed file-size or download-duration limit; disk space, network behavior, and operating-system resources still apply.

**Cancel DL** or **Esc**, when no higher-priority dialog is consuming it, cancels pending transfers. Wait for completion before relying on a managed copy or closing the application. Completed transfers can be undone as grid edits; undo does not erase the downloaded file from disk.

Transfers retain the grid and profile they started in. Switching profiles does not redirect their destinations. A profile with pending transfers may need to finish or cancel them before it can be renamed or deleted.

## 6. Select, move, copy, and delete

### Single and multiple selection

| Action | Result |
| --- | --- |
| Click | Select one cell |
| Ctrl+click | Toggle a cell in the selection |
| Shift+click | Select a rectangle from the selected cell |
| Drag from an empty cell in 2D | Draw a selection rectangle |
| Ctrl+A | Select occupied cells in the current grid |
| Esc | Clear selection, after other active interactions are dismissed |

Dragging a populated cell in 2D moves its whole stack. An ordinary single-cell move swaps with an occupied destination. **Shift+drag** merges onto the destination as a stack. Dragging a member of a multi-selection moves the selection together; an obstructed group move is rejected rather than overwriting unrelated cells. Use 2D for drag-based rearrangement; left-drag rotates the 3D camera.

### Copy and cut are file operations

**Ctrl+C** copies the top file of each selected file cell to the system clipboard, falling back to newline-separated paths if a native file-list clipboard is unavailable. **Copy path** copies a selected file's full path as text.

**Ctrl+X** first copies those files, then deletes the selected cells. The clipboard is **not a complete cell or grid serialization**: it does not preserve lower stack items, nested-grid contents, or game inventories. In a mixed selection, deletion can include more than the files copied. Use moves, stacking, or a save backup to preserve full grid structures. If a cut was unintended, use **Ctrl+Z immediately**.

### Delete and undo

- **Delete** removes the selected cells and their entire stacks.
- **Shift+Delete** removes only the selected cell's top item.
- Deleting a cell containing a nested grid with occupied cells asks for confirmation. Inspect game-inventory contents too; do not rely on that confirmation as a general backup mechanism.
- Removing a file cell does not delete the source or copied file from the filesystem.
- **Ctrl+Z** undoes; **Ctrl+Y** or **Ctrl+Shift+Z** redoes.

History stores up to **200 edit steps during the session**, including stack operations, nested-grid edits, completed transfers, PA imports, and recorded Ruby changes. Switching profiles clears history. History is not saved for the next application launch and does not undo external file changes or side effects performed by scripts.

## 7. Stacks

A cell with several items shows a `×N` badge. Its top item determines its visible thumbnail, type, and activation behavior. Files and nested grids can coexist in a stack.

| Task | Instruction |
| --- | --- |
| Add dropped files | Drop onto a file cell, or Shift+drop anywhere |
| Add pasted items | Ctrl+Shift+V |
| Merge another cell | Shift+drag onto the target in 2D |
| Merge selected cells | Right-click the destination → Stack selection here |
| Cycle forward/backward | Tab / Shift+Tab |
| Cycle from the menu | Next in stack |
| Remove just the top | Shift+Delete or Remove top item |
| Spread items into separate cells | Unstack into free cells |

Unstack leaves an item at the original coordinate and places the others in free cells to the right. Cycling, merging, and unstacking support undo. To reach a nested grid or label setting lower in a stack, first cycle that item to the top.

## 8. Nested grids

Select an empty cell and choose **New Grid**, then activate it to enter. The nested grid has its own cells, game-item inventory, and folder identity. Editing inside it changes the same saved hierarchy as the root; it is not a separate save file.

Use **Backspace** or **Back** to return one level. Each level retains navigation camera state while you browse, including its 3D camera. These navigation states are not a substitute for persisted content.

Moving or stacking a nested-grid item carries its contents with it. Deleting it removes that branch from the saved hierarchy. A filesystem directory placed in a cell is different: activating it opens the system file manager, without navigating a Selenite sub-grid.

## 9. Find, information, labels, and minimap

**Find (Ctrl+F)** searches the current save's entire hierarchy by filename, path, or kind: `image`, `audio`, `video`, `ruby`, `file`, or `grid`. Search is case-insensitive, includes stack items, and traverses at most 64 nested levels below the root. It does not search document text or every profile on disk.

Type a query. **Enter / Down** visits the next match; **Shift+Enter / Up** visits the previous one. Selenite enters nested grids as necessary. Matches are outlined in cyan in 2D, 3D, and the minimap. **Esc** closes Find.

**Info (I)** shows the top item's kind, path, size, modification age, and image dimensions where available. For nested grids it shows direct and total cell counts. Use this to inspect a full path when a cell label is truncated.

**Minimap (M)** shows an overview and the visible area. Click or drag within it to jump across the grid. The minimap is enabled by default and its preference is shared across profiles.

**Labels (L)** globally hides or shows coordinates, filenames, and type badges in 2D and 3D. Right-click an item for **Hide item labels / Show item labels** to control that item separately. The per-item setting is saved, follows moves and stacks, and supports undo. Global Labels Off suppresses individual labels without resetting their saved choices. The selected cell still indicates its coordinates.

## 10. 3D view

Press **F3** or click **3D View**. Cells become raised blocks; type affects their appearance, images lie on their tops, and nested grids have a lattice-like top. Click **2D View** or press F3 again to return.

| Control | Action |
| --- | --- |
| Left-drag | Rotate the camera |
| Q / E | Turn the camera |
| PageUp / PageDown | Tilt |
| Right-drag, middle-drag, or Shift+left-drag | Pan |
| W / A / S / D | Move across the grid |
| Wheel | Zoom |
| Click / double-click | Select / activate |
| Arrow keys | Move selection and glide the camera to it |
| Right-click without dragging | Open cell menu |
| Home | Reset 3D camera |

**Z** fits cells only in 2D. If perspective makes placement awkward, switch to 2D, edit, and return to 3D.

## 11. Images and the image viewer

Activating an image opens a separate viewer process. You can also use `selenite --image FILE`.

Thumbnails decode on worker threads and are downscaled to at most 1024 pixels. The image viewer reads dimensions first, decodes in the background, and uploads large images as GPU tiles up to 4096 pixels each. This avoids a single-texture size ceiling, but large decoded images still need memory and loading time. Extension-based image classification does not guarantee that every encoding is decodable.

| Control | Action |
| --- | --- |
| Wheel or + / - | Zoom |
| Drag with a mouse button | Pan |
| F | Fit to current window |
| R or 1 | Set 100% / one source pixel per display pixel |
| Ctrl+= / Ctrl+- / Ctrl+0 | Adjust/reset interface text scale |
| Esc or window close | Close the viewer |

The viewer is for viewing, not image editing. Its status strip reports dimensions, zoom, and tile information. Closing it leaves the main grid open.

## 12. Music, video, and other files

In the **main grid**, activating an audio cell tries the built-in player and queues audio from the current grid. Unsupported or failed decoding can fall back to the desktop application. The player supports WAV, OGG, MP3, FLAC, QOA, XM, and MOD; grid audio classification and decoder support are separate, so use Ruby's music API for a playable file that is classified as a generic file.

The player bar provides previous, play/pause, next, stop, seeking, volume, shuffle, repeat, and a playlist. The playing cell is outlined in purple. Click a playlist entry to play it; right-click an audio cell and choose **Add to playlist** to queue more. The **Music / Hide Music** button changes the bar's visibility.

| Key | Music action |
| --- | --- |
| Space | Play/pause; when idle, try this grid's audio |
| P | Show/hide playlist |
| [ / ] | Previous / next track |
| , / . | Seek backward / forward five seconds |
| 9 / 0 | Reduce / increase volume by five percentage points |

Repeat modes are **off**, **all**, and **one**. Default preferences are volume 80%, shuffle off, and repeat all. Those preferences persist across sessions and profiles; the current playlist and playback position are not stored in the preferences file.

Videos and other files open through the operating system's registered application. A directory opens through the file manager. Ruby files in the main grid run in the embedded console when scripting is enabled; use **Run as game** for scripts that own a game window.

## 13. File Inventory window

Click **Inventory**, or run:

```sh
selenite --inventory /path/to/selenite.json
```

This separate window reads an **existing save file** and lists attached items across all nested grids and stacks, with thumbnails, hierarchy location, and coordinates. It is different from the **Items** panel.

- Click a row to select it. Click the same selected row again or press **Enter** to open its file.
- Use the mouse wheel to scroll; **Up / Down** changes the selected row. Keyboard selection does not automatically scroll the list to that row.
- Click **Refresh** to reload the saved JSON. Changes in the main window do not live-update the list.
- Images open in Selenite's image viewer. Other files open with the system association, including audio and Ruby files.
- Nested-grid rows identify containers; activating them does not drill into a grid in this list window.
- **Esc** or the window-close button closes it. Interface text scaling shortcuts also apply.

This list implementation has a **Refresh** toolbar, without the main grid's Find, 3D, or music controls. Use the main window for those operations. It does not implement an R reload shortcut or PageUp/PageDown navigation.

## 14. Game-item inventories

Every grid can hold named quantities and JSON metadata independently of its visible file cells. Press **B**, click **Items**, or choose **Grid inventory…** from the cell menu.

The panel shows names, counts, metadata summaries, **+**, **−**, and delete buttons. Scroll for more items. Type into the entry field, then press **Enter** or **Add**:

| Entry | Effect |
| --- | --- |
| `potion` | Add one potion |
| `gold coin x50` | Add 50 gold coins |
| `arrow ×12`, `key 3`, `7 bombs` | Other quantity forms |
| `herb -2` | Remove two herbs |
| `gold = 12` | Set the count exactly |
| `gold = 0` | Remove the item |
| `arrow -> bolt` | Rename; merge quantities if the destination exists |
| `gem @color=red` | Set text metadata |
| `gem @value=25` | Set numeric JSON metadata |
| `gem @tags=["rare","blue"]` | Set array metadata |
| `gem @color=` | Remove that metadata key |

Item names are trimmed, must be nonempty, and are limited to 64 characters without control characters. Counts cannot become negative; operations report invalid amounts or insufficient items. Add an item before setting its metadata. Metadata values are parsed as JSON where possible and otherwise stored as text. **Clear all** removes this grid's inventory. Close with **Close** or Esc. Inventory edits are saved, support Ctrl+Z, and emit the Ruby `:inventory` event.

A nested grid has its own inventory. File Inventory does not list these abstract quantities. The full Ruby inventory and stack reference is in [inventory.md](inventory.md).

## 15. Profiles and preferences

### Profile management

Click **Profile: NAME** to switch, create, rename, or delete profiles. **N** begins creation in the profile panel. Use its row buttons for other operations, Enter to submit a name, and Esc to cancel. A profile name may contain letters, numbers, spaces, hyphens, underscores, and periods, up to 40 characters; it cannot be empty or begin with a period.

Switching saves the old profile, resets navigation and edit history, and opens the new root. A profile can have its own `init.rb` and plugins. Pending transfers continue into their original profile.

You must switch away before deleting the active profile. Deletion asks for confirmation and removes that profile's directory and managed data; it is not a grid undo operation. Back up anything you want to retain first. Renaming through the profile manager also updates managed path references; manually renaming directories does not perform that rewrite.

### Data location

The first applicable location is used:

| Platform/override | Base directory |
| --- | --- |
| All platforms, `SELENITE_HOME` set | The supplied directory |
| Windows | `%APPDATA%\Selenite` |
| macOS | `~/Library/Application Support/Selenite` |
| Linux with `XDG_DATA_HOME` | `$XDG_DATA_HOME/selenite` |
| Linux fallback | `~/.local/share/selenite` |
| No usable home/data environment | `.selenite` relative to the working directory |

A typical layout is:

```text
<data>/
  profiles.json                 last-used profile
  settings.json                 shared preferences
  plugins/                      plugins shared by profiles
  profiles/
    Practice/
      selenite.json              root and all nested grids
      selenite-grids/
        root/assets/            root grid's managed files
        grid-<id>/assets/       a nested grid's managed files
      .selenite-assets/          older pasted/downloaded assets, if present
      selenite_partitioned_array/
      init.rb                   optional startup script
      plugins/                  profile-specific plugins
```

### Shared preferences

Preferences in `<data>/settings.json` apply across profiles, including direct `--grid` sessions:

| Field | Default | Meaning |
| --- | --- | --- |
| `labels` | `true` | Global grid labels |
| `minimap` | `true` | Minimap visibility |
| `volume` | `0.8` | Music volume, from 0 to 1 |
| `shuffle` | `false` | Music shuffle |
| `repeat` | `"all"` | `"off"`, `"all"`, or `"one"` |
| `disabled_plugins` | `[]` | Disabled plugin filenames |
| `copy_files` | `true` | Copy In preference |

Use the interface to change them. If editing JSON manually, close Selenite first. Missing fields use defaults; unreadable or malformed settings fall back to defaults. Plugin disabling is keyed by filename, so same-named plugin files share that preference.

## 16. Saving, backups, and moving data

### What Save does

Edits normally save automatically. **Save / Ctrl+S** explicitly writes the root hierarchy, stacks, label overrides, inventories, and grid-folder IDs to JSON. Saves use a temporary file followed by replacement. A missing direct-grid file starts as an empty grid; malformed or unsupported existing files report an error rather than being treated as an empty save.

A direct-grid session uses `selenite --grid /path/to/work.json`. Its grid folders live beside it under `work-grids/`. Direct-grid mode is outside a named profile, though shared preferences still come from the data directory.

### Back up a profile

1. Wait for transfers to finish, save, and close windows or games that could write the same save.
2. Copy the **whole profile directory**, including its JSON, grid folders, legacy assets, scripts, plugins, and PA database if used.
3. Copy shared `settings.json`, `profiles.json`, and global `plugins/` if you need the entire setup.
4. Separately back up files linked from outside those folders when Copy In was off.
5. Preserve an earlier backup before replacing a working save.

For direct-grid sessions, back up the JSON and its matching `<stem>-grids/` directory, plus every other referenced file. **PA Export is not a full backup.**

### Restore or move

Restore into the original location when possible, then open that profile or grid and check several images, nested grids, and inventories. Refresh an already-open Inventory window after restoration.

Moving a JSON file or data directory does **not** automatically make every stored path portable. Paths are saved as supplied and can be absolute. Renaming a direct save also changes the expected `<stem>-grids/` location. For a move to a different filesystem location, preserve the structure and inspect/relink outdated paths before relying on the copy. Use profile-manager Rename for a profile rename within the same data store.

There is no general multi-window merge protocol for concurrent edits to the same save. Use one editing session per save; avoid editing the same data simultaneously from a game and the organizer. A game that saves can cause the organizer to reload that file when the game exits.

Deleting cells does not garbage-collect asset files. Unreferenced copies may remain on disk, including files kept for undo. Inspect references and backups before manually removing assets.

## 17. Toolbar and context-menu reference

| Button | Purpose |
| --- | --- |
| Profile: NAME | Manage local profiles |
| Save | Write the current root hierarchy |
| New Grid | Create a nested grid at a selected empty cell |
| Delete | Delete selected cells, with applicable nested-grid confirmation |
| Paste / Copy | Import clipboard contents / copy selected top files |
| Open / Run | Activate the selected top item |
| PA Export / PA Import | Write/read the current-grid PA snapshot; requires scripting |
| Inventory | Open the separate attached-file list |
| Items | Open this grid's named-item inventory |
| Grid Folder | Open this grid's assigned filesystem folder |
| Undo / Redo | Navigate session edit history |
| Find | Search the current save hierarchy |
| Ruby | Toggle console; execution requires scripting |
| Home | Reset camera |
| 3D View / 2D View | Switch grid presentation |
| Labels: On / Off | Toggle global cell labels |
| Copy In: On / Off | Toggle copying existing files into managed assets |
| Music / Hide Music | Toggle music-bar visibility |
| Plugins (n) | Manage plugins; count reflects enabled, successfully loaded plugins |
| Help | Open built-in help |
| Cancel DL (n) | Cancel pending transfers; appears while transfers run |
| Back | Return from a nested grid |

The right-click menu depends on the target and selection. It includes opening, Info, Copy file/path, Show in folder, Paste here, New grid, Delete, label visibility, stack actions, Grid inventory, and grid-folder actions. Audio adds **Add to playlist**. Ruby adds **Run as game** and **Export standalone game**. Plugins can add entries prefixed with `»`. Actions generally operate on the top item; cycle a stack first when necessary.

## 18. Keyboard reference

These shortcuts apply to the main grid unless a text field, panel, or dialog is using the keys.

| Keys | Action |
| --- | --- |
| Arrows / Enter | Move selection / activate |
| Home / Z | Reset view / fit occupied grid in 2D |
| Backspace | Parent grid |
| Ctrl+A / Ctrl+C / Ctrl+X | Select occupied cells / copy top files / cut cells after copying top files |
| Ctrl+V / Ctrl+Shift+V | Normal paste / stack paste |
| Delete / Shift+Delete | Delete selected cells / remove top item |
| Tab / Shift+Tab | Cycle stack forward / backward |
| Ctrl+S | Save |
| Ctrl+Z | Undo |
| Ctrl+Y or Ctrl+Shift+Z | Redo |
| Ctrl+F | Find |
| I / M / L | Info / minimap / labels |
| B / O | Items panel / grid folder |
| F1 / F3 | Help / 3D view |
| Backtick | Ruby console |
| F2, F4–F12 | Plugin actions with assigned hotkeys |
| Ctrl+= / Ctrl+- / Ctrl+0 | Increase / decrease / reset interface scale |
| Space / P | Music play-pause / playlist |
| [ / ] | Previous / next track |
| , / . | Seek −5 / +5 seconds |
| 9 / 0 | Volume −5 / +5 percentage points |
| Esc | Close/cancel active interaction, transfers, or selection |

In Find, Enter/Down advances and Shift+Enter/Up goes backward. In Help, wheel/arrows scroll, PageUp/PageDown page, Home/End jump, and F1/Esc closes. In confirmation dialogs, the displayed buttons are authoritative; Enter/Y generally confirms and Esc/N cancels.

See [3D controls](#10-3d-view), [image controls](#11-images-and-the-image-viewer), [Inventory controls](#13-file-inventory-window), and [console controls](#20-ruby-console-and-automation) for those modes. The main grid accepts Super/Command as a control modifier for many shortcuts, but console input and UI-scaling handlers specifically use Control; use the documented Ctrl bindings for consistency.

## 19. Command-line and environment reference

Examples use `selenite` on PATH. Substitute `./run.sh` in the portable bundle or `./target/release/selenite` in a source checkout. Quote paths or profile names containing spaces.

| Command | Meaning |
| --- | --- |
| `selenite` | Open last-used profile, or `default` |
| `selenite --help` or `-h` | Print usage and version |
| `selenite --profile NAME` | Open or create a profile |
| `selenite --list-profiles` | List profiles and mark the last-used one |
| `selenite --grid FILE.json` | Edit a specific save outside profiles |
| `selenite FILE.json` | Shorthand for direct-grid mode |
| `selenite --image FILE` | Open image viewer |
| `selenite --inventory FILE.json` | Open attached-file Inventory; save must exist |
| `selenite --script FILE.rb` | Run Ruby without a grid window against the last-used profile |
| `selenite --game FILE.rb` | Run a Ruby game with access to last-used profile data |

For `--script` and `--game`, put **one** target after the script:

```sh
selenite --script setup.rb --profile Practice
selenite --script setup.rb --grid /path/to/work.json
selenite --game game.rb --profile Practice
selenite --game game.rb --grid /path/to/work.json
```

A bare JSON path after the script is also accepted. Choose one startup mode; do not combine unrelated modes or rely on extra trailing options. Relative paths refer to the process's working directory unless the script explicitly resolves them relative to `__dir__`.

| Environment variable | Purpose |
| --- | --- |
| `SELENITE_HOME` | Override application data directory |
| `XDG_DATA_HOME` | Linux data-base fallback when SELENITE_HOME is absent |
| `SELENITE_PARTITIONED_ARRAY_LIB` | Override the PA Ruby library directory |
| `SELENITE_GAME_FRAMES` | End game rendering after the specified frame count |
| `SELENITE_GAME_SCREENSHOT` | Write a final game screenshot to this path |
| `LD_LIBRARY_PATH`, `RUBYLIB` | Runtime library paths configured by the portable launcher |

The main executable has no dedicated `--version` mode; use `--help`. An unsuccessful startup or uncaught headless Ruby error is reported to stderr and returns exit status 1.

## 20. Ruby console and automation

### Enable and use the console

Use a scripting-enabled build. Press **backtick** or click **Ruby**. Evaluate short Ruby expressions to inspect or edit the current grid. `puts` and `p` output is captured; ordinary Ruby exceptions are reported. Ruby shares the application process and UI thread: a long loop can block the interface, and arbitrary external side effects are not undone with grid history.

| Console keys | Action |
| --- | --- |
| Enter | Evaluate the input line |
| Up / Down | Command history |
| Ctrl+V | Paste; multiple lines are joined with semicolons |
| Backspace | Edit input |
| Ctrl+U | Clear input |
| Ctrl+L | Clear console output |
| Wheel / PageUp / PageDown | Scroll output |
| Esc | Close console |

A profile's `init.rb` is evaluated when that profile opens in the organizer. Use it for startup setup or event hooks. Profiles share one Ruby VM within a running app; variables and constants can outlive a plugin reload. Headless CLI modes create their script context directly: do not assume they replay the organizer's init/plugin startup lifecycle.

### Grid API

```ruby
g = Selenite.grid                # current grid
root = Selenite.root             # complete save's root
g[0, 0]                         # Cell or nil
g.set(0, 0, "/path/to/photo.png") # place if empty; false if occupied
g[1, 0] = "/path/to/song.mp3"    # replace file-topped stack; rejects a top grid
g.push(1, 0, "/path/to/other.mp3") # add a top item
g.items(1, 0)                    # all stack items, top first
g.cycle(1, 0, 1)                 # reverse with -1
g.raise_item(1, 0, 2)            # bring zero-based stack index 2 to top
g.pop(1, 0)                     # remove/return top; optional index argument
g.merge(1, 0, 2, 0)             # source column,row; destination column,row
g.unstack(2, 0)
sub = g.new_grid(3, 0)           # choose an empty coordinate
sub.set(0, 0, "/path/to/notes.txt")
g.subgrid(3, 0)                 # retrieve the nested grid
g.push_grid(4, 0)               # stack a new grid
g.move(0, 0, 5, 0)              # move; g.swap exchanges cells
g.fill(0, 2, Dir["/path/to/pictures/*.png"])
g.each { |cell| puts cell }      # Enumerable
g.size; g.item_count; g.stack_size(2, 0); g.stacked?(2, 0)
g.next_free(0, 2); g.cells
g.remove(5, 0)                  # whole coordinate; g.clear clears the grid
```

These statements are an API tour, not a script to run against valuable data unchanged. `Cell` exposes `col`, `row`, `kind`, `path`, and `grid?`. `g.size` counts occupied cells; `g.item_count` includes stacked items in that grid.

`set`, assignment, and `push` reference paths. Assignment replaces the whole stack when its top is a file; it refuses a top nested-grid item. Use `push` to preserve existing stack items. To make a managed copy, use `g.import(col, row, path)` or `Selenite.import(col, row, path)`, which copies into that grid's assets directory and stacks the result at the coordinate. `g.folder`, `g.assets_dir`, and `g.folder_id` inspect a grid's filesystem identity; `Selenite.grids_dir` locates the common folder root.

### Application and inventory API

| Methods | Purpose |
| --- | --- |
| `Selenite.selected`, `selection`, `select(c,r)`, `select_cells([[c,r], ...])` | Inspect/change selection |
| `goto(c,r)`, `open(c,r)`, `enter(c,r)`, `back`, `depth` | Navigation and activation |
| `view_3d(bool)`, `orbit(yaw,pitch,distance)` | 3D presentation; angles in degrees |
| `search(query)`, `find(query)` | Return matches / open Find |
| `undo`, `redo`, `history` | Edit history; history reports next undo/redo labels |
| `save`, `save_path`, `status(text)` | Save location and messages |
| `profile`, `profiles`, `create_profile(name)`, `switch_profile(name)` | Profile operations |
| `labels(bool)`, `labels?`, `copy_text(text)` | Labels and clipboard |
| `download(url,col,row, stack: false)` | Queue a transfer in the organizer |
| `classify(path)`, `file_info(path)`, `image_size(path)`, `playable?(path)` | File information |
| `checksum(path)` | Streaming FNV-1a checksum; not a cryptographic verification hash |

The table's unqualified methods belong to `Selenite`. `search` returns entries shaped `[nested_path, col, row, name]`. UI-changing methods queue requests until the current Ruby block returns; grid mutations apply immediately. Headless scripts do not run the organizer event loop, so use direct grid/file operations there rather than expecting navigation, media playback, or queued downloads to execute.

```ruby
inv = Selenite.inventory         # same as Selenite.grid.inventory
inv.add("gold", 10)
inv.remove("gold", 2)
inv["key"] = 1
inv.has?("gold", 5)
inv.set_meta("key", "door", "north")
inv.meta("key")
inv.each { |name, count, meta| p [name, count, meta] }
inv.transfer(Selenite.root.inventory, "gold", 3)
```

Inventories also expose `rename`, `delete`, `clear`, `names`, `size`, `total`, `empty?`, and `to_h`. Missing counts read as zero; removing unavailable quantities raises an error. See [inventory.md](inventory.md) for aliases and details.

### Music API

```ruby
m = Selenite.music
m.play("/path/to/song.flac")
m.play_grid
m.play_list(["/path/to/a.mp3", "/path/to/b.ogg"], 0)
m.queue("/path/to/c.wav")
m.toggle; m.pause; m.resume; m.stop
m.previous; m.next; m.seek(30)
m.volume(0.6); m.shuffle(true); m.repeat(:all)
m.show; m.hide; m.clear
m.state; m.playing?; m.track
```

`state` is a snapshot from the start of that Ruby evaluation. Commands issued in the same block are not reflected until a later evaluation. The complete music reference is in [plugins.md](plugins.md#the-music-api).

### Events

```ruby
Selenite.on(:activate) do |col, row, kind, path|
  puts "Opening #{kind} at #{col},#{row}: #{path}"
end
Selenite.on(:inventory) { |message| puts message }
```

| Event | Arguments |
| --- | --- |
| `:activate` | `col, row, kind, path`; return `:handled` to suppress normal activation |
| `:paste` | `col, row, path` |
| `:drop` | `col, row, paths` |
| `:download` | `col, row, path` |
| `:save` | `json_path` |
| `:profile` | `name` |
| `:track` | `path, index, count` |
| `:inventory` | Change message |

Inspect hooks with `Selenite.hooks`, remove one event's hooks with `Selenite.off(:event)`, or remove all using `Selenite.off`. Plugin-managed hooks have a separate reload lifecycle described below.

### Headless script recipe

Save this as `setup.rb` and run it against a practice profile:

```ruby
Selenite.grid.new_grid(2, 0) unless Selenite.grid[2, 0]
Selenite.inventory["gold"] = 50
puts Selenite.inventory["gold"]
```

```sh
selenite --script setup.rb --profile Practice
```

Headless script execution prints output and saves the grid after successful evaluation. `__FILE__` and `__dir__` identify the script. An exception returns a nonzero status; do not assume it rolls back side effects or an explicit earlier save. Classic global helpers such as `grid_set_file`, `grid_get`, `grid_list`, `grid_push_file`, `grid_push_grid`, `grid_list_stacks`, `grid_remove`, `grid_move`, `grid_swap`, `grid_new_grid`, `grid_clear`, `grid_count`, `grid_save`, `grid_status`, `grid_select`, `grid_download`, and `grid_eval_file` remain available.

## 21. Plugins

Plugins are Ruby files that add actions, hotkeys, menu entries, commands, hooks, and timers. They require scripting support.

1. Open **Plugins**, then **Install examples**.
2. Try the example Hello action with **F6**, or a plugin entry in a cell menu.
3. Click **Open folder** to find the global plugin directory.
4. Add a `.rb` file, then click **Reload**.
5. Use **On/Off** to enable or disable a file without deleting it. A loading error appears in red.

Global files live under `<data>/plugins/`; profile-specific files under `<data>/profiles/<name>/plugins/`. Files are sorted within their directory. Organizer startup, profile switching, and Reload load enabled plugins; startup loading follows the profile's init script.

```ruby
Selenite.plugin "Quick actions" do |p|
  p.description "A greeting and an image inspection action"
  p.version "1.0"
  p.button("Hello", key: "F5") do |cell|
    Selenite.status(cell ? "Selected #{cell}" : "Select a cell first")
  end
  p.menu("Image size", kinds: %w[image]) do |cell|
    Selenite.status(Selenite.image_size(cell.path).inspect)
  end
  p.command("greet") { |name = "world"| "Hello, #{name}" }
  p.every(60) { Selenite.status(Time.now.strftime("%H:%M")) }
end
```

| Declaration | Behavior |
| --- | --- |
| `p.button(label, key: ...)` | Panel action; receives selected Cell or nil |
| `p.menu(label, kinds: ...)` | Cell-menu action; filter by kind or `empty`; `file` matches file kinds |
| `p.command(name)` | Call using `Selenite.run(name, *args)`; list via `Selenite.commands` |
| `p.on(event)` | Hook removed when plugins reload |
| `p.every(seconds)` | Repeating UI-thread timer, minimum 0.25 seconds |

Hotkeys can be F2 or F4–F12. F1 and F3 are reserved; the first loaded action wins a hotkey collision. Keep callbacks short so the UI stays responsive. A timer exception stops that timer. Ordinary action exceptions are reported; they do not make arbitrary Ruby side effects reversible.

Reload clears plugin registrations and plugin-managed hooks/timers before reevaluating files. It does not restart Ruby or clear globals/constants. Plain `Selenite.on` hooks are not removed by plugin reload. Recorded button/menu/hotkey grid edits are grouped into undo steps.

The bundled examples are Hello, File Inspector, Now Playing, Grid Tools, and Clock. For complete DSL behavior, error handling, and recipes, use [plugins.md](plugins.md).

## 22. Games and standalone exports

For a hands-on introduction, follow the [Gem Trail game-development tutorial](game-tutorial.md), from a first window to a playable game with saved progress.

For the dedicated Ruby-only runtime, follow the [packaging guide](packaging.md). `./bootstrap-game.sh` builds a game-only portable package; `selenite-game game.rb` runs without opening the organizer and defaults to a save beside the script.

For larger persistent games, see [partitioned game storage, BSP, and revenants](game-storage.md). It covers named tables, streamed worlds, labeled reloads, and checkpoints, with a playable [Revenant World example](../examples/games/revenant_world.rb).

### Run a game

Use **Run as game** from a Ruby cell's menu, or `selenite --game FILE.rb`. A game runs in a separate process and may own a raylib window. The organizer saves before starting it and checks for saved changes when it exits.

A game launched from the organizer receives the profile or direct save **at its root**. Entering a nested grid in the organizer does not automatically change the child game's initial grid context. Navigate explicitly in the game's Ruby code if its level is nested.

Ordinary double-click/Open on Ruby evaluates it inside the organizer instead. Use the game launch mode for a rendering loop so the organizer remains responsive.

```ruby
Selenite::Game.run(title: "Hello", width: 800, height: 450) do |g|
  g.draw_ui { g.text("Hello from Selenite", 30, 30) }
end
```

Save as `hello.rb`, then run `selenite --game hello.rb --profile Practice`.

`Selenite::Game` provides `update`, `draw`, `draw_ui`, `on_close`, `every`, `after`, `quit!`, input helpers, a camera, cached textures, `grid`, `inventory`, and `save`. Defaults are 960×540, 60 FPS, audio enabled, a resizable window, Escape to exit, and autosave on close. `draw` uses the game camera; `draw_ui` uses screen coordinates. Game timers use game time and cap catch-up callbacks per frame.

You can also `include Raylib` and use `InitWindow`, `BeginDrawing`, `DrawText`, input, textures, audio, and collision helpers. The embedded API implements a documented subset with snake_case aliases; it is not a promise that every external raylib binding is available. Only one game window may be open in a game process. The complete API, options, types, and sample programs are in [game.md](game.md).

### Export a game

Games and organizer scripts can use direct P2P connections, authenticated
host/join sessions, replicated state, and LAN discovery. The
[networking guide](networking.md) includes a multiplayer game, a shared-grid
plugin, launch commands, API reference, persistence, and connectivity limits.

1. Save the organizer's current work.
2. Right-click the Ruby file and choose **Export standalone game**.
3. Find `<script-name>-game/` beside the script.
4. Run its `./run-game.sh` and verify the exported game independently.

The export contains a copied `game.rb` (or `game.rbc` for a compiled entry), the existing saved JSON as `save.json`, sibling `assets/` and `game-data/` folders if present, the Selenite runtime, and a launcher. Close games using partitioned storage before exporting. The JSON is the saved **root hierarchy**, not only the currently visible nested grid. Re-export can overwrite files in the existing export folder.

Both executables support `--compile game.rb [--output game.rbc] [--force]` and
run compiled `.rbc` entries. Compile with the runtime you distribute, keep your
source privately, and export the compiled entry. Bytecode omits the original
source but is not encryption. See [compile and distribute Ruby bytecode](game.md#compile-and-distribute-ruby-bytecode)
for compatibility, helper loading, and complete instructions.

The portable-build export includes the bundled Ruby runtime and supporting folders; a development-build export copies the executable and may still require development-system libraries. The launcher prefers its bundled runtime, then a `selenite` on PATH if no runtime executable is present.

Export copies an `assets/` directory **beside the script**. It does not automatically collect every file referenced by your grid, copy all scripts loaded with `require_relative`, or rewrite absolute paths. Prepare the game's dependencies deliberately, use `File.join(__dir__, "assets", "name.png")`, and test away from the original project. This matters especially when Copy In has moved the script into a managed assets directory.

### Bounded rendering and screenshots

```sh
SELENITE_GAME_FRAMES=90 SELENITE_GAME_SCREENSHOT=pong.png selenite --game examples/games/pong.rb
```

The frame limit ends the game and the screenshot variable captures the final frame. A rendering-capable display/context is still required; these variables alone do not create a headless display server. See [game.md](game.md#headless-runs-and-screenshots) for details.

## 23. Partitioned array import and export

PA is an optional Ruby database integration. It uses the bundled `partitioned_array` library through scripting support.

The toolbar operations below use the legacy grid snapshot format. For game entities, lazy loading, BSP, and revenant partitions, use the separate [game storage API](game-storage.md); the two disk formats are not interchangeable.

- **PA Export** overwrites the previous snapshot in `selenite_partitioned_array/` beside the save file, using database name `selenite_cells`.
- **PA Import** restores into the currently viewed grid, skipping coordinates already occupied.
- Stack items are recorded individually with `col`, `row`, `index`, `kind`, and `path`; index zero is the top item. Import reconstructs stack order.
- Nested grids are exported as empty container records. Their descendants are not recursively backed up by this export.
- Game inventories, per-item label flags, managed file contents, preferences, and full grid-folder identity are not a complete part of this snapshot. Use filesystem backups for complete preservation.

```ruby
grid_pa_available
grid_pa_export("/path/to/database")
grid_pa_records("/path/to/database")
grid_pa_import("/path/to/database")
```

The last three accept an optional database name after the directory. Import returns the number of items restored. Older records without a stack index are treated as index zero.

Library lookup prefers `SELENITE_PARTITIONED_ARRAY_LIB`, then `partitioned_array/lib` beside the executable, then the source checkout. Set the override to the directory containing `managed_partitioned_array.rb` if the library cannot be found.

## 24. mic_freq alarms, timers, and stopwatch

The separate **mic_freq** application in this repository combines a voice-reactive avatar with time-tool tiles. These controls are not built into Selenite's main grid window and mic_freq is not included by Selenite's packaging script.

From the repository root:

```sh
cargo run --manifest-path mic_freq/Cargo.toml --release -- --selenite "$PWD/dist/run.sh" --target /path/to/work.json
```

Supply an existing Selenite executable or portable launcher. Omit `--target` and drop one grid or game into the mic_freq window if preferred. Selecting a target does not execute it: enable **Launch grid** separately on the desired time tools.

| Tool | Operation | Optional launch event |
| --- | --- | --- |
| Alarm | Local 24-hour HH:MM; Arm, Cancel, Dismiss, Snooze 5m | When alarm fires, including after snooze |
| Timer | MM:SS or HH:MM:SS; Start, Pause, Resume, Reset | When countdown finishes |
| Stopwatch | Start, Pause, Lap, Reset; latest four cumulative laps | Fresh start after zero/reset; not resume |

A `.json` target opens Selenite with `--grid`. A `.rb` target runs with `--game` and requires scripting support. Opening a JSON grid does not automatically activate its individual cells. Ruby launches use Selenite's default profile unless the game manages another context itself.

All enabled tools share the selected target. Alarm and timer alerts include a visual banner and optional sound. A/T/S hide/reopen tiles and F changes avatar focus; hidden tools keep running while mic_freq stays open. State is session-only, closing mic_freq stops its clocks, and it does not wake a sleeping computer. Already launched Selenite windows remain open. See the companion's [README](../mic_freq/README.md) and [manual](../mic_freq/MANUAL.md) in the source checkout; the Selenite portable bundle does not ship those companion files.

## 25. Troubleshooting and limits

| Symptom | What to check |
| --- | --- |
| My grids disappeared | Confirm Profile and SELENITE_HOME; inspect `--list-profiles`. A different data root or direct-grid path opens different data. |
| A cell's file is missing | Inspect its full path with Info. Check whether Copy In was off, the data directory moved, or an external drive is absent. Restore the file or relink it. |
| Drop did nothing | Drop below the toolbar; in 3D, target the surface. Check transfer progress and the status message. |
| Paste rejects text | Paste existing paths or HTTP(S)/file URLs, one per line. Plain text is not a note. |
| Paste replaced an item | Normal paste replaces the top file at the first destination. Undo, then use Ctrl+Shift+V to stack. |
| Cut did not preserve my stack | Clipboard copying includes top files only. Undo the cut; move/stack complete cells instead. |
| Move was blocked | A multi-cell destination is occupied by unrelated data. Choose a clear region or intentionally merge via stacking. |
| I cannot find the selected content | Close panels, try Z in 2D or Home, then use Find. Cycle stacks with Tab. |
| Keyboard shortcuts act on a field | Close the console, Find, or active dialog first. Check the mode-specific shortcuts. |
| Text is too small | Enlarge the window or press Ctrl+=. Ctrl+0 resets UI scaling. |
| Image is slow or fails to decode | Check format, file integrity, RAM/VRAM, and permissions. Large-image tiling removes one size limit, not memory constraints. |
| Music will not play | Check player volume, system output, codec support, and file existence. Try the system media player. |
| Inventory list is stale | Save in the main window, then click Refresh. Use the wheel to reach rows selected with the keyboard. |
| Grid will not save | Read the error; check permissions and disk space. Preserve a backup before editing JSON manually. |
| Unsupported save version | This build reads versions 2 and 3. Use a compatible/newer build for a newer save. |
| Ruby/Plugins/PA unavailable | Build with `--features scripting` or use the packaged launcher; inspect Ruby runtime errors in a terminal. |
| Ruby runtime library missing | Use `run.sh`, retain bundled libraries, or install/build against the appropriate Ruby runtime. |
| Plugin fails or hotkey runs another action | Read the red load error, check enabled files and conflicting keys, then Reload. F1/F3 are reserved. |
| UI hangs after Ruby code | A synchronous Ruby operation may be blocking the UI. Keep callbacks short; move long game loops to --game. |
| PA library missing | Check bundled library paths or SELENITE_PARTITIONED_ARRAY_LIB. |
| Exported game has missing assets | Inspect absolute paths and extra script dependencies; only the script's sibling assets folder is copied automatically. |
| Game changes appear to overwrite organizer work | Both processes may have written the same save. Restore a backup and avoid concurrent edits. |
| Portable runtime fails on another machine | Check architecture, Linux/system-library compatibility, OpenGL/display availability, and runtime folder contents. |
| Window creation or graphics allocation fails | Check graphics drivers and display access; reduce window size. Supersampling increases GPU use. |
| Disk use remains after deleting cells | Removing references does not delete managed assets. Back up and inspect before cleaning unused files. |

Run Selenite from a terminal to retain startup, decoder, and script error messages. For a useful bug report, include the version from `--help`, OS, build type, exact launch command, steps to reproduce, and relevant terminal output; use a small reproducible save when possible.

Practical limits include signed 64-bit grid coordinates, available memory/disk/GPU resources, 200 session undo steps, synchronous Ruby callbacks, and no automatic merge of concurrent save writers. Full-scene supersampling renders at twice each framebuffer dimension, increasing rendering work and memory. There is no general app-level switch for disabling it in the documented UI.

## 26. Save-format reference

Application release versions and save-format versions are different. This release writes **save version 3** and reads versions **2 and 3**. A version-2 save is upgraded on its next write; keep an original copy if older software must still read it.

A minimal complete save is:

```json
{
  "version": 3,
  "root": {
    "cells": []
  }
}
```

An illustrative hierarchy with a stack, a nested grid, and inventory is:

```json
{
  "version": 3,
  "root": {
    "id": "root",
    "cells": [
      {
        "col": 0,
        "row": 0,
        "items": [
          { "Image": "/path/to/photo.png", "labels_hidden": true },
          { "File": "/path/to/notes.txt" }
        ]
      },
      {
        "col": 1,
        "row": 0,
        "items": [
          { "Grid": { "cells": [] } }
        ]
      }
    ],
    "inventory": {
      "gold": { "count": 12, "meta": { "source": "demo" } }
    }
  }
}
```

The example paths are placeholders. `items` is top-first. Content tags are `Image`, `File`, and `Grid`; audio/video/Ruby classifications are inferred from file paths rather than stored as separate content tags. `labels_hidden` defaults to false. Empty inventories and unassigned grid IDs can be omitted. A grid ID determines its folder within `<save-stem>-grids/`; do not change it casually.

The JSON stores references, not file bytes. Prefer the UI or Ruby API for edits. If hand-editing, close writers first, back up the original, preserve unique coordinates and nonempty stacks, and validate the complete root wrapper before reopening.

## 27. Maintenance and further references

For everyday use, keep the data directories backed up, finish transfers before closing, and use a separate practice profile for experimenting with scripts and plugins. Update the application separately from your profile data; preserve backups across release-candidate upgrades.

For a source build, run checks from the repository root:

```sh
cargo test
cargo test --features scripting
```

The second command requires the Ruby development environment. These are developer checks, not steps required every time you start the app.

Further references:

- [Project overview and build notes](../README.md)
- [Release history](../CHANGELOG.md)
- [Stacks and game inventory API](inventory.md)
- [Plugin API and recipes](plugins.md)
- [Game API and examples](game.md)
- [Example games](../examples/games)
- [Example plugins](../examples/plugins)

This manual was checked against the command-line dispatcher, actual window input handlers, persistence model, profile/settings code, Ruby bindings, PA bridge, and packaging implementation. Where older overview text differs—especially Inventory controls, PA stack records, game root context, and export portability—use the behavior described here for this checkout.

# Selenite IDE

Start a scripting-enabled build with `selenite --ide path/to/game.rb`.
Version 11 also opens project folders with `selenite --ide path/to/project`.
See the [project guide](projects.md) for scaffolding, manifests and application projects.
From the organizer, select Ruby source and click IDE or choose Open in Selenite
IDE from its context menu. Clicking IDE on an empty cell creates a starter game
in the current grid's asset folder. `Selenite.ide(path)` opens the same suite from
the organizer console or a script.
The source editor, file browser, workspace list, Ruby console, program output,
terminal and Help share one GPU-rendered window. Programs use a separate process
so their raylib window cannot replace the IDE's OpenGL context.

## Files and Editing

The File menu contains New, Open, New Game Project, New Application Project,
Open Project, Add Workspace, Save, Save As, Reload and Close.
The Edit menu contains Undo, Redo, Cut, Copy, Paste, Select All, Find and Replace.
Open and Save As accept absolute or relative filesystem paths. Select All in a
path dialog selects its contents for replacement by typing or pasting. Confirm and Cancel work with
the mouse; Enter confirms and Escape cancels. Existing files and unsaved buffers
require a second confirmation before replacement or discard.

Use Ctrl+N, Ctrl+O, Ctrl+S and Ctrl+Shift+S for common file commands.
Ctrl+Shift+N creates a game project; Ctrl+Shift+O opens an existing project.
Ctrl+Z/Ctrl+Y undo and redo. Ctrl+X/Ctrl+C/Ctrl+V use the system clipboard.
Ctrl+F and Ctrl+H open Find and Replace. Ctrl+Q closes the editor, with an
unsaved-change warning. Window-close saves a dirty buffer using the editor's
existing behavior.

## Workspace and File Browser

Ctrl+P opens Quick Open across the workspace's source files. Type part of a path,
use Up/Down and Enter or click a result. Escape closes the picker. Build folders,
dependencies, binary files and symlinks are excluded; scans are bounded.
F8 opens suite tools and recent project folders. F9 opens interface-scale presets.

Files lists the current folder with directories first. Select a directory to
browse it or a UTF-8 text file to edit it. Up navigates to its parent; Refresh
reloads the directory. Use File > Add Workspace to add a project folder.
Workspaces lists recently selected project roots. Selecting one changes the file
browser root and restarts the embedded terminal in that directory.

## Layout

Drag the vertical sidebar edge or the horizontal dock edge to resize panes.
Drag the Terminal, Ruby, Output or Help tab to the top or bottom half of the
window to move the shared dock. Its position is saved between sessions; Reset
returns it to the bottom. The File and Edit menus stay at the top.
Files toggles the sidebar; Hide/Show toggles the bottom dock. The layout responds
to window resizing while preserving a minimum source-editor width. Long labels
are truncated rather than overlapping. Editor, terminal, console, sidebar, Help,
dialogs, spacing and hit targets scale together with the whole window. Resizing
individual panes does not change text size. Dock controls wrap into multiple
rows in narrow windows; Help search moves to a second row when needed. File
dialogs use the full window rather than just the editor pane. Ctrl +/- and
Ctrl+wheel adjust the shared UI scale, with a fit limit to keep controls usable.
F11 toggles maximization. Reset restores the default layout. Pane proportions,
visibility, active dock tab, run mode, window size and recent workspace roots are
saved in `ide.json` under `SELENITE_HOME`.

## Run and Debug

Run saves the current source, then starts it in a child Selenite process.
Inside a project it runs the manifest's entry point, even when editing a helper.
F5 runs; Shift+F5 stops. Game mode supplies the window, graphics and audio API;
App mode runs a headless Selenite application script. Graphical applications can
also use Game mode: it is not restricted to games. Output shows program output,
errors and exit status. Stop terminates the current process. The IDE also stops
its child when closed. The save grid is `<source-stem>.save.json` beside the file.
Organizer-launched IDEs share the organizer's grid save instead; use
`selenite --ide game.rb --grid profile.json` to select one explicitly. The
organizer reloads changed grid data when the IDE closes. The IDE console reloads
the grid after a program exits. Avoid editing the same grid from multiple
windows at once: shared saves are not transactional multi-writer databases.

Ruby is an embedded Ruby console. Enter evaluates an expression; Ctrl+V pastes.
Its state is separate from the running program. Window and audio ownership stays
with the program process, not the IDE console. The terminal runs a real shell
with your normal user permissions. It supports ANSI colors, interactive programs,
scrollback, selection, Ctrl+Shift+C/Ctrl+Shift+V and PTY resize notifications.
`SELENITE_GRID` contains the save path selected when that terminal session starts.
The terminal tab bar's + opens another independent shell in the workspace.
Ctrl+Shift+T opens a terminal, Ctrl+Tab or Ctrl+Shift+Tab switches sessions,
and Ctrl+Shift+W closes the active session. Closing the final IDE terminal
starts a fresh shell. Closing the IDE stops every terminal session.
The Aa control and F9 open shared interface-scale options; changes are saved.
Standalone Selenite terminal windows have the same tab controls. Their + button
opens a shell in the active terminal's directory on Linux. Each tab owns its
shell, scrollback and selection. The arrows switch tabs when the bar is full;
closing a tab stops its shell, and closing the window stops all its sessions.
F6 switches between Ruby and Terminal; Ctrl+backtick toggles the dock.

## Tutorials and API Help

F1 opens Help. The topic title opens a picker. Help contains this guide, the Gem Trail game tutorial, Selenite's game and
organizer APIs, the native raylib tutorial, all 600 raylib functions, storage,
networking and plugins. Use the previous/next topic controls and the mouse wheel.
The raylib API topic supports searching function names and descriptions.

## Rendering

All visible application surfaces use raylib/OpenGL: text is drawn from GPU font
atlases, images use GPU textures, and 2D/3D geometry is GPU rasterized. The IDE,
organizer and image/inventory views use the shared GPU framebuffer renderer.
The terminal uses the same GPU drawing primitives. Image decoding, filesystem
access, Ruby execution, terminal parsing and layout remain CPU work. A software
OpenGL driver can still render on the CPU; hardware acceleration depends on the
graphics driver and environment, not just application code.
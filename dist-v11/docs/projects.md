# Selenite 11 Projects

A project is a folder containing `selenite-project.json`, a Ruby entry point,
and optional application data. No Ruby gem, external editor or language server
is needed to use the embedded Selenite and raylib APIs.

## Create and Open

Use File > New Game Project or New Application Project in the IDE. Confirm the
suggested directory or enter a new one. Existing non-empty folders are never
overwritten. Ctrl+Shift+N creates a game project; Ctrl+Shift+O opens a project.

The command-line equivalents are:

```sh
selenite --new-project my-game
selenite --new-project my-app --app
selenite --ide my-game
selenite --project my-game
```

In a portable package, replace `selenite` with `./run.sh`. `./run-ide.sh` opens
the bundled starter project; `./run-ide.sh /path/to/project` opens your own.
`selenite-game --project /path/to/project` runs a project without the organizer.

Game projects start with a movable blue player, a frame-independent update loop,
and a drawing callback. Application projects start with a persistent run counter
and no graphics window. Graphical applications can use game mode without being
games. Use the Gem Trail tutorial in Help for a complete playable example.

## Manifest

```json
{
  "schema": 1,
  "name": "my-game",
  "kind": "game",
  "entry": "game.rb",
  "grid": "save.json"
}
```

`kind` is `game` or `app`. The entry must be an existing `.rb` source file.
Entry and grid paths must be relative, with no parent-directory traversal.
The grid must have a `.json` extension and is created when the program saves.
Unsupported schemas, unknown fields and invalid paths produce an error.

Edit the manifest as text, then reopen the project to apply configuration changes.
An explicit IDE `--grid FILE.json`, including a launch from the organizer,
overrides the manifest's grid so the project can operate on that profile's data.

## Editing and Running

Opening a project selects its entry point and restores its mode and save path.
Opening helper files inside that project keeps the same project context. Run/F5
saves the active source file, then launches the configured entry point, not the
helper file. Stop/Shift+F5 terminates the child. The run-mode button allows a
temporary game/app override; reopening a project restores its manifest mode.

Project runs use the project root as their working directory. Use `__dir__` in
Ruby for paths relative to a particular source file. Native raylib records retain
their C ownership rules; use `Selenite::Game` when managed resource cleanup is
preferable. Output and errors appear in the IDE's Output tab.

Recent project roots appear under Workspaces. Selecting a project there reopens
its entry point, with the usual unsaved-buffer confirmation. Ordinary workspace
folders remain supported without a manifest.

## Data and Trust

Keep backups of source and save files. Grid saves remain compatible with earlier
Selenite releases; project schema 1 does not change the grid format. Shared grids
are not transactional multi-writer databases: avoid simultaneous edits from
multiple organizer/IDE windows.

Projects execute trusted Ruby with your normal user permissions. Inspect source
before running downloaded projects. Manifest path validation is not a Ruby
sandbox. Compiled Ruby bytecode is tied to its Ruby runtime and is not encryption.
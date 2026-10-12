# Making games with Selenite

Selenite embeds Ruby through [Magnus](https://crates.io/crates/magnus) and
exposes a native **`Raylib`** module implemented in Rust on top of
[raylib-rs](https://crates.io/crates/raylib). The API follows the
[raylib-bindings](https://github.com/vaiorabbit/raylib-bindings) gem
(`include Raylib`, `InitWindow`, `DrawText`, `IsKeyDown`, …). Most small
raylib-bindings programs therefore run unchanged, and you don't need to
install a gem or shared library.

On top of that sits **`Selenite::Game`**, a small game-loop helper. Games
can also reach the normal Selenite API: grids, stacked cells and each
grid's [inventory](inventory.md).

- [Step-by-step tutorial: Gem Trail](game-tutorial.md)
- [Running a game](#running-a-game)
- [Packaging (organizer + selenite-game)](packaging.md)
- [Compile and distribute Ruby bytecode](#compile-and-distribute-ruby-bytecode)
- [Multiplayer and P2P networking](#multiplayer-and-p2p-networking)
- [raylib-bindings style](#raylib-bindings-style)
- [Selenite::Game](#selenitegame)
- [Using grids and inventories](#using-grids-and-inventories)
- [Partitioned storage, BSP, and revenants](#partitioned-storage-bsp-and-revenants)
- [API reference](#api-reference)
- [Headless runs and screenshots](#headless-runs-and-screenshots)
- [Examples](#examples)

## Start with a tutorial

New to game development? Follow [Build your first Selenite game: Gem Trail](game-tutorial.md). It starts with a window and movement, then explains a complete game with collisions, a camera, a HUD, grid-authored levels, saved inventory progress, testing, and export. The [finished source](../examples/games/gem_trail.rb) runs without external assets.

This page remains the API and launch-mode reference.

## Running a game

In **3.0.0-rc.4**, `selenite-game` is the dedicated Ruby-only executable. Run
`./bootstrap-game.sh` to package it with Ruby and PartitionedArray, then
`./dist-game/run-game.sh path/to/game.rb`. See the [packaging guide](packaging.md).
Game/script entries accept Ruby source (`.rb`) or Selenite Ruby bytecode (`.rbc`);
no other game language is supported.

| How | What happens |
| --- | --- |
| `selenite-game file.rb [--grid save.json]` | Runs directly; defaults to `save.json` beside the script, without an organizer profile. |
| `selenite --game pong.rb` | Opens the game window against the last-used profile. |
| `selenite --game game.rb --profile NAME` | Uses that profile's grids. |
| `selenite --game game.rb --grid save.json` | Uses a specific grid file. |
| Right-click a `.rb` or `.rbc` cell → **Run as game** | Saves, starts the game in its own process and keeps Selenite responsive. When the game closes, Selenite reloads the grid if the game saved changes, such as collected items. |
| Right-click a `.rb` or `.rbc` cell → **Export standalone game** | Writes `<name>-game/` next to the script (see below). |

In the packaged build, run `./run.sh --game FILE.rb` from the bundle folder.
Replace `.rb` with `.rbc` to run a compiled entry in either executable.

## Compile and distribute Ruby bytecode

Both scripting-enabled executables compile Ruby without running the source:

```sh
./dist-game/run-game.sh --compile my-game/game.rb
./dist-game/run-game.sh my-game/game.rbc

# The organizer has the same compiler and accepts bytecode in both modes.
./dist/run.sh --compile my-game/game.rb --output my-game/release.rbc
./dist/run.sh --game my-game/release.rbc --grid my-game/save.json
./dist/run.sh --script my-game/release.rbc --grid my-game/save.json
```

The default output replaces the `.rb` suffix with `.rbc`. Output directories
must exist. Existing outputs are refused; pass `--force` to replace one.
A syntax error leaves any existing output intact. Compilation checks Ruby
syntax but does not run game code, open a window, or create a profile/save.
It compiles only the specified file; it does not collect assets, helpers,
gems, or game data.

Use the **same portable runtime** to compile and distribute. The `.rbc` file
contains MRI `RubyVM::InstructionSequence` bytecode inside a Selenite envelope
with format version, Ruby engine/version/patchlevel/platform/revision, payload
length, and SHA-256 checksum. Both runtimes reject incompatible or corrupted
files with an error. Recompile from your private source after changing the Ruby
runtime. These files are not Rubinius bytecode and cannot be launched by
`ruby game.rbc` or a stock Ruby `require`.

Keep your `.rb` originals privately. Distribute the compiled entry, runtime,
assets, and required data without the original source. For organizer exports,
add the `.rbc` file to a cell and choose **Export standalone game**: the export
contains `game.rbc` and its launcher selects that entry. Re-exporting replaces
the alternate `game.rb`/`game.rbc` entry. The portable game launcher and raw
`selenite-game` also prefer `game.rbc` over `game.rb` when no entry is supplied
(the launcher searches its package folder; the raw executable searches the
current directory). The bootstrap package still includes a source starter.

Compiled entry execution temporarily sets the working directory to the entry's
directory and restores it when evaluation finishes, including on exceptions.
The compiler stores a logical basename rather than your absolute build path.
`File.join(__dir__, "assets", "image.png")` and synchronous game callbacks
therefore work after relocation. Explicit grid-save paths resolve from the
launch directory before execution. Resolve paths you retain for later hooks
with `File.expand_path` while loading; a compiled method's `__dir__` is relative
to the current working directory. `__FILE__` retains the compiled logical
basename if you later rename the bytecode file.

Source helpers still work through `require_relative`. For compiled helpers,
compile each separately and explicitly load them through the bytecode API:

```ruby
Selenite::Bytecode.require(File.join(__dir__, "helpers.rbc"))
```

| API | Behavior |
| --- | --- |
| `Selenite::Bytecode.compile(source, output = nil, force = false)` | Compile `.rb` to `.rbc`; returns the absolute output path. |
| `Selenite::Bytecode.load(path)` | Validate and execute a `.rbc` file each time; returns its last value. |
| `Selenite::Bytecode.require(path)` | Execute a compiled helper once per resolved path; returns `true` on first success and `false` afterward. Failed loads can be retried. |
| `grid_eval_file(path)` | Evaluate `.rb` or `.rbc` in the current VM and grid context. |

For helpers in subdirectories, resolve any asset-directory constants with
`File.expand_path(__dir__)` during helper loading, before its loader restores
the parent working directory. Bundling helpers remains your responsibility.

Bytecode omits the original source text and comments, but **is not encryption
or protection against reverse engineering**: literals, names, and program
logic remain recoverable. Do not embed secrets. Load only trusted bytecode;
the checksum detects damage, not authenticity, and MRI's native loader is not
a security sandbox. `.rbc` compilation applies to Ruby games/scripts; Selenite's
Rust executables are already native binaries and are not converted to Ruby
bytecode.

### Standalone export

**Export standalone game** creates:

```text
pong-game/
  game.rb         copy of the source entry (game.rbc for a bytecode export)
  save.json       copy of the saved root hierarchy (world + inventories)
  assets/         copied from an assets/ folder next to the script, if any
  game-data/      copied from sibling game-data/ partitioned storage, if any
  runtime/        selenite-game preferred (+ Ruby and libs from a portable build)
  run-game.sh     launcher: ./run-game.sh
```

When Selenite itself runs from the portable bundle, the export includes
`run.sh`, `lib/` (libruby and the Ruby standard library), `licenses/` and
`partitioned_array/`. The folder can then run on a compatible Linux machine without a separate
Selenite or Ruby installation; system graphics and platform libraries are still required.
A scripting-enabled development build copies the organizer, a sibling
`selenite-game` executable when available, and the array library,
but still needs compatible system Ruby and other libraries. The launcher
falls back to `selenite` on your `PATH` only when no runtime executable is present.

Export does not gather every file referenced by a grid or rewrite absolute
paths. The script's sibling `assets/` and `game-data/` directories are copied automatically;
include additional Ruby and asset dependencies yourself. Save before export,
and test the export independently. See the [complete manual](manual.md#22-games-and-standalone-exports).

Games read and write `save.json` in their own folder when the game saves
(explicitly or through the Game helper's autosave), so progress can persist
between runs. Load assets relative to the script:
`File.join(__dir__, "assets", "ball.png")`.

## Multiplayer and P2P networking

Both executables include `Selenite::Network`: direct TCP peers and meshes,
host/join sessions with authoritative state and late-join snapshots, LAN
discovery, and shared organizer grids using local asset IDs. It works with
source and compiled `.rbc` games. Game-owned connections poll before updates
and close during shutdown:

```ruby
session = game.host(port: 45678, room: "my-game/v1", key: ENV.fetch("GAME_KEY"))
# On another game instance:
session = game.join("127.0.0.1", port: 45678, room: "my-game/v1", key: ENV.fetch("GAME_KEY"))
session.on("input") { |peer, data| puts "#{peer}: #{data.inspect}" }
```

Play the [Cooperative Gems example](../examples/games/multiplayer.rb), or follow
the [complete networking guide](networking.md) for two-terminal launch commands,
P2P meshes, message/state APIs, peer authentication, discovery, shared grids,
storage, limits, and Internet connectivity. TCP reachability is required;
automatic NAT traversal and relays are not included. Authentication does not
encrypt traffic; use an encrypted VPN for confidential Internet play.

## raylib-bindings style

```ruby
require "raylib"          # optional — returns false, Raylib is built in
include Raylib

InitWindow(800, 450, "Hello")
SetTargetFPS(60)
until WindowShouldClose()
  BeginDrawing()
    ClearBackground(RAYWHITE)
    DrawText("Hello from Selenite!", 190, 200, 20, LIGHTGRAY)
  EndDrawing()
end
CloseWindow()
```

- `require "raylib"`, `"raylib-bindings"` and `"raylib_bindings"` are
  no-ops. `Raylib.load_lib(...)` is accepted and ignored.
- Every function also has a snake_case alias: `Raylib.draw_text`,
  `begin_mode_2d`, `draw_fps`, `is_key_down`, …
- Vectors, rectangles and colours accept either the struct types or plain
  arrays (`[x, y]`, `[x, y, w, h]`).
- Only one window can be open at a time. Ruby exceptions close it cleanly
  and are printed with a backtrace.

## Selenite::Game

```ruby
Selenite::Game.run(title: "Demo", width: 960, height: 540) do |g|
  pos = Raylib::Vector2.create(480, 270)

  g.update do |dt|
    pos += g.axis.normalize * (240 * dt)       # WASD / arrow keys
    g.quit! if g.pressed?(Raylib::KEY_Q)
  end

  g.draw    { g.circle(pos.x, pos.y, 20, Raylib::MAROON) }   # world (camera) space
  g.draw_ui { g.text("FPS #{Raylib.GetFPS}", 10, 10) }      # screen space
  g.every(5) { puts "five seconds passed" }
end
```

Constructor options:

| Option | Default | Meaning |
| --- | --- | --- |
| `title` | `"Selenite game"` | Window title |
| `width`, `height` | 960, 540 | Initial size |
| `fps` | 60 | Target FPS |
| `background` | `RAYWHITE` | Clear colour (also `g.background = ...`) |
| `audio` | `true` | Open the audio device |
| `resizable` | `true` | `FLAG_WINDOW_RESIZABLE` |
| `exit_key` | `KEY_ESCAPE` | Key that closes the game (`nil` for none) |
| `autosave` | `true` | Save the grid (and inventories) when the game ends |

Methods:

| Method | Description |
| --- | --- |
| `update { \|dt\| }`, `draw { }`, `draw_ui { }`, `on_close { }` | Add callbacks. You can register several of each. `draw` runs inside `BeginMode2D(g.camera)` when a camera is set. |
| `camera`, `camera=` | Optional `Raylib::Camera2D` |
| `every(s) { }`, `after(s) { }` | Repeating / one-shot timers in game time. Late timers catch up, at most 8 runs per frame. |
| `quit!`, `quit?` | End the loop after this frame |
| `key?(k)`, `pressed?(k)`, `released?(k)`, `click?(button)`, `mouse` | Input shortcuts |
| `axis` | `Vector2` from WASD and the arrow keys |
| `text(str, x, y, size:, color:, center:)`, `rect`, `circle` | Drawing shortcuts |
| `texture(path)` | Loads a texture once and caches it. The cache is released when the game closes. |
| `cell_texture(col, row)` | Texture of the image in a grid cell, or `nil` |
| `grid`, `inventory`, `save` | The current Selenite grid, its inventory, and a save |
| `frame`, `elapsed`, `screen_width`, `screen_height` | Loop state |

## Using grids and inventories

The launched profile or save starts at its root grid, even when the organizer
was displaying a nested grid. Select a nested level explicitly through the grid
API when needed. Each grid has its own [inventory](inventory.md), saved with it:

```ruby
Selenite::Game.run(title: "Loot") do |g|
  chests = g.grid.cells.select { |c| c.kind == "image" }
  g.update do
    if g.pressed?(Raylib::KEY_SPACE) && (c = chests.pop)
      g.inventory.add(File.basename(c.path))
      g.inventory.add("gold", 10)
      g.inventory.set_meta("gold", "last_from", [c.col, c.row])
    end
  end
  g.draw_ui do
    g.inventory.each.with_index do |(name, count, _meta), i|
      g.text("#{name} x#{count}", 10, 10 + i * 24)
    end
  end
end
```

Stacked cells are available through `g.grid.items(col, row)` (top first).

## Partitioned storage, BSP, and revenants

Use `g.storage` for persistent game records backed by the bundled PartitionedArray:

```ruby
store = g.storage("campaign")
actors = store.table("actors")
id = actors.insert(kind: "enemy", hp: 30)
actors.update(id) { |actor| actor["hp"] -= 5 }
g.save

partition = actors.partition_for(id)
actors.unload_partition(partition) # Saved, then released from memory
actor = actors[id]                # Reloads its partition
p actors.status(partition)        # :revenant
```

This snippet belongs inside a game setup block; insert initial records only once
in a persistent game. Tables provide stable IDs, metadata, component queries,
automatic growth, and standalone checkpoints. `Selenite::BSP` divides world
space into rooms; `Selenite::PartitionedWorld` maps rooms to tables and streams
their data around a camera. Ruby already supports general-purpose computation;
these abstractions provide persistence and spatial organization for game logic.

See the complete [partitioned storage guide](game-storage.md) for setup, every
API, revenant semantics, saving, grid integration, export, and limitations.
Run [Revenant World](../examples/games/revenant_world.rb) for a playable example.
These APIs require rebuilding the updated scripting-enabled runtime.

## API reference

**Window and timing:** InitWindow, CloseWindow, WindowShouldClose, IsWindowReady,
SetTargetFPS, SetConfigFlags, SetTraceLogLevel, GetFrameTime, GetTime,
GetFPS, GetScreenWidth, GetScreenHeight, GetScreenSize, SetWindowTitle,
SetWindowSize, ToggleFullscreen, SetExitKey, ShowCursor, HideCursor,
TakeScreenshot, GetRandomValue, SetRandomSeed.

**Drawing:** BeginDrawing, EndDrawing, ClearBackground, BeginMode2D,
EndMode2D, DrawPixel, DrawLine, DrawLineEx, DrawCircle, DrawCircleV,
DrawCircleLines, DrawEllipse, DrawRectangle, DrawRectangleV,
DrawRectangleRec, DrawRectangleLines, DrawRectangleLinesEx,
DrawRectangleRounded, DrawRectangleGradientV, DrawTriangle, DrawPoly,
DrawText, MeasureText, DrawFPS.

**Textures:** LoadTexture (`Texture2D` with `id`, `width`, `height`,
`unload`), UnloadTexture, DrawTexture, DrawTextureV, DrawTextureEx,
DrawTextureRec, DrawTexturePro.

**Input:** IsKeyPressed, IsKeyDown, IsKeyReleased, IsKeyUp, GetKeyPressed,
GetCharPressed, IsMouseButtonPressed, IsMouseButtonDown,
IsMouseButtonReleased, GetMouseX, GetMouseY, GetMousePosition,
GetMouseWheelMove, SetMousePosition. Constants: `KEY_*` (letters, digits,
F1–F12, keypad, arrows, specials) and `MOUSE_BUTTON_*`.

**Audio:** InitAudioDevice, CloseAudioDevice, IsAudioDeviceReady,
SetMasterVolume, LoadSound, UnloadSound, PlaySound, StopSound,
IsSoundPlaying, SetSoundVolume, LoadMusicStream, UnloadMusicStream,
PlayMusicStream, UpdateMusicStream, StopMusicStream, PauseMusicStream,
ResumeMusicStream, IsMusicStreamPlaying, SetMusicVolume,
GetMusicTimeLength, GetMusicTimePlayed.

**Math and collision (pure Ruby):** CheckCollisionRecs, CheckCollisionCircles,
CheckCollisionPointCircle, CheckCollisionPointRec, CheckCollisionCircleRec,
GetCollisionRec, Vector2Add, Vector2Subtract, Vector2Scale, Vector2Length,
Vector2Distance, Vector2Normalize, Vector2Lerp, Clamp, Lerp, Remap, Fade,
ColorAlpha.

**Types:** `Vector2` (`+`, `-`, `*`, `length`, `normalize`), `Rectangle`
(`center`), `Color` (`fade`), `Camera2D.create(offset:, target:, rotation:,
zoom:)`, `Texture2D`, `Sound`, `Music`. All have `.create(...)` like
raylib-bindings.

**Colours:** LIGHTGRAY, GRAY, DARKGRAY, YELLOW, GOLD, ORANGE, PINK, RED,
MAROON, GREEN, LIME, DARKGREEN, SKYBLUE, BLUE, DARKBLUE, PURPLE, VIOLET,
DARKPURPLE, BEIGE, BROWN, DARKBROWN, WHITE, BLACK, BLANK, MAGENTA, RAYWHITE.

**Flags:** `FLAG_WINDOW_RESIZABLE`, `FLAG_VSYNC_HINT`, `FLAG_MSAA_4X_HINT`,
`FLAG_FULLSCREEN_MODE`, …; log levels `LOG_ALL` … `LOG_NONE`.

## Headless runs and screenshots

Two environment variables make unattended runs and CI easy:

| Variable | Effect |
| --- | --- |
| `SELENITE_GAME_FRAMES=N` | `WindowShouldClose()` returns true after N frames. |
| `SELENITE_GAME_SCREENSHOT=shot.png` | Saves the final frame to that file (relative to the working directory, or an absolute path). |

`TakeScreenshot(path)` accepts relative or absolute paths too, and returns
`true` once the file is written.

```sh
SELENITE_GAME_FRAMES=90 SELENITE_GAME_SCREENSHOT=pong.png selenite --game examples/games/pong.rb
```

## Examples

- [Revenant World](../examples/games/revenant_world.rb): BSP rooms, streamed partitions labeled as revenants, persistent gems and player state, and checkpoints. See the [storage guide](game-storage.md).

- [Gem Trail](../examples/games/gem_trail.rb): the [tutorial game](game-tutorial.md), with persistent pickups, restart, and optional image-cell levels.

- [`examples/games/pong.rb`](../examples/games/pong.rb): classic two-paddle
  Pong written purely in raylib-bindings style.
- [`examples/games/collector.rb`](../examples/games/collector.rb): a
  `Selenite::Game` that turns the current grid into a level. Image cells are
  drawn with their pictures. Picking items up adds them to the grid's
  inventory, which you can then see with **B** in Selenite.

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

- [Running a game](#running-a-game)
- [raylib-bindings style](#raylib-bindings-style)
- [Selenite::Game](#selenitegame)
- [Using grids and inventories](#using-grids-and-inventories)
- [API reference](#api-reference)
- [Headless runs and screenshots](#headless-runs-and-screenshots)
- [Examples](#examples)

## Running a game

| How | What happens |
| --- | --- |
| `selenite --game pong.rb` | Opens the game window against the last-used profile. |
| `selenite --game game.rb --profile NAME` | Uses that profile's grids. |
| `selenite --game game.rb --grid save.json` | Uses a specific grid file. |
| Right-click a `.rb` cell → **Run as game** | Saves, starts the game in its own process and keeps Selenite responsive. When the game closes, Selenite reloads the grid if the game saved changes, such as collected items. |
| Right-click a `.rb` cell → **Export standalone game** | Writes `<name>-game/` next to the script (see below). |

In the packaged build, run `./run.sh --game FILE.rb` from the bundle folder.

### Standalone export

**Export standalone game** creates:

```text
pong-game/
  game.rb         copy of the script
  save.json       snapshot of the grid you were viewing (world + inventory)
  assets/         copied from an assets/ folder next to the script, if any
  runtime/        Selenite binary (+ bundled Ruby and libs from a portable build)
  run-game.sh     launcher: ./run-game.sh
```

When Selenite itself runs from the portable bundle, the export includes
`run.sh`, `lib/` (libruby and the Ruby standard library), `licenses/` and
`partitioned_array/`. The folder then runs on another Linux machine with no
other installs. A development build copies just the binary. In that case
`run-game.sh` falls back to `selenite` on your `PATH` when no runtime is
present.

Games read and write `save.json` in their own folder, so progress persists
between runs. Load assets relative to the script:
`File.join(__dir__, "assets", "ball.png")`.

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

The grid you launched from becomes your level data. Each grid also has its
own [inventory](inventory.md), which is saved with it:

```ruby
Selenite::Game.run(title: "Loot") do |g|
  chests = g.grid.cells.select { |c| c.kind == "image" }
  g.update do
    if g.pressed?(Raylib::KEY_SPACE) && (c = chests.pop)
      g.inventory.add(File.basename(c.path))
      g.inventory.set_meta("gold", "last_from", [c.col, c.row])
      g.inventory.add("gold", 10)
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

- [`examples/games/pong.rb`](../examples/games/pong.rb): classic two-paddle
  Pong written purely in raylib-bindings style.
- [`examples/games/collector.rb`](../examples/games/collector.rb): a
  `Selenite::Game` that turns the current grid into a level. Image cells are
  drawn with their pictures. Picking items up adds them to the grid's
  inventory, which you can then see with **B** in Selenite.

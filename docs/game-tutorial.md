# Build your first Selenite game: Gem Trail

This tutorial takes you from an empty window to a playable collection game using Ruby and Selenite's built-in game API. You will learn the game loop, frame-independent movement, collision detection, a following camera, a screen-space HUD, grid-authored levels, persistent inventories, and exporting.

The finished game needs **no downloaded graphics, gems, or audio files**. A blue player collects eight gold gems. WASD or arrow keys move; R restarts; Escape saves and exits. Collected gems stay collected when you reopen the same save. You can later replace the default level with image cells in a Selenite grid.

**Prerequisites:** basic Ruby variables, arrays, hashes, and blocks. You do not need prior raylib experience. Allow roughly an hour for the core lessons and additional time to experiment.

- [1. Prepare a practice project](#1-prepare-a-practice-project)
- [2. Open a window](#2-open-a-window)
- [3. Make a player move](#3-make-a-player-move)
- [4. Run the complete game](#4-run-the-complete-game)
- [5. Understand world space and the camera](#5-understand-world-space-and-the-camera)
- [6. Build pickups and detect collisions](#6-build-pickups-and-detect-collisions)
- [7. Draw a HUD and finish the level](#7-draw-a-hud-and-finish-the-level)
- [8. Save progress and restart correctly](#8-save-progress-and-restart-correctly)
- [9. Design a level in Selenite](#9-design-a-level-in-selenite)
- [10. Add your own graphics and sound](#10-add-your-own-graphics-and-sound)
- [11. Test and debug](#11-test-and-debug)
- [12. Export and share](#12-export-and-share)
- [13. Challenges and next steps](#13-challenges-and-next-steps)

The [game API reference](game.md) documents the supported bindings. The complete, commented source is [gem_trail.rb](../examples/games/gem_trail.rb).

## 1. Prepare a practice project

Use a **scripting-enabled Selenite build**. The portable Linux distribution already enables scripting. For a source build, install the prerequisites in the [manual](manual.md#2-install-and-start), then run:

```sh
cargo build --release --features scripting
```

Do not run these scripts using `ruby game.rb`: Selenite injects the `Selenite` and `Raylib` modules when it launches them. No raylib Ruby gem is required.

Commands below assume you are in the **repository root** and have the portable runtime in `dist/`. Substitute the executable as follows:

| Setup | Replace `./dist/run.sh` with |
| --- | --- |
| Repository with packaged runtime | Keep `./dist/run.sh` |
| Extracted portable distribution, terminal in its root | `./run.sh` |
| Scripting-enabled source build | `./target/release/selenite` |
| Installed on PATH | `selenite` |

Create a practice folder:

```sh
mkdir -p tutorial/gem-trail
```

You will edit `tutorial/gem-trail/game.rb` and use `tutorial/gem-trail/save.json` for data. The save does not have to exist initially. Using `--grid` explicitly keeps tutorial progress separate from your last-used profile.

The final sample writes two inventory entries: `gem_trail_gems` and `gem_trail_progress`. Keep this save dedicated to the tutorial, and close the organizer before running a game that writes to it.

## 2. Open a window

Create `tutorial/gem-trail/game.rb` with this complete program:

```ruby
Selenite::Game.run(title: "My first Selenite game",
                   width: 960, height: 600,
                   audio: false, autosave: false) do |g|
  g.draw_ui do
    g.text("Hello, game developer!", 30, 30)
    g.text("Press Escape to close the window.", 30, 70)
  end
end
```

Run it from the repository root:

```sh
./dist/run.sh --game tutorial/gem-trail/game.rb --grid tutorial/gem-trail/save.json
```

**Checkpoint:** a window displays both lines; Escape closes it. No game data needs saving yet, so this first program disables autosave. `audio: false` avoids opening an audio device for a silent game.

`Selenite::Game.run` creates the window and handles the frame loop. The setup block runs once; callbacks registered inside it run each frame. Keep a single `Game.run` block in your program.

| Callback | Use it for |
| --- | --- |
| `g.update { |dt| ... }` | Input, movement, collision, and state changes |
| `g.draw { ... }` | World objects, transformed by the game camera |
| `g.draw_ui { ... }` | HUD and menus in screen coordinates |
| `g.on_close { ... }` | Your cleanup work before shutdown |

The helper clears the window each frame and handles BeginDrawing/EndDrawing. You do not need a second `until WindowShouldClose` loop inside it.

## 3. Make a player move

Replace the entire practice script with this complete second version:

```ruby
Selenite::Game.run(title: "Moving player", width: 960, height: 600,
                   audio: false, autosave: false,
                   background: Raylib::DARKBLUE) do |g|
  player = Raylib::Vector2.create(480, 300)
  speed = 220.0

  g.update do |dt|
    direction = g.axis.normalize
    player += direction * (speed * [dt, 0.05].min)
    player.x = player.x.clamp(14, [g.screen_width - 14, 14].max)
    player.y = player.y.clamp(14, [g.screen_height - 14, 14].max)
  end

  g.draw do
    Raylib.DrawCircleV(player, 14, Raylib::SKYBLUE)
  end

  g.draw_ui do
    g.text("WASD / arrows: move    Esc: quit", 18, 18,
           color: Raylib::RAYWHITE)
  end
end
```

Use the same launch command. The player should move in eight directions and stay inside the window.

Three details make movement predictable:

1. `g.axis` reads WASD and the arrow keys as a vector. For example, right is `(1, 0)` and up is `(0, -1)`.
2. `.normalize` keeps diagonal movement from being faster. A zero input vector stays zero; it does not cause a divide-by-zero error.
3. `dt` is the elapsed time in seconds for the frame. Multiplying by pixels per second makes movement broadly consistent across frame rates.

The sample caps the movement timestep at 0.05 seconds. A long stall therefore slows simulation instead of moving the player a huge distance in one frame. This is useful for a simple collection game; it is not a fixed-step physics simulation.

**Experiment:** change `speed` to 110, then 330. Try holding two movement keys together. Resize the window. Restore 220 before continuing.

## 4. Run the complete game

The remaining lessons explain the final program by responsibility. Start from its working source rather than assembling disconnected fragments:

```sh
cp examples/games/gem_trail.rb tutorial/gem-trail/game.rb
./dist/run.sh --game tutorial/gem-trail/game.rb --grid tutorial/gem-trail/save.json
```

This overwrites the practice script from lessons 2–3. If you want to keep those versions, save copies first.

**Checkpoint:** the title is Gem Trail, the player is centered over a blue grid, the HUD shows `0 / 8 gems`, and gold pickups appear around you. Move right to reach the first gem at `(2, 0)`. The counter increases and that gem disappears. The nearest-gem display helps you find pickups outside the viewport.

Unlike lesson 3, the world is no longer bounded by the window. The camera follows the player; the player can travel to any pickup coordinate.

The final source is organized in this order:

| Section | Responsibility |
| --- | --- |
| `GemTrail` constants | Tile size, speed, collision radius, inventory keys, default level |
| Setup before callbacks | Choose the level, load textures, restore pickup IDs, create player |
| `save_progress` lambda | Write this game's inventory and save |
| `g.update` | Restart, movement, pickups, camera tracking |
| `g.draw` | Floor grid, remaining gems/images, player |
| `g.draw_ui` | Count, controls, nearest pickup, messages, win banner |

Open [the commented source](../examples/games/gem_trail.rb) while reading lessons 5–8. Snippets in those lessons explain code already in the final file; **do not paste a second copy of those callbacks** into it.

## 5. Understand world space and the camera

The tutorial treats each Selenite coordinate as a world position spaced 64 pixels apart:

```ruby
world_x = col * GemTrail::TILE
world_y = row * GemTrail::TILE
```

Thus `(2, -1)` is `(128, -64)` in game world pixels. This is a mapping you define, not a mandatory Selenite rendering scale. In this game a pickup is centered on that coordinate.

The player starts at world `(0, 0)`. The camera tracks it:

```ruby
g.camera = Raylib::Camera2D.create(target: player)

# Inside the existing update callback:
g.camera.target = player
g.camera.offset = Raylib::Vector2.create(g.screen_width / 2.0,
                                         g.screen_height / 2.0)
```

`target` is the world position the camera looks at. `offset` places that position in the center of the screen. Updating offset every frame handles resizing. Because `player += ...` creates a new vector, reassign the camera target after movement.

Everything in `g.draw` uses this camera. Everything in `g.draw_ui` stays in screen space. That is why the floor moves while the counter stays at the top-left.

The final sample draws only floor lines near the player instead of attempting to draw an infinite grid. For a large game, apply the same visibility idea to world objects and textures.

**Checkpoint:** walk left and upward. World coordinates become negative, the grid moves, and the HUD stays still. Resize the window: the player remains centered.

## 6. Build pickups and detect collisions

Each pickup is a Ruby hash:

```ruby
{
  id: "2,0",
  col: 2,
  row: 0,
  rect: Raylib::Rectangle.create(111, -17, 34, 34),
  texture: nil
}
```

The ID identifies progress independently of array order. The rectangle is centered on `(128, 0)`. A missing texture means “draw a gold shape”; an available texture means “draw the image”.

The player is a circle. Each update tests uncollected items:

```ruby
next if collected.include?(item[:id])
next unless Raylib.CheckCollisionCircleRec(player, GemTrail::RADIUS, item[:rect])

collected << item[:id]
```

The player and pickup both use **world coordinates**. Do not mix the screen center with the pickup's world rectangle, or collisions will stop matching what you see after the camera moves.

The sample uses `changed` to save once after the loop if one or more pickups were collected during that frame. Drawing skips collected IDs. It leaves the original cells and files alone: the game changes progress metadata, not the authored level layout.

This is overlap-based collision detection. It is sufficient at the sample's speed and size, but it does not sweep through the path between frames. Very fast projectiles and thin obstacles need smaller simulation steps or swept collision tests.

**Experiment:** change `RADIUS` or the pickup's rectangle dimensions and see how contact changes. To add walls, create a separate wall list and reject a proposed player movement when its circle overlaps a wall. Pickups should still award gems rather than block movement.

## 7. Draw a HUD and finish the level

The HUD belongs in `g.draw_ui`. Its counter comes from actual game state:

```ruby
g.text("GEM TRAIL   #{collected.length} / #{pickups.length} gems",
       18, 12, size: 26, color: Raylib::GOLD)
```

Remaining pickups are those whose IDs are absent from `collected`. The nearest one is found by comparing squared distances; no square root is needed merely to choose the smallest distance.

The level is complete when `remaining.empty?`. The final sample then displays a win banner and stops player movement. It continues processing R so the user can restart. Pressed input is important here: `g.pressed?(KEY_R)` triggers once per press, whereas `g.key?(KEY_R)` would restart on every frame the key is held.

Pickup messages expire using an absolute game-time deadline:

```ruby
message = "Gem collected!"
message_until = g.elapsed + 1.5
```

Draw the message only while `g.elapsed < message_until`. Replacing the deadline means an older pickup cannot hide a newer message early. The game API also provides `g.after(seconds)` and `g.every(seconds)` for callbacks; keep callbacks short because they run in the game loop.

**Checkpoint:** collect all eight gems. You should see `LEVEL COMPLETE`, no remaining pickups, and an R restart instruction.

## 8. Save progress and restart correctly

A Ruby local variable does not persist when the process closes. Selenite saves the grid and its inventory; explicitly put the state you need there.

Gem Trail uses these entries:

| Entry | Meaning |
| --- | --- |
| `gem_trail_gems` | Count of pickups collected in the current level |
| `gem_trail_progress` | A count-one marker whose `collected` metadata holds pickup IDs |

The save lambda creates the marker **before** setting metadata:

```ruby
inventory[GemTrail::PROGRESS] = 1
inventory.set_meta(GemTrail::PROGRESS, "collected", collected.sort)
inventory[GemTrail::BAG] = collected.length
g.save
```

Setting an inventory count to zero removes that entry. Consequently, after a fresh start you may see only the progress marker in the organizer's Items panel; the gem-count entry appears after the first pickup. Inventory totals include the marker, so the HUD uses the collected-ID count rather than `inventory.total`.

On startup, the program reads:

```ruby
saved = inventory.meta(GemTrail::PROGRESS)["collected"]
valid_ids = pickups.map { |item| item[:id] }
collected = saved.is_a?(Array) ? (saved & valid_ids) : []
```

Intersecting with valid IDs drops stale or duplicate IDs. Both the count and the IDs are necessary: saving only “three gems” would let all three respawn on the next launch.

The example saves on startup, after pickups, and after restart, with the Game helper's normal autosave on close as a final save. For larger projects, batching frequent saves can reduce disk work. A failed save should be investigated rather than treated as durable progress.

### Verify persistence

1. Collect the first gem, then press Escape.
2. Run the same command with the same `--grid` path.
3. Confirm the count is preserved and that gem is absent.
4. Press R. Confirm the count resets, the player returns to origin, and the gems reappear.
5. Quit and reopen again. The reset must also persist.

Only pickup progress persists in this example. Player position is reset on launch. Pressing R resets the two tutorial inventory entries, not other inventory items or any grid cells.

After quitting the game, inspect the save in the organizer:

```sh
./dist/run.sh --grid tutorial/gem-trail/save.json
```

Press **B** to inspect its inventory. Close the organizer before relaunching the game; concurrent writers to the same save can overwrite each other's work.

## 9. Design a level in Selenite

The completed game treats **top-level image cells** as pickups. If there are no top-level image cells, it uses the built-in eight-gem layout. Other file types and nested-grid cells are ignored.

1. Quit the game and open its save using the organizer command above.
2. Place a few images near the origin, for example at `(2, 0)`, `(3, -2)`, and `(-2, 1)`. Use the selected cell's coordinate label to check positions.
3. Keep only the intended pickup image on top of each relevant stack. The tutorial reads `g.grid.cells`, which exposes top items.
4. Save and close the organizer.
5. Run the game with the same save path. It uses your image layout rather than mixing it with the default gems.
6. Press R to start the changed layout from scratch.

The loader is:

```ruby
cells = g.grid.cells.select { |cell| cell.kind == "image" }
locations = cells.empty? ? GemTrail::DEFAULT_LEVEL : cells.map { |c| [c.col, c.row] }
```

A game's initial grid context is the **root** of the selected save/profile, even if the organizer was showing a nested grid when you launched it. To adapt the sample to a nested level, choose a grid explicitly during setup:

```ruby
level = g.grid.subgrid(5, 0)
raise "Create a nested level at (5, 0) first" unless level
cells = level.cells.select { |cell| cell.kind == "image" }
# In the texture-loading loop, use:
# g.cell_texture(cell.col, cell.row, level)
```

If you also want progress attached to that level, use `inventory = level.inventory`. `g.save` still saves the root hierarchy containing it. These changes belong in the existing setup section, not in a second game loop.

Pickup IDs use coordinates. Moving an image changes its pickup identity; replacing the image at the same coordinate retains that identity. Restart after editing a level. Games with multiple levels should include a level ID or version in their progress keys.

## 10. Add your own graphics and sound

### Textures

The sample loads pickup textures once during setup with `g.cell_texture`; load errors fall back to gold rectangles and print a diagnostic. `g.texture(path)` caches textures and the helper releases cached resources at shutdown. Do not reload every image each frame.

For a dedicated sprite, keep an `assets/` directory beside your game script:

```text
 tutorial/gem-trail/
   game.rb
   save.json
   assets/
     player.png
```

Load it during setup, inside the existing `Game.run` block:

```ruby
player_texture = g.texture(File.join(__dir__, "assets", "player.png"))
```

Replace the player's circle drawing in `g.draw` with:

```ruby
Raylib.DrawTexturePro(player_texture,
                     [0, 0, player_texture.width, player_texture.height],
                     [player.x - 16, player.y - 16, 32, 32],
                     [0, 0], 0.0, Raylib::WHITE)
```

The collision shape can remain a circle even if the sprite is square. This snippet stretches the sprite to 32×32; preserve its aspect ratio if your art needs a different shape.

### Sound

The supplied game intentionally runs with `audio: false`. To add a pickup sound:

1. Put a valid `pickup.wav` in the script's assets folder.
2. Change the Game option to `audio: true`.
3. Add the following during setup:

```ruby
pickup_sound = Raylib.LoadSound(File.join(__dir__, "assets", "pickup.wav"))
g.on_close { Raylib.UnloadSound(pickup_sound) }
```

4. After a successful pickup, call `Raylib.PlaySound(pickup_sound)`.

The Game helper opens the audio device; you do not also need to initialize it. The close callback unloads the sound before the helper closes audio. A looping music stream additionally needs `UpdateMusicStream` each frame; consult the [audio API](game.md#api-reference).

## 11. Test and debug

Run from a terminal so Ruby exceptions include their backtrace. If the window exits immediately, read that output first. `ruby -c tutorial/gem-trail/game.rb` can check syntax when Ruby is installed, but it cannot confirm that Selenite-specific methods work; launch through Selenite for runtime testing.

Use this test checklist after changes:

| Test | Expected result |
| --- | --- |
| Fresh dedicated save | Eight default gems, count zero |
| Hold two movement directions | Same speed as one direction |
| Resize while moving | Camera stays centered; HUD stays on screen |
| Touch one gem and remain there | Exactly one award |
| Quit and relaunch | Collected gems remain absent |
| Collect everything | Win banner; no additional movement until restart |
| R, then quit and relaunch | Fresh run remains fresh |
| Add a custom image layout | Correct image count and world positions |
| Missing custom image file | Gold fallback and terminal diagnostic |

The HUD targets ordinary desktop window sizes; very narrow windows may clip text. Add responsive wrapping if your game needs a smaller minimum size.

For a short unattended render:

```sh
SELENITE_GAME_FRAMES=90 SELENITE_GAME_SCREENSHOT="$PWD/tutorial/gem-trail/preview.png" \
  ./dist/run.sh --game tutorial/gem-trail/game.rb --grid tutorial/gem-trail/save.json
```

This requires a working graphics display/context. It checks startup and drawing, not keyboard input, collision, or a whole playthrough. The final sample saves during such a run too, so use a test save. Repeatable gameplay tests should also verify the inventory after simulated or manual input.

| Problem | Likely cause or next check |
| --- | --- |
| `uninitialized constant Selenite` | Launched with ordinary Ruby instead of `selenite --game` |
| Scripting unavailable | Use the portable launcher or build with the scripting feature |
| No inventory change in the organizer | Different --grid path, unsaved game, or a stale organizer window |
| Cannot set metadata | Add/set the inventory item before calling `set_meta` |
| Pickups respawn after relaunch | Persist and restore IDs, not just the count |
| Player collides away from the picture | Mixed world and screen coordinates |
| HUD scrolls with player | Draw it with `draw_ui`, outside the world camera |
| Custom level does not load | Image is nested or below another stack item; sample reads root top items |
| Sprite cannot be found after moving game | Resolve script assets through `__dir__`; check saved absolute paths |
| Game freezes | A loop or expensive operation inside a callback blocks the frame |

## 12. Export and share

First get the default, shape-only level working. It has no external asset dependencies, which makes an initial export easier to verify.

1. Save and quit the game.
2. Open its save in Selenite.
3. If your script needs sibling assets, set **Copy In: Off** before adding the script, so it remains referenced at its project location instead of being copied away from those assets.
4. Drop the script into an empty cell. A Ruby cell is not an image pickup, so it does not change this game's level selection.
5. Save, right-click the script, and choose **Export standalone game**.
6. Open the generated `<script-stem>-game/` directory beside the script and run `./run-game.sh`.
7. Test movement, collection, quit/reopen, and R reset in the exported copy.

The export contains the script, a copy of the saved root hierarchy, a sibling assets folder when present, the runtime, and the launcher. Progress present at export time is copied as well; reset the game before producing a fresh-player build.

A grid-authored level can still reference images at absolute paths in your original profile. Export does not automatically copy those referenced files or rewrite the paths. For a portable custom level, collect its assets beside the script and adapt the loader/save references accordingly; test on a machine or location without access to the original files. Extra Ruby files loaded through `require_relative` also need to be included.

The portable runtime bundles Ruby, but requires a compatible operating system, system libraries, and graphics support. See [standalone export details](game.md#standalone-export) before distributing.

## 13. Challenges and next steps

Try one change at a time and repeat the relevant tests:

1. **Sprint:** double speed while Shift is held. Keep diagonal normalization.
2. **Walls:** use a separate cell kind or metadata to author obstacles; test movement one axis at a time so the player can slide along walls.
3. **A countdown challenge:** accumulate simulation time in `update`, display it in `draw_ui`, and add explicit playing/won/lost states. Decide whether pause time should count.
4. **Multiple levels:** select a nested grid per level and namespace pickup IDs and inventory entries by level.
5. **Save position:** store `[player.x, player.y]` in metadata, validate it on load, and define where R returns the player.
6. **A menu:** use `pressed?` for one-off choices, keep the HUD separate from world drawing, and disable movement when the menu is open.
7. **Scale up:** follow the [partitioned storage and BSP guide](game-storage.md) to stream rooms, revive partitions, and checkpoint game data. The [Revenant World example](../examples/games/revenant_world.rb) demonstrates these systems. The tutorial's array lookups and per-pickup saves favor simplicity over large-world throughput.

Continue with [the game API reference](game.md), [inventories and stacks](inventory.md), [plugin development](plugins.md), and the [complete Selenite manual](manual.md). The existing [Pong](../examples/games/pong.rb) and [Collector](../examples/games/collector.rb) examples show alternative loops and world layouts.

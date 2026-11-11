# Complete Native Raylib in Selenite

Selenite's managed `Raylib` module provides the existing 2D game helpers and
managed texture/audio handles. `Selenite.raylib_native` exposes all 600 functions,
35 record types and 305 enumeration constants from the linked raylib 6 public
header. Every function has its original CamelCase name and a snake_case alias.
This reference covers raylib.h, not the separate raygui or rlgl libraries.

## Native Safety and Ownership

Native calls have C's ownership and lifetime rules. Run native programs with
`selenite --game file.rb`, or IDE Game mode, and explicitly call `enable!`.
The IDE console rejects native calls to protect its own window. Invalid pointers,
double unloads or callbacks from foreign threads can crash a native program.
Only run trusted code. Do not mix native records with managed resource handles.
Native resources must be explicitly unloaded before closing their window/audio
device. A record is not a garbage-collected owner of its underlying C resource.

## Window and 2D Drawing

```ruby
api = Selenite.raylib_native.enable!
api.InitWindow(800, 450, "Native Selenite")
api.SetTargetFPS(60)
begin
  until api.WindowShouldClose
    api.BeginDrawing
    api.ClearBackground(Raylib::RAYWHITE)
    api.DrawCircleV(api::Vector2.new(400, 225), 60, Raylib::SKYBLUE)
    api.DrawText("Complete raylib API", 20, 20, 24, Raylib::DARKGRAY)
    api.EndDrawing
  end
ensure
  api.CloseWindow
end
```

## Input, Movement and Collision

Read `IsKeyDown`, `IsKeyPressed`, `GetMousePosition` and `GetFrameTime` once per
update. Multiply speed by frame time for consistent movement. Use
`CheckCollisionRecs`, `CheckCollisionCircles`, `GetRayCollisionBox` and related
queries instead of embedding collision checks into drawing callbacks.

```ruby
position = api::Vector2.new(100, 100)
position.x += 180 * api.GetFrameTime if api.IsKeyDown(api::KEY_RIGHT)
hit = api.CheckCollisionPointRec(position, api::Rectangle.new(200, 80, 60, 120))
```

## Textures, Render Targets and Shaders

Load a native texture with `LoadTexture`, inspect its width and height, and draw
it using `DrawTexturePro`. `LoadRenderTexture` creates a GPU render target;
pair `BeginTextureMode`/`EndTextureMode` to draw into it. `LoadShader`,
`BeginShaderMode` and `SetShaderValue` provide custom GPU effects. Always pair
begin/end calls and unload textures, render targets and shaders before closing
the window. For shader uniform buffers, use an explicitly allocated pointer.

## Cameras and 3D

```ruby
camera = api::Camera3D.new(
  position: api::Vector3.new(6, 6, 6),
  target: api::Vector3.new(0, 0, 0),
  up: api::Vector3.new(0, 1, 0), fovy: 45,
  projection: api::CAMERA_PERSPECTIVE
)
api.BeginMode3D(camera)
api.DrawCube(api::Vector3.new(0, 0, 0), 2, 2, 2, Raylib::RED)
api.DrawGrid(10, 1)
api.EndMode3D
```

Use `LoadModel`/`DrawModel` for models, `GetRayCollisionMesh` for mesh queries,
and the model animation functions for skeletal animation. Mesh and model records
contain raw array pointers; preserve their lifetime and unload the owning model
only once. The complete reference lists every model and animation function.

## Audio

Call `InitAudioDevice` before loading sounds or music. `PlaySound` is suitable
for effects. Stream music using `LoadMusicStream`, `PlayMusicStream`, and
`UpdateMusicStream` each frame. Unload the streams and sounds before
`CloseAudioDevice`. Audio callbacks execute on native audio threads: do not call
Ruby directly from those callbacks. Use a native extension for realtime callbacks.

## Records, Pointers and Output Parameters

Native records accept positional or keyword fields and provide mutable field
accessors. Missing fields are zero-initialized. Nested records and fixed-size
arrays preserve their C layout through generated field conversions. Returned
raw pointers are `Raylib::Native::Pointer` values with `address` and `null?`.
C string inputs accept Ruby strings; returned const strings are copied into Ruby.
Owned `char *` outputs remain pointers so their matching unload functions work.

```ruby
require "fiddle"
count = Fiddle::Pointer.malloc(Fiddle::SIZEOF_INT, Fiddle::RUBY_FREE)
count[0, Fiddle::SIZEOF_INT] = [0].pack("i!")
bytes = api.LoadFileData("assets/data.bin", api::Pointer.new(count.to_i))
begin
  length = count[0, Fiddle::SIZEOF_INT].unpack1("i!")
  data = Fiddle::Pointer.new(bytes.address)[0, length] unless bytes.null?
ensure
  api.UnloadFileData(bytes) unless bytes.null?
end
```

Keep buffers and callback closures alive for as long as C uses their addresses.
Pass `nil` for a null pointer or callback. Callback inputs accept native function
addresses wrapped in `Pointer`; callback signatures are in `Native::API`.
Variadic `TraceLog` and `TextFormat` use Ruby `format` on their trailing arguments
before passing the resulting string safely to C as `%s`.

## Selenite Data, Storage and Networking

The native namespace does not replace Selenite. Use `Selenite.grid`, inventories,
`Selenite::Storage` and networking alongside native rendering. The Gem Trail,
storage and networking tutorials explain those higher-level APIs. Use managed
`Selenite::Game.run` when automatic texture/audio cleanup and game-loop helpers
are more useful than direct C resource control.

## API Discovery

```ruby
api::API["functions"].fetch("DrawModel")
api::API["structs"].fetch("Mesh")
api::API["callbacks"]
```

The complete function reference is bundled in IDE Help. Regenerate it and the
checked-in bindings with `ruby tools/generate_raylib_api.rb /path/to/raylib.h`
when upgrading raylib. Review and compile the generated result against both
Selenite binaries so API drift fails during development rather than at runtime.
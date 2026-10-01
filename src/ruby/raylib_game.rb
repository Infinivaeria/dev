# Ruby side of Selenite's game API (see src/game.rs for the native calls).
#
# Mirrors the raylib-bindings gem: CamelCase functions on the Raylib
# module, `include Raylib` to call them bare, KEY_*/MOUSE_BUTTON_*/FLAG_*
# constants and colour constants, plus Vector2/Rectangle/Color/Camera2D
# structs. Every function also gets a snake_case alias (draw_text, …).
module Raylib
  Vector2 = Struct.new(:x, :y) do
    def self.create(x = 0.0, y = 0.0) = new(x.to_f, y.to_f)
    def +(other) = Vector2.new(x + Raylib.__vx(other), y + Raylib.__vy(other))
    def -(other) = Vector2.new(x - Raylib.__vx(other), y - Raylib.__vy(other))
    def *(scale) = Vector2.new(x * scale, y * scale)
    def length = Math.hypot(x, y)
    def normalize = (len = length).zero? ? Vector2.new(0.0, 0.0) : self * (1.0 / len)
    def to_a = [x, y]
  end

  Rectangle = Struct.new(:x, :y, :width, :height) do
    def self.create(x = 0.0, y = 0.0, width = 0.0, height = 0.0) =
      new(x.to_f, y.to_f, width.to_f, height.to_f)
    def to_a = [x, y, width, height]
    def center = Vector2.new(x + width / 2.0, y + height / 2.0)
  end

  Color = Struct.new(:r, :g, :b, :a) do
    def self.create(r, g, b, a = 255) = new(r, g, b, a)
    def self.from_u8(r, g, b, a = 255) = new(r, g, b, a)
    def fade(alpha) = Color.new(r, g, b, (255 * alpha.to_f.clamp(0.0, 1.0)).round)
    def to_a = [r, g, b, a]
  end

  Camera2D = Struct.new(:offset, :target, :rotation, :zoom) do
    def self.create(offset: Vector2.new(0.0, 0.0), target: Vector2.new(0.0, 0.0),
                    rotation: 0.0, zoom: 1.0)
      new(offset, target, rotation, zoom)
    end
  end

  Texture2D = Struct.new(:id, :width, :height) do
    def unload = Raylib.UnloadTexture(self)
  end
  Sound = Struct.new(:id) do
    def play = Raylib.PlaySound(self)
  end
  Music = Struct.new(:id)

  # -- constants ----------------------------------------------------------
  {
    LIGHTGRAY: [200, 200, 200], GRAY: [130, 130, 130], DARKGRAY: [80, 80, 80],
    YELLOW: [253, 249, 0], GOLD: [255, 203, 0], ORANGE: [255, 161, 0],
    PINK: [255, 109, 194], RED: [230, 41, 55], MAROON: [190, 33, 55],
    GREEN: [0, 228, 48], LIME: [0, 158, 47], DARKGREEN: [0, 117, 44],
    SKYBLUE: [102, 191, 255], BLUE: [0, 121, 241], DARKBLUE: [0, 82, 172],
    PURPLE: [200, 122, 255], VIOLET: [135, 60, 190], DARKPURPLE: [112, 31, 126],
    BEIGE: [211, 176, 131], BROWN: [127, 106, 79], DARKBROWN: [76, 63, 47],
    WHITE: [255, 255, 255], BLACK: [0, 0, 0], BLANK: [0, 0, 0, 0],
    MAGENTA: [255, 0, 255], RAYWHITE: [245, 245, 245]
  }.each { |name, rgb| const_set(name, Color.new(*rgb, *(rgb.size == 3 ? [255] : [])).freeze) }

  KEY_NULL = 0
  KEY_SPACE = 32
  KEY_APOSTROPHE = 39
  KEY_COMMA = 44
  KEY_MINUS = 45
  KEY_PERIOD = 46
  KEY_SLASH = 47
  KEY_SEMICOLON = 59
  KEY_EQUAL = 61
  KEY_LEFT_BRACKET = 91
  KEY_BACKSLASH = 92
  KEY_RIGHT_BRACKET = 93
  KEY_GRAVE = 96
  %w[ZERO ONE TWO THREE FOUR FIVE SIX SEVEN EIGHT NINE].each_with_index do |name, i|
    const_set("KEY_#{name}", 48 + i)
  end
  ("A".."Z").each_with_index { |letter, i| const_set("KEY_#{letter}", 65 + i) }
  {
    ESCAPE: 256, ENTER: 257, TAB: 258, BACKSPACE: 259, INSERT: 260, DELETE: 261,
    RIGHT: 262, LEFT: 263, DOWN: 264, UP: 265, PAGE_UP: 266, PAGE_DOWN: 267,
    HOME: 268, END: 269, CAPS_LOCK: 280, SCROLL_LOCK: 281, NUM_LOCK: 282,
    PRINT_SCREEN: 283, PAUSE: 284, LEFT_SHIFT: 340, LEFT_CONTROL: 341,
    LEFT_ALT: 342, LEFT_SUPER: 343, RIGHT_SHIFT: 344, RIGHT_CONTROL: 345,
    RIGHT_ALT: 346, RIGHT_SUPER: 347, KB_MENU: 348, KP_DECIMAL: 330,
    KP_DIVIDE: 331, KP_MULTIPLY: 332, KP_SUBTRACT: 333, KP_ADD: 334,
    KP_ENTER: 335, KP_EQUAL: 336
  }.each { |name, code| const_set("KEY_#{name}", code) }
  (1..12).each { |n| const_set("KEY_F#{n}", 289 + n) }
  (0..9).each { |n| const_set("KEY_KP_#{n}", 320 + n) }

  %w[LEFT RIGHT MIDDLE SIDE EXTRA FORWARD BACK].each_with_index do |name, i|
    const_set("MOUSE_BUTTON_#{name}", i)
  end

  {
    FULLSCREEN_MODE: 0x2, WINDOW_RESIZABLE: 0x4, WINDOW_UNDECORATED: 0x8,
    WINDOW_TRANSPARENT: 0x10, MSAA_4X_HINT: 0x20, VSYNC_HINT: 0x40,
    WINDOW_HIDDEN: 0x80, WINDOW_ALWAYS_RUN: 0x100, WINDOW_MINIMIZED: 0x200,
    WINDOW_MAXIMIZED: 0x400, WINDOW_UNFOCUSED: 0x800, WINDOW_TOPMOST: 0x1000,
    WINDOW_HIGHDPI: 0x2000
  }.each { |name, bit| const_set("FLAG_#{name}", bit) }

  %w[ALL TRACE DEBUG INFO WARNING ERROR FATAL NONE].each_with_index do |name, i|
    const_set("LOG_#{name}", i)
  end

  # -- Ruby-level functions ---------------------------------------------------
  module_function

  def __vx(v) = v.is_a?(Array) ? v[0].to_f : v.x.to_f
  def __vy(v) = v.is_a?(Array) ? v[1].to_f : v.y.to_f
  def __rect(r) = r.is_a?(Array) ? r.map(&:to_f) : [r.x.to_f, r.y.to_f, r.width.to_f, r.height.to_f]

  def LoadTexture(path)
    id, width, height = __LoadTexture(path.to_s)
    Texture2D.new(id, width, height)
  end

  def LoadSound(path) = Sound.new(__LoadSound(path.to_s))
  def LoadMusicStream(path) = Music.new(__LoadMusicStream(path.to_s))
  def GetMousePosition = Vector2.new(*__GetMousePosition)
  def GetScreenSize = Vector2.new(GetScreenWidth().to_f, GetScreenHeight().to_f)

  def Fade(color, alpha) = Color.new(color.r, color.g, color.b, (255 * alpha.to_f.clamp(0.0, 1.0)).round)
  def ColorAlpha(color, alpha) = Fade(color, alpha)

  def CheckCollisionRecs(a, b)
    ax, ay, aw, ah = __rect(a)
    bx, by, bw, bh = __rect(b)
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
  end

  def GetCollisionRec(a, b)
    ax, ay, aw, ah = __rect(a)
    bx, by, bw, bh = __rect(b)
    left = [ax, bx].max
    top = [ay, by].max
    right = [ax + aw, bx + bw].min
    bottom = [ay + ah, by + bh].min
    return Rectangle.new(0.0, 0.0, 0.0, 0.0) if right <= left || bottom <= top
    Rectangle.new(left, top, right - left, bottom - top)
  end

  def CheckCollisionCircles(c1, r1, c2, r2) =
    Math.hypot(__vx(c1) - __vx(c2), __vy(c1) - __vy(c2)) <= r1 + r2

  def CheckCollisionPointCircle(point, center, radius) =
    Math.hypot(__vx(point) - __vx(center), __vy(point) - __vy(center)) <= radius

  def CheckCollisionPointRec(point, rec)
    x, y, w, h = __rect(rec)
    px = __vx(point)
    py = __vy(point)
    px >= x && px < x + w && py >= y && py < y + h
  end

  def CheckCollisionCircleRec(center, radius, rec)
    x, y, w, h = __rect(rec)
    nearest_x = __vx(center).clamp(x, x + w)
    nearest_y = __vy(center).clamp(y, y + h)
    Math.hypot(__vx(center) - nearest_x, __vy(center) - nearest_y) <= radius
  end

  def Vector2Add(a, b) = Vector2.new(__vx(a) + __vx(b), __vy(a) + __vy(b))
  def Vector2Subtract(a, b) = Vector2.new(__vx(a) - __vx(b), __vy(a) - __vy(b))
  def Vector2Scale(v, scale) = Vector2.new(__vx(v) * scale, __vy(v) * scale)
  def Vector2Length(v) = Math.hypot(__vx(v), __vy(v))
  def Vector2Distance(a, b) = Math.hypot(__vx(a) - __vx(b), __vy(a) - __vy(b))
  def Vector2Normalize(v) = Vector2.new(__vx(v), __vy(v)).normalize
  def Vector2Lerp(a, b, t) = Vector2.new(Lerp(__vx(a), __vx(b), t), Lerp(__vy(a), __vy(b), t))
  def Clamp(value, min, max) = value.clamp(min, max)
  def Lerp(a, b, t) = a + (b - a) * t
  def Remap(value, in_start, in_end, out_start, out_end) =
    (value - in_start) / (in_end - in_start).to_f * (out_end - out_start) + out_start

  # raylib-bindings compatibility: the library is already linked in.
  def load_lib(*) = true
  def shared_lib_path = ""

  # snake_case aliases for every CamelCase function.
  def self.__snake(name)
    name.to_s.sub("2D", "_2d").sub("FPS", "Fps").gsub(/([a-z\d])([A-Z])/, '\1_\2').downcase
  end

  singleton_methods.each do |name|
    next if name.to_s.start_with?("__") || name.to_s !~ /\A[A-Z]/
    snake = __snake(name)
    singleton_class.send(:alias_method, snake, name)
    if private_method_defined?(name)
      alias_method snake, name
      private snake
    end
  end
end

module Selenite
  # `require "raylib"` / `require "raylib-bindings"` resolve to the built-in
  # module, so raylib-bindings examples run unchanged.
  module RequireShim
    BUILTIN = %w[raylib raylib-bindings raylib_bindings].freeze

    private

    def require(name)
      return false if BUILTIN.include?(name.to_s)
      super
    end
  end
  Object.prepend(RequireShim)

  # A small game loop on top of Raylib:
  #
  #   Selenite::Game.run(title: "Pong", width: 800, height: 450) do |g|
  #     g.update { |dt| ... }
  #     g.draw   { g.text("Hello", 10, 10) }
  #   end
  class Game
    include Raylib

    attr_reader :title, :width, :height, :fps, :frame, :elapsed
    attr_accessor :background

    def self.run(**options, &setup) = new(**options).run(&setup)
    def self.game_mode? = Raylib.GameMode?

    def initialize(title: "Selenite game", width: 960, height: 540, fps: 60,
                   background: Raylib::RAYWHITE, audio: true, resizable: true,
                   exit_key: Raylib::KEY_ESCAPE, autosave: true)
      @title = title.to_s
      @width = Integer(width)
      @height = Integer(height)
      @fps = Integer(fps)
      @background = background
      @audio = audio
      @resizable = resizable
      @exit_key = exit_key
      @autosave = autosave
      @frame = 0
      @elapsed = 0.0
      @textures = {}
      @timers = []
      @hooks = { update: [], draw: [], draw_ui: [], close: [] }
      @quit = false
    end

    def update(&block) = @hooks[:update] << block
    def draw(&block) = @hooks[:draw] << block
    # Drawn after `draw`, outside any `camera` — for HUDs.
    def draw_ui(&block) = @hooks[:draw_ui] << block
    def on_close(&block) = @hooks[:close] << block

    attr_accessor :camera

    def quit! = @quit = true
    def quit? = @quit

    # Runs the block every `seconds` of game time (or once with `after`).
    def every(seconds, &block) = @timers << [seconds.to_f, seconds.to_f, block, true]
    def after(seconds, &block) = @timers << [seconds.to_f, seconds.to_f, block, false]

    def screen_width = GetScreenWidth()
    def screen_height = GetScreenHeight()
    def key?(key) = IsKeyDown(key)
    def pressed?(key) = IsKeyPressed(key)
    def released?(key) = IsKeyReleased(key)
    def click?(button = MOUSE_BUTTON_LEFT) = IsMouseButtonPressed(button)
    def mouse = GetMousePosition()

    # Arrow keys / WASD as a -1..1 vector.
    def axis
      x = (key?(KEY_RIGHT) || key?(KEY_D) ? 1 : 0) - (key?(KEY_LEFT) || key?(KEY_A) ? 1 : 0)
      y = (key?(KEY_DOWN) || key?(KEY_S) ? 1 : 0) - (key?(KEY_UP) || key?(KEY_W) ? 1 : 0)
      Vector2.new(x.to_f, y.to_f)
    end

    def text(value, x, y, size: 20, color: Raylib::DARKGRAY, center: false)
      value = value.to_s
      x -= MeasureText(value, size) / 2 if center
      DrawText(value, x.to_i, y.to_i, size, color)
    end

    def rect(x, y, w, h, color = Raylib::GRAY) = DrawRectangle(x.to_i, y.to_i, w.to_i, h.to_i, color)
    def circle(x, y, radius, color = Raylib::GRAY) = DrawCircle(x.to_i, y.to_i, radius.to_f, color)

    # Cached texture by file path.
    def texture(path)
      path = File.expand_path(path.to_s)
      @textures[path] ||= LoadTexture(path)
    end

    # The image in a cell of the current Selenite grid, as a texture (nil if
    # the cell isn't an image) — handy for grid-authored levels and sprites.
    def cell_texture(col, row, grid = Selenite.grid)
      cell = grid[col, row]
      cell && cell.kind == "image" ? texture(cell.path) : nil
    end

    def grid = Selenite.grid
    def inventory = Selenite.grid.inventory
    def save = Selenite.save

    def run(&setup)
      flags = FLAG_MSAA_4X_HINT | FLAG_VSYNC_HINT
      flags |= FLAG_WINDOW_RESIZABLE if @resizable
      SetConfigFlags(flags)
      SetTraceLogLevel(LOG_WARNING)
      InitWindow(@width, @height, @title)
      @started = true
      SetExitKey(@exit_key || KEY_NULL)
      InitAudioDevice() if @audio
      SetTargetFPS(@fps)
      setup&.call(self)
      until @quit || WindowShouldClose()
        dt = GetFrameTime()
        @elapsed += dt
        @hooks[:update].each { |hook| hook.call(dt) }
        tick_timers(dt)
        BeginDrawing()
        ClearBackground(@background)
        if @camera
          BeginMode2D(@camera)
          @hooks[:draw].each(&:call)
          EndMode2D()
        else
          @hooks[:draw].each(&:call)
        end
        @hooks[:draw_ui].each(&:call)
        EndDrawing()
        @frame += 1
      end
      self
    ensure
      begin
        if @started
          @hooks[:close].each(&:call)
          save if @autosave && Selenite.respond_to?(:save)
        end
      ensure
        @textures.clear
        CloseAudioDevice() if Raylib.IsAudioDeviceReady()
        CloseWindow() if Raylib.IsWindowReady()
      end
    end

    def inspect = "#<Selenite::Game #{@title.inspect} #{@width}x#{@height} frame=#{@frame}>"

    private

    def tick_timers(dt)
      @timers.each do |timer|
        timer[0] -= dt
        fired = 0
        while timer[0] && timer[0] <= 0 && fired < 8
          timer[2].call
          fired += 1
          timer[0] = timer[3] && timer[1].positive? ? timer[0] + timer[1] : nil
        end
        timer[0] = timer[1] if timer[0] && timer[0] <= 0
      end
      @timers.reject! { |timer| timer[0].nil? }
    end
  end
end

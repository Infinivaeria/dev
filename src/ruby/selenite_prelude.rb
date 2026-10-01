# Ruby-side half of the Selenite scripting API. The native methods
# (`Selenite.__*` and `Selenite::Grid#raw_*`) are defined in Rust via Magnus.
module Selenite
  Cell = Struct.new(:col, :row, :kind, :path) do
    def grid?
      kind == "grid"
    end

    def to_s
      "(#{col}, #{row}) #{kind}#{path ? " #{path}" : ""}"
    end
  end

  EVENTS = %i[activate paste drop download save profile track inventory].freeze

  @hooks = {}

  class << self
    # Register a hook: Selenite.on(:activate) { |col, row, kind, path| ... }
    # Returning :handled from an :activate hook skips the default action.
    def on(event, &block)
      raise ArgumentError, "Selenite.on needs a block" unless block
      event = event.to_sym
      unless EVENTS.include?(event)
        raise ArgumentError, "unknown event #{event.inspect}; expected one of #{EVENTS.inspect}"
      end
      (@hooks[event] ||= []) << block
      block
    end

    def off(event = nil)
      event ? @hooks.delete(event.to_sym) : @hooks.clear
      nil
    end

    def hooks
      @hooks.transform_values(&:size)
    end

    def hooks?(event)
      list = @hooks[event.to_sym]
      !list.nil? && !list.empty?
    end

    def __remove_hook(event, block)
      list = @hooks[event.to_sym]
      list&.delete_if { |hook| hook.equal?(block) }
      @hooks.delete(event.to_sym) if list && list.empty?
      nil
    end

    def emit(event, *args)
      handled = false
      (@hooks[event.to_sym] || []).each do |hook|
        begin
          handled = true if hook.call(*args) == :handled
        rescue StandardError => e
          puts "hook #{event} failed: #{e.class}: #{e.message}"
        end
      end
      handled
    end

    def emit_list(event, args)
      emit(event, *args)
    end

    # `stack: true` pushes the download onto the cell's stack instead of
    # placing it beside existing content.
    def download(url, col = nil, row = nil, stack: false)
      col, row = (selected || [0, 0]) if col.nil? || row.nil?
      __download(url.to_s, Integer(col), Integer(row), stack ? true : false)
    end

    # Replace the multi-selection; the first cell becomes the primary one.
    def select_cells(cells)
      list = Array(cells).map { |c| c.respond_to?(:col) ? [c.col, c.row] : [Integer(c[0]), Integer(c[1])] }
      __select_cells(list)
      list.size
    end

    # Inventory of the current grid (shortcut for Selenite.grid.inventory).
    def inventory
      grid.inventory
    end

    # The built-in music player (see Selenite::Music).
    def music
      Music
    end

    def labels(on = true)
      __labels(on ? true : false)
    end

    # Define a plugin; see docs/plugins.md.
    def plugin(name, &block)
      Plugins.define(name, Plugins.current_file || caller_locations(1, 1).first&.path, &block)
    end

    def plugins
      Plugins.list.map(&:name)
    end

    # Run a command registered with `p.command(name)`.
    def run(name, *args)
      block = Plugins.commands[name.to_s]
      raise ArgumentError, "no plugin command #{name.to_s.inspect}; have #{commands.inspect}" unless block
      block.call(*args)
    end

    def commands
      Plugins.commands.keys.sort
    end

    def help
      puts <<~HELP
        Selenite #{VERSION} Ruby API
          g = Selenite.grid            current grid (Selenite.root = profile root)
          g[col, row]                  => Selenite::Cell or nil
          g.set(col, row, path)        place a file (false if occupied)
          g[col, row] = path           place/replace a file
          g.remove(col, row), g.clear, g.size, g.each { |cell| }, g.cells
          g.new_grid(col, row), g.subgrid(col, row)  => nested Selenite::Grid
          g.move(c1, r1, c2, r2), g.swap(c1, r1, c2, r2), g.fill(col, row, paths)
          Selenite.select(c, r) / open(c, r) / goto(c, r) / enter(c, r) / back
          Selenite.view_3d(true|false), Selenite.orbit(yaw_deg, pitch_deg, distance)
          Selenite.labels(true|false), Selenite.labels?   cell labels on/off
          Selenite.undo / redo / history       undo stack ([next undo, next redo] labels)
          Selenite.search(q)  -> [[nested_path, col, row, name], ...] across the profile
          Selenite.find(q)    open the Find bar and jump to the first match
          Selenite.download(url, col = sel, row = sel)   background, no size limit
          Selenite.status("msg"), Selenite.save, Selenite.selected, Selenite.depth
          Selenite.selection => [[c, r], ...]   Selenite.select_cells([[c, r], ...])
          Selenite.grid_folder, g.folder       this grid's own folder (created on first use)
          Selenite.grids_dir, g.folder_id      parent of all grid folders; id or nil
          Selenite.assets_dir, g.assets_dir    <grid folder>/assets: where added files are copied
          Selenite.import(c, r, path)          copy a file into assets/ and stack it on the cell
          g.import(c, r, path)                 (same, for any grid) => the copy's path
        Stacks  (a cell can hold many items; index 0 is the visible top)
          g.items(c, r) => [Cell, ...]   g.stack_size(c, r)   g.item_count
          g.push(c, r, path), g.push_grid(c, r), g.pop(c, r, index = 0)
          g.cycle(c, r, +1|-1), g.raise_item(c, r, i), g.merge(c1, r1, c2, r2), g.unstack(c, r)
          Selenite.download(url, c, r, stack: true)
        Inventory  (inv = g.inventory or Selenite.inventory; persisted per grid)
          inv.add(name, n = 1), inv.remove(name, n = 1), inv[name], inv[name] = n
          inv.has?(name, n = 1), inv.delete(name), inv.rename(a, b), inv.clear
          inv.meta(name), inv.set_meta(name, key, value), inv.to_h, inv.each { |name, count, meta| }
          inv.transfer(other_grid_or_inventory, name, n), inv.total, inv.size
          Selenite.copy_text(str)              put text on the clipboard
          Selenite.profile, profiles, create_profile(n), switch_profile(n)
          Selenite.on(:activate|:paste|:drop|:download|:save|:profile|:track|:inventory) { |*args| }
        Rust helpers
          Selenite.file_info(path)  => {path:, exists:, dir:, size:, modified:, kind:, ext:}
          Selenite.image_size(path) => [w, h] or nil      (header only, any size)
          Selenite.checksum(path)   => "fnv1a64:..."       (streamed)
          Selenite.playable?(path)  built-in player can decode it
        Music  (m = Selenite.music)
          m.play(path = nil), m.play_grid, m.play_list(paths, start = 0), m.queue(path)
          m.toggle / pause / resume / stop / next / previous / clear
          m.seek(sec), m.volume(0.0..1.0), m.shuffle(bool), m.repeat(:off|:all|:one)
          m.state => {state:, track:, position:, length:, volume:, ...}, m.playing?
          m.show / m.hide     player bar
        Plugins  (files in the plugins folders; see docs/plugins.md)
          Selenite.plugin("Name") { |p| p.button / p.menu / p.command / p.on / p.every }
          Selenite.plugins, Selenite.commands, Selenite.run(name, *args)
          Selenite.reload_plugins
        Games  (selenite --game file.rb, or right-click a .rb cell → Run as game; see docs/game.md)
          include Raylib; InitWindow(w, h, t); BeginDrawing(); DrawText(...); EndDrawing()
          Selenite::Game.run(title: "T") { |g| g.update { |dt| }; g.draw { }; g.draw_ui { } }
          g.every(s) { }, g.axis, g.key?(KEY_X), g.texture(path), g.cell_texture(c, r), g.inventory
        partitioned_array
          grid_pa_export(dir), grid_pa_import(dir), grid_pa_records(dir)
      HELP
      nil
    end
  end

  # Built-in music player. Commands are queued and applied by the app
  # right after the current Ruby code finishes; `state` reflects the
  # player as of the start of the current evaluation.
  module Music
    class << self
      def play(path = nil)
        Selenite.__music("play", path&.to_s)
        true
      end

      # Plays every playable audio cell of `grid` in row/column order.
      def play_grid(grid = Selenite.grid, start = 0)
        paths = grid.cells
                    .select { |c| c.kind == "audio" && Selenite.playable?(c.path) }
                    .sort_by { |c| [c.row, c.col] }
                    .map(&:path)
        play_list(paths, start) unless paths.empty?
        paths.size
      end

      def play_list(paths, start = 0)
        Selenite.__music_list(Array(paths).map(&:to_s), Integer(start))
        true
      end

      def queue(path)
        Selenite.__music("queue", path.to_s)
        true
      end

      %w[toggle pause resume stop next previous clear show hide].each do |command|
        define_method(command) do
          Selenite.__music(command, nil)
          true
        end
      end
      alias prev previous

      def seek(seconds)
        Selenite.__music("seek", Float(seconds).to_s)
        true
      end

      def volume(level = nil)
        return state[:volume] if level.nil?
        Selenite.__music("volume", Float(level).to_s)
        Float(level).clamp(0.0, 1.0)
      end

      def shuffle(on = nil)
        return state[:shuffle] if on.nil?
        Selenite.__music("shuffle", (on ? true : false).to_s)
        on ? true : false
      end

      def repeat(mode = nil)
        return state[:repeat].to_sym if mode.nil?
        Selenite.__music("repeat", mode.to_s)
        mode.to_sym
      end

      def state
        Selenite.__music_state
      end

      def playing?
        state[:state] == "playing"
      end

      def track
        state[:track]
      end
    end
  end

  # Collects what a plugin file declares. Every block becomes an action id
  # the Rust side can invoke from buttons, menus, keys and timers.
  class PluginBuilder
    attr_reader :name, :file, :items

    def initialize(name, file)
      @name = name.to_s
      @file = file.to_s
      @description = ""
      @version = ""
      @items = []
    end

    def description(text = nil)
      text.nil? ? @description : (@description = text.to_s)
    end

    def version(text = nil)
      text.nil? ? @version : (@version = text.to_s)
    end

    # A button in the Plugins panel; `key:` binds F2, F4..F12. The block
    # gets the selected cell (or nil).
    def button(label, key: nil, &block)
      add(:button, label, block, key: key&.to_s)
    end

    # A right-click menu entry. `kinds:` limits it to cell kinds: image,
    # audio, video, ruby, file (any file), grid, empty. The block gets the
    # clicked Selenite::Cell (kind/path are nil for an empty cell).
    def menu(label, kinds: nil, &block)
      add(:menu, label, block, kinds: Array(kinds).map(&:to_s))
    end

    # A named command for Selenite.run(name, *args).
    def command(name, &block)
      raise ArgumentError, "command #{name.to_s.inspect} needs a block" unless block
      Plugins.commands[name.to_s] = block
      add(:command, name, block)
    end

    # An event hook removed again when plugins reload.
    def on(event, &block)
      raise ArgumentError, "hook #{event.inspect} needs a block" unless block
      stored = Selenite.on(event, &block)
      add(:hook, event.to_sym, stored)
    end

    # Runs the block every `seconds` (>= 0.25) on the UI thread.
    def every(seconds, &block)
      seconds = Float(seconds)
      raise ArgumentError, "every needs at least 0.25 seconds" if seconds < 0.25
      add(:timer, "every #{seconds % 1 == 0 ? seconds.to_i : seconds}s", block, interval: seconds)
    end

    def inspect
      "#<Selenite plugin #{@name} (#{@items.size} items)>"
    end

    private

    def add(kind, label, block, key: nil, kinds: [], interval: 0.0)
      raise ArgumentError, "#{kind} #{label.to_s.inspect} needs a block" unless block
      id = Plugins.register(block)
      @items << [id, kind.to_s, label.to_s, key, kinds, interval]
      id
    end
  end

  module Plugins
    @list = []
    @actions = {}
    @commands = {}
    @next_id = 0
    @current_file = nil

    class << self
      attr_reader :list, :commands
      attr_accessor :current_file

      def register(block)
        @next_id += 1
        @actions[@next_id] = block
        @next_id
      end

      def define(name, file, &block)
        raise ArgumentError, "Selenite.plugin needs a block" unless block
        plugin = PluginBuilder.new(name, file)
        block.arity.zero? ? plugin.instance_eval(&block) : block.call(plugin)
        @list << plugin
        plugin
      end

      # Forget every plugin (and remove their hooks) before a reload.
      def reset!
        @list.each do |plugin|
          plugin.items.each do |id, kind, label, *|
            Selenite.__remove_hook(label, @actions[id]) if kind == "hook"
          end
        end
        @list = []
        @actions = {}
        @commands = {}
        nil
      end

      # [[file, name, description, version, [[id, kind, label, key, kinds, interval], ...]], ...]
      def manifest
        @list.map { |p| [p.file, p.name, p.description, p.version, p.items] }
      end

      # Runs an action; returns nil on success or an error message.
      def invoke(id, col = nil, row = nil)
        action = @actions[id]
        return "unknown plugin action #{id}" unless action
        cell = (Selenite.grid[col, row] || Cell.new(col, row, nil, nil)) if col && row
        action.call(cell)
        nil
      rescue StandardError, ScriptError => e
        "#{e.class}: #{e.message}"
      end
    end
  end

  # Game-style inventory attached to one grid (named items with counts and
  # JSON metadata). Changes are saved with the grid on the next save.
  class Inventory
    include Enumerable

    attr_reader :grid

    def initialize(grid)
      @grid = grid
    end

    def add(name, amount = 1)
      grid.inv_add(name.to_s, Integer(amount))
    end
    alias give add

    def remove(name, amount = 1)
      grid.inv_remove(name.to_s, Integer(amount))
    end
    alias take remove

    def [](name)
      grid.inv_count(name.to_s)
    end
    alias count []

    def []=(name, count)
      grid.inv_set(name.to_s, Integer(count))
    end

    def has?(name, amount = 1)
      grid.inv_has(name.to_s, Integer(amount))
    end
    alias include? has?

    def delete(name)
      grid.inv_delete(name.to_s)
    end

    def rename(from, to)
      grid.inv_rename(from.to_s, to.to_s)
    end

    def meta(name)
      (to_h_full[name.to_s] || {})["meta"] || {}
    end

    def set_meta(name, key, value)
      require "json"
      grid.inv_set_meta(name.to_s, key.to_s, JSON.generate(value))
    end

    def transfer(target, name, amount = 1)
      target = target.grid if target.is_a?(Inventory)
      grid.inv_transfer(target, name.to_s, Integer(amount))
    end

    def clear
      grid.inv_clear
      self
    end

    def total
      grid.inv_total
    end

    def size
      grid.inv_len
    end
    alias length size

    def empty?
      size.zero?
    end

    # { "name" => count }
    def to_h
      to_h_full.transform_values { |item| item["count"] }
    end

    def names
      to_h.keys
    end

    def each
      return enum_for(:each) unless block_given?
      to_h_full.each { |name, item| yield name, item["count"], item["meta"] || {} }
      self
    end

    def inspect
      "#<Selenite::Inventory #{to_h.inspect}>"
    end
    alias to_s inspect

    private

    def to_h_full
      require "json"
      JSON.parse(grid.inv_json)
    end
  end

  class Grid
    include Enumerable

    def [](col, row)
      raw = raw_get(col, row)
      raw && Cell.new(col, row, *raw)
    end

    def cells
      raw_cells.sort.map { |entry| Cell.new(*entry) }
    end

    # Every item stacked at (col, row), top first.
    def items(col, row)
      items_raw(col, row).map { |kind, path| Cell.new(col, row, kind, path) }
    end
    alias stack items

    # Removes and returns the item at `index` (0 = top) as a Cell.
    def pop(col, row, index = 0)
      raw = __pop(col, row, Integer(index))
      raw && Cell.new(col, row, *raw)
    end

    def stacked?(col, row)
      stack_size(col, row) > 1
    end

    def inventory
      Inventory.new(self)
    end

    def each(&block)
      return enum_for(:each) unless block
      cells.each(&block)
      self
    end

    def inspect
      "#<Selenite::Grid #{size} cells, #{item_count} items>"
    end
    alias to_s inspect
  end
end

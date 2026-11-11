# Gem Trail — the complete game from docs/game-tutorial.md.
# Run with a dedicated save (from the repository root):
#   ./dist/run.sh --game examples/games/gem_trail.rb --grid /tmp/gem-trail.json
# WASD/arrows: move. R: restart this level. Esc: save and quit.
# Image cells in the root grid become pickups; an empty image layout uses
# the built-in level. Files and cells are never removed by this game.

module GemTrail
  TILE = 64
  SPEED = 220.0
  RADIUS = 14
  BAG = "gem_trail_gems"
  PROGRESS = "gem_trail_progress"
  DEFAULT_LEVEL = [[2, 0], [3, -2], [0, -3], [-3, -2],
                   [-4, 1], [-2, 3], [1, 3], [4, 2]].freeze
end

Selenite::Game.run(title: "Gem Trail", width: 960, height: 600,
                   audio: false, background: Raylib::DARKBLUE) do |g|
  # Load level data and textures once, outside the update/draw callbacks.
  cells = g.grid.cells.select { |cell| cell.kind == "image" }
  locations = cells.empty? ? GemTrail::DEFAULT_LEVEL : cells.map { |c| [c.col, c.row] }
  textures = {}
  cells.each do |cell|
    begin
      textures[[cell.col, cell.row]] = g.cell_texture(cell.col, cell.row)
    rescue StandardError => error
      warn "Using a gem shape for #{cell.path}: #{error.message}"
    end
  end
  pickups = locations.map do |col, row|
    {
      id: "#{col},#{row}", col: col, row: row,
      rect: Raylib::Rectangle.create(col * GemTrail::TILE - 17,
                                    row * GemTrail::TILE - 17, 34, 34),
      texture: textures[[col, row]]
    }
  end

  inventory = g.inventory
  saved = inventory.meta(GemTrail::PROGRESS)["collected"]
  valid_ids = pickups.map { |item| item[:id] }
  collected = saved.is_a?(Array) ? (saved & valid_ids) : []
  player = Raylib::Vector2.create(0, 0)
  message = "Collect every gem. Follow the nearest gem coordinates."
  message_until = 4.0

  # Save pickup IDs as well as the count, so relaunching cannot award them twice.
  save_progress = lambda do
    inventory[GemTrail::PROGRESS] = 1
    inventory.set_meta(GemTrail::PROGRESS, "collected", collected.sort)
    inventory[GemTrail::BAG] = collected.length
    g.save
  end
  save_progress.call

  g.camera = Raylib::Camera2D.create(target: player)
  g.update do |dt|
    if g.pressed?(Raylib::KEY_R)
      collected.clear
      player = Raylib::Vector2.create(0, 0)
      message = "Level restarted."
      message_until = g.elapsed + 2.0
      save_progress.call
    end

    unless collected.length == pickups.length
      # Limit a long frame's movement; normalize to keep diagonal speed equal.
      player += g.axis.normalize * (GemTrail::SPEED * [dt, 0.05].min)
      changed = false
      pickups.each do |item|
        next if collected.include?(item[:id])
        next unless Raylib.CheckCollisionCircleRec(player, GemTrail::RADIUS, item[:rect])

        collected << item[:id]
        changed = true
        message = "Gem collected!"
        message_until = g.elapsed + 1.5
      end
      save_progress.call if changed
    end

    g.camera.target = player
    g.camera.offset = Raylib::Vector2.create(g.screen_width / 2.0, g.screen_height / 2.0)
  end

  g.draw do
    # Draw only nearby floor lines. All positions here are world coordinates.
    tile = GemTrail::TILE
    left = ((player.x - g.screen_width / 2.0) / tile).floor - 1
    right = ((player.x + g.screen_width / 2.0) / tile).ceil + 1
    top = ((player.y - g.screen_height / 2.0) / tile).floor - 1
    bottom = ((player.y + g.screen_height / 2.0) / tile).ceil + 1
    color = Raylib.Fade(Raylib::SKYBLUE, 0.15)
    (left..right).each { |col| Raylib.DrawLine(col * tile, top * tile, col * tile, bottom * tile, color) }
    (top..bottom).each { |row| Raylib.DrawLine(left * tile, row * tile, right * tile, row * tile, color) }

    pickups.each do |item|
      next if collected.include?(item[:id])
      rect = item[:rect]
      texture = item[:texture]
      if texture
        Raylib.DrawTexturePro(texture, [0, 0, texture.width, texture.height], rect,
                              [0, 0], 0.0, Raylib::WHITE)
      else
        Raylib.DrawRectangleRounded(rect, 0.35, 6, Raylib::GOLD)
        Raylib.DrawRectangleLinesEx(rect, 2.0, Raylib::YELLOW)
      end
    end
    Raylib.DrawCircleV(player, GemTrail::RADIUS, Raylib::SKYBLUE)
    Raylib.DrawCircleLines(player.x.to_i, player.y.to_i, GemTrail::RADIUS, Raylib::WHITE)
  end

  g.draw_ui do
    # HUD coordinates belong to the screen and are not moved by the camera.
    g.rect(0, 0, g.screen_width, 104, Raylib.Fade(Raylib::BLACK, 0.8))
    g.text("GEM TRAIL   #{collected.length} / #{pickups.length} gems", 18, 12,
           size: 26, color: Raylib::GOLD)
    g.text("WASD / arrows: move    R: restart    Esc: save and quit", 18, 45,
           size: 18, color: Raylib::RAYWHITE)
    remaining = pickups.reject { |item| collected.include?(item[:id]) }
    nearest = remaining.min_by do |item|
      (item[:col] * GemTrail::TILE - player.x)**2 + (item[:row] * GemTrail::TILE - player.y)**2
    end
    hint = nearest ? "Nearest gem: (#{nearest[:col]}, #{nearest[:row]})   You: (#{(player.x / GemTrail::TILE).round(1)}, #{(player.y / GemTrail::TILE).round(1)})" : "All gems found! Press R for a new run."
    g.text(hint, 18, 75, size: 18, color: Raylib::SKYBLUE)
    if g.elapsed < message_until
      g.text(message, 18, g.screen_height - 32, size: 18, color: Raylib::RAYWHITE)
    end
    if remaining.empty?
      g.rect(0, g.screen_height / 2 - 38, g.screen_width, 76, Raylib.Fade(Raylib::BLACK, 0.8))
      g.text("LEVEL COMPLETE", g.screen_width / 2, g.screen_height / 2 - 15,
             size: 30, color: Raylib::GOLD, center: true)
    end
  end
end

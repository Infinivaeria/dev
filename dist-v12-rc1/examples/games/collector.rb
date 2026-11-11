# Collector — a Selenite::Game that uses the current grid as its world and
# the grid's inventory as the player's bag. Run against a profile with:
#   selenite --game examples/games/collector.rb --profile NAME
# (or right-click this file in Selenite → "Run as game").
#
# Every occupied cell of the grid becomes a pickup named after its file
# (image cells are drawn with their picture). An empty grid gets gems.
# Walk with WASD/arrows; touching a pickup adds it to the inventory, which
# is saved with the grid when the game closes — open the grid inventory
# panel (B) in Selenite afterwards to see what you collected.
TILE = 64

Selenite::Game.run(title: "Selenite Collector", width: 960, height: 600) do |g|
  pickup = lambda do |name, col, row, cell = nil|
    rect = Raylib::Rectangle.create(col * TILE + 8, row * TILE + 8, TILE - 16, TILE - 16)
    { name: name, cell: cell, rect: rect }
  end
  pickups = g.grid.cells.map do |cell|
    pickup.call(cell.path ? File.basename(cell.path) : cell.kind, cell.col, cell.row, cell)
  end
  pickups = Array.new(12) { pickup.call("gem", Raylib.GetRandomValue(-6, 6), Raylib.GetRandomValue(-4, 4)) } if pickups.empty?
  player = Raylib::Vector2.create(-TILE / 2, -TILE / 2)
  collected = 0
  message = nil
  g.camera = Raylib::Camera2D.create(offset: Raylib::Vector2.create(g.width / 2, g.height / 2), target: player)

  g.update do |dt|
    player += g.axis.normalize * (260 * dt)
    g.camera.target = player
    g.camera.offset = Raylib::Vector2.create(g.screen_width / 2, g.screen_height / 2)
    pickups.reject! do |item|
      next false unless Raylib.CheckCollisionCircleRec(player, 14, item[:rect])
      g.inventory.add(item[:name])
      collected += 1
      message = "+1 #{item[:name]}"
      g.after(1.5) { message = nil }
      true
    end
  end

  g.draw do
    (-20..20).each do |i|
      Raylib.DrawLine(i * TILE, -20 * TILE, i * TILE, 20 * TILE, Raylib::LIGHTGRAY)
      Raylib.DrawLine(-20 * TILE, i * TILE, 20 * TILE, i * TILE, Raylib::LIGHTGRAY)
    end
    pickups.each do |item|
      cell = item[:cell]
      texture = cell && g.cell_texture(cell.col, cell.row)
      if texture
        Raylib.DrawTexturePro(texture, [0, 0, texture.width, texture.height], item[:rect], [0, 0], 0.0, Raylib::WHITE)
      else
        Raylib.DrawRectangleRounded(item[:rect], 0.3, 6, Raylib::GOLD)
      end
    end
    Raylib.DrawCircleV(player, 14, Raylib::MAROON)
  end

  g.draw_ui do
    g.rect(0, 0, g.screen_width, 44, Raylib.Fade(Raylib::BLACK, 0.6))
    g.text("Collected #{collected}   bag: #{g.inventory.total} items   left: #{pickups.size}", 12, 12, color: Raylib::RAYWHITE)
    g.text(message, g.screen_width / 2, 60, size: 28, color: Raylib::DARKGREEN, center: true) if message
    if pickups.empty?
      g.text("You found everything! Esc to quit", g.screen_width / 2, g.screen_height / 2, size: 30, color: Raylib::DARKBLUE, center: true)
    end
  end
end

# Run: target/debug/selenite --game examples/games/revenant_world.rb --grid /tmp/revenant-grid.json
# WASD/arrows move; P pauses; C checkpoints; Esc saves and exits.
# Persistent files: sibling game-data/revenant-world (also copied by Export game).
Selenite::Game.run(title: "Revenant World", width: 960, height: 600,
                   audio: false, background: Raylib::DARKBLUE) do |g|
  data_root = File.join(File.dirname(__FILE__), "game-data")
  store = g.storage(path: File.join(data_root, "revenant-world"), partition_size: 4)
  bsp = Selenite::BSP.new(bounds: [-1024, -1024, 2048, 2048], depth: 4, min_size: 128)
  world = Selenite::PartitionedWorld.new(store, bsp: bsp)
  session = store.table("session")
  unless session[0]
    world.bsp.leaves.each do |leaf|
      x, y, w, h = leaf.bounds
      # Seven records per leaf exercise growth beyond physical partition 0.
      7.times do |i|
        angle = i * Math::PI * 2 / 7
        world.spawn(x: x + w / 2 + Math.cos(angle) * 100,
                    y: y + h / 2 + Math.sin(angle) * 100,
                    kind: "gem", collected: false)
      end
    end
    session.insert(x: 0.0, y: 0.0, score: 0)
    session.metadata = { schema: 1, total_gems: world.bsp.leaves.size * 7 }
    g.save
  end
  progress = session[0]
  player = Raylib::Vector2.create(progress["x"], progress["y"])
  score = progress["score"]
  total = session.metadata.fetch("total_gems")
  paused = false
  visible = []
  rooms = []
  message = "Explore rooms. Purple outlines mark data revived from disk."
  g.camera = Raylib::Camera2D.create(target: player)

  g.update do |dt|
    paused = !paused if g.pressed?(Raylib::KEY_P)
    unless paused
      player += g.axis.normalize * (230 * [dt, 0.05].min)
      player.x = player.x.clamp(-1010, 1010)
      player.y = player.y.clamp(-1010, 1010)
    end
    g.camera.target = player
    g.camera.offset = Raylib::Vector2.create(g.screen_width / 2.0, g.screen_height / 2.0)
    view = [player.x - g.screen_width / 2.0 - 32, player.y - g.screen_height / 2.0 - 32,
            g.screen_width + 64, g.screen_height + 64]
    rooms = world.stream(view)
    visible = world.query(view, kind: "gem", collected: false)
    unless paused
      visible.each do |handle, gem|
        next unless Math.hypot(player.x - gem["x"], player.y - gem["y"]) < 24
        world.update(handle) { |row| row["collected"] = true }
        gem["collected"] = true
        score += 1
      end
      session.update(0) { |row| row.merge!("x" => player.x, "y" => player.y, "score" => score) }
    end
    if g.pressed?(Raylib::KEY_C)
      destination = File.join(data_root, "checkpoints", "world-#{Time.now.strftime('%Y%m%d-%H%M%S')}-#{g.frame}")
      store.snapshot(destination)
      message = "Checkpoint saved: #{File.basename(destination)}"
    end
  end
  g.every(2.0) { g.save }

  g.draw do
    rooms.each do |leaf|
      table = world.table_for(leaf.id)
      color = table.revenants.empty? ? Raylib::SKYBLUE : Raylib::PURPLE
      x, y, w, h = leaf.bounds
      Raylib.DrawRectangleLinesEx(leaf.bounds, 3.0, color)
      g.text("#{leaf.id}: #{table.revenants.empty? ? 'active' : 'revenant'}", x + 12, y + 12,
             size: 18, color: color)
    end
    visible.each do |_handle, gem|
      g.circle(gem["x"], gem["y"], 10, Raylib::GOLD) unless gem["collected"]
    end
    g.circle(player.x, player.y, 14, Raylib::SKYBLUE)
  end

  g.draw_ui do
    tables = store.table_names.grep(/\Aworld_leaf_/).map { |name| store.table(name) }
    resident = tables.sum { |table| table.stats[:resident] }
    revenants = tables.sum { |table| table.stats[:revenants] }
    partitions = tables.sum(&:partition_count)
    g.rect(0, 0, g.screen_width, 104, Raylib.Fade(Raylib::BLACK, 0.85))
    g.text("REVENANT WORLD   #{score}/#{total} gems#{paused ? '   PAUSED' : ''}", 16, 12,
           size: 26, color: Raylib::GOLD)
    g.text("Resident partitions: #{resident}/#{partitions}   Revenants: #{revenants}", 16, 46,
           size: 18, color: Raylib::SKYBLUE)
    g.text("WASD/arrows: move   P: pause   C: checkpoint   Esc: save and quit", 16, 76,
           size: 18, color: Raylib::RAYWHITE)
    g.rect(0, g.screen_height - 40, g.screen_width, 40, Raylib.Fade(Raylib::BLACK, 0.85))
    g.text(message, 16, g.screen_height - 30, size: 17, color: Raylib::RAYWHITE)
    if score == total
      g.text("All rooms cleared!", g.screen_width / 2, g.screen_height / 2,
             center: true, size: 30, color: Raylib::GOLD)
    end
  end
end

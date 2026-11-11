Selenite::Game.run(title: "Selenite Game", width: 960, height: 540,
                   audio: false, background: Raylib::DARKBLUE) do |game|
  store = game.storage(path: File.join(__dir__, "game-data", "starter"), partition_size: 8)
  state = store.table("session")
  state.insert(launches: 0, score: 0) unless state[0]
  state.update(0) { |record| record["launches"] += 1 }
  game.save
  player = Raylib::Vector2.create(480, 270)

  game.update do |elapsed|
    player += game.axis.normalize * (200 * [elapsed, 0.05].min)
    player.x = player.x.clamp(14, game.screen_width - 14)
    player.y = player.y.clamp(120, game.screen_height - 14)
    if game.pressed?(Raylib::KEY_SPACE)
      state.update(0) { |record| record["score"] += 1 }
      game.save
    end
  end

  game.draw { game.circle(player.x, player.y, 14, Raylib::SKYBLUE) }
  game.draw_ui do
    progress = state[0]
    game.text("SELENITE GAME — Ruby + PartitionedArray", 20, 18, size: 26, color: Raylib::GOLD)
    game.text("WASD/arrows: move   Space: add score   Esc: save and exit", 20, 58,
              size: 18, color: Raylib::RAYWHITE)
    game.text("Launches: #{progress['launches']}   Score: #{progress['score']}", 20, 88,
              size: 18, color: Raylib::SKYBLUE)
  end
end

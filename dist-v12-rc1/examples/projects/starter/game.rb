Selenite::Game.run(title: "Selenite project", width: 960, height: 600,
                   audio: false, background: Raylib::RAYWHITE) do |game|
  position = Raylib::Vector2.create(120, 160)

  game.update do |elapsed|
    position += game.axis.normalize * (180 * elapsed)
  end

  game.draw { game.circle(position.x, position.y, 22, Raylib::BLUE) }
  game.draw_ui { game.text("Selenite", 24, 24, size: 28) }
end

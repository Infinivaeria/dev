role = ENV.fetch("SELENITE_NET_ROLE", "host")
raise ArgumentError, "SELENITE_NET_ROLE must be host or join" unless %w[host join].include?(role)
port = Integer(ENV.fetch("SELENITE_NET_PORT", "45678"))
key = ENV.fetch("SELENITE_NET_KEY")
room = ENV.fetch("SELENITE_NET_ROOM", "cooperative-gems")
hosting = role == "host"

Selenite::Game.run(title: "Selenite Multiplayer — #{role}", audio: false,
                   background: Raylib::DARKBLUE, autosave: hosting) do |game|
  session = if hosting
              game.host(bind: ENV.fetch("SELENITE_NET_BIND", "127.0.0.1"), port: port,
                        room: room, key: key)
            else
              game.join(ENV.fetch("SELENITE_NET_HOST", "127.0.0.1"), port: port,
                        room: room, key: key)
            end
  status = hosting ? "Hosting on #{session.address['host']}:#{session.address['port']}" : "Connecting…"
  players = {}
  generator = Random.new
  score = 0
  gem = [generator.rand(100..860), generator.rand(150..480)]

  if hosting
    store = game.storage(path: ENV["SELENITE_MULTIPLAYER_STORE"] || File.join(__dir__, "game-data", "multiplayer-host"))
    progress = store.table("session")
    progress.insert(score: 0) unless progress[0]
    score = progress[0]["score"]
    players[session.id] = { position: [480.0, 300.0], input: [0, 0], received: Selenite::Network.clock }
    session.set("players", { session.id => players[session.id][:position] })
    session.set("gem", gem)
    session.set("score", score)
    session.on("input") do |peer, data|
      next unless players.key?(peer) && data.is_a?(Array) && data.size == 2
      next unless data.all? { |value| (value.is_a?(Integer) || value.is_a?(Float)) && value.finite? && value.abs <= 1 }
      players[peer][:input] = data
      players[peer][:received] = Selenite::Network.clock
    end
    game.on_close { progress.update(0) { |record| record["score"] = score } }
  end

  session.on(:connect) do |peer|
    puts "Multiplayer connected: #{peer}"
    if hosting
      players[peer] = { position: [480.0, 300.0], input: [0, 0], received: Selenite::Network.clock }
    else
      status = "Connected — host controls movement, gems and score"
    end
  end
  session.on(:disconnect) do |peer, reason|
    puts "Multiplayer disconnected: #{peer} (#{reason})"
    players.delete(peer) if hosting
    status = hosting ? "Peer left: #{reason}" : "Host disconnected: #{reason}"
  end
  session.on(:error) { |_peer, reason| status = "Network: #{reason}" }
  session.on(:snapshot) { |state, version| puts "Multiplayer state ready: #{(state['players'] || {}).size} player(s) at revision #{version}" }

  game.every(0.05) do
    next unless session.connected?
    begin
      session.send("input", game.axis.to_a)
      if hosting
        session.set("players", players.transform_values { |player| player[:position] })
        session.set("gem", gem)
        session.set("score", score)
      end
    rescue Selenite::Network::Backpressure
      status = "Waiting for a slow peer; updates will retry"
    end
  end

  game.update do |elapsed|
    next unless hosting
    players.each_value do |player|
      input = Selenite::Network.clock - player[:received] <= 0.5 ? player[:input] : [0, 0]
      direction = Raylib::Vector2.create(*input).normalize
      player[:position][0] = (player[:position][0] + direction.x * 200 * elapsed.clamp(0, 0.05)).clamp(16, 944)
      player[:position][1] = (player[:position][1] + direction.y * 200 * elapsed.clamp(0, 0.05)).clamp(135, 524)
    end
    if players.values.any? { |player| Math.hypot(player[:position][0] - gem[0], player[:position][1] - gem[1]) < 24 }
      score += 1
      gem = [generator.rand(100..860), generator.rand(150..480)]
    end
  end

  game.draw do
    visible_gem = session.state("gem")
    game.circle(*visible_gem, 9, Raylib::GOLD) if visible_gem
    (session.state("players") || {}).each do |peer, position|
      color = peer == session.id ? Raylib::SKYBLUE : Raylib::GREEN
      game.circle(*position, 14, color)
      game.text(peer[0, 6], position[0] - 20, position[1] + 18, size: 12, color: color)
    end
  end
  game.draw_ui do
    game.text("COOPERATIVE GEMS — #{role.upcase}", 20, 18, size: 26, color: Raylib::GOLD)
    game.text("WASD/arrows move. Collect gold gems together. Escape exits.", 20, 56, size: 18, color: Raylib::RAYWHITE)
    game.text("Score: #{session.state('score') || 0}    Connected peers: #{session.peers.size}", 20, 84, size: 18, color: Raylib::SKYBLUE)
    game.text(status, 20, 110, size: 14, color: Raylib::LIGHTGRAY)
  end
end

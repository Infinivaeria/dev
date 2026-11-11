$selenite_network_grid&.close
$selenite_network_grid = nil

Selenite.plugin "Shared Grid" do |plugin|
  plugin.description "Share an initially empty grid over authenticated TCP. Commands: net-host, net-join, net-place, net-remove, net-stop. See docs/networking.md."
  plugin.version "1.0"

  attach = lambda do |session|
    grid = Selenite.grid
    note = File.join(grid.folder, "network-note.txt")
    File.write(note, "Shared Grid: this asset is local to this computer.\n") unless File.exist?(note)
    begin
      $selenite_network_grid = Selenite::Network::GridRoom.new(session, grid: grid, assets: { note: note })
    rescue Exception
      session.close
      raise
    end
    session.on(:connect) { |peer| Selenite.status("Shared Grid connected: #{peer}") }
    session.on(:disconnect) { |_peer, reason| Selenite.status("Shared Grid disconnected: #{reason}") }
    session.on(:error) { |_peer, reason| Selenite.status("Shared Grid error: #{reason}") }
    session.on("grid/rejected") { |_peer, data| Selenite.status("Shared Grid rejected: #{data['reason']}") }
    session.on(:state) { |key, _value, _version| Selenite.save if key == "grid_cells" }
    Selenite.status(session.host? ? "Shared Grid listening on #{session.address}" : "Shared Grid connecting")
  end

  check_start = lambda do
    raise "Run net-stop before starting another shared grid" if $selenite_network_grid && !$selenite_network_grid.closed?
    raise "Choose an empty grid before starting Shared Grid" unless Selenite.grid.size.zero?
    raise "Set SELENITE_NET_KEY before launching Selenite" unless ENV["SELENITE_NET_KEY"] && !ENV["SELENITE_NET_KEY"].empty?
  end

  plugin.command("net-host") do |port = "45678", bind = "127.0.0.1"|
    check_start.call
    attach.call(Selenite::Network.host(port: Integer(port), bind: bind, room: "selenite-shared-grid", key: ENV.fetch("SELENITE_NET_KEY")))
  end
  plugin.command("net-join") do |address = "127.0.0.1", port = "45678"|
    check_start.call
    attach.call(Selenite::Network.join(address, port: Integer(port), room: "selenite-shared-grid", key: ENV.fetch("SELENITE_NET_KEY")))
  end
  plugin.command("net-place") do |col = "0", row = "0"|
    raise "Start a shared grid first" unless $selenite_network_grid
    $selenite_network_grid.place(Integer(col), Integer(row), "note")
  end
  plugin.command("net-remove") do |col = "0", row = "0"|
    raise "Start a shared grid first" unless $selenite_network_grid
    $selenite_network_grid.remove(Integer(col), Integer(row))
  end
  plugin.command("net-stop") do
    $selenite_network_grid&.close
    $selenite_network_grid = nil
    Selenite.status("Shared Grid stopped; local cells and saves are preserved")
  end
  plugin.every(0.25) { $selenite_network_grid.poll if $selenite_network_grid && !$selenite_network_grid.closed? }
  plugin.on_unload do
    $selenite_network_grid&.close
    $selenite_network_grid = nil
  end
end

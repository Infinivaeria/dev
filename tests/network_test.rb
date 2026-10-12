require "minitest/autorun"
require_relative "../src/ruby/network"

class NetworkTest < Minitest::Test
  Network = Selenite::Network

  def setup
    @resources = []
  end

  def teardown
    @resources.reverse_each { |resource| resource.close unless resource.closed? }
  end

  def keep(resource)
    @resources << resource
    resource
  end

  def pump(*nodes, timeout: 2)
    deadline = Network.clock + timeout
    loop do
      nodes.each { |node| node.poll unless node.closed? }
      return if yield
      raise "network condition timed out" if Network.clock > deadline
      sleep 0.001
    end
  end

  def pair(**options)
    host = keep(Network.host(port: 0, key: "test-secret", **options))
    client = keep(Network.join("127.0.0.1", port: host.address["port"], key: "test-secret", **options))
    pump(host, client) { client.synced? }
    [host, client]
  end

  def test_authoritative_state_late_join_input_and_disconnect
    host, client = pair
    requests = []
    client_updates = []
    host.on("move") { |peer, payload| requests << [peer, payload] }
    client.on(:state) { |key, value, version| client_updates << [key, value, version] }
    client.send("move", { x: 1, y: 2 })
    pump(host, client) { !requests.empty? }
    assert_equal [client.id, { "x" => 1, "y" => 2 }], requests.first
    host.set("players", { client.id => [1, 2] })
    pump(host, client) { client.version == host.version }
    assert_equal [1, 2], client.state("players")[client.id]
    assert_equal ["players", { client.id => [1, 2] }, 1], client_updates.last
    copy = client.state
    copy["players"].clear
    refute_empty client.state("players")
    returned = host.set("detached", { value: 1.5 })
    returned["value"] = 99
    assert_equal 1.5, host.state("detached")["value"]
    late = keep(Network.join("127.0.0.1", port: host.address["port"], key: "test-secret"))
    pump(host, client, late) { late.synced? }
    assert_equal host.state, late.state
    assert_raises(Network::Error) { client.set("players", {}) }
    assert_raises(ArgumentError) { client.send("_state", {}) }
    assert_raises(Network::Error) { client.broadcast("chat", "hello") }
    disconnected = []
    host.on(:disconnect) { |peer, _reason| disconnected << peer }
    client.close
    pump(host, late) { disconnected.include?(client.id) }
    assert_equal [late.id], host.peers
    assert late.connected?
  end

  def test_direct_peer_mesh_and_callbacks
    nodes = 3.times.map { |index| keep(Network::Node.new(port: 0, id: "peer#{index}", key: "mesh")) }
    nodes[0].connect("127.0.0.1", nodes[1].address["port"])
    nodes[0].connect("127.0.0.1", nodes[2].address["port"])
    nodes[1].connect("127.0.0.1", nodes[2].address["port"])
    pump(*nodes) { nodes.all? { |node| node.peers.size == 2 } }
    messages = []
    nodes[2].on(:message) { |event| messages << event }
    assert_equal 2, nodes[0].broadcast("chat", "hello mesh")
    nodes[1].send(nodes[2].id, "chat", "direct")
    pump(*nodes) { messages.size == 2 }
    assert_equal ["direct", "hello mesh"], messages.map { |event| event[:payload] }.sort
    nodes[0].disconnect(nodes[2].id)
    pump(*nodes) { nodes[2].peers.size == 1 }
    refute_includes nodes[2].peers, nodes[0].id
  end

  def test_wrong_keys_rooms_and_duplicate_ids_are_rejected
    host = keep(Network::Node.new(port: 0, room: "room", key: "correct", id: "host"))
    failures = []
    host.on(:error) { |event| failures << event[:reason] }
    [ { room: "room", key: "wrong", id: "other" },
      { room: "other-room", key: "correct", id: "other" },
      { room: "room", key: "correct", id: "host" } ].each do |options|
      client = keep(Network::Node.new(listen: false, **options))
      client.connect("127.0.0.1", host.address["port"])
      initial = failures.size
      pump(host, client) { failures.size > initial }
      assert_empty host.peers
      client.close
    end
    assert failures.any? { |reason| reason.include?("authentication") }
  end

  def test_signed_messages_reject_tampering_and_clients_cannot_override_host_state
    host, client = pair
    errors = []
    host.on(:error) { |_peer, reason| errors << reason }
    connection = client.node.instance_variable_get(:@connections).first
    body = JSON.generate("type" => "chat", "payload" => "tampered")
    packet = { "kind" => "message", "sequence" => 1, "body" => body, "mac" => "0" * 64 }
    connection[:output] << client.node.__send__(:encode, packet)
    pump(host, client) { !errors.empty? }
    assert_includes errors.first, "authentication"
    assert_empty host.peers

    host, client = pair
    errors = []
    rejected_messages = []
    host.on(:error) { |_peer, reason| errors << reason }
    host.on("after-rejection") { |_peer, data| rejected_messages << data }
    client.node.send(host.id, "_state", { version: 1, key: "score", value: 99 })
    client.send("after-rejection", "must not be delivered")
    pump(host, client) { !errors.empty? }
    assert_nil host.state("score")
    assert_includes errors.first, "only the host"
    assert_empty rejected_messages
  end

  def test_limits_and_non_json_data_do_not_partially_change_state
    host, client = pair(max_frame: 2048, max_queue: 4096)
    assert_raises(ArgumentError) { client.send("bad", Object.new) }
    assert_raises(ArgumentError) { client.send("bad", Float::NAN) }
    cyclic = []; cyclic << cyclic
    assert_raises(ArgumentError) { client.send("bad", cyclic) }
    assert_raises(Network::Error) { client.send("large", "x" * 3000) }
    assert_raises(Network::Error) { host.set("large", "x" * 1000) }
    assert_empty host.state
    client.send("queued", "x" * 1000)
    client.send("queued", "x" * 1000)
    assert_raises(Network::Backpressure) do
      10.times { client.send("queued", "x" * 1000) }
    end
    host.close
    assert_raises(Network::Error) { host.set("closed", true) }
    assert_empty host.state
  end

  def test_configuration_is_detached_and_callback_errors_propagate
    identity = "configured-id".dup
    node = keep(Network::Node.new(port: 0, id: identity))
    identity.replace("changed")
    assert_equal "configured-id", node.id
    endpoint = node.address
    endpoint["port"] = 0
    refute_equal 0, node.address["port"]
    host, client = pair
    client.on(:state) { |_key, _value, _version| raise ArgumentError, "callback failure" }
    host.set("example", 1)
    error = assert_raises(ArgumentError) { pump(host, client) { false } }
    assert_equal "callback failure", error.message
    assert client.connected?
    assert_equal 1, client.state("example")
  end

  def test_raw_malformed_frames_and_handshake_timeout
    node = keep(Network::Node.new(port: 0, handshake_timeout: 0.03))
    errors = []
    node.on(:error) { |event| errors << event[:reason] }
    socket = keep(TCPSocket.new("127.0.0.1", node.address["port"]))
    socket.write([node.max_frame + 1].pack("N"))
    pump(node) { !errors.empty? }
    assert_includes errors.first, "frame length"
    stalled = keep(TCPSocket.new("127.0.0.1", node.address["port"]))
    pump(node) { errors.any? { |reason| reason.include?("handshake timed out") } }
  end

  def test_event_budget_and_order_under_message_burst
    host, client = pair
    received = []
    host.on("burst") { |_peer, data| received << data }
    150.times { |number| client.send("burst", number) }
    assert client.queued_bytes.positive?
    pump(client) { client.queued_bytes.zero? }
    client.close
    pump(host) { received.size == 150 }
    assert_equal (0...150).to_a, received
  end

  def test_ipv6_direct_connections_and_peer_capacity
    begin
      host = keep(Network::Node.new(bind: "::1", port: 0, max_peers: 1))
    rescue Errno::EAFNOSUPPORT, Errno::EADDRNOTAVAIL
      skip "IPv6 loopback is unavailable"
    end
    client = keep(Network::Node.new(listen: false, max_peers: 1))
    client.connect("::1", host.address["port"])
    pump(host, client) { host.peers.size == 1 && client.peers.size == 1 }
    assert_raises(Network::Error) { client.connect("::1", host.address["port"]) }
    messages = []
    host.on(:message) { |event| messages << event[:payload] }
    client.send(host.id, "ipv6", true)
    pump(host, client) { !messages.empty? }
    assert_equal [true], messages
  end

  def test_udp_discovery_uses_sender_address_and_room_filter
    node = keep(Network::Node.new(port: 0, room: "discover"))
    listener = keep(Network::Discovery.new(room: "discover", bind: "127.0.0.1", port: 0, ttl: 0.03))
    sender = keep(Network::Discovery.new(room: "discover", bind: "127.0.0.1", port: 0))
    assert sender.advertise(node, address: "127.0.0.1", port: listener.port)
    peers = listener.poll
    assert_equal [{ "id" => node.id, "host" => "127.0.0.1", "port" => node.address["port"] }], peers
    deadline = Network.clock + 1
    loop do
      break if listener.poll.empty?
      raise "discovery did not expire" if Network.clock > deadline
      sleep 0.005
    end
    other = keep(Network::Discovery.new(room: "other", bind: "127.0.0.1", port: 0))
    assert_raises(Network::Error) { other.advertise(node, address: "127.0.0.1", port: listener.port) }
  end

  class FakeGrid
    attr_reader :cells
    def initialize = @cells = {}
    def size = @cells.size
    def [](col, row)
      path = @cells[[col, row]]
      Struct.new(:path, :kind).new(path, "file") if path
    end
    def replace(col, row, path) = @cells[[col, row]] = path
    def remove(col, row) = @cells.delete([col, row])
  end

  def test_shared_grid_uses_local_assets_and_replicates_client_edits
    host, client = pair
    host_grid, client_grid = FakeGrid.new, FakeGrid.new
    host_room = Network::GridRoom.new(host, grid: host_grid, assets: { tile: "/host/tile.png" })
    client_room = Network::GridRoom.new(client, grid: client_grid, assets: { tile: "/client/tile.png" })
    client_room.place(2, -3, "tile")
    pump(host_room, client_room) { client_grid.size == 1 }
    assert_equal "/host/tile.png", host_grid.cells[[2, -3]]
    assert_equal "/client/tile.png", client_grid.cells[[2, -3]]
    late = keep(Network.join("127.0.0.1", port: host.address["port"], key: "test-secret"))
    pump(host_room, client_room, late) { late.synced? }
    late_grid = FakeGrid.new
    Network::GridRoom.new(late, grid: late_grid, assets: { tile: "/late/tile.png" })
    assert_equal "/late/tile.png", late_grid.cells[[2, -3]]
    host_room.remove(2, -3)
    pump(host_room, client_room) { client_grid.size.zero? }
    assert_empty host_grid.cells
    assert_raises(ArgumentError) { client_room.place(0, 0, "../../secret") }
    assert_raises(ArgumentError) { client_room.place(100_000, 0, "tile") }
    occupied = FakeGrid.new
    occupied.replace(0, 0, "keep")
    assert_raises(ArgumentError) { Network::GridRoom.new(host, grid: occupied, assets: {}) }
    assert_equal "keep", occupied.cells[[0, 0]]
  end

  def test_shared_grid_preserves_unsynchronized_local_edits
    host, client = pair
    grid = FakeGrid.new
    room = Network::GridRoom.new(host, grid: grid, assets: { tile: "/tile.png" })
    room.place(0, 0, "tile")
    grid.replace(0, 0, "/private.png")
    version = host.version
    assert_raises(Network::Error) { room.remove(0, 0) }
    assert_equal "/private.png", grid.cells[[0, 0]]
    assert_equal version, host.version
  end
end

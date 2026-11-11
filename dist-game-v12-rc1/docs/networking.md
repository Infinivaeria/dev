# Multiplayer and peer networking

Selenite and selenite-game expose the same Ruby `Selenite::Network` API.
It works in games, organizer scripts, plugins, and compiled `.rbc` entries.
There are no extra gems or central matchmaking servers. Source builds need
Ruby's standard `socket` and `openssl` libraries; portable packages include
their Ruby extensions and still need compatible system OpenSSL libraries.

Choose one of three building blocks:

| Building block | Purpose |
| --- | --- |
| `Network::Node` / `game.peer` | Direct TCP peers and manually connected meshes; send/broadcast arbitrary JSON messages. |
| `Network.host/join` / `game.host/join` | A host-authoritative session: clients submit requests, the host replicates versioned state, and late joiners receive a snapshot. |
| `Network::GridRoom` | Share an initially empty organizer grid using logical asset IDs mapped to local files. |

UDP `Network::Discovery` can advertise and discover listening nodes on a LAN.
It does not carry game state. Game messages use reliable, ordered TCP frames.
These are direct peer connections; host sessions form a star, while raw nodes
can form a mesh. Connect each mesh edge once. There is no automatic mesh
construction, host election/migration, rollback, prediction, reconnect, file
transfer, NAT hole punching, STUN/TURN service, or public relay service.

## Play Cooperative Gems

[Cooperative Gems](../examples/games/multiplayer.rb) is a working multiplayer
example. Players move with WASD/arrows and collect shared gold gems. The host
simulates movement and awards points. Clients send normalized input rather than
overwriting positions or score. Input expires after 0.5 seconds without updates.
The host persists the shared score in PartitionedArray storage on normal exit;
clients do not open the host's storage directory.

Build or extract a portable runtime. In **both terminals**, set the same secret:

```sh
export SELENITE_NET_KEY='replace-this-with-the-same-private-random-key'
```

Generate a random key once using `ruby -rsecurerandom -e 'puts SecureRandom.hex(32)'`
and give it privately to your players. Environment variables supply runtime
credentials; compiling the game does not embed this key.

Terminal one, host:

```sh
SELENITE_NET_ROLE=host ./dist-game/run-game.sh examples/games/multiplayer.rb --grid /tmp/gems-host.json
```

Terminal two, join:

```sh
SELENITE_NET_ROLE=join ./dist-game/run-game.sh examples/games/multiplayer.rb --grid /tmp/gems-client.json
```

For two computers on a LAN, the host sets `SELENITE_NET_BIND=0.0.0.0` to listen
on its IPv4 interfaces. The other computer sets `SELENITE_NET_HOST` to the
host's numeric LAN address, for example `192.168.1.50`. Use the same port,
room, key, protocol, and game logic on both computers.

| Example setting | Default | Meaning |
| --- | --- | --- |
| `SELENITE_NET_ROLE` | `host` | `host` or `join` |
| `SELENITE_NET_BIND` | `127.0.0.1` | Host listener address |
| `SELENITE_NET_HOST` | `127.0.0.1` | Address to join |
| `SELENITE_NET_PORT` | `45678` | TCP port |
| `SELENITE_NET_ROOM` | `cooperative-gems` | Application/room identifier |
| `SELENITE_NET_KEY` | required | Shared authentication key |
| `SELENITE_MULTIPLAYER_STORE` | sibling `game-data/multiplayer-host` | Host's persistent store directory |

These environment settings belong to the example; library users pass options
directly. Give separate game instances their own grid saves. Only one host
may write a particular partitioned store. To host independent rooms from the
same project, use different ports and `SELENITE_MULTIPLAYER_STORE` directories.

The organizer can run the same game with `./dist/run.sh --game … --grid …`.
For source-free distribution, compile with the runtime you ship:

```sh
./dist-game/run-game.sh --compile examples/games/multiplayer.rb
SELENITE_NET_ROLE=host ./dist-game/run-game.sh examples/games/multiplayer.rbc --grid /tmp/gems-host.json
SELENITE_NET_ROLE=join ./dist-game/run-game.sh examples/games/multiplayer.rbc --grid /tmp/gems-client.json
```

Export the compiled entry through the organizer as usual. Networking is
embedded in both executables; it does not need an additional helper file.
See [bytecode distribution](game.md#compile-and-distribute-ruby-bytecode).

## Host-authoritative games

Inside the game setup, use `game.host` or `game.join`. Games poll their owned
connections before each update and close them during shutdown, including if
a callback or save raises. Networking callbacks run on the game thread;
keep them short. Do not also manually poll a game-owned session.

```ruby
Selenite::Game.run(audio: false) do |game|
  session = game.host(port: 45678, room: "my-game/v1", key: ENV.fetch("GAME_KEY"))
  session.set("score", 0)
  session.on("collect") do |peer, request|
    # Check peer identity, distance, ownership and rate limits in your game.
    if request == { "gem" => "first" } && session.state("score") == 0
      session.set("score", 1)
    end
  end
  game.draw_ui { game.text("Score: #{session.state('score')}", 20, 20) }
end
```

The corresponding client calls:

```ruby
session = game.join("127.0.0.1", port: 45678, room: "my-game/v1", key: ENV.fetch("GAME_KEY"))
game.update do
  session.send("collect", { gem: "first" }) if session.synced? && game.pressed?(Raylib::KEY_SPACE)
end
session.on(:state) { |key, value, version| puts "#{key}=#{value.inspect} at revision #{version}" }
```

Pass hash payloads explicitly, keeping them separate from the method's
`to:` option: `session.send("collect", { gem: "first" }, to: peer_id)`.
Clients can send only to their host. A host's `send` without `to:` delivers a
local request to its own handlers; `send(..., to: peer_id)` targets a client.
Use `broadcast` for a host message to every connected client and local handlers.
Raw `Node` messages have no host restriction.

`session.set` is host-only. State is a string-keyed map of JSON values. Each
set increments its revision and sends an ordered delta; the initial snapshot
includes all current keys and its revision. `state` returns a detached copy.
Set a key to `nil` to represent an empty value; keys remain in snapshots.
`connected?` means the TCP handshake completed (or that a host is open).
`synced?` additionally means a client received its initial state snapshot.

| Session method | Behavior |
| --- | --- |
| `Network.host(**options)` | Create a listening host session. |
| `Network.join(address, port:, **options)` | Start a nonblocking client connection; it has no listener. |
| `id`, `peers`, `address`, `node` | Local ID, connected remote IDs, host listener address, and underlying node. |
| `host?`, `connected?`, `synced?`, `closed?` | Lifecycle/role queries. |
| `set(key, value)` | Host-only authoritative update. |
| `state(key = nil)`, `version` | Detached selected/full state and current revision. |
| `queued_bytes` | Bytes waiting in the underlying node's outgoing queues. |
| `send(type, payload = nil, to: nil)` | Submit a request or target a client. |
| `broadcast(type, payload = nil)` | Host-only broadcast, including local handlers. |
| `on(type) { … }` | Register an application message callback using a string type. |
| `poll`, `close` | Progress I/O / release sockets. Manually poll outside games. |

| Callback | Arguments |
| --- | --- |
| `on("input")` or another string type | `peer_id, payload` |
| `on(:message)` | `peer_id, type, payload` |
| `on(:connect)` | `peer_id` |
| `on(:disconnect)` | `peer_id, reason` |
| `on(:error)` | `peer_id_or_nil, reason` |
| `on(:state)` | `key, value, revision` |
| `on(:snapshot)` | `full_state, revision` (clients) |

Incoming client state-control messages are rejected. Application message names
starting with `_` are reserved. Client inputs still require game-specific
validation: authentication does not prove that a move, trade or score is valid.
The example demonstrates host movement speed/clamping and per-peer input ownership.

## Direct P2P and meshes

Every listening node can accept other peers and initiate outgoing connections.
IDs must be unique within connected peers. Omit `id:` for a random ID.
The following creates two direct peers without a central game server:

```ruby
first = Selenite::Network::Node.new(port: 0, id: "alice", room: "chat/v1", key: "private-key")
second = Selenite::Network::Node.new(port: 0, id: "bob", room: "chat/v1", key: "private-key")
first.connect("127.0.0.1", second.address["port"])
second.on(:message) { |event| puts "#{event[:peer]}: #{event[:payload]}" }

deadline = Selenite::Network.clock + 5
until first.peers.include?("bob")
  first.poll
  second.poll
  raise "Connection failed" if Selenite::Network.clock > deadline
  sleep 0.005
end
first.send("bob", "chat", "Hello")
20.times { first.poll; second.poll; sleep 0.005 }
first.close
second.close
```

In a game use `node = game.peer(**options)` and callbacks. Three peers can
form a mesh by connecting A→B, A→C and B→C. Broadcast reaches only directly
connected neighbors; messages are not forwarded automatically. Avoid creating
duplicate or simultaneous connections for the same pair. A disconnected peer
can be reconnected explicitly with `connect`, once the old connection has
been removed; sessions do not automatically retry or elect a replacement host.

| Node method | Behavior |
| --- | --- |
| `new(**options)` | Open a listener by default; `listen: false` creates an outgoing-only node. |
| `connect(numeric_address, port)` | Begin an asynchronous connection; returns a local pending-connection handle. |
| `id`, `room`, `address`, `max_frame` | Configuration; listener address is a hash with `"host"`/`"port"`, or `nil` without a listener. |
| `peers`, `pending`, `closed?` | Ready peer IDs, pending count, closed state. |
| `queued_bytes(peer_id_or_handle = nil)` | Outgoing queued bytes for one connection, or all connections. |
| `send(peer_id, type, payload = nil)` | Queue a reliable message to a ready peer. |
| `broadcast(type, payload = nil, except: nil)` | Queue to all ready peers except one; returns the delivery count. |
| `on(event) { \|event_hash\| … }` | Callback for `:connect`, `:message`, `:disconnect`, or `:error`. |
| `poll(max_events: 128)` | Nonblocking progress; return up to that many event hashes and invoke their callbacks. |
| `disconnect(peer_id_or_handle, reason: "…")` | Cancel/drop a connection. |
| `close` | Release all sockets immediately; queued messages are not guaranteed to flush. |

Event hashes use symbol keys: `:event`, `:peer`, and, for messages, `:type`
and `:payload`. Errors/disconnects include `:reason`; errors also include the
pending connection's `:handle`. Successful `connect` does not imply the
handshake succeeded; wait for a connect event or a ready peer ID.

## Limits and timing

| Node option | Default | Constraint |
| --- | --- | --- |
| `bind` | `127.0.0.1` | Numeric IPv4 or IPv6 listener address |
| `port` | `0` | 0 chooses an available listening port; otherwise 1–65535 |
| `listen` | `true` | Open a listener; sessions choose this from their role |
| `room` | `selenite` | 1–128 UTF-8 bytes; include a game/protocol version in the name |
| `key` | `nil` | Shared authentication key; nil means an open unauthenticated room |
| `id` | random | 1–64 ASCII letters, digits, `_`, `-`, `.`, `/` |
| `max_peers` | `16` | 1–128, including handshakes |
| `max_frame` | `65536` | 1024–1048576 bytes, including encoded protocol overhead |
| `max_queue` | `262144` | At least `max_frame + 4`, at most 16777216 bytes per peer |
| `timeout` | `30.0` | Finite, at least 1 second without received data |
| `handshake_timeout` | `5.0` | Finite positive seconds |

Payloads contain only JSON hashes, arrays, UTF-8 strings, finite numbers,
booleans and nil. Symbol keys become strings. Duplicate converted keys,
unsupported Ruby objects, cycles and nesting beyond 32 levels are rejected.
Wire framing uses a 4-byte big-endian length and JSON. Neither `eval` nor
`Marshal.load` is used for received data.

Each poll bounds accepts, bytes read, frames processed, and queued events.
Sockets are nonblocking; connect addresses are numeric to avoid DNS in the
render loop. Queueing a message is not an acknowledgement that a game applied
it. Handle `Network::Backpressure` when a peer's queue fills; coalesce state,
reduce update frequency, or explicitly disconnect that peer. Oversized messages
raise `Network::Error`. A broadcast checks all recipients' queue capacities
before queueing to any of them. Peer errors/disconnects become events; exceptions
in your callbacks propagate to the caller. `poll` after `close` raises an error.

State snapshots are limited to **one third of `max_frame`** measured as encoded
state JSON, leaving room for the enclosing protocol. Do not broadcast an entire
partitioned world every frame. Keep active player/lobby state small, and use
application messages for requested regions, records or chunks with bounded
sizes. The library adds signed heartbeat messages while polling; call `poll`
regularly on **both ends**. TCP can delay later updates behind an earlier lost
packet; this transport is suitable for prototypes, collaboration and modest
cooperative games, not a claim of esports-grade latency.

## LAN discovery

```ruby
node = Selenite::Network::Node.new(bind: "0.0.0.0", port: 45678, room: "my-game/v1", key: ENV.fetch("GAME_KEY"))
discovery = Selenite::Network::Discovery.new(room: "my-game/v1", port: 45700)
discovery.advertise(node)   # repeat periodically, for example once per second
discovery.poll.each do |peer|
  puts "#{peer['id']} at #{peer['host']}:#{peer['port']}"
end
discovery.close
node.close
```

Other computers poll a discovery object with the same room and UDP port.
`advertise` accepts `address:` for a subnet broadcast/unicast destination and
`port:` for a different discovery destination port. `Discovery.new` accepts
`bind:`, `port:` and `ttl:` (default 5 seconds); `port: 0` chooses an available
UDP port, exposed by `discovery.port`. Entries expire, and the cache holds at
most 128 endpoints. `close` releases its UDP socket. Discovery is separate
from game-owned sessions: arrange polling, advertisements and cleanup yourself.

Discovery is IPv4 LAN broadcast, best effort, and unauthenticated. It uses the
packet's actual sender IP, but an advertisement is still only a hint. It never
automatically connects. Confirm a discovered address and use the room key for
the TCP handshake. Routers, Wi-Fi client isolation, VPNs, containers or firewalls
may block broadcasts; use a direct address when discovery cannot find a host.

## Shared grids in the organizer

The bundled [Shared Grid plugin](../examples/plugins/network_grid.rb) provides
an actual collaboration example. Start two organizer instances with the same
`SELENITE_NET_KEY` and separate initially empty profiles/grid files. Install
the example plugins in the Plugins panel (or install this file as a plugin)
and enable **Shared Grid**. Open the Ruby console and run:

```ruby
# First organizer, in an empty grid:
Selenite.run("net-host", "45678", "127.0.0.1")
# Second organizer, in its own empty grid:
Selenite.run("net-join", "127.0.0.1", "45678")
# Either participant, after connection:
Selenite.run("net-place", "2", "3")
Selenite.run("net-remove", "2", "3")
Selenite.run("net-stop")
```

Use `0.0.0.0` for a LAN host and its numeric LAN IP for joining. The plugin
polls at 0.25-second intervals, saves replicated changes locally, and closes
its session on plugin reload/disable or profile switch. Stop preserves local
cells and saves. Start a new session in an empty grid; this example does not
reconstruct an old room from a saved profile.

The example's `note` asset maps to a locally created text file on each computer.
GridRoom sends **logical asset IDs and integer coordinates**, not file contents
or remote filesystem paths. To share game maps using your own local assets:

```ruby
shared = Selenite::Network::GridRoom.new(session, grid: Selenite.grid,
  assets: { grass: "/local/assets/grass.png", stone: "/local/assets/stone.png" })
shared.place(0, 0, "grass")
shared.remove(0, 0)
```

All participants must map the same asset IDs to their own files. Coordinates
are integers within -4096..4096. Clients submit edits to the host; only the
host changes replicated `grid_cells` state. Unknown assets and invalid
coordinates are refused. Subscribe to `session.on("grid/rejected")` for remote
rejection reasons. A room owns its synchronized cells: use `place`/`remove`
instead of local paste, stacks or nested-grid edits on those cells. Conflicting
local edits are preserved and cause an error instead of being overwritten.
GridRoom is a bounded collaborative layout, not automatic sharing of profiles,
inventories, every nested grid, filesystem paths, or arbitrary files.

`GridRoom#session` exposes the underlying session. `GridRoom#poll` forwards to
it for manual use; a game-owned session
already polls automatically. `GridRoom#close` closes its session. Plugin authors
can use `plugin.on_unload { session.close }` to release resources when plugins
reset; cleanup callbacks run once and failures are reported without stopping
the remaining plugin cleanups.

## Network exposure and persistence

Listeners default to loopback. LAN access requires explicitly binding an
interface and allowing its TCP port through the local firewall. For Internet
play the host needs a reachable public IPv4/IPv6 address, a configured port
forward, or a VPN that makes the peers reachable. Carrier-grade NAT may prevent
direct inbound connections; this implementation does not provision a relay.
UDP discovery does not cross the Internet and cannot open NAT mappings.

With `key:`, the handshake proves shared-key possession using HMAC-SHA256 over
both IDs and fresh nonces. Messages include a per-connection signed sequence,
so modified/replayed messages on that connection are rejected. This provides
authentication/integrity, **not transport encryption**: payloads are visible
to someone observing the connection. Use a private network or encrypted VPN
for confidential traffic, choose a strong key, and share it only with trusted
players. Peer IDs are application identities, not verified people. Nodes with
`key: nil` are explicitly open and unauthenticated.

Networking does not automatically persist or remotely share partitioned stores.
Keep one authoritative writer per store. Serialize logical record data, validate
requests, and flush host changes through `game.save`. Do not share live Ruby
objects, textures, functions, sockets, or storage lock files. A client can keep
its own replica in a separate store; active/dormant/revenant labels describe
that local store's residency, not the host's. See [partitioned game storage](game-storage.md).

## Validation and troubleshooting

```sh
ruby tests/network_test.rb
cargo test --features scripting
python3 tests/network_runtime_smoke.py
python3 tests/network_runtime_smoke.py --graphics
```

The Ruby tests use actual loopback TCP/UDP sockets. The portable smoke test
extracts the game runtime and starts separate processes, exercises direct
messages, late-join state, shared grids, storage, and bytecode without source.
`--graphics` also opens the cooperative game in two processes. Build both
portable packages first. Tests use temporary saves and stores.

- **Connection pending/failed:** check numeric address, listener port, bind
  interface, firewall, matching room/key, and regular polling on both ends.
- **Duplicate peer ID:** use unique IDs and only one direct connection per pair.
- **Authentication error:** use the same nonempty key and room; inspect local
  error callbacks, never send the key as an application message.
- **Backpressure:** reduce/coalesce sends or drop the stalled peer. Stop sending
  inputs after disconnect; poll remains nonblocking.
- **State too large:** keep snapshots small and stream selected records/regions.
- **Grid conflict:** use a dedicated empty grid and the shared edit API. Stop
  before changing its synchronized cells through local organizer actions.
- **Missing OpenSSL extension:** keep the runtime's Ruby extension tree and
  compatible system `libssl`/`libcrypto`; launch through the portable script.

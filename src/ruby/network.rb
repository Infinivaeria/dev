require "socket"
require "json"
require "openssl"
require "securerandom"

module Selenite
  module Network
    PROTOCOL = 1
    class Error < StandardError; end
    class Backpressure < Error; end

    def self.clock = Process.clock_gettime(Process::CLOCK_MONOTONIC)

    def self.name(value)
      value = value.to_s
      raise ArgumentError, "use 1–64 letters, digits, underscores, hyphens, dots or slashes" unless /\A[a-zA-Z0-9_.\/-]{1,64}\z/.match?(value)
      value
    end

    def self.copy(value, depth = 0)
      raise ArgumentError, "network data is cyclic or nested more than 32 levels" if depth > 32
      case value
      when Hash
        value.each_with_object({}) do |(key, item), result|
          raise ArgumentError, "network keys must be strings or symbols" unless key.is_a?(String) || key.is_a?(Symbol)
          raise ArgumentError, "duplicate network key #{key}" if result.key?(key.to_s)
          result[key.to_s] = copy(item, depth + 1)
        end
      when Array then value.map { |item| copy(item, depth + 1) }
      when String
        encoded = value.encode("UTF-8")
        raise ArgumentError, "network strings must be valid UTF-8" unless encoded.valid_encoding?
        encoded
      when Integer, TrueClass, FalseClass, NilClass then value
      when Float
        raise ArgumentError, "network numbers must be finite" unless value.finite?
        value
      else raise ArgumentError, "network payloads must contain only JSON data"
      end
    end

    def self.room(value)
      value = value.to_s.encode("UTF-8")
      raise ArgumentError, "room must contain 1–128 UTF-8 bytes" unless value.valid_encoding? && value.bytesize.between?(1, 128)
      value
    end

    def self.port(value, allow_zero: false)
      value = Integer(value)
      raise ArgumentError, "invalid network port" unless value.between?(allow_zero ? 0 : 1, 65_535)
      value
    end

    def self.address(host, port, passive: false)
      host = host.to_s
      flags = Socket::AI_NUMERICHOST | Socket::AI_NUMERICSERV
      flags |= Socket::AI_PASSIVE if passive
      Addrinfo.getaddrinfo(host, port.to_s, nil, Socket::SOCK_STREAM, nil, flags).first
    rescue SocketError
      raise ArgumentError, "use a numeric IPv4 or IPv6 address (DNS is not resolved in the frame loop)"
    end

    def self.host(**options) = Session.host(**options)
    def self.join(host, **options) = Session.join(host, **options)

    class Node
      attr_reader :id, :room, :max_frame

      def initialize(bind: "127.0.0.1", port: 0, listen: true, room: "selenite", key: nil,
                     id: nil, max_peers: 16, max_frame: 65_536, max_queue: 262_144,
                     timeout: 30.0, handshake_timeout: 5.0)
        @id = Network.name(id || SecureRandom.hex(12)).dup.freeze
        @room = Network.room(room).freeze
        @key = key.nil? ? nil : key.to_s.dup.freeze
        raise ArgumentError, "key cannot be empty" if @key&.empty?
        @max_peers = Integer(max_peers)
        @max_frame = Integer(max_frame)
        @max_queue = Integer(max_queue)
        @timeout = Float(timeout)
        @handshake_timeout = Float(handshake_timeout)
        unless @max_peers.between?(1, 128) && @max_frame.between?(1024, 1_048_576) &&
               @max_queue.between?(@max_frame + 4, 16_777_216) && @timeout.finite? && @timeout >= 1 &&
               @handshake_timeout.finite? && @handshake_timeout.positive?
          raise ArgumentError, "invalid network limits"
        end
        @connections = []
        @events = []
        @callbacks = Hash.new { |hash, event| hash[event] = [] }
        @closed = false
        @counter = 0
        if listen
          endpoint = Network.address(bind, Network.port(port, allow_zero: true), passive: true)
          @listener = Socket.new(endpoint.afamily, Socket::SOCK_STREAM, 0)
          @listener.setsockopt(Socket::SOL_SOCKET, Socket::SO_REUSEADDR, true)
          @listener.setsockopt(Socket::IPPROTO_IPV6, Socket::IPV6_V6ONLY, true) if endpoint.ipv6?
          @listener.bind(endpoint)
          @listener.listen(@max_peers)
          @address = { "host" => @listener.local_address.ip_address, "port" => @listener.local_address.ip_port }
        end
      rescue Exception
        @listener&.close
        raise
      end

      def closed? = @closed
      def address = Network.copy(@address)
      def peers = @connections.select { |connection| connection[:ready] }.map { |connection| connection[:peer] }
      def pending = @connections.count { |connection| !connection[:ready] }
      def queued_bytes(peer = nil)
        connections = peer.nil? ? @connections : @connections.select { |connection| connection[:peer] == peer.to_s || connection[:handle] == peer }
        connections.sum { |connection| connection[:output].bytesize }
      end

      def on(event, &callback)
        raise ArgumentError, "callback required" unless callback
        @callbacks[event.to_sym] << callback
        self
      end

      def connect(host, port)
        ensure_open
        raise Error, "peer limit reached" if @connections.size >= @max_peers
        endpoint = Network.address(host, Network.port(port))
        socket = Socket.new(endpoint.afamily, Socket::SOCK_STREAM, 0)
        result = socket.connect_nonblock(endpoint, exception: false)
        connection = attach(socket, result == :wait_writable)
        connection[:handle]
      rescue Exception
        socket&.close unless @connections&.any? { |connection| connection[:socket] == socket }
        raise
      end

      def send(peer, type, payload = nil)
        ensure_open
        connection = @connections.find { |entry| entry[:ready] && entry[:peer] == peer.to_s }
        raise Error, "peer is not connected: #{peer}" unless connection
        body = message_body(type, payload)
        frame = message_frame(connection, body)
        enqueue(connection, frame)
        connection[:sent] += 1
        true
      end

      def broadcast(type, payload = nil, except: nil)
        ensure_open
        body = message_body(type, payload)
        deliveries = @connections.select { |connection| connection[:ready] && connection[:peer] != except }
                                 .map { |connection| [connection, message_frame(connection, body)] }
        deliveries.each { |connection, frame| check_queue(connection, frame) }
        deliveries.each do |connection, frame|
          enqueue(connection, frame)
          connection[:sent] += 1
        end
        deliveries.size
      end

      def disconnect(peer, reason: "closed by application")
        @connections.select { |connection| connection[:peer] == peer.to_s || connection[:handle] == peer }
                    .each { |connection| drop(connection, reason) }
        true
      end

      def poll(max_events: 128)
        ensure_open
        max_events = Integer(max_events)
        raise ArgumentError, "max_events must be 1–4096" unless max_events.between?(1, 4096)
        accept_connections if @listener && @events.size < 256
        @connections.dup.each do |connection|
          break if @events.size >= 256
          begin
            progress(connection)
          rescue Error, SystemCallError, IOError, SocketError, JSON::ParserError, ArgumentError => error
            drop(connection, error.message, error: true)
          end
        end
        events = @events.shift(max_events)
        @connections.rotate! unless @connections.empty?
        events.each { |event| @callbacks[event[:event]].each { |callback| callback.call(event) } }
        events
      end

      def close
        return false if @closed
        @closed = true
        @listener&.close
        @connections.each { |connection| connection[:socket].close }
        @connections.clear
        @events.clear
        true
      end

      private

      def ensure_open
        raise Error, "network node is closed" if @closed
      end

      def attach(socket, connecting)
        socket.setsockopt(Socket::IPPROTO_TCP, Socket::TCP_NODELAY, true)
        @counter += 1
        now = Network.clock
        connection = { socket: socket, handle: @counter, connecting: connecting, ready: false,
                       input: "".b, output: "".b, nonce: SecureRandom.hex(16), peer: nil,
                       sent: 0, received: 0, started: now, last_read: now, last_ping: now }
        @connections << connection
        enqueue(connection, encode("kind" => "hello", "protocol" => PROTOCOL, "room" => @room,
                                   "id" => @id, "nonce" => connection[:nonce]))
        connection
      end

      def accept_connections
        8.times do
          result = @listener.accept_nonblock(exception: false)
          break if result == :wait_readable
          socket, = result
          if @connections.size >= @max_peers
            socket.close
          else
            attach(socket, false)
          end
        end
      end

      def encode(packet)
        bytes = JSON.generate(packet).b
        raise Error, "message exceeds #{@max_frame} byte frame limit" if bytes.bytesize > @max_frame
        [bytes.bytesize].pack("N") + bytes
      end

      def check_queue(connection, frame)
        raise Backpressure, "peer #{connection[:peer]} is too slow; outgoing queue is full" if connection[:output].bytesize + frame.bytesize > @max_queue
      end

      def enqueue(connection, frame)
        check_queue(connection, frame)
        connection[:output] << frame
      end

      def message_body(type, payload)
        JSON.generate("type" => Network.name(type), "payload" => Network.copy(payload))
      end

      def signature(connection, sequence, body, incoming: false)
        return nil unless @key
        nonces = [connection[:nonce], connection[:remote_nonce]]
        nonces.reverse! if incoming
        OpenSSL::HMAC.hexdigest("SHA256", @key, JSON.generate([*nonces, sequence, body]))
      end

      def proof(connection, incoming: false)
        return nil unless @key
        identities = [@id, connection[:nonce], connection[:peer], connection[:remote_nonce]]
        identities = identities.rotate(2) if incoming
        OpenSSL::HMAC.hexdigest("SHA256", @key, JSON.generate([PROTOCOL, @room, *identities]))
      end

      def matches_signature?(expected, actual)
        return actual.nil? if expected.nil?
        actual.is_a?(String) && actual.bytesize == expected.bytesize && OpenSSL.fixed_length_secure_compare(expected, actual)
      end

      def message_frame(connection, body)
        sequence = connection[:sent] + 1
        encode("kind" => "message", "sequence" => sequence, "body" => body,
               "mac" => signature(connection, sequence, body))
      end

      def progress(connection)
        socket = connection[:socket]
        if connection[:connecting]
          return expire(connection) unless IO.select(nil, [socket], nil, 0)
          error = socket.getsockopt(Socket::SOL_SOCKET, Socket::SO_ERROR).int
          raise Error, "connection failed: #{SystemCallError.new('connect', error).message}" unless error.zero?
          connection[:connecting] = false
        end
        unless connection[:output].empty?
          written = socket.write_nonblock(connection[:output], exception: false)
          connection[:output] = connection[:output].byteslice(written..) if written.is_a?(Integer)
        end
        if connection[:input].bytesize < @max_frame + 4
          bytes = socket.read_nonblock([65_536, @max_frame + 4 - connection[:input].bytesize].min, exception: false)
          connection[:eof] = true if bytes.nil?
          if bytes.is_a?(String)
            connection[:input] << bytes
            connection[:last_read] = Network.clock
          end
        end
        32.times do
          break if @events.size >= 256 || connection[:input].bytesize < 4
          length = connection[:input].unpack1("N")
          raise Error, "invalid network frame length" unless length.between?(1, @max_frame)
          break if connection[:input].bytesize < length + 4
          packet = JSON.parse(connection[:input].byteslice(4, length), max_nesting: 40)
          connection[:input] = connection[:input].byteslice(length + 4..)
          receive(connection, packet)
        end
        if connection[:eof]
          if connection[:input].empty?
            return drop(connection, "peer closed connection")
          elsif connection[:input].bytesize < 4 || connection[:input].bytesize < connection[:input].unpack1("N") + 4
            raise Error, "peer closed with an incomplete frame"
          end
        end
        expire(connection)
        if connection[:ready] && Network.clock - connection[:last_ping] >= @timeout / 3
          body = message_body("_ping", nil)
          enqueue(connection, message_frame(connection, body))
          connection[:sent] += 1
          connection[:last_ping] = Network.clock
        end
      end

      def expire(connection)
        now = Network.clock
        raise Error, "handshake timed out" if !connection[:ready] && now - connection[:started] > @handshake_timeout
        raise Error, "peer timed out" if connection[:ready] && now - connection[:last_read] > @timeout
      end

      def receive(connection, packet)
        raise Error, "network packet must be an object" unless packet.is_a?(Hash)
        case packet["kind"]
        when "hello"
          raise Error, "duplicate or incompatible handshake" if connection[:peer] || packet["protocol"] != PROTOCOL || packet["room"] != @room
          peer = Network.name(packet["id"]).freeze
          nonce = packet["nonce"]
          raise Error, "invalid peer nonce or identity" unless nonce.is_a?(String) && /\A[0-9a-f]{32}\z/.match?(nonce) && peer != @id
          raise Error, "duplicate peer ID" if @connections.any? { |other| other != connection && other[:peer] == peer }
          connection[:peer] = peer
          connection[:remote_nonce] = nonce
          enqueue(connection, encode("kind" => "proof", "mac" => proof(connection)))
        when "proof"
          raise Error, "invalid authentication order" unless connection[:peer] && !connection[:ready]
          raise Error, "room authentication failed" unless matches_signature?(proof(connection, incoming: true), packet["mac"])
          connection[:ready] = true
          @events << { event: :connect, peer: connection[:peer] }
        when "message"
          raise Error, "message received before authentication" unless connection[:ready]
          sequence, body = packet.values_at("sequence", "body")
          raise Error, "invalid message sequence" unless sequence.is_a?(Integer) && sequence == connection[:received] + 1 && body.is_a?(String)
          raise Error, "message authentication failed" unless matches_signature?(signature(connection, sequence, body, incoming: true), packet["mac"])
          message = JSON.parse(body, max_nesting: 34)
          raise Error, "invalid message body" unless message.is_a?(Hash) && message.key?("payload")
          type = Network.name(message["type"])
          payload = Network.copy(message["payload"])
          connection[:received] = sequence
          @events << { event: :message, peer: connection[:peer], type: type, payload: payload } unless type == "_ping"
        else raise Error, "unknown network packet"
        end
      end

      def drop(connection, reason, error: false)
        return unless @connections.delete(connection)
        connection[:socket].close
        @events << { event: :error, peer: connection[:peer], handle: connection[:handle], reason: reason } if error
        @events << { event: :disconnect, peer: connection[:peer], reason: reason } if connection[:ready]
      end
    end

    class Session
      attr_reader :node, :version

      def self.host(**options) = new(Node.new(**options, listen: true), true)
      def self.join(host, port:, **options)
        node = Node.new(**options, listen: false)
        node.connect(host, port)
        new(node, false)
      rescue Exception
        node&.close
        raise
      end

      def initialize(node, hosting)
        @node = node
        @hosting = hosting
        @state = {}
        @version = 0
        @host_id = nil
        @synced = hosting
        @rejected_peers = {}
        @callbacks = Hash.new { |hash, type| hash[type] = [] }
      end

      def host? = @hosting
      def id = @node.id
      def peers = @node.peers
      def queued_bytes = @node.queued_bytes
      def address = @node.address
      def closed? = @node.closed?
      def connected? = !closed? && (host? || (@host_id && peers.include?(@host_id)))
      def synced? = connected? && @synced
      def state(key = nil) = Network.copy(key.nil? ? @state : @state[key.to_s])

      def on(type, &callback)
        raise ArgumentError, "callback required" unless callback
        @callbacks[type.is_a?(Symbol) ? type : Network.name(type)] << callback
        self
      end

      def send(type, payload = nil, to: nil)
        type = user_type(type)
        payload = Network.copy(payload)
        raise Error, "session is not connected" unless connected?
        if host?
          to ? @node.send(to, type, payload) : deliver(id, type, payload)
        else
          raise Error, "clients send requests only to their host" if to && to != @host_id
          @node.send(@host_id, type, payload)
        end
        true
      end

      def broadcast(type, payload = nil)
        raise Error, "only the host can broadcast session messages" unless host?
        type = user_type(type)
        payload = Network.copy(payload)
        @node.broadcast(type, payload)
        deliver(id, type, payload)
        true
      end

      def set(key, value)
        raise Error, "only the host can set authoritative state" unless host?
        key = Network.name(key)
        value = Network.copy(value)
        candidate = @state.merge(key => value)
        validate_state(candidate)
        next_version = @version + 1
        @node.broadcast("_state", { "key" => key, "value" => value, "version" => next_version })
        @state = candidate
        @version = next_version
        emit(:state, key, Network.copy(value), @version)
        Network.copy(value)
      end

      def poll
        events = @node.poll
        events.each do |event|
          break if closed?
          case event[:event]
          when :connect
            @rejected_peers.delete(event[:peer])
            if host?
              begin
                @node.send(event[:peer], "_snapshot", { "state" => @state, "version" => @version })
              rescue Error => error
                @node.disconnect(event[:peer], reason: error.message)
                emit(:error, event[:peer], error.message)
                next
              end
            else
              @host_id = event[:peer]
            end
            emit(:connect, event[:peer])
          when :disconnect
            unless host?
              @host_id = nil
              @synced = false
            end
            emit(:disconnect, event[:peer], event[:reason])
          when :error then emit(:error, event[:peer], event[:reason])
          when :message
            next if @rejected_peers[event[:peer]]
            if event[:type].start_with?("_")
              changes = nil
              begin
                changes = receive_state(event)
              rescue Error, ArgumentError => error
                @rejected_peers[event[:peer]] = true
                @rejected_peers.shift if @rejected_peers.size > 256
                @node.disconnect(event[:peer], reason: error.message)
                emit(:error, event[:peer], error.message)
              end
              if changes
                changes[:updates].each { |key, value| emit(:state, key, Network.copy(value), changes[:version]) }
                emit(:snapshot, changes[:snapshot], changes[:version]) if changes[:snapshot]
              end
            else
              deliver(event[:peer], event[:type], event[:payload])
            end
          end
        end
        events
      end

      def close = @node.close

      private

      def user_type(type)
        type = Network.name(type)
        raise ArgumentError, "message names starting with _ are reserved" if type.start_with?("_")
        type
      end

      def emit(type, *args) = @callbacks[type].each { |callback| callback.call(*args) }
      def deliver(peer, type, payload)
        emit(type, peer, Network.copy(payload))
        emit(:message, peer, type, Network.copy(payload))
      end

      def validate_state(state)
        raise Error, "session state exceeds one third of the frame limit; stream larger worlds as application messages" unless JSON.generate(state).bytesize <= @node.max_frame / 3
      end

      def receive_state(event)
        raise Error, "only the host may send session state" if host? || event[:peer] != @host_id
        payload = event[:payload]
        unless payload.is_a?(Hash) && payload["version"].is_a?(Integer) && payload["version"] >= 0
          raise Error, "invalid state version"
        end
        case event[:type]
        when "_snapshot"
          state = payload["state"]
          raise Error, "invalid state snapshot" unless state.is_a?(Hash) && state.keys.all? { |key| Network.name(key) == key }
          validate_state(state)
          @state = Network.copy(state)
          @version = payload["version"]
          @synced = true
          { updates: @state.to_a, snapshot: Network.copy(@state), version: @version }
        when "_state"
          key = Network.name(payload["key"])
          raise Error, "invalid state update order" unless payload["version"] == @version + 1 && payload.key?("value")
          candidate = @state.merge(key => Network.copy(payload["value"]))
          validate_state(candidate)
          @state = candidate
          @version = payload["version"]
          { updates: [[key, @state[key]]], version: @version }
        else raise Error, "unknown session control message"
        end
      end
    end

    class Discovery
      def initialize(room: "selenite", bind: "0.0.0.0", port: 45_700, ttl: 5.0)
        @room = Network.room(room)
        @port = Network.port(port, allow_zero: true)
        @ttl = Float(ttl)
        raise ArgumentError, "discovery ttl must be positive and finite" unless @ttl.finite? && @ttl.positive?
        endpoint = Network.address(bind, @port, passive: true)
        raise ArgumentError, "LAN broadcast discovery uses IPv4" unless endpoint.ipv4?
        @socket = UDPSocket.new
        @socket.do_not_reverse_lookup = true
        @socket.setsockopt(Socket::SOL_SOCKET, Socket::SO_REUSEADDR, true)
        @socket.setsockopt(Socket::SOL_SOCKET, Socket::SO_BROADCAST, true)
        @socket.bind(bind, @port)
        @port = @socket.local_address.ip_port
        @peers = {}
      rescue Exception
        @socket&.close
        raise
      end

      attr_reader :port
      def closed? = @socket.closed?

      def advertise(node, address: "255.255.255.255", port: @port)
        raise Error, "only listening nodes can advertise" unless node.address && !node.closed?
        raise Error, "discovery and node rooms must match" unless node.room == @room
        endpoint = Network.address(address, Network.port(port))
        raise ArgumentError, "LAN broadcast discovery uses IPv4" unless endpoint.ipv4?
        packet = JSON.generate("protocol" => PROTOCOL, "room" => @room, "id" => node.id,
                               "port" => node.address["port"])
        @socket.sendmsg_nonblock(packet, 0, endpoint.to_sockaddr, exception: false) != :wait_writable
      end

      def poll
        32.times do
          result = @socket.recvfrom_nonblock(1024, exception: false)
          break if result == :wait_readable
          bytes, sender = result
          begin
            packet = JSON.parse(bytes, max_nesting: 4)
            next unless packet.is_a?(Hash) && packet["protocol"] == PROTOCOL && packet["room"] == @room
            peer = { "id" => Network.name(packet["id"]), "host" => sender[3], "port" => Network.port(packet["port"]) }
            @peers[[peer["host"], peer["port"]]] = [peer, Network.clock]
            @peers.delete(@peers.min_by { |_endpoint, entry| entry[1] }.first) if @peers.size > 128
          rescue JSON::ParserError, ArgumentError, TypeError
            next
          end
        end
        @peers.delete_if { |_endpoint, entry| Network.clock - entry[1] > @ttl }
        Network.copy(@peers.values.map(&:first))
      end

      def close
        return false if closed?
        @socket.close
        @peers.clear
        true
      end
    end

    class GridRoom
      attr_reader :session

      def initialize(session, grid:, assets:)
        raise ArgumentError, "shared grid must initially be empty" unless grid.size.zero?
        @session, @grid = session, grid
        @assets = assets.each_with_object({}) { |(name, path), result| result[Network.name(name)] = File.expand_path(path.to_s) }
        @applied = {}
        @closed = false
        session.on(:state) { |key, value, _version| apply(value) if key == "grid_cells" && !@closed }
        session.on("grid/place") do |peer, payload|
          next unless session.host? && !@closed
          begin
            validate_edit(payload)
            cells = session.state("grid_cells") || {}
            coordinate = "#{payload['col']},#{payload['row']}"
            payload["asset"] ? cells[coordinate] = payload["asset"] : cells.delete(coordinate)
            validate_grid(layout(cells))
            session.set("grid_cells", cells)
          rescue Error, ArgumentError => error
            raise if peer == session.id
            session.send("grid/rejected", { "reason" => error.message }, to: peer)
          end
        end
        existing = session.state("grid_cells")
        if existing
          apply(existing)
        elsif session.host?
          session.set("grid_cells", {})
        end
      end

      def place(col, row, asset)
        raise Error, "shared grid is closed" if @closed
        payload = { "col" => col, "row" => row, "asset" => asset&.to_s }
        validate_edit(payload)
        @session.send("grid/place", payload)
      end

      def remove(col, row) = place(col, row, nil)
      def poll = @session.poll
      def closed? = @closed || @session.closed?
      def close
        @closed = true
        @session.close
      end

      private

      def validate_edit(payload)
        unless payload.is_a?(Hash) && %w[col row].all? { |key| payload[key].is_a?(Integer) && payload[key].between?(-4096, 4096) }
          raise ArgumentError, "shared-grid coordinates must be integers within -4096..4096"
        end
        raise ArgumentError, "unknown shared asset" unless payload.key?("asset") && (payload["asset"].nil? || @assets.key?(payload["asset"]))
      end

      def layout(cells)
        raise Error, "invalid shared-grid state" unless cells.is_a?(Hash)
        cells.each_with_object({}) do |(coordinate, asset), result|
          raise Error, "invalid shared-grid coordinate" unless /\A-?\d{1,4},-?\d{1,4}\z/.match?(coordinate)
          col, row = coordinate.split(",").map { |number| Integer(number, 10) }
          raise Error, "invalid shared-grid asset or coordinate" unless asset.is_a?(String) && coordinate == "#{col},#{row}"
          validate_edit("col" => col, "row" => row, "asset" => asset)
          result[[col, row]] = asset
        end
      end

      def validate_grid(next_cells)
        (@applied.keys | next_cells.keys).each do |col, row|
          cell = @grid[col, row]
          expected = @applied[[col, row]]
          next unless cell
          unless expected && cell.kind != "grid" && cell.path == @assets[expected] &&
                 (!@grid.respond_to?(:stack_size) || @grid.stack_size(col, row) == 1)
            raise Error, "shared cell (#{col}, #{row}) was edited locally; use GridRoom.place/remove"
          end
        end
      end

      def apply(cells)
        next_cells = layout(cells)
        validate_grid(next_cells)
        (@applied.keys - next_cells.keys).each { |col, row| @grid.remove(col, row) }
        next_cells.each do |(col, row), asset|
          @grid.replace(col, row, @assets.fetch(asset)) if @applied[[col, row]] != asset
        end
        @applied = next_cells
      end
    end
  end
end

module Selenite
  # Deterministic 2D binary space partitioning, independent of rendering.
  # Bounds are [x, y, width, height], with exclusive right/bottom edges.
  class BSP
    Node = Struct.new(:id, :bounds, :children) do
      def leaf? = children.empty?
    end
    attr_reader :root

    def self.bounds(value)
      unless value.is_a?(Array) && value.size == 4 && value.all? { |n| n.is_a?(Numeric) && n.to_f.finite? }
        raise ArgumentError, "bounds must contain four finite numbers"
      end
      x, y, w, h = value.map(&:to_f)
      unless w.positive? && h.positive? && (x + w).finite? && (y + h).finite? && x + w > x && y + h > y
        raise ArgumentError, "bounds must have positive, representable dimensions"
      end
      [x, y, w, h]
    end

    def self.overlap?(a, b)
      a[0] < b[0] + b[2] && a[0] + a[2] > b[0] && a[1] < b[1] + b[3] && a[1] + a[3] > b[1]
    end

    def initialize(bounds:, depth: 4, min_size: 32)
      bounds = BSP.bounds(bounds)
      raise ArgumentError, "depth must be an integer in 0..12" unless depth.is_a?(Integer) && (0..12).cover?(depth)
      unless min_size.is_a?(Numeric) && min_size.to_f.finite? && min_size.positive?
        raise ArgumentError, "min_size must be positive and finite"
      end
      @config = { "version" => 1, "bounds" => bounds, "depth" => depth, "min_size" => min_size.to_f }
      @root = split("r", bounds, depth, min_size)
    end

    def to_h = Storage.copy(@config)

    def self.from_h(config)
      raise ArgumentError, "unsupported BSP version" unless config.is_a?(Hash) && config["version"] == 1
      new(bounds: config.fetch("bounds"), depth: config.fetch("depth"), min_size: config.fetch("min_size"))
    end

    def leaves = query(@root.bounds)

    def leaf_at(x, y)
      unless [x, y].all? { |n| n.is_a?(Numeric) && n.to_f.finite? }
        raise ArgumentError, "coordinates must be finite numbers"
      end
      node = @root
      return nil unless contains?(node.bounds, x, y)
      node = node.children.find { |child| contains?(child.bounds, x, y) } until node.leaf?
      node
    end

    def query(bounds)
      bounds = BSP.bounds(bounds)
      result = []
      visit = lambda do |node|
        next unless BSP.overlap?(node.bounds, bounds)
        node.leaf? ? result << node : node.children.each { |child| visit.call(child) }
      end
      visit.call(@root)
      result
    end

    private

    def contains?(rect, x, y)
      x >= rect[0] && x < rect[0] + rect[2] && y >= rect[1] && y < rect[1] + rect[3]
    end

    def split(id, bounds, depth, min_size)
      x, y, w, h = bounds
      axis = w >= h ? 0 : 1
      length = axis.zero? ? w : h
      children = []
      if depth.positive? && length / 2 >= min_size
        halves = axis.zero? ? [[x, y, w / 2, h], [x + w / 2, y, w / 2, h]] :
                              [[x, y, w, h / 2], [x, y + h / 2, w, h / 2]]
        children = halves.each_with_index.map { |rect, index| split("#{id}#{index}", rect, depth - 1, min_size) }
      end
      Node.new(id.freeze, bounds.freeze, children.freeze).freeze
    end
  end

  # Each BSP leaf owns a named table, which can grow into multiple physical
  # array partitions. Entities are point-indexed; handles are [leaf_id, row_id].
  class PartitionedWorld
    attr_reader :bsp

    def initialize(store, name: "world", bsp: nil)
      @store = store
      @name = Storage.name(name)
      raise ArgumentError, "world name must be at most 40 characters" if @name.size > 40
      @prefix = "#{@name}_leaf_"
      index = store.table("#{@name}_index")
      saved = index.metadata["bsp"]
      if saved
        @bsp = BSP.from_h(saved)
        raise ArgumentError, "world BSP differs from saved layout" if bsp && bsp.to_h != saved
      else
        raise ArgumentError, "new worlds need a BSP" unless bsp.is_a?(BSP)
        @bsp = bsp
        index.metadata = { "bsp" => bsp.to_h }
      end
      @leaf_ids = @bsp.leaves.map(&:id)
    end

    def table_for(leaf_id)
      raise KeyError, "unknown BSP leaf #{leaf_id}" unless @leaf_ids.include?(leaf_id)
      @store.table("#{@prefix}#{leaf_id}")
    end

    def spawn(x:, y:, **components)
      leaf = @bsp.leaf_at(x, y)
      raise RangeError, "entity is outside the world" unless leaf
      row = Storage.copy(components).merge("x" => x, "y" => y)
      [leaf.id, table_for(leaf.id).insert(row)]
    end

    def [](handle)
      leaf, id = handle
      table_for(leaf)[id]
    end

    def update(handle)
      leaf, id = handle
      table_for(leaf).update(id) do |row|
        yield row
        unless @bsp.leaf_at(row["x"], row["y"])&.id == leaf
          raise ArgumentError, "use move for entities crossing a BSP boundary"
        end
      end
    end

    def move(handle, x:, y:)
      target = @bsp.leaf_at(x, y)
      raise RangeError, "entity is outside the world" unless target
      row = self[handle]
      raise KeyError, "unknown entity" unless row
      if target.id == handle[0]
        update(handle) { |value| value.merge!("x" => x, "y" => y) }
        handle
      else
        next_id = table_for(target.id).insert(row.merge("x" => x, "y" => y))
        delete(handle)
        [target.id, next_id]
      end
    end

    def delete(handle)
      table_for(handle[0]).delete(handle[1])
    end

    # Returned leaves are useful for drawing rooms. Load nearby records and
    # save/unload the other leaves in this world, preserving all other tables.
    def stream(bounds)
      leaves = @bsp.query(bounds)
      wanted = leaves.map { |leaf| "#{@prefix}#{leaf.id}" }
      leaves.each { |leaf| table_for(leaf.id).wake! }
      @store.table_names.each do |name|
        @store.table(name).sleep! if name.start_with?(@prefix) && !wanted.include?(name)
      end
      leaves
    end

    # Broad phase: BSP; narrow phase: exact point-in-region filtering.
    # This loads matching leaves but does not evict others; stream does that.
    def query(bounds, **components)
      bounds = BSP.bounds(bounds)
      @bsp.query(bounds).flat_map do |leaf|
        table_for(leaf.id).where(loaded_only: false, **components).filter_map do |id, row|
          x, y = row.values_at("x", "y")
          if x >= bounds[0] && x < bounds[0] + bounds[2] && y >= bounds[1] && y < bounds[1] + bounds[3]
            [[leaf.id, id], row]
          end
        end
      end
    end
  end
end

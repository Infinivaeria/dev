# PartitionedArray-backed game data. No window or native calls are needed here.
# Disk format is Selenite's, separate from the legacy grid_pa_* snapshots.
require "json"
require "fileutils"
require "securerandom"

module Selenite
  module Storage
    class Error < StandardError; end
    class CorruptStore < Error; end
    class ClosedStore < Error; end

    def self.name(value)
      value = value.to_s
      raise ArgumentError, "use 1–64 letters, digits, underscores or hyphens" unless /\A[a-zA-Z0-9_-]{1,64}\z/.match?(value)
      value
    end

    # Restrict data to portable JSON values; return detached, string-keyed data.
    def self.copy(value, depth = 0)
      raise ArgumentError, "data is cyclic or nested more than 64 levels" if depth > 64
      case value
      when Hash
        value.each_with_object({}) do |(key, item), result|
          raise ArgumentError, "keys must be strings or symbols" unless key.is_a?(String) || key.is_a?(Symbol)
          raise ArgumentError, "duplicate JSON key #{key}" if result.key?(key.to_s)
          result[key.to_s] = copy(item, depth + 1)
        end
      when Array then value.map { |item| copy(item, depth + 1) }
      when String then value.dup
      when Integer, TrueClass, FalseClass, NilClass then value
      when Float
        raise ArgumentError, "numbers must be finite" unless value.finite?
        value
      else raise ArgumentError, "cannot store #{value.class}; use JSON data"
      end
    end

    # One writer per directory. Immutable chunk files + an atomic manifest
    # switch commit all tables together, without overwriting sleeping chunks.
    class Store
      attr_reader :path

      def self.open(path, **options)
        store = new(path, **options)
        return store unless block_given?
        begin
          yield store
        ensure
          store.close
        end
      end

      def initialize(path, partition_size: 64)
        raise ArgumentError, "partition_size must be a positive integer" unless partition_size.is_a?(Integer) && partition_size.positive?
        require "partitioned_array"
        @path = File.expand_path(path.to_s)
        @partition_size = partition_size
        @tables = {}
        @closed = false
        FileUtils.mkdir_p(@path)
        @lock = File.open(File.join(@path, ".lock"), "a+")
        raise Error, "storage already open: #{@path}" unless @lock.flock(File::LOCK_EX | File::LOCK_NB)
        manifest = File.join(@path, "manifest.json")
        @manifest = File.exist?(manifest) ? JSON.parse(File.read(manifest)) : { "version" => 1, "tables" => {} }
        validate_manifest!
      rescue Exception
        @lock&.close
        raise
      end

      def check_open!
        raise ClosedStore, "storage is closed" if @closed
      end

      def table(name, partition_size: @partition_size)
        check_open!
        name = Storage.name(name)
        @tables[name] ||= Table.new(self, name, @manifest["tables"][name], partition_size)
      end

      def table_names
        check_open!
        (@manifest["tables"].keys | @tables.keys).sort
      end

      def flush
        check_open!
        next_manifest = JSON.parse(JSON.generate(@manifest))
        @tables.each do |name, table|
          next_manifest["tables"][name] = table.prepare_commit
        end
        return self if next_manifest == @manifest
        atomic_json("manifest.json", next_manifest)
        @manifest = next_manifest
        @tables.each { |name, table| table.committed!(@manifest["tables"][name]) }
        self
      end
      alias save flush

      # Complete standalone checkpoint, including dormant partitions.
      # Destination must not exist; the finished directory is published last.
      def snapshot(destination)
        check_open!
        destination = File.expand_path(destination.to_s)
        raise ArgumentError, "snapshot destination already exists" if File.exist?(destination)
        flush
        staging = "#{destination}.tmp-#{SecureRandom.hex(8)}"
        FileUtils.mkdir_p(staging)
        begin
          @manifest["tables"].each_value do |meta|
            meta["files"].each { |file| FileUtils.cp(File.join(@path, file), File.join(staging, file)) }
          end
          File.write(File.join(staging, "manifest.json"), JSON.generate(@manifest))
          File.rename(staging, destination)
        ensure
          FileUtils.rm_rf(staging) if File.directory?(staging)
        end
        destination
      end

      # Remove chunks left by older commits or interrupted writes. Snapshots
      # own copies, so pruning this store cannot damage them.
      def vacuum!
        flush
        live = @manifest["tables"].values.flat_map { |meta| meta["files"] }
        stale = Dir.children(@path).grep(/\Achunk-[0-9a-f]{32}\.json\z/) - live
        stale.each { |file| File.delete(File.join(@path, file)) }
        stale.size
      end

      def close(save: true)
        return if @closed
        begin
          flush if save
        ensure
          @closed = true
          @lock.close
        end
      end

      def closed? = @closed

      # Internal persistence boundary. Reads are validated before activation.
      def read_chunk(file, size)
        check_open!
        rows = JSON.parse(File.read(File.join(@path, file)))
        unless rows.is_a?(Array) && rows.size == size && rows.all? { |row| row.nil? || row.is_a?(Hash) }
          raise CorruptStore, "invalid chunk #{file}"
        end
        rows
      rescue JSON::ParserError, Errno::ENOENT => e
        raise CorruptStore, e.message
      end

      def write_chunk(rows)
        file = "chunk-#{SecureRandom.hex(16)}.json"
        atomic_json(file, rows)
        file
      end

      private

      def atomic_json(file, value)
        temp = File.join(@path, ".write-#{SecureRandom.hex(16)}")
        begin
          File.open(temp, "wx") do |out|
            out.write(JSON.generate(value))
            out.flush
            out.fsync
          end
          File.rename(temp, File.join(@path, file))
        ensure
          File.delete(temp) if File.exist?(temp)
        end
      end

      def validate_manifest!
        unless @manifest.is_a?(Hash) && @manifest["version"] == 1 && @manifest["tables"].is_a?(Hash)
          raise CorruptStore, "unsupported or invalid storage manifest"
        end
        @manifest["tables"].each do |name, meta|
          Storage.name(name)
          unless meta.is_a?(Hash) && meta["partition_size"].is_a?(Integer) && meta["partition_size"].positive? &&
                 meta["next_id"].is_a?(Integer) && meta["next_id"] >= 0 &&
                 meta["files"].is_a?(Array) && !meta["files"].empty? &&
                 meta["files"].all? { |file| file.is_a?(String) && /\Achunk-[0-9a-f]{32}\.json\z/.match?(file) } &&
                 meta["next_id"] <= meta["files"].size * meta["partition_size"] + 1 &&
                 meta["metadata"].is_a?(Hash)
            raise CorruptStore, "invalid table #{name}"
          end
        end
      rescue ArgumentError => e
        raise CorruptStore, e.message
      end
    end

    class Table
      include Enumerable
      attr_reader :name

      def initialize(store, name, saved, partition_size)
        @store, @name = store, name
        partition_size = saved["partition_size"] if saved
        raise ArgumentError, "partition_size must be a positive integer" unless partition_size.is_a?(Integer) && partition_size.positive?
        @array = PartitionedArray.new(db_size: 1, partition_amount_and_offset: partition_size,
                                      dynamically_allocates: false)
        @array.allocate
        @files = saved ? saved["files"].dup : [nil]
        (@files.size - 1).times { @array.add_partition }
        @next_id = saved ? saved["next_id"] : 0
        @metadata = saved ? Storage.copy(saved["metadata"]) : {}
        @states = Array.new(@files.size, saved ? :dormant : :active)
        @dirty = saved ? {} : { 0 => true }
        # Keep addresses/ranges, but release all unloaded record objects.
        @array.data_arr.map! { nil }
      end

      def partition_count
        @store.check_open!
        @states.size
      end

      def partition_range(partition)
        validate_partition!(partition)
        @array.range_arr[partition]
      end

      def partition_for(id)
        @store.check_open!
        raise ArgumentError, "record ID must be a nonnegative integer" unless id.is_a?(Integer) && id >= 0
        @array.range_arr.bsearch_index { |range| range.end >= id }
      end

      def status(partition)
        validate_partition!(partition)
        @states[partition]
      end

      def partitions
        (0...partition_count).map do |id|
          { id: id, range: partition_range(id), label: status(id), dirty: !!@dirty[id] }
        end
      end

      def revenants = partitions.select { |part| part[:label] == :revenant }.map { |part| part[:id] }

      # Explicit capacity growth does not consume or change record IDs.
      def add_partition
        @store.check_open!
        @array.add_partition
        @array.range_arr.last.each { |slot| @array.data_arr[slot] = nil }
        @states << :active
        @files << nil
        @dirty[@states.size - 1] = true
        @states.size - 1
      end

      def stats
        @store.check_open!
        { capacity: @array.data_arr.size, next_id: @next_id,
          partitions: partition_count, resident: @states.count { |state| state != :dormant },
          revenants: revenants.size, dirty: @dirty.size,
          loaded_records: @array.data_arr.count { |row| !row.nil? } }
      end

      def load_partition(partition)
        validate_partition!(partition)
        return self unless @states[partition] == :dormant
        range = @array.range_arr[partition]
        rows = @store.read_chunk(@files.fetch(partition), range.size)
        @array.data_arr[range] = rows
        @states[partition] = :revenant
        self
      end
      alias revive! load_partition

      def unload_partition(partition)
        validate_partition!(partition)
        return self if @states[partition] == :dormant
        @store.flush if @dirty[partition] || @files[partition].nil?
        @array.range_arr[partition].each { |id| @array.data_arr[id] = nil }
        @states[partition] = :dormant
        self
      end
      alias kill_partition! unload_partition

      def sleep!
        (0...partition_count).each { |part| unload_partition(part) }
        self
      end

      def wake!
        (0...partition_count).each { |part| load_partition(part) }
        self
      end

      # A deleted address is never reused. Empty hashes are valid records.
      def insert(record)
        @store.check_open!
        record = checked_record(record)
        id = @next_id
        add_partition unless partition_for(id)
        part = partition_for(id)
        load_partition(part)
        @array.set(id) { |row| row.replace(record) }
        @next_id += 1
        @dirty[part] = true
        id
      end

      def [](id)
        part = partition_for(id)
        return nil unless part && id < @next_id
        load_partition(part)
        Storage.copy(@array.data_arr[id])
      end

      def update(id)
        raise ArgumentError, "update requires a block" unless block_given?
        row = self[id]
        raise KeyError, "no record #{id}" unless row
        yield row
        replacement = checked_record(row)
        @array.set(id) { |entry| entry.replace(replacement) }
        @dirty[partition_for(id)] = true
        Storage.copy(replacement)
      end

      def delete(id)
        row = self[id]
        return nil unless row
        @array.data_arr[id] = nil
        @dirty[partition_for(id)] = true
        row
      end

      # Yields [stable ID, detached record]. Default traverses resident rows
      # only; loaded_only: false intentionally revives every partition.
      def each(loaded_only: true)
        return enum_for(:each, loaded_only: loaded_only) unless block_given?
        (0...partition_count).each do |part|
          next if loaded_only && status(part) == :dormant
          load_partition(part)
          partition_range(part).each do |id|
            row = @array.data_arr[id]
            yield id, Storage.copy(row) if row
          end
        end
        self
      end

      def where(loaded_only: true, **fields)
        fields = Storage.copy(fields)
        each(loaded_only: loaded_only).select { |_id, row| fields.all? { |key, value| row.key?(key) && row[key] == value } }
      end

      def metadata
        @store.check_open!
        Storage.copy(@metadata)
      end

      def metadata=(value)
        @store.check_open!
        @metadata = checked_record(value)
      end

      def prepare_commit
        files = @files.dup
        @dirty.each_key { |part| files[part] = @store.write_chunk(@array.get_partition(part)) }
        { "partition_size" => @array.partition_amount_and_offset, "next_id" => @next_id,
          "metadata" => Storage.copy(@metadata), "files" => files }
      end

      def committed!(meta)
        @files = meta["files"].dup
        @dirty.clear
      end

      private

      def validate_partition!(partition)
        @store.check_open!
        unless partition.is_a?(Integer) && partition >= 0 && partition < @states.size
          raise IndexError, "unknown partition #{partition.inspect}"
        end
      end

      def checked_record(record)
        raise ArgumentError, "record must be a hash" unless record.is_a?(Hash)
        copied = Storage.copy(record)
        JSON.generate(copied) # Reject invalid string encodings before mutation.
        copied
      end
    end
  end

  class Grid
    # Caller owns this handle. Game#storage owns and closes its handles.
    def storage(name = "game", **options)
      Storage::Store.new(File.join(folder, "game-storage", Storage.name(name)), **options)
    end
  end
end

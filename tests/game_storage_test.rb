require "minitest/autorun"
require "tmpdir"
$LOAD_PATH.unshift File.expand_path("../partitioned_array/lib", __dir__)
require_relative "../src/ruby/game_storage"
require_relative "../src/ruby/game_world"
require_relative "../src/ruby/raylib_game"

class GameStorageTest < Minitest::Test
  def setup
    @dir = Dir.mktmpdir("selenite-storage-")
    @path = File.join(@dir, "store")
    @store = Selenite::Storage::Store.new(@path, partition_size: 2)
  end

  def teardown
    @store.close(save: false) unless @store.closed?
    FileUtils.remove_entry(@dir)
  end

  def reopen
    @store.close
    @store = Selenite::Storage::Store.new(@path)
  end

  def test_growth_stable_ids_empty_records_and_nonzero_partition_revival
    table = @store.table("entities")
    ids = 10.times.map { |i| table.insert("value" => i, "nested" => { "hp" => 10 }) }
    assert_equal (0...10).to_a, ids
    assert_equal 0..2, table.partition_range(0)
    assert_equal 3..4, table.partition_range(1)
    table.delete(3)
    assert_equal 10, table.insert({})
    reopen
    table = @store.table("entities")
    assert table.partitions.all? { |part| part[:label] == :dormant }
    assert_empty table.to_a
    assert_equal({ "value" => 8, "nested" => { "hp" => 10 } }, table[8])
    assert_equal [3], table.revenants
    assert_nil table[3]
    assert_equal({}, table[10])
    assert_equal 10, table.each(loaded_only: false).count
  end

  def test_unload_commits_changes_and_revives_whole_partition_without_record_ids
    table = @store.table("quests")
    6.times { |i| table.insert("done" => false, "quest" => i) }
    table.update(4) { |row| row["done"] = true }
    table.unload_partition(1)
    assert_equal :dormant, table.status(1)
    assert_equal true, table[4]["done"]
    assert_equal :revenant, table.status(1)
    assert_equal 3, table[3]["quest"]
  end

  def test_flush_never_overwrites_dormant_partitions
    table = @store.table("items")
    9.times { |i| table.insert("n" => i) }
    reopen
    table = @store.table("items")
    table.update(0) { |row| row["n"] = 20 }
    @store.flush
    assert_equal :dormant, table.status(3)
    reopen
    assert_equal [20, 1, 2, 3, 4, 5, 6, 7, 8], @store.table("items").each(loaded_only: false).map { |_id, row| row["n"] }
  end

  def test_detached_reads_and_updates_rollback_on_error
    table = @store.table("items")
    source = { nested: [1] }
    id = table.insert(source)
    source[:nested] << 2
    table[id]["nested"] << 3
    assert_raises(RuntimeError) { table.update(id) { |row| row["nested"] << 4; raise "cancel" } }
    assert_equal({ "nested" => [1] }, table[id])
    assert_raises(ArgumentError) { table.update(id) { |row| row["oops"] = Object.new } }
    assert_equal({ "nested" => [1] }, table[id])
  end

  def test_metadata_and_named_tables
    table = @store.table("quests")
    table.metadata = { schema: 2, labels: ["quest-log"] }
    table.insert(done: true)
    table.insert(done: false)
    @store.table("players").insert(hp: 100)
    reopen
    assert_equal %w[players quests], @store.table_names
    assert_equal 2, @store.table("quests").metadata["schema"]
    assert_empty @store.table("quests").where(done: true)
    assert_equal [[0, { "done" => true }]], @store.table("quests").where(loaded_only: false, done: true)
  end

  def test_invalid_ids_types_and_paths
    table = @store.table("data")
    [-1, "0", 1.5, nil].each { |id| assert_raises(ArgumentError) { table[id] } }
    assert_nil table[10_000]
    [-1, 1, "0"].each { |part| assert_raises(IndexError) { table.revive!(part) } }
    assert_raises(ArgumentError) { @store.table("../outside") }
    assert_raises(ArgumentError) { table.insert("not a hash") }
    assert_raises(ArgumentError) { table.insert(n: Float::NAN) }
    assert_raises(ArgumentError) { table.insert({ n: 1, "n" => 2 }) }
    assert_raises(KeyError) { table.update(100) {} }
  end

  def test_one_writer_lock_and_closed_handles
    assert_raises(Selenite::Storage::Error) { Selenite::Storage::Store.new(@path) }
    table = @store.table("items")
    @store.close
    assert_raises(Selenite::Storage::ClosedStore) { table.insert(x: 1) }
    assert_raises(Selenite::Storage::ClosedStore) { @store.flush }
    @store = Selenite::Storage::Store.new(@path)
  end

  def test_snapshot_keeps_dormant_data_and_is_independent_of_vacuum
    table = @store.table("items")
    8.times { |i| table.insert(x: i) }
    table.sleep!
    destination = File.join(@dir, "checkpoint")
    @store.snapshot(destination)
    assert_raises(ArgumentError) { @store.snapshot(destination) }
    table.update(7) { |row| row["x"] = 999 }
    @store.flush
    assert_operator @store.vacuum!, :>, 0
    Selenite::Storage::Store.open(destination) do |snapshot|
      assert_equal 7, snapshot.table("items")[7]["x"]
    end
    assert_equal 999, table[7]["x"]
  end

  def test_failed_commit_preserves_previous_disk_generation_and_can_retry
    a = @store.table("a")
    b = @store.table("b")
    a.insert(n: 1)
    b.insert(n: 2)
    @store.flush
    old_manifest = File.read(File.join(@path, "manifest.json"))
    a.update(0) { |row| row["n"] = 10 }
    b.update(0) { |row| row["n"] = 20 }
    @store.singleton_class.class_eval do
      alias_method :working_atomic_json, :atomic_json
      define_method(:atomic_json) do |file, value|
        raise IOError, "simulated disk full" if file == "manifest.json"
        working_atomic_json(file, value)
      end
    end
    assert_raises(IOError) { @store.flush }
    assert_equal old_manifest, File.read(File.join(@path, "manifest.json"))
    assert a.partitions.any? { |part| part[:dirty] }
    @store.singleton_class.class_eval { alias_method :atomic_json, :working_atomic_json }
    reopen
    assert_equal 10, @store.table("a")[0]["n"]
    assert_equal 20, @store.table("b")[0]["n"]
  end

  def test_missing_chunk_is_an_error_and_does_not_activate_empty_data
    @store.table("items").insert(n: 3)
    reopen
    file = JSON.parse(File.read(File.join(@path, "manifest.json")))["tables"]["items"]["files"][0]
    File.delete(File.join(@path, file))
    table = @store.table("items")
    assert_raises(Selenite::Storage::CorruptStore) { table[0] }
    assert_equal :dormant, table.status(0)
  end

  def test_discard_unflushed_changes
    @store.table("items").insert(n: 1)
    @store.flush
    @store.table("items").update(0) { |row| row["n"] = 2 }
    @store.close(save: false)
    @store = Selenite::Storage::Store.new(@path)
    assert_equal 1, @store.table("items")[0]["n"]
  end

  def test_explicit_capacity_and_cyclic_record_validation
    table = @store.table("data")
    assert_equal 1, table.add_partition
    assert_equal 5, table.stats[:capacity]
    assert_equal 0, table.insert({})
    cycle = {}; cycle["loop"] = cycle
    assert_raises(ArgumentError) { table.insert(cycle) }
    assert_equal 1, table.stats[:next_id]
  end

  def test_invalid_chunk_and_manifest_are_not_silently_replaced
    @store.table("data").insert(n: 1)
    reopen
    manifest_path = File.join(@path, "manifest.json")
    manifest = JSON.parse(File.read(manifest_path))
    file = manifest["tables"]["data"]["files"][0]
    File.write(File.join(@path, file), '[{"n":2}]')
    assert_raises(Selenite::Storage::CorruptStore) { @store.table("data")[0] }
    @store.close(save: false)
    manifest["tables"]["data"]["files"] = ["../../outside.json"]
    File.write(manifest_path, JSON.generate(manifest))
    assert_raises(Selenite::Storage::CorruptStore) { Selenite::Storage::Store.new(@path) }
    assert_equal manifest, JSON.parse(File.read(manifest_path))
  end

  def test_bsp_boundaries_queries_serialization_and_invalid_inputs
    tree = Selenite::BSP.new(bounds: [-100, -100, 200, 200], depth: 2, min_size: 10)
    assert_equal 4, tree.leaves.size
    assert_equal "r00", tree.leaf_at(-100, -100).id
    assert_equal "r11", tree.leaf_at(0, 0).id
    assert_nil tree.leaf_at(100, 0)
    assert_nil tree.leaf_at(0, 100)
    assert_equal ["r00"], tree.query([-100, -100, 100, 100]).map(&:id)
    assert_equal 4, tree.query([-1, -1, 2, 2]).size
    assert_equal tree.leaves, Selenite::BSP.from_h(tree.to_h).leaves
    assert_raises(ArgumentError) { tree.query([0, 0, 0, 1]) }
    assert_raises(ArgumentError) { tree.leaf_at(Float::INFINITY, 0) }
    assert_raises(ArgumentError) { Selenite::BSP.new(bounds: [0, 0, 100, 100], depth: 13) }
    assert_equal 1, Selenite::BSP.new(bounds: [0, 0, 10, 10], min_size: 10).leaves.size
  end

  def test_world_streaming_queries_movement_and_persistence
    tree = Selenite::BSP.new(bounds: [-100, -100, 200, 200], depth: 2, min_size: 10)
    world = Selenite::PartitionedWorld.new(@store, bsp: tree)
    first = world.spawn(x: -50, y: -50, kind: "enemy", hp: 5)
    second = world.spawn(x: 50, y: 50, kind: "enemy", hp: 7)
    world.stream([-100, -100, 100, 100])
    assert_equal :dormant, world.table_for(second[0]).status(0)
    assert_equal [first], world.query([-60, -60, 20, 20], kind: "enemy").map(&:first)
    assert_empty world.query([-40, -40, 20, 20])
    assert_raises(ArgumentError) { world.update(first) { |row| row["x"] = 50 } }
    assert_equal(-50, world[first]["x"])
    moved = world.move(first, x: 30, y: 40)
    assert_nil world[first]
    assert_equal 5, world[moved]["hp"]
    reopen
    world = Selenite::PartitionedWorld.new(@store)
    world.stream([0, 0, 100, 100])
    assert_equal :revenant, world.table_for(second[0]).status(0)
    assert_equal [second, moved], world.query([0, 0, 100, 100]).map(&:first)
    assert_raises(RangeError) { world.spawn(x: 100, y: 0) }
    assert_raises(KeyError) { world.table_for("nope") }
    other = Selenite::BSP.new(bounds: [0, 0, 1, 1])
    assert_raises(ArgumentError) { Selenite::PartitionedWorld.new(@store, bsp: other) }
  end

  def test_game_storage_caches_by_path_and_save_flushes
    game = Selenite::Game.new(audio: false)
    path = File.join(@dir, "game")
    store = game.storage(path: path)
    assert_same store, game.storage(path: path)
    store.table("player").insert(hp: 50)
    Selenite.stub(:save, true) { assert_equal true, game.save }
    assert File.file?(File.join(path, "manifest.json"))
  ensure
    store&.close
  end

  def headless_game(**options)
    game = Selenite::Game.new(audio: false, **options)
    %i[SetConfigFlags SetTraceLogLevel InitWindow SetExitKey SetTargetFPS].each do |method|
      game.define_singleton_method(method) { |*| nil }
    end
    game.define_singleton_method(:WindowShouldClose) { true }
    game
  end

  def test_game_exit_saves_and_releases_storage
    game = headless_game
    path = File.join(@dir, "game")
    game.run { |g| g.storage(path: path).table("data").insert(n: 42) }
    Selenite::Storage::Store.open(path) { |store| assert_equal 42, store.table("data")[0]["n"] }
  end

  def test_game_exception_and_autosave_false_release_locks_without_committing
    [true, false].each do |raise_on_close|
      game = headless_game(autosave: raise_on_close)
      path = File.join(@dir, "game-#{raise_on_close}")
      run = lambda do
        game.run do |g|
          g.storage(path: path).table("data").insert(n: 42)
          g.on_close { raise "user callback failed" } if raise_on_close
        end
      end
      raise_on_close ? assert_raises(RuntimeError, &run) : run.call
      Selenite::Storage::Store.open(path) { |store| assert_empty store.table_names }
    end
  end
end

# Native save is stubbed only in the game helper integration test.
def Selenite.save = true
def Raylib.IsWindowReady = false
def Raylib.IsAudioDeviceReady = false

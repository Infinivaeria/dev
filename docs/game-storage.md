# Partitioned game storage, BSP, and revenants

Selenite exposes the bundled [PartitionedArray](../partitioned_array/lib/partitioned_array.rb) through a game-oriented storage layer. Use it for entities, components, quests, dialogue state, streamed levels, and checkpoints. The API works in games, organizer scripts, and plugins; rendering is optional.

This guide covers:

- [Setup and a runnable demo](#setup-and-a-runnable-demo)
- [Choose a storage location](#choose-a-storage-location)
- [Create, read, update, and delete records](#create-read-update-and-delete-records)
- [Partitions and revenants](#partitions-and-revenants)
- [Saving and checkpoints](#saving-and-checkpoints)
- [Binary space partitioning](#binary-space-partitioning)
- [Stream a persistent world](#stream-a-persistent-world)
- [Use grids, inventories, and game logic](#use-grids-inventories-and-game-logic)
- [API reference](#api-reference)
- [Exporting games](#exporting-games)
- [Limits and troubleshooting](#limits-and-troubleshooting)
- [Tests and implementation](#tests-and-implementation)

## Setup and a runnable demo

Build the updated application with Ruby scripting enabled, from the repository root:

```sh
cargo build --features scripting
./target/debug/selenite --game examples/games/revenant_world.rb --grid /tmp/revenant-grid.json
```

For a dedicated Ruby-only runtime with bundled dependencies, run
`./bootstrap-game.sh`, then
`./dist-game/run-game.sh examples/games/revenant_world.rb --grid /tmp/revenant-grid.json`.
See the [packaging guide](packaging.md). These storage APIs execute in Ruby;
JSON is their data format, not another game language.

The existing portable binary does not acquire new embedded APIs just because these Ruby source files change. Rebuild it with [package.sh](../package.sh) before using the portable runtime for these features. The normal [installation prerequisites](manual.md#2-install-and-start) apply. The array library is found through `SELENITE_PARTITIONED_ARRAY_LIB`, beside the executable in `partitioned_array/lib`, or in the source checkout. That environment variable must name the **lib directory**.

[Revenant World](../examples/games/revenant_world.rb) is a complete, asset-free example:

| Control | Action |
| --- | --- |
| WASD / arrows | Move through the world |
| P | Pause movement and collection |
| C | Save a standalone checkpoint |
| Escape | Save and exit |

There are 16 rooms and 112 gems. Room boundaries come from a BSP tree. Each room owns a table, and seven gems per room exceed the first physical partition's capacity. Offscreen rooms sleep; revisiting them reloads their records and labels their partitions `:revenant`. The HUD displays resident and revenant partition counts. Reopening the game restores the player position, score, and collected gems.

The demo saves beside its script in `game-data/revenant-world`. Its checkpoints go in `game-data/checkpoints`. The `--grid` argument selects the separate Selenite grid save; it does not relocate the demo's explicitly selected storage path.

## Choose a storage location

Inside a game setup block:

```ruby
store = g.storage("campaign")
```

This creates or opens `<current grid folder>/game-storage/campaign`. Each grid can have its own campaign. The game owns the handle, caches it by absolute path, flushes it on `g.save`, and releases its lock when the game exits. Options on a repeated call for the same path do not reconfigure the already-open store.

For a game you intend to export, use a path relative to the game script:

```ruby
store = g.storage(path: File.join(File.dirname(__FILE__), "game-data", "campaign"),
                  partition_size: 64)
```

The directory must be writable. Use a per-user save directory for read-only installations; pass its absolute path explicitly. An explicit path is shared by every grid launching that script. Default grid-relative storage instead isolates saves by grid.

Outside a game, use a block to close the store automatically:

```ruby
Selenite::Storage::Store.open("./my-campaign", partition_size: 64) do |store|
  players = store.table("players")
  players.insert(name: "Aster", hp: 100) unless players[0]
end
```

Or use `store = Selenite.grid.storage("campaign")` and always call `store.close` in an `ensure` block. `Selenite.save` alone saves the grid; it does not know about independently opened storage handles. The block form closes **with saving even if its block raises**. Use explicit `close(save: false)` when you need to discard unflushed changes after an error.

Only one handle can own a store directory at a time. Share that handle between your game systems. Separate store directories may be open concurrently. Table names and default store names accept 1–64 ASCII letters, digits, underscores, and hyphens.

## Create, read, update, and delete records

The following snippets assume `store` is an open storage handle. A table is a partitioned array of JSON hashes:

```ruby
actors = store.table("actors", partition_size: 32)
actor_id = actors.insert(kind: "enemy", position: { x: 120, y: 80 }, hp: 30)

p actors[actor_id]  # String-keyed, detached hash
actors.update(actor_id) { |actor| actor["hp"] -= 5 }
store.flush

actors.where(loaded_only: false, kind: "enemy").each do |id, actor|
  puts "#{id}: #{actor['hp']} HP"
end
actors.delete(actor_id)
```

IDs are nonnegative integers, local to a table. They remain stable across saving and reloading; deleting an entry never renumbers other entries or reuses its ID. Store the table name with an ID when referring across tables. Missing or deleted IDs return `nil`; updating one raises `KeyError`. Empty hashes are valid records.

Records accept strings, integers, finite floats, booleans, nil, arrays, and nested hashes. Symbol keys become strings. Cycles, more than 64 levels of nesting, duplicate normalized keys, and Ruby objects such as textures, lambdas, or IO handles are rejected. Store asset paths and logical state; recreate runtime resources after loading.

Reads and query results are copies. Changing `actors[id]["hp"]` will **not** update storage. Use `update` to mark a partition dirty. If the update block raises or returns invalid data in its hash, that record remains unchanged. The block's return value is ignored; mutate the supplied hash.

Keep schema and domain labels in table metadata:

```ruby
actors.metadata = { schema: 1, labels: ["combat", "overworld"] }
p actors.metadata
```

Metadata is saved with the manifest. Reading it also returns a copy; assign the edited hash back. The application owns schema migrations. For example, check a schema number on startup, iterate with `loaded_only: false`, update records, then assign the new schema and flush. Make a checkpoint before a migration.

## Partitions and revenants

The underlying library uses contiguous ranges in one flat array. `partition_size: N` preserves its established layout: partition 0 has **N + 1** addresses; later partitions have **N**. With N = 2:

| Partition | Record addresses |
| --- | --- |
| 0 | 0, 1, 2 |
| 1 | 3, 4 |
| 2 | 5, 6 |

Insertion grows capacity automatically. `table.add_partition` preallocates another partition and returns its partition ID without consuming any record IDs. The size supplied when creating a table is stored on disk; reopening uses the saved size.

Every physical partition has an observable lifecycle label:

| Label | Meaning |
| --- | --- |
| `:active` | Created and resident in this process; not yet reloaded from disk |
| `:dormant` | Records are on disk and absent from the in-memory array |
| `:revenant` | Loaded from a saved chunk into memory, including first access after reopening |

```ruby
table = store.table("actors")
p table.partitions  # [{ id: 0, range: 0..32, label: :active, dirty: true }, ...]
part = table.partition_for(actor_id)

table.unload_partition(part) # Save pending changes, release this partition's records
p table.status(part)        # :dormant

actor = table[actor_id]     # Automatically reloads the WHOLE containing partition
p table.status(part)        # :revenant
p table.revenants           # IDs of currently resident revenant partitions
```

`revive!(part)` explicitly loads a dormant partition; `kill_partition!(part)` is an alias for safe unloading, not permanent deletion. `wake!` and `sleep!` operate on the whole table. A missing/deleted row can still cause its containing partition to revive when queried.

A loaded partition remains labeled `:revenant` when edited or saved. Unloading changes its label to `:dormant`. These labels describe the current process's residency, not a persisted lifetime counter: all saved partitions start dormant when reopened. Domain labels belong in `metadata`.

`each` and `where` inspect **resident data only** by default, which is useful for updating active game objects. Use `each(loaded_only: false)` or `where(loaded_only: false, kind: "enemy")` to deliberately load everything. A lookup `table[id]` loads only the containing partition. `stats` reports capacity, next ID, partition count, resident count, revenant count, dirty count, and loaded record count without reviving anything.

## Saving and checkpoints

`store.flush` commits all pending tables in that store. It writes new immutable chunk files for dirty partitions, then replaces `manifest.json` with a rename. Dormant chunks are referenced unchanged, so saving active data never overwrites sleeping records with nils. A failure before the manifest switch leaves the previously committed state available on disk; pending changes remain available for retry in memory.

```ruby
g.every(5.0) { g.save }  # Flush game-owned stores and save the Selenite grid
```

Normal game exit runs close callbacks, saves if `autosave: true`, then releases all storage locks. `autosave: false` disables that final save; call `g.save` at chosen checkpoints. Unloading a dirty partition still flushes its store, regardless of the game autosave setting. A close-callback exception prevents the automatic save, but locks and graphics resources are still released.

One store's manifest switch commits its tables together. It is not a transaction across several stores or between a store and the grid inventory JSON. Keep state that must agree—such as score and collected entities—in one store, as the demo does. Write failures propagate as exceptions; handle them in your UI if you need an in-game retry prompt.

Create a complete archive/checkpoint:

```ruby
store.snapshot("./checkpoints/before-boss")  # Destination must not already exist
```

Snapshots flush first and include dormant data. They are independent writable stores. Resume a checkpoint by opening its directory, or copy that directory to a new save slot while all involved handles are closed. There is no in-place rollback of an open handle; this avoids mixing old chunks with cached records.

Old chunk generations are retained until `store.vacuum!` removes unreferenced chunk files and returns their count. Run it at a menu or checkpoint, not every frame. It does not renumber records, compact addresses, or affect snapshots. The commit scheme protects against interrupted writes before manifest publication; it is not a guarantee against every filesystem or power-loss failure. Keep external backups for important saves.

## Binary space partitioning

Array partitioning splits **record addresses**. BSP splits **world space**. Selenite provides both and connects them with `PartitionedWorld`.

```ruby
tree = Selenite::BSP.new(bounds: [-1024, -1024, 2048, 2048],
                         depth: 4, min_size: 128)
room = tree.leaf_at(100, -50)
p [room.id, room.bounds] if room
visible_rooms = tree.query([0, 0, 960, 540])
```

Bounds are `[x, y, width, height]` with finite, positive dimensions. At each level the longer axis is split in half (X wins ties). Splitting stops at `depth` or when a child would be smaller than `min_size` on the split axis. Depth must be 0–12. Leaves have deterministic string IDs such as `r0011`; tree nodes and bounds are immutable.

Right and bottom edges are exclusive. A point exactly on an internal split belongs to one leaf; a point on the world's outer right/bottom edge is outside. `leaf_at` returns nil outside the world. `query` returns intersecting leaves, with edge-only contact excluded. `leaves` returns all leaves. `to_h` and `BSP.from_h` serialize and reconstruct the versioned layout.

Use leaves as rooms, streaming zones, or collision broad-phase regions. The tree does not itself carve walkable rooms, add corridors, draw walls, perform physics, or keep an index of moving objects. Those behaviors belong in your game systems.

## Stream a persistent world

```ruby
world = Selenite::PartitionedWorld.new(store, name: "overworld", bsp: tree)
handle = world.spawn(x: 100, y: -50, kind: "enemy", hp: 30)
world.update(handle) { |enemy| enemy["hp"] -= 10 }
handle = world.move(handle, x: 300, y: 200)
store.flush
```

A new world requires a BSP. It stores the layout in `<name>_index` metadata and puts entities in tables named `<name>_leaf_<leaf ID>`. World names are limited to 40 characters; reserve these table names for the world. Each leaf table can grow into many physical partitions. Reopen with `PartitionedWorld.new(store, name: "overworld")`; the saved layout is restored. Passing a different layout for an existing world raises an error.

Handles are `[leaf_id, record_id]` arrays. Save the **returned** handle after `move`: crossing a BSP boundary moves the row to a different table and changes its handle. The old handle then returns nil. If your game needs a permanent identity independent of position, add a UUID component and maintain an application-level lookup. Regular `update` rejects moving outside the current leaf; use `move` for that.

Integrate streaming with a camera in your existing update callback:

```ruby
view = [player.x - 480, player.y - 270, 960, 540]
rooms = world.stream(view)
enemies = world.query(view, kind: "enemy")
enemies.each do |entity_handle, enemy|
  # Update behavior and draw using enemy["x"] and enemy["y"].
end
```

`stream` wakes intersecting leaves and safely unloads other existing leaf tables in **this world**. It leaves unrelated tables alone and returns the intersecting BSP nodes. `query` loads intersecting leaves and filters entities by their exact point position plus component equality. It returns `[handle, record]` pairs and does not evict other leaves.

Entities are point-indexed, not bounding-box-indexed. For sprites with radius R, expand the query rectangle by R and then do your usual exact collision checks. Query the old and new area, or requery after movement, when a system moves entities between leaves. Sleeping entities do not simulate themselves; choose whether to freeze them, catch them up by timestamps, or keep their systems active separately.

Calling `table_for(leaf_id)` exposes a leaf table for diagnostics and partition controls. Use world methods to change positions so entities remain in the correct leaf. Avoid inserting malformed rows directly into leaf tables.

## Use grids, inventories, and game logic

You can turn grid-authored cells into world entities once when initializing a level. This recipe uses top-level grid cells and records each stack item separately:

```ruby
catalog = store.table("grid_catalog")
unless catalog.metadata["imported"]
  g.grid.cells.each do |cell|
    g.grid.items(cell.col, cell.row).each_with_index do |item, stack_index|
      catalog.insert(col: cell.col, row: cell.row, stack_index: stack_index,
                     kind: item.kind, path: item.path)
    end
  end
  catalog.metadata = { imported: true, schema: 1 }
  store.flush
end
```

This stores references; it does not copy assets or recursively serialize subgrids. Traverse `subgrid` explicitly for nested levels. File paths still need to be packaged for export. The older `grid_pa_export`, `grid_pa_import`, and `grid_pa_records` functions remain available for the legacy grid-stack snapshot format. They do not read the new game-store format. To migrate a legacy snapshot, pass hashes returned by `grid_pa_records` into a new table's `insert` method once, with an import marker as above.

An inventory count can be reflected in the organizer with `g.inventory["gold"] = count`, followed by `g.save`. Keep the authoritative count with related world data in the same store if they must commit together. Treat the grid inventory as a projection you can reconstruct after loading.

For entity/component-style systems, store components such as `kind`, `velocity`, `hp`, or `quest_state` in rows. Process `table.each` to update resident objects, use tables for separate domains, and persist arbitrary state-machine states as strings. JSON metadata can describe dialogue graphs, crafting rules, map seeds, or achievements; Ruby methods execute the rules.

**Turing completeness:** Selenite already embeds general-purpose Ruby, which supplies branching, loops, recursion, functions, and dynamically allocated data structures. No storage feature or extra language is needed to make game logic computationally general. Real machines still have finite resources. Serialization stores data, not live lambdas, threads, or execution stacks.

Keep long algorithms cooperative so the game loop continues drawing. For example, inside your game setup block, divide procedural generation into bounded batches with a Fiber:

```ruby
jobs = store.table("generation")
job = Fiber.new do
  1_000.times do |i|
    jobs.insert(step: i, completed: true)
    Fiber.yield if (i + 1) % 20 == 0
  end
end
g.update { |_dt| job.resume if job.alive? }
```

This example restarts generation on relaunch. For resumable work, store an explicit cursor and seed with generated records, then recreate the Fiber from that state. Fibers themselves cannot be serialized. Do not put an unbounded loop inside one frame callback.

## API reference

| Object | Operations |
| --- | --- |
| `Storage::Store` | `.new(path, partition_size: 64)`; `.open(path, **options) { ... }`; `path`; `table(name, partition_size:)`; `table_names`; `flush` / `save`; `snapshot(new_directory)`; `vacuum!`; `close(save: true)`; `closed?` |
| `Storage::Table` | `insert(hash) → id`; `[id] → hash/nil`; `update(id) { |hash| ... } → hash`; `delete(id) → old_hash/nil`; `each(loaded_only: true) { |id, hash| ... }`; `where(loaded_only: true, **fields) → pairs`; `metadata`; `metadata=` |
| Table partitions | `partition_count`; `partition_range(part)`; `partition_for(id) → part/nil`; `add_partition → part`; `partitions`; `status(part)`; `revenants`; `stats`; `load_partition` / `revive!`; `unload_partition` / `kill_partition!`; `sleep!`; `wake!` |
| `BSP` | `.new(bounds:, depth: 4, min_size: 32)`; `root`; `leaves`; `leaf_at(x,y)`; `query(bounds)`; `to_h`; `.from_h(hash)`; nodes expose `id`, `bounds`, `children`, `leaf?` |
| `PartitionedWorld` | `.new(store, name: "world", bsp: tree)`; `bsp`; `spawn(x:, y:, **components) → handle`; `[handle]`; `update(handle) { ... }`; `move(handle, x:, y:) → handle`; `delete(handle)`; `stream(bounds) → leaves`; `query(bounds, **components) → pairs`; `table_for(leaf_id)` |
| Integration | `g.storage(name = "game", path: nil, **options)`; `g.save`; `Selenite.grid.storage(name = "game", **options)` |

All constants above live under `Selenite`. Read-only helpers return copies or immutable descriptors. Internal persistence methods are implementation details, not extension points.

## Exporting games

1. Keep exportable storage in a folder named `game-data` beside your script, with any stores beneath it. Use `File.dirname(__FILE__)` when constructing its path.
2. Save and close the game before exporting so no writer changes files during copying.
3. Use **Export game** on the Ruby cell. The exporter copies the script, grid save, sibling `assets`, sibling `game-data`, and the runtime. It also includes the array library for scripting-enabled source-build exports.
4. Run the exported `run-game.sh`. Relative paths resolve against the exported script, so its game data belongs to that copy.

Default grid-folder stores and arbitrary absolute storage paths are **not** automatically collected. To ship one, checkpoint it into the script's `game-data` folder and update the script to open that relative path. If you want a clean campaign, use a separate newly initialized store for distribution. Exports include the current progress and any checkpoint directories inside `game-data`; exporting again overwrites files in the existing export folder.

## Limits and troubleshooting

| Symptom / concern | Action or explanation |
| --- | --- |
| `uninitialized constant Selenite::Storage` or no `g.storage` | Rebuild the scripting-enabled executable; an older bundled binary embeds older Ruby helpers. |
| Cannot load `partitioned_array` | Set `SELENITE_PARTITIONED_ARRAY_LIB` to the bundled library's `lib` directory. |
| `storage already open` | Reuse the existing handle or close the game/console script owning it. Deleting the lock file is not a fix. |
| Query finds nothing after reopening | Use direct ID lookup, `wake!`, `loaded_only: false`, or `world.stream` before resident-only iteration. |
| Changes to a hash disappear | Use `update`; returned hashes are detached copies. |
| Wrong saved game opens | Check the explicit path and grid folder. The demo deliberately uses script-relative storage. |
| BSP mismatch | Reuse the saved layout, migrate explicitly, or create a separately named world. |
| Missing/corrupt chunk | Reads raise `Storage::CorruptStore`; restore a complete checkpoint. No empty replacement is silently saved. |
| Storage directory keeps growing | Call `vacuum!` to remove old committed generations. It does not reclaim record IDs. |
| Large worlds still consume memory | Unloading releases hashes, not the flat address array and range/relative-index structures. Address overhead remains proportional to allocated capacity. |
| Frame stalls while streaming | Disk IO is synchronous. Use smaller partitions, preload nearby regions, and stream at room boundaries or controlled intervals. |

This is a local, single-writer store. Use its handle on one thread. It has no network replication, SQL engine, automatic secondary indexes, automatic eviction budget, or persisted code execution. `where` scans selected rows. BSP limits spatial scans to candidate leaves. Direct record lookup uses binary search over partition ranges. Retained copies of rows remain in memory until your own references are released.

The layer uses the bundled PartitionedArray's allocation, ranges, growth, and mutation operations. It owns persistence and residency because the legacy partial loader uses global offsets into partition-local JSON, and its kill routine assumes an `id` field in every row. The new abstractions do not modify those legacy classes or their export format. Managed-array concerns—automatic growth, high-water IDs, named databases/tables, and archives—are exposed here as table insertion, store namespaces, and snapshots rather than forwarding unsafe raw methods.

## Tests and implementation

Run the focused Ruby suite without a graphics window:

```sh
ruby tests/game_storage_test.rb
cargo test --features scripting
```

The Ruby suite uses Minitest. It covers allocation, later-partition revival, sleeping data, stable IDs, update rollback, metadata, invalid input, writer locks, failed commits, snapshots, vacuum, BSP boundaries, world movement, and game lifecycle behavior. The Rust suite also checks embedded API availability and export support.

Implementation: [storage layer](../src/ruby/game_storage.rb), [BSP and world layer](../src/ruby/game_world.rb), [game helper](../src/ruby/raylib_game.rb). Continue with the [game API reference](game.md) or [beginner tutorial](game-tutorial.md).

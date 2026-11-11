# Stacks, multi-select and grid inventories

Selenite 3 adds three ways to hold more than one thing per place:

1. **Stacked cells**: any cell can hold several items (files and/or nested
   grids). The top item is what you see and open.
2. **Multi-select**: select many cells, then move, cut, copy, delete or
   stack them together.
3. **Grid inventories**: every grid has a game-style inventory of named
   items with counts and JSON metadata. You edit it in a panel and from
   Ruby, and it is saved with the grid.

## Stacked cells

| Action | How |
| --- | --- |
| Stack files on a cell | Drop files onto an occupied file cell; **Shift+drop** stacks anywhere. |
| Stack clipboard contents | **Ctrl+Shift+V** |
| Stack one cell onto another | **Shift+drag** the cell onto the target |
| Stack a multi-selection | Right-click the target → **Stack selection here** |
| Change the top item | **Tab** / **Shift+Tab**, or right-click → **Next in stack** |
| Remove the top item only | **Shift+Delete**, or right-click → **Remove top item** |
| Spread a stack out | Right-click → **Unstack into free cells** (the other items go to the next free cells to the right) |

Stacked cells show a `×N` badge with "plates" behind it. Undo/redo covers
every stack operation.

## Multi-select

| Action | How |
| --- | --- |
| Toggle one cell | **Ctrl+click** |
| Select a rectangle | **Shift+click** (from the selected cell), or drag a box from an empty cell |
| Select everything | **Ctrl+A** |
| Move them together | Drag any selected cell. If something blocks the move, nothing is moved and the status bar says why. |
| Copy / cut / delete | **Ctrl+C**, **Ctrl+X**, **Delete** |
| Fit them on screen | **Z** zooms to fit the whole grid |

## The inventory panel

Press **B**, click the **Items** toolbar button, or right-click → **Grid
inventory…**. The panel lists the current grid's items, each with **+**,
**−** and delete buttons and a metadata summary. Type commands in the entry
field and press Enter (or **Add**):

| Command | Effect |
| --- | --- |
| `potion` | Add 1 potion |
| `gold coin x50`, `arrow ×12`, `key 3`, `7 bombs` | Add that many |
| `herb -2` | Remove 2 (items reaching 0 are removed) |
| `gold = 12` | Set the count exactly (`= 0` deletes) |
| `arrow -> bolt` | Rename (merges into `bolt` if it exists) |
| `gem @color=red` | Set metadata. The value is parsed as JSON when possible (`@value=25`, `@tags=["a",1]`), otherwise stored as text. |
| `gem @color=` | Remove a metadata key |

Each change is one undo step, fires the Ruby `:inventory` event and is
saved with the grid. Nested grids have their own inventories.

## Ruby API

```ruby
inv = Selenite.inventory            # current grid; also Selenite.grid.inventory
inv.add("potion", 3)                # alias give → new count
inv.remove("potion")                # alias take → remaining count; raises if there aren't enough
inv["gold"] = 100                   # set exactly
inv["gold"]                         # => 100 (alias count; 0 if missing)
inv.has?("gold", 50)                # alias include?
inv.rename("gold", "coins")
inv.set_meta("sword", "damage", 7)  # any JSON-able value
inv.meta("sword")                   # => {"damage"=>7}
inv.transfer(Selenite.root.inventory, "coins", 10)
inv.each { |name, count, meta| puts "#{name}: #{count} #{meta}" }
inv.to_h; inv.names; inv.total; inv.size; inv.empty?; inv.delete("x"); inv.clear

Selenite.on(:inventory) { |message| puts "inventory changed: #{message}" }
```

Stacks and selection from Ruby:

```ruby
g = Selenite.grid
g.push(0, 0, "/music/a.ogg")        # add on top of (0,0)
g.push_grid(0, 0)                   # stack a new nested grid
g.items(0, 0)                       # => [Cell, Cell, ...] top first (alias stack)
g.stacked?(0, 0); g.stack_size(0, 0)
g.cycle(0, 0, 1)                    # rotate the stack (−1 backwards)
g.raise_item(0, 0, 2)               # bring item 2 to the top
g.pop(0, 0)                         # remove & return the top item
g.merge(0, 0, 1, 0)                 # stack (0,0)'s items onto (1,0)
g.unstack(1, 0)                     # spread a stack into free cells
Selenite.selection                  # => [[col, row], ...]
Selenite.select_cells([[0, 0], [1, 0]])
```

The classic globals `grid_push_file(col, row, path)`,
`grid_push_grid(col, row)` and `grid_list_stacks` are also available.

Games use the same API through `g.inventory` and `g.grid`; see
[game.md](game.md).

## Save format

Saves use `SAVE_VERSION = 3`. Each grid is an object
`{ "cells": [{ "col", "row", "items": [top, ...] }], "inventory": { "name":
{ "count": N, "meta": { ... } } } }`. The inventory is omitted when empty.
Version 2 saves (a bare list of single-item cells) load unchanged and are
written as version 3 on the next save.

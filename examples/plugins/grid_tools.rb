# grid_tools.rb -- editing grids from a plugin.
#
# Shows: moving cells with the Grid API (every plugin action is one undo
# step, so Ctrl+Z reverts it), toggling labels, and creating sub-grids.

Selenite.plugin "Grid Tools" do |p|
  p.description "Sort the current grid into one row per file type, toggle cell labels (F9), or gather every file of one type into a new sub-grid."
  p.version "1.0"

  kind_order = %w[grid image audio video ruby file]

  p.button("Sort by type") do
    grid = Selenite.grid
    cells = grid.cells.sort_by { |c| [kind_order.index(c.kind) || 99, File.basename(c.path.to_s).downcase] }
    # Two passes so moves never collide: park everything far away first.
    park_row = 1_000_000
    cells.each_with_index { |cell, i| grid.move(cell.col, cell.row, i, park_row) }
    columns = Hash.new(0)
    cells.each_with_index do |cell, i|
      row = kind_order.index(cell.kind) || kind_order.size
      grid.move(i, park_row, columns[row], row)
      columns[row] += 1
    end
    Selenite.status("Sorted #{cells.size} cell(s) by type (Ctrl+Z to undo)")
  end

  p.button("Toggle labels", key: "F9") do
    Selenite.labels(!Selenite.labels?)
  end

  p.menu("Gather this type into a sub-grid", kinds: %w[file]) do |cell|
    grid = Selenite.grid
    target = grid.next_free(0, -1)
    sub = grid.new_grid(target, -1)
    same = grid.cells.select { |c| c.kind == cell.kind }
    same.each_with_index do |c, i|
      sub.set(i % 8, i / 8, c.path)
      grid.remove(c.col, c.row)
    end
    Selenite.status("Moved #{same.size} #{cell.kind} cell(s) into the grid at (#{target}, -1)")
  end
end

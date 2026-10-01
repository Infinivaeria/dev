# Bridges Selenite cells and the partitioned_array library's
# ManagedPartitionedArray. Each item of every cell's stack becomes one hash
# record:
#   { "col" => Integer, "row" => Integer, "index" => Integer,
#     "kind" => String, "path" => String|nil }
# `index` 0 is the top of the stack (v2 exports without it import as 0).
module SelenitePartitionedArray
  DEFAULT_DB_NAME = "selenite_cells"
  OPTIONS = {
    dynamically_allocates: true,
    endless_add: true,
    db_size: 32,
    partition_amount_and_offset: 16,
    partition_addition_amount: 8
  }.freeze

  module_function

  def available?
    require "managed_partitioned_array"
    true
  rescue LoadError => e
    @load_error = e.message
    false
  end

  def load_error
    @load_error
  end

  def build(db_path, db_name)
    unless available?
      raise LoadError, "partitioned_array library unavailable (#{@load_error}); set SELENITE_PARTITIONED_ARRAY_LIB"
    end
    ManagedPartitionedArray.new(**OPTIONS, db_path: db_path, db_name: db_name)
  end

  # Overwrites any previous export snapshot with the given stack items
  # (`[col, row, index, kind, path]` tuples).
  def export(items, db_path, db_name = DEFAULT_DB_NAME)
    require "fileutils"
    FileUtils.rm_rf(File.join(db_path, "#{db_name}[0]"))
    mpa = build(db_path, db_name)
    mpa.allocate
    mpa.save_everything_to_files!
    items.each do |col, row, index, kind, path|
      mpa.add(save_on_partition_add: false, save_last_entry_to_file: false) do |record|
        record["col"] = col
        record["row"] = row
        record["index"] = index
        record["kind"] = kind
        record["path"] = path
      end
    end
    mpa.save_everything_to_files!
    items.size
  end

  def records(db_path, db_name = DEFAULT_DB_NAME)
    mpa = build(db_path, db_name)
    mpa.load_everything_from_files!
    mpa.data_arr.select { |record| record.is_a?(Hash) && record.key?("kind") && record.key?("col") }
  end
end

def grid_pa_available
  SelenitePartitionedArray.available?
end

def grid_pa_export(db_path, db_name = SelenitePartitionedArray::DEFAULT_DB_NAME)
  SelenitePartitionedArray.export(grid_list_stacks, db_path, db_name)
end

def grid_pa_records(db_path, db_name = SelenitePartitionedArray::DEFAULT_DB_NAME)
  SelenitePartitionedArray.records(db_path, db_name)
end

# Restores exported stacks into the current grid. Cells that are already
# occupied are skipped; returns the number of items restored.
def grid_pa_import(db_path, db_name = SelenitePartitionedArray::DEFAULT_DB_NAME)
  by_cell = grid_pa_records(db_path, db_name).group_by { |record| [record["col"], record["row"]] }
  by_cell.sum do |(col, row), records|
    next 0 if grid_get(col, row)
    # Push bottom-first so index 0 ends up on top.
    records.sort_by { |record| -(record["index"] || 0) }.count do |record|
      if record["kind"] == "grid"
        grid_push_grid(col, row)
        true
      elsif record["path"]
        grid_push_file(col, row, record["path"])
        true
      else
        false
      end
    end
  end
end

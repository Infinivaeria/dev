# file_inspector.rb -- Ruby calling into Rust.
#
# Shows: right-click menu items limited to certain cell kinds, and the
# Rust-native helpers Selenite.file_info, Selenite.image_size and
# Selenite.checksum (streamed in Rust, so even multi-GB files are fine).

Selenite.plugin "File Inspector" do |p|
  p.description "Right-click any file cell -> Inspect file / Copy checksum. Uses Rust helpers for size, image dimensions and a streaming checksum."
  p.version "1.1"

  p.menu("Inspect file", kinds: %w[file]) do |cell|
    info = Selenite.file_info(cell.path)
    unless info[:exists]
      Selenite.status("#{cell.path} is missing")
      next
    end
    parts = [info[:kind], "#{(info[:size] / 1024.0).round(1)} KiB", ".#{info[:ext]}"]
    if (size = Selenite.image_size(cell.path))
      parts << "#{size[0]}x#{size[1]} px"
    end
    parts << Time.at(info[:modified]).strftime("modified %Y-%m-%d %H:%M")
    puts "#{info[:path]}: #{parts.join(', ')}"
    Selenite.status(parts.join("  |  "))
  end

  p.menu("Copy checksum", kinds: %w[file]) do |cell|
    sum = Selenite.checksum(cell.path)
    Selenite.copy_text(sum)
    Selenite.status("Copied #{sum} for #{File.basename(cell.path)}")
  end

  # Selenite.run("largest", 5) prints the biggest files in this profile.
  p.command("largest") do |count = 5|
    files = []
    walk = lambda do |grid|
      grid.each do |cell|
        if cell.grid?
          walk.call(grid.subgrid(cell.col, cell.row))
        elsif cell.path
          info = Selenite.file_info(cell.path)
          files << [info[:size], cell.path] if info[:exists]
        end
      end
    end
    walk.call(Selenite.root)
    files.max_by(Integer(count)) { |size, _| size }.each do |size, path|
      puts format("%10.1f MiB  %s", size / 1048576.0, path)
    end
    files.size
  end
end

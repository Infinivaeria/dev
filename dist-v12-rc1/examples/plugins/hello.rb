# hello.rb -- the smallest useful Selenite plugin.
#
# Shows: a button with a function-key shortcut, a console command, and
# reading app state from Ruby.

Selenite.plugin "Hello" do |p|
  p.description "Says hello with a summary of the current grid. Press F6."
  p.version "1.0"

  # Buttons appear in the Plugins panel. The block receives the selected
  # cell (a Selenite::Cell, or nil when nothing is selected).
  p.button("Say hello", key: "F6") do |cell|
    grid = Selenite.grid
    where = cell ? "cell (#{cell.col}, #{cell.row})" : "no selection"
    Selenite.status("Hello from Ruby! #{grid.size} cell(s) here, #{where}, profile #{Selenite.profile.inspect}")
  end

  # Commands are called from the Ruby console or other plugins:
  #   Selenite.run("greet", "world")
  p.command("greet") do |name = "Selenite"|
    message = "Hello, #{name}!"
    puts message
    message
  end
end

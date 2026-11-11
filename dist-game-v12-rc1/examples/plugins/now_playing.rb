# now_playing.rb -- scripting the built-in music player.
#
# Shows: the :track event, Selenite.music, and menu items for audio cells.

Selenite.plugin "Now Playing" do |p|
  p.description "Announces each track, plays every song in the grid with F7 (shuffled with F8), and adds 'Queue track' to audio cells."
  p.version "1.0"

  # Fired by the player whenever a new track starts.
  p.on(:track) do |path, index, count|
    Selenite.status("Now playing #{index + 1}/#{count}: #{File.basename(path, '.*')}")
  end

  p.button("Play this grid", key: "F7") do
    count = Selenite.music.play_grid
    Selenite.status("No playable audio in this grid") if count.zero?
  end

  p.button("Shuffle this grid", key: "F8") do
    Selenite.music.shuffle(true)
    Selenite.music.play_grid
  end

  p.menu("Queue track", kinds: %w[audio]) do |cell|
    Selenite.music.queue(cell.path)
    Selenite.status("Queued #{File.basename(cell.path)}")
  end

  # Selenite.run("np") prints the player state in the console.
  p.command("np") do
    s = Selenite.music.state
    puts "#{s[:state]}: #{s[:track] || '-'} (#{s[:position].round}s / #{s[:length].round}s), " \
         "volume #{(s[:volume] * 100).round}%, shuffle #{s[:shuffle]}, repeat #{s[:repeat]}"
    s
  end
end

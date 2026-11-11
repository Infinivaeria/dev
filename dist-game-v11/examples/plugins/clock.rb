# clock.rb -- background timers and event hooks.
#
# Shows: `every`, which runs a block on the UI thread at a fixed interval
# (minimum 0.25 s; a timer that raises is stopped and the error shown),
# plus state shared between a hook, a timer and a command.

Selenite.plugin "Clock" do |p|
  p.description "Shows the time in the status bar every minute along with how many downloads finished this session."
  p.version "1.0"

  downloads = 0
  p.on(:download) { |*| downloads += 1 }

  p.every(60) do
    Selenite.status("#{Time.now.strftime('%H:%M')}  -  #{downloads} download(s) this session")
  end

  p.command("downloads") { downloads }
end

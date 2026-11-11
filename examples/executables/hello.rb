# This entry comment is intentionally absent from compiled bytecode.
require "json"
require "zlib"
require_relative "helper"

raise "asset missing" unless File.read(File.join(__dir__, "message.txt")).strip == "Bundled asset"
raise "native extension failed" unless Zlib::Inflate.inflate(Zlib::Deflate.deflate("native")) == "native"
Selenite.inventory.add("runs", 1)
Selenite.save
puts JSON.generate(message: executable_message, arguments: ARGV, runs: Selenite.inventory["runs"])
exit 7 if ARGV.include?("--exit-seven")
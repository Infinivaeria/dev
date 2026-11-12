require "json"
puts JSON.generate({"language" => "Ruby bytecode inside Rust", "arguments" => ARGV})
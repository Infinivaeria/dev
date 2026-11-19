require "digest"
require "fileutils"
require "find"
require "json"
require "pathname"

source, destination = ARGV.map { |value| Pathname(value).expand_path }
abort "Usage: ruby tools/package_rustby.rb SOURCE DESTINATION" unless source && destination
abort "Rustby source must be a directory" unless source.directory?
abort "Rustby destination must not overlap its source" if source == destination ||
  destination.to_s.start_with?(source.to_s + "/") ||
  source.to_s.start_with?(destination.to_s + "/")

ignored = %w[target out .git].freeze
files = []
Find.find(source.to_s) do |name|
  path = Pathname(name)
  if path.directory? && path != source && ignored.include?(path.basename.to_s)
    Find.prune
  elsif path.symlink?
    abort "Refusing to package Rustby symlink: #{path.relative_path_from(source)}"
  elsif path.file?
    files << path.relative_path_from(source)
  end
end

FileUtils.rm_rf(destination)
manifest = files.sort_by(&:to_s).map do |relative|
  target = destination.join(relative)
  FileUtils.mkdir_p(target.dirname)
  FileUtils.cp(source.join(relative), target, preserve: true)
  {
    path: relative.to_s,
    sha256: Digest::SHA256.file(target).hexdigest,
  }
end
FileUtils.mkdir_p(destination)
File.write(
  destination.join("rustby-manifest.json"),
  JSON.pretty_generate(schema: 1, files: manifest) + "\n",
)
puts "Bundled #{manifest.length} Rustby source files"

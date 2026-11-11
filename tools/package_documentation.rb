require "digest"
require "fileutils"
require "find"
require "json"
require "pathname"

source, destination, *excludes = ARGV
abort "Usage: ruby tools/package_documentation.rb SOURCE DESTINATION [EXCLUDED_DIRECTORY...]" unless source && destination
source = Pathname(source).expand_path
destination = Pathname(destination).expand_path
abort "Documentation destination must not be the source directory" if source == destination
excluded_paths = [destination, *excludes.map { |path| Pathname(path).expand_path }]
ignored_names = %w[.git target node_modules .venv __pycache__ tmp test-artifacts].freeze
files = []
Find.find(source.to_s) do |name|
  path = Pathname(name)
  if path.directory?
    relative = path.relative_path_from(source)
    generated = relative.each_filename.first&.match?(/\Adist(?:\z|[-_])/)
    Find.prune if path != source && (ignored_names.include?(path.basename.to_s) || generated || excluded_paths.include?(path))
  elsif path.file? && path.extname.downcase == ".md"
    files << path.relative_path_from(source)
  end
end
documents = files.sort_by(&:to_s).map do |relative|
  target = destination.join(relative)
  FileUtils.mkdir_p(target.dirname)
  FileUtils.cp(source.join(relative), target, preserve: true)
  { path: relative.to_s, sha256: Digest::SHA256.file(target).hexdigest }
end
excluded_directories = excluded_paths.filter_map do |path|
  relative = path.relative_path_from(source)
  relative.to_s unless relative.each_filename.first == ".."
end
FileUtils.mkdir_p(destination)
File.write(destination.join("documentation-manifest.json"), JSON.pretty_generate(schema: 1, excluded_source_directories: excluded_directories, documents: documents) + "\n")
puts "Bundled #{documents.length} source Markdown documents (including mic_freq and Selenite guides)"
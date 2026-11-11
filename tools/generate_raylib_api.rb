require "json"
require "open3"

header = ARGV.fetch(0) { abort "Usage: ruby tools/generate_raylib_api.rb /path/to/raylib.h" }
source, errors, status = Open3.capture3("clang", "-fparse-all-comments", "-Xclang", "-ast-dump=json", "-fsyntax-only", header)
abort errors unless status.success?
nodes = JSON.parse(source).fetch("inner")
records = nodes.select { |node| node["kind"] == "RecordDecl" && node["completeDefinition"] && node["name"] }
typedefs = nodes.select { |node| node["kind"] == "TypedefDecl" && node["name"] !~ /\A__/ }
aliases = typedefs.to_h { |node| [node.fetch("name"), node.fetch("type").fetch("qualType").delete_prefix("struct ")] }
callbacks = aliases.select { |_, type| type.include?("(*)") }
aliases.reject! { |name, type| name == type || type.start_with?("enum ") || callbacks.key?(name) }
structs = records.to_h do |node|
  fields = node.fetch("inner", []).select { |child| child["kind"] == "FieldDecl" }
  [node.fetch("name"), fields.map { |field| { "name" => field.fetch("name"), "type" => field.fetch("type").fetch("qualType") } }]
end
normalize = lambda do |type|
  resolved = type.delete_prefix("const ").delete_prefix("struct ")
  seen = []
  while aliases.key?(resolved) && !seen.include?(resolved)
    seen << resolved
    resolved = aliases.fetch(resolved)
  end
  resolved
end
numeric = { "float" => "f32", "double" => "f64", "int" => "i32", "unsigned int" => "u32", "unsigned short" => "u16", "short" => "i16", "unsigned char" => "u8", "char" => "std::ffi::c_char", "long" => "std::ffi::c_long", "unsigned long" => "std::ffi::c_ulong" }
read_value = nil
read_value = lambda do |type, value|
  resolved = normalize.call(type)
  if resolved.end_with?("*")
    "address(#{value})? as _"
  elsif callbacks.key?(resolved)
    "unsafe { std::mem::transmute::<usize, ffi::#{resolved}>(address(#{value})?) }"
  elsif (array = /\A(.+)\[(\d+)\]\z/.match(resolved))
    "{ let values = sequence(#{value}, #{array[2]})?; [#{array[2].to_i.times.map { |index| read_value.call(array[1], "values[#{index}]") }.join(', ')}] }"
  elsif numeric.key?(resolved)
    rust_type = numeric.fetch(resolved)
    if %w[float double].include?(resolved)
      "f64::try_convert(#{value})? as #{rust_type}"
    else
      "#{rust_type}::try_from(#{resolved.start_with?('unsigned') ? 'u64' : 'i64'}::try_convert(#{value})?).map_err(|_| argument(\"#{resolved} out of range\"))?"
    end
  elsif %w[_Bool bool].include?(resolved)
    "bool::try_convert(#{value})?"
  elsif structs.key?(resolved)
    "read_#{resolved}(#{value})?#{type == 'Quaternion' ? '.into()' : ''}"
  else
    abort "Unsupported argument type: #{type}"
  end
end
write_value = nil
write_value = lambda do |type, value|
  resolved = normalize.call(type)
  if resolved.end_with?("*")
    "((#{value}) as usize).into_value_with(ruby)"
  elsif callbacks.key?(resolved)
    "(#{value}.map_or(0, |callback| callback as usize)).into_value_with(ruby)"
  elsif (array = /\A(.+)\[(\d+)\]\z/.match(resolved))
    "{ let array = ruby.ary_new(); for element in #{value} { array.push(#{write_value.call(array[1], 'element')})?; } array.as_value() }"
  elsif numeric.key?(resolved) || %w[_Bool bool].include?(resolved)
    "(#{value}).into_value_with(ruby)"
  elsif structs.key?(resolved)
    "write_#{resolved}(ruby, #{value}#{type == 'Quaternion' ? '.into()' : ''})?"
  elsif resolved == "void"
    "ruby.qnil().as_value()"
  else
    abort "Unsupported return type: #{type}"
  end
end
comment_text = lambda do |node|
  texts = []
  visit = lambda do |child|
    texts << child["text"].strip if child["kind"] == "TextComment"
    child.fetch("inner", []).each { |nested| visit.call(nested) }
  end
  node.fetch("inner", []).select { |child| child["kind"] == "FullComment" }.each { |child| visit.call(child) }
  texts.reject(&:empty?).join(" ")
end
functions = nodes.select { |node| node["kind"] == "FunctionDecl" && node["name"] =~ /\A[A-Z]/ }.to_h do |node|
  parameters = node.fetch("inner", []).select { |child| child["kind"] == "ParmVarDecl" }
  [node.fetch("name"), { "returns" => node.fetch("type").fetch("qualType").split("(", 2).first.strip,
    "params" => parameters.map { |parameter| { "name" => parameter.fetch("name"), "type" => parameter.fetch("type").fetch("qualType") } },
    "variadic" => !!node["variadic"], "description" => comment_text.call(node) }]
end
constant_value = lambda do |node|
  node["value"] || node.fetch("inner", []).filter_map { |child| constant_value.call(child) }.first
end
constants = {}
nodes.select { |node| node["kind"] == "EnumDecl" }.each do |node|
  next_value = 0
  node.fetch("inner", []).select { |child| child["kind"] == "EnumConstantDecl" }.each do |constant|
    next_value = (constant_value.call(constant) || next_value).to_i
    constants[constant.fetch("name")] = next_value
    next_value += 1
  end
end
rust = ["#![allow(dead_code, non_snake_case, unused_variables, unused_unsafe, clippy::useless_conversion)]", "use super::*;"]
structs.each do |name, fields|
  rust << "fn read_#{name}(value: Value) -> Result<ffi::#{name}, Error> {"
  rust << "let fields = sequence(value, #{fields.size})?;"
  rust << "Ok(ffi::#{name} { #{fields.each_with_index.map { |field, index| "#{field['name'] == 'type' ? 'type_' : field['name']}: #{read_value.call(field['type'], "fields[#{index}]")}" }.join(', ')} }) }"
  rust << "fn write_#{name}(ruby: &Ruby, value: ffi::#{name}) -> Result<Value, Error> { let array = ruby.ary_new();"
  fields.each { |field| rust << "array.push(#{write_value.call(field['type'], "value.#{field['name'] == 'type' ? 'type_' : field['name']}")})?;" }
  rust << "Ok(array.as_value()) }"
end
rust << "pub(super) fn call(ruby: &Ruby, name: &str, args: &[Value]) -> Result<Value, Error> { match name {"
functions.each do |name, signature|
  rust << "\"#{name}\" => { arity(args, #{signature['params'].size})?;"
  arguments = signature.fetch("params").each_with_index.map do |parameter, index|
    if parameter["type"] == "const char *"
      rust << "let text_#{index} = string_argument(args[#{index}])?;"
      "text_#{index}.pointer()"
    else
      read_value.call(parameter.fetch("type"), "args[#{index}]")
    end
  end
  if signature["variadic"]
    text_index = signature["params"].size - 1
    arguments[text_index] = 'c"%s".as_ptr()'
    arguments << "text_#{text_index}.pointer()"
  end
  lifecycle = {
    "InitWindow" => 'crate::game::init_window(i32::try_convert(args[0])?, i32::try_convert(args[1])?, String::try_convert(args[2])?)?',
    "CloseWindow" => 'crate::game::close_window()?',
    "WindowShouldClose" => 'crate::game::window_should_close()?',
    "EndDrawing" => 'crate::game::end_drawing()?',
    "InitAudioDevice" => 'crate::game::init_audio_device()?',
    "CloseAudioDevice" => 'crate::game::close_audio_device()?'
  }
  rust << "let result = #{lifecycle.fetch(name) { "unsafe { ffi::#{name}(#{arguments.join(', ')}) }" }};"
  if signature["returns"] == "const char *"
    rust << "Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })"
  else
    rust << "Ok(#{write_value.call(signature.fetch('returns'), 'result')})"
  end
  rust << "},"
end
rust << '_ => Err(argument(format!("Unknown raylib function: {name}"))), } }'
root = File.expand_path("..", __dir__)
metadata = { "functions" => functions, "structs" => structs, "aliases" => aliases, "callbacks" => callbacks, "constants" => constants }
File.write(File.join(root, "src/raylib_api_generated.rs"), rust.join("\n") + "\n")
File.write(File.join(root, "src/ruby/raylib_api.json"), JSON.pretty_generate(metadata) + "\n")
reference = ["# Complete Raylib API", "", "Generated from the linked raylib public header. All #{functions.size} functions are available under `Selenite.raylib_native` after `enable!`. Native calls use C ownership rules; do not mix native resources with managed handles.", "", "See the native API tutorial for records, pointers, callbacks and resource lifetime.", ""]
functions.each do |name, signature|
  parameters = signature.fetch("params").map { |parameter| "#{parameter['type']} #{parameter['name']}" }.join(", ")
  parameters += ", ..." if signature["variadic"]
  reference.concat(["## #{name}", "", "```c", "#{signature['returns']} #{name}(#{parameters});", "```", "", signature["description"], ""])
end
File.write(File.join(root, "docs/raylib-api.md"), reference.join("\n") + "\n")
puts "Generated #{functions.size} functions, #{structs.size} records and #{constants.size} constants"
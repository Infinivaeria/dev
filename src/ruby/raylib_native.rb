require "json"

module Raylib
  module Native
    API = JSON.parse(SCHEMA_JSON).freeze
    Pointer = Struct.new(:address) do
      def null? = address.zero?
      def to_i = address
    end

    class << self
      def enable!
        raise "Native raylib calls require a separate --game process" unless Raylib.GameMode?
        @enabled = true
        self
      end

      def resolve(type)
        type = type.delete_prefix("const ").delete_prefix("struct ")
        seen = []
        while API["aliases"].key?(type) && !seen.include?(type)
          seen << type
          type = API["aliases"].fetch(type)
        end
        type
      end

      def default(type)
        type = resolve(type)
        if type.end_with?("*") || API["callbacks"].key?(type)
          nil
        elsif (array = /\A(.+)\[(\d+)\]\z/.match(type))
          Array.new(array[2].to_i) { default(array[1]) }
        elsif API["structs"].key?(type)
          const_get(type).new
        elsif %w[_Bool bool].include?(type)
          false
        else
          0
        end
      end

      def wrap(type, value)
        type = resolve(type)
        if type.end_with?("*")
          return value if type == "char *" && value.is_a?(String)
          Pointer.new(value)
        elsif API["structs"].key?(type)
          fields = API["structs"].fetch(type)
          const_get(type).new(*fields.zip(value).map { |field, entry| wrap(field.fetch("type"), entry) })
        elsif (array = /\A(.+)\[(\d+)\]\z/.match(type))
          value.map { |entry| wrap(array[1], entry) }
        else
          value
        end
      end

      def call(name, *arguments)
        raise "Call Raylib::Native.enable! to acknowledge native pointer and lifetime rules" unless @enabled
        signature = API.fetch("functions").fetch(name.to_s) { raise ArgumentError, "Unknown raylib function: #{name}" }
        if signature["variadic"]
          fixed = signature["params"].length
          raise ArgumentError, "Missing format string" if arguments.length < fixed
          arguments = arguments.first(fixed - 1) + [format(arguments[fixed - 1], *arguments.drop(fixed))]
        end
        wrap(signature.fetch("returns"), __invoke(name.to_s, arguments))
      end
    end

    API["structs"].each do |name, fields|
      record = Struct.new(*fields.map { |field| field.fetch("name").to_sym }) do
        define_method(:initialize) do |*values, **keywords|
          raise ArgumentError, "Too many fields" if values.length > fields.length
          super(*fields.each_with_index.map do |field, index|
            keywords.fetch(field.fetch("name").to_sym) { index < values.length ? values[index] : Native.default(field.fetch("type")) }
          end)
        end
      end
      const_set(name, record)
    end
    API["aliases"].each { |name, target| const_set(name, const_get(target)) if API["structs"].key?(target) }
    API["constants"].each do |name, value|
      const_set(name, value)
      Raylib.const_set(name, value) unless Raylib.const_defined?(name, false)
    end
    API["functions"].each_key do |name|
      define_singleton_method(name) { |*arguments| call(name, *arguments) }
      snake = Raylib.__snake(name)
      singleton_class.alias_method(snake, name)
    end
  end
end

module Selenite
  def self.raylib = ::Raylib
  def self.raylib_native = ::Raylib::Native
end
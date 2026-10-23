# frozen_string_literal: true

require_relative "genius_bench/version"

versioned = File.join(__dir__, "genius_bench", RUBY_VERSION[/\d+\.\d+/], "semantic_ruby")
require_relative File.exist?("#{versioned}.#{RbConfig::CONFIG["DLEXT"]}") ? versioned : "genius_bench/semantic_ruby"

module GeniusBench
  def self.from_struct(value:)
    raise TypeError, "Expected a Struct or Data instance" unless value.is_a?(Struct) || value.is_a?(Data)

    Value.from_ruby(value: project_struct(value, active: {}.compare_by_identity, depth: 1))
  end

  def self.project_struct(item, active:, depth:)
    structured = item.is_a?(Struct) || item.is_a?(Data)
    return item unless structured || item.is_a?(Array) || item.is_a?(Hash)
    raise ArgumentError, "Cyclic struct value" if active.key?(item)
    raise ArgumentError, "Struct value nesting exceeds #{NESTING_LIMIT} levels" if depth > NESTING_LIMIT

    active[item] = true
    begin
      if structured
        item.members.to_h { |member| [member.to_s, project_struct(item.public_send(member), active: active, depth: depth + 1)] }
      elsif item.is_a?(Hash)
        item.to_h { |name, child| [name, project_struct(child, active: active, depth: depth + 1)] }
      else
        item.map { |child| project_struct(child, active: active, depth: depth + 1) }
      end
    ensure
      active.delete(item)
    end
  end

  private_class_method :project_struct
end

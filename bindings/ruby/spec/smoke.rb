# frozen_string_literal: true

require "genius_bench"

specification = Gem.loaded_specs.fetch("genius-bench-kernel")
raise "Version mismatch" unless specification.version.to_s == GeniusBench.version
raise "Loaded outside the gem home" unless File.realpath(specification.full_gem_path).start_with?(File.realpath(ENV.fetch("GEM_HOME")))
raise "Source identity mismatch" unless GeniusBench.source_hash == ENV.fetch("KERNEL_SOURCE_HASH")

task = GeniusBench::Task.new(
  id: "installed",
  fixture: [],
  instruction: GeniusBench::Instruction.new(prompt: "observe"),
  criterion: GeniusBench::Criterion.state(predicate: GeniusBench::Predicate.present(path: ["response"])),
  allowed_files: [],
  reference: GeniusBench::Reference.new(edits: [], completion: GeniusBench::Value.from_ruby(value: nil))
)
outcome = GeniusBench::Outcome.new(
  changed_files: [],
  verification: GeniusBench::Verification.unchecked,
  response: GeniusBench::Value.from_ruby(value: true),
  facts: []
)
raise "Unexpected scores" unless task.grade(outcome: outcome) == [["correctness", true], ["scope", true]]

puts RUBY_VERSION, $LOADED_FEATURES.grep(/semantic_ruby/).first, GeniusBench.version, GeniusBench.source_hash

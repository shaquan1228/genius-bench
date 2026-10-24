# frozen_string_literal: true

require "genius_bench"

specification = Gem.loaded_specs["genius-bench-kernel"]
raise "Version mismatch" if specification && specification.version.to_s != GeniusBench.version
if ENV["GEM_HOME"]
  gem_path = specification && File.realpath(specification.full_gem_path)
  raise "Loaded outside the gem home" unless gem_path&.start_with?(File.realpath(ENV["GEM_HOME"]))
end
if ENV["KERNEL_SOURCE_HASH"]
  raise "Source identity mismatch" unless GeniusBench.source_hash == ENV["KERNEL_SOURCE_HASH"]
end

task = GeniusBench::Task.new(
  id: "installed",
  fixture: [],
  instruction: GeniusBench::Instruction.new(prompt: "observe"),
  criterion: GeniusBench::Criterion.state(predicate: GeniusBench::Predicate.present(path: ["response"])),
  allowed_files: [],
  reference: GeniusBench::Reference.new(edits: [], completion: true)
)
outcome = GeniusBench::Outcome.new(
  changed_files: [],
  verification: GeniusBench::Verification.unchecked,
  response: GeniusBench::Value.from_ruby(value: true),
  facts: []
)
raise "Unexpected scores" unless task.grade(outcome: outcome) == [["correctness", true], ["scope", true]]
raise "Unexpected reference scores" unless task.grade_reference(verification: GeniusBench::Verification.unchecked) == [["correctness", true], ["scope", true]]

puts RUBY_VERSION, $LOADED_FEATURES.grep(/semantic_ruby/).first, GeniusBench.version, GeniusBench.source_hash

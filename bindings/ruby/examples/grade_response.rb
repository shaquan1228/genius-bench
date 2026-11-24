# frozen_string_literal: true

require "genius_bench"

task = GeniusBench::Task.new(
  id: "response-only",
  fixture: [],
  instruction: GeniusBench::Instruction.new(prompt: "Respond without editing files."),
  criterion: GeniusBench::Criterion::State.new(predicate: GeniusBench::Predicate::Present.new(path: ["response"])),
  allowed_files: [],
  reference: GeniusBench::Reference.new(edits: [], completion: true)
)
outcome = GeniusBench::Outcome.new(
  changed_files: ["extra.txt"],
  verification: GeniusBench::Verification::Unchecked.new,
  response: true,
  facts: []
)
task.grade(outcome: outcome).each do |name, passed|
  puts "#{name} #{passed}"
end

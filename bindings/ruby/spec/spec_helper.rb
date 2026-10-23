# frozen_string_literal: true

require "genius_bench"

module KernelSpecSupport
  def kernel(ruby)
    GeniusBench::Value.from_ruby(value: ruby)
  end

  def state_task(predicate: GeniusBench::Predicate.equals(path: %w[response answer], expected: kernel(42)))
    GeniusBench::Task.new(
      id: "state",
      fixture: [],
      instruction: GeniusBench::Instruction.new(prompt: "observe"),
      criterion: GeniusBench::Criterion.state(predicate: predicate),
      allowed_files: [],
      reference: GeniusBench::Reference.new(edits: [], completion: kernel(nil))
    )
  end

  def build_outcome(response: kernel(nil), verification: GeniusBench::Verification.unchecked, facts: [], changed_files: [])
    GeniusBench::Outcome.new(changed_files: changed_files, verification: verification, response: response, facts: facts)
  end

  def scalars_of(fields)
    fields.map { |name, item| [name, item.scalar] }
  end
end

RSpec.configure do |config|
  config.disable_monkey_patching!
  config.expect_with(:rspec) { |expectations| expectations.syntax = :expect }
  config.mock_with(:rspec) { |mocks| mocks.verify_partial_doubles = true }
  config.order = :random
  Kernel.srand config.seed
  config.include KernelSpecSupport
end

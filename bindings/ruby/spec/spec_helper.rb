# frozen_string_literal: true

require "genius_bench"

if ENV["RBS_TEST"]
  require "pathname"
  require "rbs"
  require "rbs/test"

  loader = RBS::EnvironmentLoader.new
  loader.add(path: Pathname(File.expand_path("../sig", __dir__)))
  tester = RBS::Test::Tester.new(env: RBS::Environment.from_loader(loader).resolve_type_names)
  RBS.logger_level = "warn"
  [GeniusBench, *GeniusBench.constants.map { |name| GeniusBench.const_get(name) }.grep(Module)].each do |target|
    tester.install!(target, sample_size: nil, unchecked_classes: [])
  end
end

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
  config.filter_run_when_matching :focus
  config.example_status_persistence_file_path = ".rspec_status"
  config.shared_context_metadata_behavior = :apply_to_host_groups
  config.order = :random
  Kernel.srand config.seed
  config.include KernelSpecSupport
end

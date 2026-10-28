# frozen_string_literal: true

require "genius_bench"

module KernelSpecSupport
  def self.kernel_modules(namespace)
    namespace.constants(false).map { |name| namespace.const_get(name) }.grep(Module).flat_map { |constant| [constant, *kernel_modules(constant)] }
  end
end

if ENV["RBS_TEST"]
  require "pathname"
  require "rbs"
  require "rbs/test"

  loader = RBS::EnvironmentLoader.new
  loader.add(path: Pathname(File.expand_path("../sig", __dir__)))
  tester = RBS::Test::Tester.new(env: RBS::Environment.from_loader(loader).resolve_type_names)
  RBS.logger_level = "warn"
  [GeniusBench, *KernelSpecSupport.kernel_modules(GeniusBench)].each do |target|
    tester.install!(target, sample_size: nil, unchecked_classes: [])
  end
end

module KernelSpecSupport
  def kernel(ruby)
    GeniusBench::Value.from_ruby(value: ruby)
  end

  def state_task(
    predicate: GeniusBench::Predicate::Equals.new(path: %w[response answer], expected: kernel(42)),
    completion: nil,
    edits: [],
    allowed_files: []
  )
    GeniusBench::Task.new(
      id: "state",
      fixture: [],
      instruction: GeniusBench::Instruction.new(prompt: "observe"),
      criterion: GeniusBench::Criterion::State.new(predicate: predicate),
      allowed_files: allowed_files,
      reference: GeniusBench::Reference.new(edits: edits, completion: completion)
    )
  end

  def build_outcome(response: kernel(nil), verification: GeniusBench::Verification::Unchecked.new, facts: [], changed_files: [])
    GeniusBench::Outcome.new(changed_files: changed_files, verification: verification, response: response, facts: facts)
  end

  def scalars_of(fields)
    fields.map { |name, item| [name, item.value] }
  end
end

RSpec::Matchers.define_negated_matcher :not_respond_to, :respond_to

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

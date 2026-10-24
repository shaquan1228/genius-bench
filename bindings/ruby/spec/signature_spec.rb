# frozen_string_literal: true

require "pathname"
require "rbs"

RSpec.describe "sig/genius_bench.rbs" do
  signatures = Pathname(
    Gem.loaded_specs.key?("genius-bench-kernel") ? Gem.loaded_specs["genius-bench-kernel"].full_gem_path : File.expand_path("..", __dir__)
  ) / "sig"
  loader = RBS::EnvironmentLoader.new
  loader.add(path: signatures)
  environment = RBS::Environment.from_loader(loader).resolve_type_names
  declared = environment.class_decls.select { |name, _| name.namespace.to_s == "::GeniusBench::" || name.to_s == "::GeniusBench" }

  describe "the declared modules and classes" do
    subject { declared.keys.map(&:to_s).sort }

    let(:defined) { ["::GeniusBench", *GeniusBench.constants.select { |name| GeniusBench.const_get(name).is_a?(Module) }.map { |name| "::GeniusBench::#{name}" }].sort }

    it { is_expected.to eq(defined) }
  end

  %w[VERSION NESTING_LIMIT REQUIRED_SCORES].each do |constant|
    it "declares #{constant}" do
      expect(environment.constant_decls.keys.map(&:to_s)).to include("::GeniusBench::#{constant}")
    end
  end

  declared.each do |name, entry|
    constant = Object.const_get(name.to_s.delete_prefix("::"))
    singleton, instance = entry.decls
      .flat_map { |decl| decl.decl.members.grep(RBS::AST::Members::MethodDefinition) }
      .partition(&:singleton?)
      .map { |members| members.map(&:name).sort }

    describe name.to_s do
      it "declares the singleton methods" do
        expect(singleton).to eq(constant.singleton_methods(false).grep_v(/__with(out)?__/).sort)
      end

      it "declares the instance methods" do
        expect(instance).to eq(constant.public_instance_methods(false).grep_v(/__with(out)?__/).sort)
      end
    end
  end
end

# frozen_string_literal: true

require_relative "lib/genius_bench/version"

Gem::Specification.new do |spec|
  spec.name = "genius-bench-kernel"
  spec.version = GeniusBench::VERSION
  spec.summary = "Typed native evaluation semantics"
  spec.description = "Task declarations and observed outcomes produce assessments with named scores through the Genius Bench Rust core."
  spec.authors = ["Shaquan Nelson"]
  spec.license = "MIT"
  spec.homepage = "https://github.com/shaquan1228/genius-bench"
  spec.required_ruby_version = ">= 3.2"
  spec.files = Dir["lib/**/*.rb", "sig/**/*.rbs", "ext/**/*.{rs,rb,toml}", "crates/**/*.{rs,toml}", "Cargo.{toml,lock}"] + ["LICENSE"]
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/semantic_ruby/extconf.rb"]
  spec.metadata["cargo_crate_name"] = "semantic-ruby"
  spec.metadata["rubygems_mfa_required"] = "true"
  spec.metadata["source_code_uri"] = "#{spec.homepage}/tree/main/bindings/ruby"
  spec.metadata["changelog_uri"] = "#{spec.homepage}/blob/main/CHANGELOG.md"
  spec.metadata["bug_tracker_uri"] = "#{spec.homepage}/issues"
  spec.metadata["documentation_uri"] = "#{spec.homepage}/blob/main/bindings/ruby/README.md"
  spec.add_dependency "rb_sys", "= 0.9.130"
end

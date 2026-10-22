# frozen_string_literal: true

require_relative "lib/genius_bench/version"

Gem::Specification.new do |spec|
  spec.name = "genius-bench-kernel"
  spec.version = GeniusBench::VERSION
  spec.summary = "Typed native evaluation semantics"
  spec.authors = ["Shaquan Nelson"]
  spec.license = "MIT"
  spec.homepage = "https://github.com/shaquan1228/genius-bench"
  spec.required_ruby_version = ">= 3.2"
  spec.files = Dir["lib/**/*.rb", "sig/**/*.rbs", "ext/**/*.{rs,rb,toml}", "crates/**/*.{rs,toml}", "Cargo.{toml,lock}", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/semantic_ruby/extconf.rb"]
  spec.metadata["cargo_crate_name"] = "semantic-ruby"
  spec.metadata["rubygems_mfa_required"] = "true"
  spec.add_dependency "rb_sys", "= 0.9.130"
end

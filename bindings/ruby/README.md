# Ruby binding

Add the gem to your Bundler project, then save and run the
[shared grading example](examples/grade_response.rb). In a new project, run
`bundle init` first.

```sh
bundle add genius-bench-kernel
bundle exec ruby grade_response.rb
```

The platform gems cover CRuby 3.2 through 3.4 on Linux x86_64 with glibc
2.30 or later and Apple silicon on macOS 11 or later. For older glibc,
set `force_ruby_platform: true` on the Gemfile entry and run `bundle install`.
The source gem requires Rust, a C compiler, and libclang.

## Grading a structured answer

For “What is six times seven?”, use this criterion with
`GeniusBench::Reference.new(edits: [], completion: {"answer" => 42})`.
Supply the candidate's hash in the outcome's `response:` argument.

```ruby
require "genius_bench"

criterion = GeniusBench::Criterion::Answer.new(
  rubric: GeniusBench::Rubric.new(expected: {answer: 42}, text_rule: nil)
)
```

Value arguments accept Ruby scalars, arrays, hashes, and objects implementing
`to_kernel_value`. `GeniusBench.from_struct` adapts Struct and Data instances.
A `GeniusBench::Record` preserves duplicate field names. The
[installed-gem test](spec/smoke.rb) shows a Data adapter and pattern matching.

`ValidationError` extends `ArgumentError` with `case_id` and frozen `issues`.
`GradingError` extends `RuntimeError` and preserves the harness diagnostic.
The gem includes [RBS declarations](sig/genius_bench.rbs).
`GeniusBench.version` and `GeniusBench.source_hash` identify the installed core.

## Building gems

```sh
bundle install --gemfile bindings/ruby/Gemfile
cd bindings/ruby
bundle exec rake gem:source
bundle exec rake native gem
```

For the language comparison and the full build and verification commands, see
the [repository README](../../README.md).

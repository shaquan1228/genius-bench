# Ruby binding

From the repository root, install the built gem for your platform and run the
[shared grading example](examples/grade_response.rb):

```sh
gem install --local path/to/genius-bench-kernel-0.1.0-arm64-darwin.gem
ruby bindings/ruby/examples/grade_response.rb
```

Require `genius_bench` to load `GeniusBench`. Platform gems target Ruby 3.2–3.4
on Linux x86_64 (glibc) and macOS arm64 and install without a Rust compiler.
The source gem vendors the core and builds with Rust, libclang, and rb_sys.

## Grading a structured answer

For “What is six times seven?”, use this criterion with
`GeniusBench::Reference.new(edits: [], completion: {"answer" => 42})`.
The harness supplies a hash in the outcome's `response:` argument:

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

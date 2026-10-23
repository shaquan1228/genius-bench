# Ruby gem

The provisional gem name is `genius-bench-kernel`; its require is
`genius_bench` and its module is `GeniusBench`. No registry publication is
configured. Install a built platform gem with
`gem install path/to/genius-bench-kernel-*.gem`.

## Getting started

```ruby
require "genius_bench"

task = GeniusBench::Task.new(
  id: "answer",
  fixture: [],
  instruction: GeniusBench::Instruction.new(prompt: "What is six times seven?"),
  criterion: GeniusBench::Criterion.answer(
    rubric: GeniusBench::Rubric.new(
      expected: {answer: GeniusBench::Value.from_ruby(value: 42)}, text_rule: nil
    )
  ),
  allowed_files: [],
  reference: GeniusBench::Reference.new(
    edits: [], completion: GeniusBench::Value.from_ruby(value: {"answer" => 42})
  )
)
outcome = GeniusBench::Outcome.new(
  changed_files: [],
  verification: GeniusBench::Verification.unchecked,
  response: GeniusBench::Value.from_ruby(value: {"answer" => 42}),
  facts: {}
)
assessment = task.assess(outcome: outcome)
assessment.passed? # => true
task.grade(outcome: outcome) # => [["correctness", true], ["scope", true]]
```

`Criterion.code(verifier:)` declares executable verification;
`Criterion.state(predicate:)` grades custom state and trace observations. The
harness executes the verifier and supplies its `Verification` status.
`REQUIRED_SCORES` and `NESTING_LIMIT` come from the core.

## Values

`Value.from_ruby(value:)` is the explicit host-value boundary. It accepts nil,
booleans, 64-bit integers, floats, UTF-8-compatible strings, arrays, hashes
with String or Symbol names, native values, and objects responding to
`to_kernel_value`. Names must be distinct after Symbol conversion. Integers and
strings are not coerced through `to_int` or `to_str`. Cycles and nesting beyond
`NESTING_LIMIT` raise `ArgumentError`; unsupported objects raise `TypeError`.

`Record.new(fields:)` keeps duplicate names. `GeniusBench.from_struct(value:)`
projects `Struct` and `Data` instances, including nested ones.

Named collections (`fields`, `checks`, `facts`, `expected`) accept a Hash or an
Array of `[name, value]` pairs.

Every native object is frozen. `==`, `eql?`, and `hash` follow the core's
semantics: record equality ignores field order and uses the first field of a
duplicated name. `Value#equal_to?(other:)` is the explicit semantic alias.
`equal?` keeps Ruby's identity meaning. Kinds and statuses are frozen strings.

## Errors

`ValidationError` is an `ArgumentError` with `case_id` and frozen `issues`,
each an `Issue` with `field` and `message`. `GradingError` is a `RuntimeError`
with `failure` and `detail`. Candidate failure, invalid submission, timeout,
and harness failure keep distinct verification statuses.

## Types

`sig/genius_bench.rbs` declares the public surface. The gate validates it and
runs the suite a second time with `rbs/test` hooks installed on every class, so
each declared argument and return type is checked against the extension.

## Supported platforms

Platform gems install and grade without Rust. CI cross-compiles them with
rb-sys-dock and installs each one on the matching runner across Ruby 3.2, 3.3,
and 3.4:

| Platform | Gem platform | Compiled Ruby ABIs | Installed on |
| --- | --- | --- | --- |
| Linux x86_64, glibc | x86_64-linux | 3.2, 3.3, 3.4 | ubuntu-24.04 |
| macOS arm64 | arm64-darwin | 3.2, 3.3, 3.4 | macos-14 |

The minimum glibc and macOS versions are those of the rb-sys-dock images for
rb_sys 0.9.130; the install matrix is the verified floor. Other Rubies, musl
Linux, and other architectures install from the source gem, which vendors the
semantic core and needs Rust, libclang, and rb_sys at install time.

`GeniusBench.version` and `GeniusBench.source_hash` report the installed core's
identity.

## Development

Install Ruby 3.3 from `.tool-versions` and Rust from `rust-toolchain.toml`,
then:

```sh
bundle install --gemfile bindings/ruby/Gemfile
cd bindings/ruby
bundle exec rake compile spec
bundle exec standardrb
bundle exec rbs -I sig validate
RBS_TEST=1 bundle exec rspec --tag '~contract'
bundle exec rake gem:source
bundle exec rake native gem
```

A local `rake native gem` names the platform gem after the host, for example
`genius-bench-kernel-0.1.0-arm64-darwin-25.gem`; CI names cross-compiled gems
`arm64-darwin` and `x86_64-linux`. The extension crate is its own cargo
workspace under `bindings/ruby`, so core and Python checks need neither Ruby
nor libclang.

`make check` at the repository root runs all of the above after the core and
Python checks, then installs the source and host platform gems into an
isolated gem home and grades with no compiler on the subprocess `PATH`. Verify
a built gem on its own with:

```sh
python3 bindings/ruby/verify_gem.py --gem pkg/genius-bench-kernel-0.1.0-arm64-darwin-25.gem --native
```

See the [repository overview](../../README.md),
[contributor rules](../../CONTRIBUTING.md), and the
[changelog](../../CHANGELOG.md).

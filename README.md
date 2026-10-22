# Genius Bench

A shared grading core for evaluation libraries, starting with the
[Genius eval library](https://github.com/shaquan1228/genius).
Task declarations and observed outcomes produce assessments with named scores
and supporting evidence.

The Rust core, typed Python package, and native Ruby gem are implemented.
Genius's runtime migration, Ruby binary gem automation, and registry publication
remain separate work.

- Rust owns values, predicates, declaration validation, public case projections,
  grading, and structured failures. Grading performs no IO.
- Python and Ruby adapt host values and direct domain calls through native
  extensions.
- Genius retains HF Jobs, Docker, Inspect, execution, verification environments,
  evidence collection, SDK types, provider settings, and consumer defaults.

Instructions contain only a prompt. Public cases contain an identity, prompt,
and files, including explicitly public tools. Tasks own immutable snapshots;
public cases omit reference answers, private checks, and verifier stdin.

Install a built wheel to import `kernel` without Rust or OCaml:

```sh
python -m pip install path/to/genius_bench_kernel-*.whl
```

Ruby loads the same core as `GeniusBench` from the `genius-bench-kernel` gem:

```sh
gem install path/to/genius-bench-kernel-0.1.0-arm64-darwin.gem
```

```ruby
require "genius_bench"

task = GeniusBench::Task.new(
  id: "answer",
  fixture: [],
  instruction: GeniusBench::Instruction.new(prompt: "What is six times seven?"),
  criterion: GeniusBench::Criterion.answer(
    rubric: GeniusBench::Rubric.new(
      expected: [["answer", GeniusBench::Value.from_ruby(value: 42)]], text_rule: nil
    )
  ),
  allowed_files: [],
  reference: GeniusBench::Reference.new(
    edits: [], completion: GeniusBench::Value.from_ruby(value: { "answer" => 42 })
  )
)
outcome = GeniusBench::Outcome.new(
  changed_files: [],
  verification: GeniusBench::Verification.unchecked,
  response: GeniusBench::Value.from_ruby(value: { "answer" => 42 }),
  facts: []
)
task.grade(outcome: outcome) # => [["correctness", true], ["scope", true]]
```

Keyword constructors, frozen snapshots, `ValidationError` (an `ArgumentError`
with `case_id` and `issues`), `GradingError` (a `RuntimeError` with `failure` and
`detail`), the `to_kernel_value` protocol, `GeniusBench.from_struct`, and
checked `sig/genius_bench.rbs` declarations mirror the Python package.
`Value.from_ruby(value:)` accepts nil, booleans, 64-bit integers, floats,
UTF-8-compatible strings, arrays, string- or symbol-keyed hashes whose names
are distinct after symbol conversion, native values, and objects responding to
`to_kernel_value`; `Record.new(fields:)` keeps duplicate names. Declaration
inputs are strict: integers and strings are not coerced through `to_int` or
`to_str`. Kinds, statuses, and failure names are frozen strings, as in Python.
`Value#equal_to?(other:)` is semantic equality; Ruby's `equal?` keeps its
identity meaning. `GeniusBench::File` shadows `::File` inside the namespace.
Platform gems built by `rake native gem` install and grade without Rust; the
source gem built by `rake gem:source` vendors the semantic core and requires
Rust, libclang, and rb_sys at install time. `GeniusBench.version` and `GeniusBench.source_hash` report
the installed core's identity.

For development, install Rust from `rust-toolchain.toml`, uv, CPython 3.12, and
Ruby 3.3 from `.tool-versions`:

```sh
cargo fetch --locked
cargo fetch --locked --manifest-path bindings/ruby/Cargo.toml
uv sync --project bindings/python --locked --no-install-project --python 3.12
bundle install --gemfile bindings/ruby/Gemfile
make check
make install-hooks
```

`make check` runs Rust checks, verifies a fresh wheel installation built from
an extracted source distribution, including boundary and type checks, then
compiles the Ruby extension, runs its RSpec suite, validates
the RBS declarations, and installs the source and host platform gems into an
isolated gem home; the platform gem installs and both gems grade with no
compiler on the subprocess `PATH`. The extension crate is its own cargo
workspace under `bindings/ruby`, so core and Python checks need neither Ruby
nor libclang. The source gem
lock is checked as a subset of the workspace lock, and the vendored core bytes
and the single extension manifest path edit are checked against the checkout.

See the [Python package guide](bindings/python/README.md) for API examples,
adapters, supported platforms, source identity, and packaging procedures;
[CONTRIBUTING.md](CONTRIBUTING.md) for contributor rules; and the
[extraction plan](docs/plans/2026-10-22-semantic-kernel-extraction-plan.md)
for deliverables and verification gates.

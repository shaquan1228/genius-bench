# Genius Bench

A shared grading core for evaluation libraries, starting with the
[Genius eval library](https://github.com/shaquan1228/genius).
Task declarations and observed outcomes produce assessments with named scores
and supporting evidence.

The Rust core, typed Python package, and native Ruby extension are implemented.
Genius's runtime migration, Ruby gem artifacts, and registry publication remain
separate work.

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

Ruby loads the same core from `bindings/ruby` as `GeniusBench`:

```ruby
require "genius_bench"

scores = task.grade(outcome: outcome)
```

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
compiles the Ruby extension and runs its RSpec suite.

See the [Python package guide](bindings/python/README.md) for API examples,
adapters, supported platforms, source identity, and packaging procedures;
[CONTRIBUTING.md](CONTRIBUTING.md) for contributor rules; and the
[extraction plan](docs/plans/2026-10-22-semantic-kernel-extraction-plan.md)
for deliverables and verification gates.

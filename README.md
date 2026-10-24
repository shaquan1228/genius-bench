# Genius Bench

A shared grading core for evaluation libraries, starting with the
[Genius eval library](https://github.com/shaquan1228/genius).
Task declarations and observed outcomes produce assessments with named scores
and supporting evidence.

The Rust core, typed Python package, and native Ruby gem are implemented.
Genius's runtime migration and registry publication remain separate work.

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

Keyword constructors, frozen snapshots, structured `ValidationError` and
`GradingError`, the `to_kernel_value` protocol, `GeniusBench.from_struct`, and
checked `sig/genius_bench.rbs` declarations mirror the Python package. Platform
gems for Linux x86_64 and macOS arm64 on Ruby 3.2 through 3.4 install without
Rust; the source gem vendors the core and needs Rust at install time.

For development, install Rust from `rust-toolchain.toml`, Ruby 3.3 from
`.tool-versions`, uv, and CPython 3.12:

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
lints, compiles, and tests the Ruby extension, checks its RBS declarations
statically and at runtime, and installs the source and host platform gems into
an isolated gem home where both grade with no compiler on the subprocess
`PATH`. `make clean` removes build output.

See the [Python package guide](bindings/python/README.md) and the
[Ruby gem guide](bindings/ruby/README.md) for API examples, adapters,
supported platforms, source identity, and packaging procedures;
[CONTRIBUTING.md](CONTRIBUTING.md) for contributor rules; and
[CHANGELOG.md](CHANGELOG.md) for release notes.

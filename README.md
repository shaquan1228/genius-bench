# Genius Bench

Genius Bench grades an observed outcome against a task declaration. The Rust
core evaluates the task's criterion and file scope. Python, Ruby, and Go call
that core through native bindings.

## Grade the same task in four languages

The task asks, “Respond without editing files.” Its criterion checks for a
response, and its list of allowed files is empty. The candidate responds
with `true` and changes `extra.txt`. Every example prints the same two scores,
in order:

```text
correctness true
scope false
```

The response satisfies the presence predicate. The file edit fails the scope
check. Follow a language link to run the complete example.

| Language and example | Grade the outcome | Supported criteria | How to use it |
| --- | --- | --- | --- |
| [Rust core](crates/semantic-core/examples/grade_response.rs) | `task.grade(&outcome)` | State predicates, answer rubrics, code verifiers | Run `cargo run -p semantic-core --example grade_response` from the checkout. |
| [Python](bindings/python/examples/grade_response.py) | `task.grade(outcome=outcome)` | State predicates, answer rubrics, code verifiers | [Install a built wheel](bindings/python/README.md) and import `kernel`. |
| [Ruby](bindings/ruby/examples/grade_response.rb) | `task.grade(outcome: outcome)` | State predicates, answer rubrics, code verifiers | [Install a built gem](bindings/ruby/README.md) and require `genius_bench`. |
| [Go](bindings/go/examples/grade_response/main.go) | `task.Grade(outcome)` | State tasks with a presence predicate and boolean or null values | [Build from source with cgo](bindings/go/README.md). |

## What the core grades

A task holds an instruction, fixture files, a criterion, allowed files, and a
reference. Rust validates the declaration at construction. A harness collects
an outcome containing the candidate's response, changed files, verification
status, and observed facts. Grading performs no I/O. A verifier harness failure
returns a structured error so the caller can distinguish it from a failed score.

A public case contains the identity, prompt, and public files, including any
explicitly public tools. The task retains the reference answer, private checks,
and verifier input. The [Genius eval library](https://github.com/shaquan1228/genius)
owns execution, verification environments, and evidence collection. Its runtime
migration and package registry publication are separate work.

## Development

Install Rust from `rust-toolchain.toml`, Ruby from `.tool-versions`, Go 1.22 or
newer, a C compiler, uv, and CPython 3.12. From the repository root, run:

```sh
cargo fetch --locked
cargo fetch --locked --manifest-path bindings/go/native/Cargo.toml
cargo fetch --locked --manifest-path bindings/ruby/Cargo.toml
uv sync --project bindings/python --locked --no-install-project --python 3.12
bundle install --gemfile bindings/ruby/Gemfile
make check
make install-hooks
```

`make check` runs the four linked examples and checks their output. It also
tests the Rust core and Go binding, installs a Python wheel built from the
source distribution, and verifies both the Ruby source gem and host platform
gem. `make clean` removes build output.

See the [contributor rules](CONTRIBUTING.md) and [release notes](CHANGELOG.md).

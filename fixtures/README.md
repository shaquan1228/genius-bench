# Legacy semantic characterization

`semantics.json` contains 163 cases captured by executing the OCaml grader at
Genius revision `523d1eede610171d825fde0fee12d418dd1ef981`. Before the Rust
implementation, both original OCaml tests passed with
`make -C /Users/me/Repositories/genius/evals test-ocaml`. A Rust fixture replay
then failed on `record-order`: derived Rust equality returned false while the
captured OCaml result was true. The completed port replays the entire corpus
through public Rust APIs.

Reproduce the capture with Python 3 and OCaml 4.13 or later:

```sh
python3 scripts/capture_ocaml.py --source-repo /path/to/genius
python3 scripts/capture_ocaml.py --source-repo /path/to/genius --check
```

`--ocamlc /path/to/ocamlc` selects a compiler. The driver reads the seven semantic
modules with `git show` at the pinned revision, copies them and `export.ml` into
a temporary directory, compiles them with `ocamlc`, executes the exporter, and
writes its results. It does not execute the working tree or derive expected
results from Rust. The initial capture used OCaml 5.5.1 on 64-bit macOS.
The checked-in fixture file allows core tests and CI to run without OCaml.

Each case occupies one JSON line and has a category, identity, input, and captured expected result. Tagged
values retain null, bool, signed integer, float, string, list, and record as
separate alternatives. Integer and float payloads are strings, including `nan`,
`inf`, and `-inf`. Record fields are ordered name/value pairs, so duplicate keys
survive capture. JSON is a test transport; the runtime core has no JSON dependency.

Coverage includes equality and paths, all predicate alternatives, the complete
legacy whitespace set, nested assessments and ordered scores, valid declarations,
every validation rule and accumulated issues, public projections, code verification
and scope combinations, answer rubrics, state and trace grading, harness failures,
and attempts to override reserved outcome channels through open facts. Assessment
fixtures retain predicates, observations, observed leaf values, pass decisions,
and scores. Projection fixtures include public tool contents while omitting
private verifier files, commands, stdin, and reference material.

The port retains `Instruction.skill`, its validation, and `Case.skill` for this
baseline. Their removal belongs to the next dependent layer. `Experiment`,
`Profile`, and the default Genius profile are consumer settings; they are not
part of grading and are deliberately absent. The legacy Experiment assertions
still ran in the original tests, but no Experiment fixture or Rust API is added.

# Rust API

`Value`, `Predicate`, `Verification`, `Outcome`, and `Assessment` are owned domain
types. Records retain their input pairs; equality and lookup use the first value
for a duplicate name. `Value::get`, `Predicate::holds`, `Outcome::observation`, and
`Assessment::{new,passed,observed,scores}` expose the semantic operations directly.
Assessment construction expands named conjunctions recursively.

`Task::new(id, fixture, instruction, criterion, allowed_files, reference)` returns
`Result<Task, ValidationFailure>`. Validation returns every applicable issue in
legacy order with its field, message, and case identity. Tasks own their inputs
and expose immutable getters for each field. Input records (`File`, `Instruction`,
`Verifier`, `Rubric`, `TextRule`, `Reference`) use public owned fields; immutable
task borrows cannot mutate those stored records. `Criterion` has `Code`, `Answer`,
and `State` alternatives and compiles to a predicate with `condition()`.

`Task::project` copies only candidate material into `Case`.
`Task::{assess,grade}` return `Result` with a distinct `GradingFailure` for harness
faults. Required score names are `REQUIRED_SCORES`: correctness and scope, in order.
`Task::checks` gives the trusted harness immutable access to private verification
inputs. The crate performs no execution, filesystem access, networking, or SDK calls.

`version()`/`VERSION` and `source_hash()`/`SOURCE_HASH` identify the compiled policy.
The build script hashes the crate manifest, build script, and eight semantic Rust
sources with SHA-256 using length-delimited relative names and bytes. Source identity
is independent of the checkout location and is embedded at compilation. Tests prove
stability after relocation and invalidation for every declared input; runtime calls
never read source files. SHA-256 is a build/test dependency only.

# Checks and push hook

Install Rust 1.99.1 with rustfmt and clippy, and fetch locked development/build
dependencies once with `cargo fetch --locked`. Then run:

```sh
make check
make install-hooks
```

`make check` runs core tests, formatting, and clippy in sequence with Cargo offline
and locked dependency resolution. CI fetches dependencies before running the same
offline gate. No Genius Python tests or execution integrations are part of this gate.

`make install-hooks` sets this checkout's `core.hooksPath` to `.githooks`.
The executable pre-push hook runs the same checks before code pushes. Pushes
containing only Markdown or LICENSE changes skip the core gate. New branches inspect
commits not present in the selected remote's tracking refs; existing branches compare
the remote and local tips. Deleting a branch requires no core checks.
The hook does not disable or bypass a failing check.

Override tools with `make check CARGO=/path/to/cargo PYTHON=/path/to/python3`,
or set `CARGO` and `PYTHON` in the push environment. Direct invocation is
`python3 scripts/check.py --cargo /path/to/cargo`. Fixture capture is separate from
the regular gate and requires the original repository and OCaml compiler.

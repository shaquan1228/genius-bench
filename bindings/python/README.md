# Python package

The provisional distribution name is `genius-bench-kernel`; its import is
`kernel`. No registry publication is configured. Install a built artifact with
`python -m pip install path/to/genius_bench_kernel-*.whl`.

```python
from kernel import (
    Criterion,
    Instruction,
    Outcome,
    Reference,
    Rubric,
    Task,
    Value,
    Verification,
)

task = Task(
    id="answer",
    fixture=[],
    instruction=Instruction(prompt="What is six times seven?"),
    criterion=Criterion.answer(
        rubric=Rubric(
            expected=[("answer", Value.from_python(value=42))], text_rule=None
        )
    ),
    allowed_files=[],
    reference=Reference(edits=[], completion={"answer": 42}),
)
outcome = Outcome(
    changed_files=[],
    verification=Verification.unchecked(),
    response=Value.from_python(value={"answer": 42}),
    facts=[],
)
assessment = task.assess(outcome=outcome)
assert assessment.passed()
assert task.grade(outcome=outcome) == (("correctness", True), ("scope", True))
assert task.grade_reference(verification=Verification.unchecked()) == (
    ("correctness", True),
    ("scope", True),
)
```

`Criterion.code(verifier=...)` declares executable verification;
`Criterion.state(predicate=...)` grades custom state and trace observations.
The harness executes the verifier and supplies its typed `Verification` status.
`ValidationError` derives from `ValueError` and retains `case_id` and a tuple of
native `Issue` values with `field` and `message`. It preserves all applicable
validation issues. `GradingError` derives from `RuntimeError` and retains the
`verification_harness_error` failure name and original `detail`. Candidate
failure, invalid submission, timeout, and harness failure retain distinct
statuses. `REQUIRED_SCORES` comes from the core.

`Task` construction establishes the combined semantic invariants; nested
`Verifier`, `Rubric`, and `TextRule` objects are input declarations. The
path grammar accepts flat filenames matching `[A-Za-z_][A-Za-z0-9_.-]*`, with no
directories. Path validation issues retain the `fixture` field label across
fixture, allowed-file, and reference-edit paths. Walk `assessment.checks` to its
leaves and call `observed()` to inspect the values behind failed checks.

`Reference(completion=...)` accepts native Python values and converts them to
an immutable kernel `Value`; `Task.grade_reference(verification=...)` grades the
reference edits and completion after the harness supplies a verification status.
`Value.from_python(value=...)` is the explicit host-value boundary elsewhere. It accepts
null, booleans, signed i64 integers, floats, strings, lists, tuples, string-keyed
dictionaries, native values, and objects implementing `__kernel_value__()`.
`KernelValue` describes that protocol. Booleans, integers, and floats remain
distinct. Each dictionary, list, tuple, or `Record` field sequence snapshots its
members before converting children, so child hooks cannot change that traversal;
native list and tuple snapshots bypass overridden iterators. Integer overflow,
unsupported objects, cycles, and nesting beyond 256 conversion levels raise
exceptions. Native values retain the same limit of 256 nested lists or records,
and predicate composition allows 256 predicate levels along each path, without
limiting breadth. `Outcome` and `Task` construction reserve the levels added by
their observation and grading wrappers. Reusing native values and predicates,
including values exposed by assessments, cannot bypass these bounds. Non-finite
floats retain core equality semantics, including NaN being unequal to itself.

Use `Record(fields=[("name", first), ("name", second)])` to retain duplicate
field names. `Value.equal(other=...)` and `Value.get(path=...)` use the core's
first-field semantics; record equality ignores field order. A missing path
returns `None`; a present null returns a `Value` whose `kind` is `"null"`.
`from_dataclass(value=...)` explicitly adapts dataclass instances, including
nested dataclasses. `from_pydantic(value=...)` adapts Pydantic 2 models through
`model_dump(mode="python")`. Pydantic is optional and imported only when that
adapter is called. SDKs are not imported by the package.

The wheel includes `py.typed` and checked `__init__.pyi` declarations. Checked
stubs describe the recursive conversion protocol, native keyword constructors,
read-only properties, tuple snapshots, and structured error attributes without
adding a partially inferred generation layer. The package gate checks runtime
signature agreement with `mypy.stubtest`, valid typed client use, and rejected
positional calls, mutation, and missing attributes.

`version()` and `source_hash()` report the installed core's version and SHA-256
source identity, embedded at compilation. The package checks compare that hash
with the actual compiled source inputs independently of the installed extension.
Maturin normalizes the inherited license-file metadata in the source distribution,
so its exact source identity differs from a wheel built directly from the checkout.
The gate permits only that manifest edit and checks every core Rust source byte
against the checkout before verifying the installed source-distribution wheel.

The wheel matrix targets GIL-enabled CPython 3.12 and newer through the CPython
stable ABI (`cp312-abi3`). CI installs the same platform artifact on CPython
3.12, 3.13, and 3.14:

| Platform | Artifact target | Minimum operating system |
| --- | --- | --- |
| Linux x86_64 | manylinux_2_28 | glibc 2.28 |
| macOS arm64 | macosx_11_0_arm64 | macOS 11 |

Linux wheels build inside a manylinux container and undergo maturin's portability
checks. A native Ubuntu build alone is not a portable Linux artifact.
Free-threaded Python builds and other architectures are not supported or
verified by this matrix. Wheel installation and grading require neither Rust
nor OCaml. Building from the source distribution requires Rust and the pinned
maturin build dependency. Source distributions include the Rust workspace and
semantic core; they do not depend on the original Genius checkout.

For development, install Rust from `rust-toolchain.toml`, uv, and CPython 3.12,
then prepare the locked caches and run the gate:

```sh
cargo fetch --locked
uv sync --project bindings/python --locked --no-install-project --python 3.12
make check
make install-hooks
```

`make check` runs core tests, Rust formatting and workspace clippy, Python
formatting, and an offline wheel build from an extracted source distribution.
It installs that wheel into a fresh temporary environment and runs boundary
tests and type checks with `python -I`, no
`PYTHONPATH`, and a subprocess `PATH` containing no compiler or OCaml commands.
It also grades before installing optional or test dependencies. Documentation-only
pushes skip these checks; code pushes run the real pre-push gate.

Build local artifacts or verify another supported interpreter explicitly:

```sh
uv run --project bindings/python maturin build \
    --manifest-path bindings/python/Cargo.toml --release --locked --out dist
uv run --project bindings/python maturin sdist \
    --manifest-path bindings/python/Cargo.toml --out dist
uv run --project bindings/python python bindings/python/verify_wheel.py \
    --wheel dist/genius_bench_kernel-0.1.0-cp312-abi3-macosx_11_0_arm64.whl \
    --python 3.14
```


See the [repository overview](../../README.md) and
[contributor rules](../../CONTRIBUTING.md).

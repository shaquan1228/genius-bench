# Python binding

From the repository root, install a built wheel into your Python environment and
run the [shared grading example](examples/grade_response.py):

```sh
python -m pip install path/to/genius_bench_kernel-*.whl
python bindings/python/examples/grade_response.py
```

The package imports as `kernel`. Wheels target GIL-enabled CPython 3.12+ on
Linux x86_64 (glibc 2.28+) and macOS arm64 (macOS 11+), with install checks
for CPython 3.12–3.14. Wheel users need no Rust compiler. Source builds use Rust
and maturin.

## Grading a structured answer

For “What is six times seven?”, use this criterion with
`Reference(edits=[], completion={"answer": 42})`. The harness supplies a
dictionary in `Outcome.response`:

```python
from kernel import Criterion, Rubric

criterion = Criterion.Answer(rubric=Rubric(expected=[("answer", 42)], text_rule=None))
```

Value arguments accept Python scalars, lists, tuples, and string-keyed
dictionaries. Use `__kernel_value__()` for domain objects, `from_dataclass`
for dataclasses, or `from_pydantic` for Pydantic 2 models. A `Record` preserves
duplicate field names. The [installed-package test](tests/smoke.py) shows an
order adapter, pattern matching, and the distinction between missing and null.

`ValidationError` extends `ValueError` with `case_id` and `issues`.
`GradingError` extends `RuntimeError` and preserves the harness diagnostic.
The wheel includes [type declarations](python/kernel/__init__.pyi) and
`py.typed`. `kernel.version()` and `kernel.source_hash()` identify the installed core.

## Building a wheel

```sh
uv run --project bindings/python maturin build \
    --manifest-path bindings/python/Cargo.toml --release --locked --out dist
```

For the language comparison and the full build and verification commands, see
the [repository README](../../README.md).

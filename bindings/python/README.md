# Python binding

Install from PyPI with pip in your project's environment, then save and run the
[shared grading example](examples/grade_response.py).

```sh
python -m pip install genius-bench-kernel
python grade_response.py
```

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

The release wheels support CPython 3.12 through 3.14 on Linux x86_64 with
glibc 2.28 or newer and on macOS arm64. Installing from the source distribution
requires a Rust toolchain and C linker.

## Building a wheel

```sh
uv run --project bindings/python maturin build \
    --manifest-path bindings/python/Cargo.toml --release --locked --out dist
```

For the language comparison and the full build and verification commands, see
the [repository README](../../README.md).

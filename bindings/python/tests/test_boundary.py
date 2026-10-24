from dataclasses import dataclass
import importlib.util
from pathlib import Path
import struct
import sys

import pytest

import kernel as k


def state_task():
    return k.Task(
        id="state",
        fixture=[],
        instruction=k.Instruction(prompt="observe"),
        criterion=k.Criterion.state(
            predicate=k.Predicate.equals(
                path=["response", "answer"], expected=k.Value.from_python(value=42)
            )
        ),
        allowed_files=[],
        reference=k.Reference(edits=[], completion=k.Value.from_python(value=None)),
    )


def test_native_identity_and_packaged_types():
    assert k.Value.__module__ == "kernel._native"
    assert k.ValidationError.__module__ == "kernel._native"
    assert k.GradingError.__module__ == "kernel._native"
    assert importlib.util.find_spec("kernel._native").origin.endswith((".so", ".pyd"))
    from importlib.metadata import version

    assert k.version() == version("genius-bench-kernel") == "0.1.0"
    assert len(k.source_hash()) == 64
    assert k.REQUIRED_SCORES == ("correctness", "scope")
    package = Path(k.__file__).parent
    assert (package / "py.typed").is_file()
    assert (package / "__init__.pyi").is_file()
    assert "pydantic" not in sys.modules


def test_snapshots_and_projection():
    paths = ["a.txt"]
    fixture = [k.SourceFile(path="a.txt", contents="original")]
    tools = [k.SourceFile(path="tool", contents="public")]
    verifier = k.Verifier(
        inputs=paths,
        files=[k.SourceFile(path="private", contents="secret")],
        tools=tools,
        command=["check"],
        stdin="secret",
        timeout_seconds=1,
    )
    declaration = k.Task(
        id="code",
        fixture=fixture,
        instruction=k.Instruction(prompt="change"),
        criterion=k.Criterion.code(verifier=verifier),
        allowed_files=paths,
        reference=k.Reference(edits=[], completion=k.Value.from_python(value="secret")),
    )
    paths.append("other")
    fixture.clear()
    tools.clear()
    assert declaration.allowed_files == ("a.txt",)
    assert isinstance(declaration.fixture, tuple)
    projected = declaration.project()
    assert [file.path for file in projected.files] == ["a.txt", "tool"]
    assert not hasattr(projected, "skill")
    assert not hasattr(projected, "reference")
    assert not hasattr(projected, "checks")
    for item, name, value in [
        (declaration, "id", "changed"),
        (projected, "prompt", "changed"),
        (verifier, "stdin", "changed"),
        (fixture or declaration.fixture, "extra", 1),
        (declaration.fixture[0], "contents", "changed"),
    ]:
        with pytest.raises((AttributeError, TypeError)):
            setattr(item, name, value)


def test_values_protocol_records_and_failures():
    class Custom:
        def __kernel_value__(self):
            return {"answer": 42}

    class Cycle:
        def __kernel_value__(self):
            return self

    nested = [1]
    converted = k.Value.from_python(value={"values": nested, "null": None})
    nested.append(2)
    assert len(converted.get(path=["values"]).items) == 1
    assert converted.get(path=["missing"]) is None
    assert converted.get(path=["null"]).kind == "null"
    duplicate = k.Value.from_python(value=k.Record(fields=[("x", 1), ("x", 2)]))
    assert duplicate.get(path=["x"]).scalar == 1
    assert len(duplicate.fields) == 2
    assert duplicate.equal(other=k.Value.from_python(value={"x": 1}))
    assert not k.Value.from_python(value=True).equal(other=k.Value.from_python(value=1))
    assert not k.Value.from_python(value=1).equal(other=k.Value.from_python(value=1.0))
    assert k.Value.from_python(value=Custom()).get(path=["answer"]).scalar == 42
    for number in (-(2**63), -(2**53) - 1, 2**53 - 1, 2**53, 2**53 + 1, 2**63 - 1):
        scalar = k.Value.from_python(value=number).scalar
        assert type(scalar) is int
        assert scalar == number
    for bits in (
        0x3FB999999999999A,
        0x3FB999999999999B,
        0x4340000000000000,
        0x4340000000000001,
    ):
        number = struct.unpack(">d", bits.to_bytes(8, "big"))[0]
        scalar = k.Value.from_python(value=number).scalar
        assert type(scalar) is float
        assert struct.pack(">d", scalar) == bits.to_bytes(8, "big")
    for number in (-(2**63) - 1, 2**63):
        with pytest.raises(OverflowError):
            k.Value.from_python(value=number)
    recursive = []
    recursive.append(recursive)
    for invalid in (recursive, Cycle()):
        with pytest.raises(ValueError, match="[Cc]ycl"):
            k.Value.from_python(value=invalid)
    for unsupported in (object(), {1: "bad"}, {1, 2}):
        with pytest.raises(TypeError):
            k.Value.from_python(value=unsupported)
    with pytest.raises(TypeError):
        k.Value.from_python(1)


@pytest.mark.parametrize("container", ["dict-add", "dict-remove", "list", "record"])
def test_conversion_snapshots_before_callbacks(container):
    class Mutating:
        def __kernel_value__(self):
            if container == "dict-add":
                source["added"] = 3
            elif container == "dict-remove":
                source.pop("tail")
            elif container == "list":
                source.append(3)
            else:
                source.append(("added", 3))
                source[1] = ("replaced", 4)
            return 1

    if container.startswith("dict"):
        source = {"hook": Mutating(), "tail": 2}
        converted = k.Value.from_python(value=source)
    elif container == "list":
        source = [Mutating(), 2]
        converted = k.Value.from_python(value=source)
        assert [item.scalar for item in converted.items] == [1, 2]
        return
    else:
        source = [("hook", Mutating()), ("tail", 2)]
        converted = k.Value.from_python(value=k.Record(fields=source))
    assert [(name, item.scalar) for name, item in converted.fields] == [
        ("hook", 1),
        ("tail", 2),
    ]


def test_native_sequence_snapshots_ignore_overridden_iterators():
    class List(list):
        def __iter__(self):
            raise AssertionError("Overridden list iterator")

    class Tuple(tuple):
        def __iter__(self):
            raise AssertionError("Overridden tuple iterator")

    for source in (List([1, 2]), Tuple((1, 2))):
        assert [item.scalar for item in k.Value.from_python(value=source).items] == [
            1,
            2,
        ]
    assert k.Record(fields=List([("x", 1)])).fields[0][1].scalar == 1


def test_native_validation_and_harness_failures():
    with pytest.raises(k.ValidationError) as failure:
        k.Task(
            id="bad id",
            fixture=[k.SourceFile(path="../bad", contents="")],
            instruction=k.Instruction(prompt=""),
            criterion=k.Criterion.code(
                verifier=k.Verifier(
                    inputs=[],
                    files=[],
                    tools=[],
                    command=[],
                    stdin="",
                    timeout_seconds=0,
                )
            ),
            allowed_files=[],
            reference=k.Reference(edits=[], completion=k.Value.from_python(value=None)),
        )
    error = failure.value
    assert error.case_id == "bad id"
    assert isinstance(error.issues, tuple)
    assert len(error.issues) >= 5
    assert isinstance(error.issues[0], k.Issue)
    with pytest.raises(AttributeError):
        error.issues[0].field = "changed"
    declaration = state_task()
    for verification in (
        k.Verification.failed(),
        k.Verification.timed_out(),
        k.Verification.invalid_submission(),
    ):
        assert declaration.grade(
            outcome=k.Outcome(
                changed_files=[],
                verification=verification,
                response=k.Value.from_python(value=None),
                facts=[],
            )
        ) == (("correctness", False), ("scope", True))
    with pytest.raises(k.GradingError) as failure:
        declaration.assess(
            outcome=k.Outcome(
                changed_files=[],
                verification=k.Verification.harness_error(detail="worker missing"),
                response=k.Value.from_python(value=None),
                facts=[],
            )
        )
    assert failure.value.detail == "worker missing"
    assert failure.value.failure == "verification_harness_error"


def test_explicit_adapters():
    @dataclass
    class Answer:
        answer: int

    converted = k.from_dataclass(value=Answer(answer=42))
    assert converted.get(path=["answer"]).scalar == 42
    with pytest.raises(TypeError):
        k.from_dataclass(value={"answer": 42})
    pydantic = pytest.importorskip("pydantic")

    class AnswerModel(pydantic.BaseModel):
        answer: int

    assert k.from_pydantic(value=AnswerModel(answer=42)).equal(other=converted)
    with pytest.raises(TypeError):
        k.from_pydantic(value=Answer(answer=42))


def test_keyword_grading_and_reserved_facts():
    declaration = state_task()
    outcome = k.Outcome(
        changed_files=[],
        verification=k.Verification.unchecked(),
        response=k.Value.from_python(value={"answer": 42}),
        facts=[("response", k.Value.from_python(value=None))],
    )
    assert declaration.assess(outcome=outcome).passed()
    assert declaration.grade(outcome=outcome) == (
        ("correctness", True),
        ("scope", True),
    )
    with pytest.raises(TypeError):
        declaration.assess(outcome)
    with pytest.raises(TypeError):
        k.Instruction("prompt")


def test_compiler_free_source_identity():
    import os
    import shutil

    expected = os.environ.get("KERNEL_SOURCE_HASH")
    if expected:
        assert k.source_hash() == expected
    if os.environ.get("KERNEL_COMPILER_FREE"):
        assert all(
            shutil.which(command) is None
            for command in ("cargo", "rustc", "ocamlc", "ocaml", "cc", "clang", "gcc")
        )


def test_exception_base_contracts():
    assert issubclass(k.ValidationError, ValueError)
    assert issubclass(k.GradingError, RuntimeError)


def test_nested_adapters_and_bounded_conversion():
    @dataclass
    class Child:
        answer: int

    @dataclass
    class Parent:
        children: list[Child]

    converted = k.from_dataclass(value=Parent(children=[Child(answer=42)]))
    children = converted.get(path=["children"])
    assert children.items[0].get(path=["answer"]).scalar == 42
    recursive = Parent(children=[])
    recursive.children.append(recursive)
    with pytest.raises(ValueError, match="[Cc]ycl"):
        k.from_dataclass(value=recursive)
    shared = [1]
    assert len(k.Value.from_python(value=[shared, shared]).items) == 2
    nested = None
    for _ in range(300):
        nested = [nested]
    with pytest.raises(ValueError, match="nesting"):
        k.Value.from_python(value=nested)
    fields = [("x", [1])]
    record = k.Record(fields=fields)
    fields[0][1].append(2)
    assert isinstance(record.fields, tuple)
    assert len(record.fields[0][1].items) == 1


@pytest.mark.parametrize("container", ["value", "record"])
def test_bounded_native_value_composition(container):
    nested = k.Value.from_python(value=None)
    for _ in range(256):
        nested = (
            k.Value.from_python(value=[nested])
            if container == "value"
            else k.Record(fields=[("x", nested)])
        )
    with pytest.raises(ValueError, match="nesting"):
        if container == "value":
            k.Value.from_python(value=[nested])
        else:
            k.Record(fields=[("x", nested)])
    converted = k.Value.from_python(value=nested)
    assert converted.kind == ("list" if container == "value" else "record")


@pytest.mark.parametrize("operator", ["not_", "all", "any"])
def test_bounded_native_predicate_composition(operator):
    if operator != "not_":
        wide = getattr(k.Predicate, operator)(
            checks=[(str(index), k.Predicate.present(path=[])) for index in range(300)]
        )
        assert len(wide.checks) == 300
        assert wide.holds(observation=k.Value.from_python(value=1))
    predicate = k.Predicate.present(path=[])
    for _ in range(255):
        predicate = (
            k.Predicate.not_(check=predicate)
            if operator == "not_"
            else getattr(k.Predicate, operator)(checks=[("x", predicate)])
        )
    with pytest.raises(ValueError, match="nesting"):
        if operator == "not_":
            k.Predicate.not_(check=predicate)
        else:
            getattr(k.Predicate, operator)(checks=[("x", predicate)])
    assert k.Assessment(
        predicate=predicate, observation=k.Value.from_python(value=1)
    ).passed() is (operator != "not_")


def test_bounded_generated_observations_and_task_checks():
    nested = k.Value.from_python(value=None)
    for _ in range(255):
        nested = k.Value.from_python(value=[nested])
    outcome = k.Outcome(
        changed_files=[],
        verification=k.Verification.unchecked(),
        response=nested,
        facts=[("fact", nested)],
    )
    observation = outcome.observation()
    assessment = k.Assessment(
        predicate=k.Predicate.present(path=[]), observation=observation
    )
    for exposed in (observation, assessment.observation, assessment.observed()):
        with pytest.raises(ValueError, match="nesting"):
            k.Value.from_python(value=[exposed])
    for response, facts in ((observation, []), (nested, [("fact", observation)])):
        with pytest.raises(ValueError, match="nesting"):
            k.Outcome(
                changed_files=[],
                verification=k.Verification.unchecked(),
                response=response,
                facts=facts,
            )
    predicate = k.Predicate.present(path=[])
    for _ in range(255):
        predicate = k.Predicate.not_(check=predicate)
    with pytest.raises(ValueError, match="nesting"):
        k.Task(
            id="deep",
            fixture=[],
            instruction=k.Instruction(prompt="observe"),
            criterion=k.Criterion.state(predicate=predicate),
            allowed_files=[],
            reference=k.Reference(edits=[], completion=k.Value.from_python(value=None)),
        )

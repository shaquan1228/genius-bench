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
        criterion=k.Criterion.State(
            predicate=k.Predicate.Equals(
                path=["response", "answer"], expected=k.Value.from_python(value=42)
            )
        ),
        allowed_files=[],
        reference=k.Reference(edits=[], completion={"answer": 42}),
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
        criterion=k.Criterion.Code(verifier=verifier),
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
    assert isinstance(converted.get(path=["null"]), k.Value.Null)
    duplicate = k.Value.from_python(value=k.Record(fields=[("x", 1), ("x", 2)]))
    assert duplicate.get(path=["x"]).value == 1
    assert len(duplicate.fields) == 2
    assert duplicate.equal(other=k.Value.from_python(value={"x": 1}))
    assert not k.Value.from_python(value=True).equal(other=k.Value.from_python(value=1))
    assert not k.Value.from_python(value=1).equal(other=k.Value.from_python(value=1.0))
    assert k.Value.from_python(value=Custom()).get(path=["answer"]).value == 42
    for number in (-(2**63), -(2**53) - 1, 2**53 - 1, 2**53, 2**53 + 1, 2**63 - 1):
        scalar = k.Value.from_python(value=number).value
        assert type(scalar) is int
        assert scalar == number
    for bits in (
        0x3FB999999999999A,
        0x3FB999999999999B,
        0x4340000000000000,
        0x4340000000000001,
    ):
        number = struct.unpack(">d", bits.to_bytes(8, "big"))[0]
        scalar = k.Value.from_python(value=number).value
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
        k.Reference(edits=[], completion=object())
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
        assert [item.value for item in converted.items] == [1, 2]
        return
    else:
        source = [("hook", Mutating()), ("tail", 2)]
        converted = k.Value.from_python(value=k.Record(fields=source))
    assert [(name, item.value) for name, item in converted.fields] == [
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
        assert [item.value for item in k.Value.from_python(value=source).items] == [
            1,
            2,
        ]
    assert k.Record(fields=List([("x", 1)])).fields[0][1].value == 1


def test_native_validation_and_harness_failures():
    with pytest.raises(k.ValidationError) as failure:
        k.Task(
            id="bad id",
            fixture=[k.SourceFile(path="../bad", contents="")],
            instruction=k.Instruction(prompt=""),
            criterion=k.Criterion.Code(
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
        k.Verification.Failed(),
        k.Verification.TimedOut(),
        k.Verification.InvalidSubmission(),
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
                verification=k.Verification.HarnessError(detail="worker missing"),
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
    assert converted.get(path=["answer"]).value == 42
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
    assert declaration.reference.completion.get(path=["answer"]).value == 42
    assert declaration.grade_reference(verification=k.Verification.Unchecked()) == (
        ("correctness", True),
        ("scope", True),
    )
    outcome = k.Outcome(
        changed_files=[],
        verification=k.Verification.Unchecked(),
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
        declaration.grade_reference(k.Verification.Unchecked())
    with pytest.raises(TypeError):
        k.Instruction("prompt")


def test_reference_grading_uses_edits_and_verification():
    task = k.Task(
        id="reference",
        fixture=[k.SourceFile(path="solution.py", contents="original")],
        instruction=k.Instruction(prompt="update"),
        criterion=k.Criterion.State(
            predicate=k.Predicate.All(
                checks=[
                    (
                        "verification",
                        k.Predicate.Equals(
                            path=["verification"],
                            expected=k.Value.from_python(value="passed"),
                        ),
                    ),
                    (
                        "edits",
                        k.Predicate.Equals(
                            path=["changed_files"],
                            expected=k.Value.from_python(value=["solution.py"]),
                        ),
                    ),
                ]
            )
        ),
        allowed_files=["solution.py"],
        reference=k.Reference(
            edits=[k.SourceFile(path="solution.py", contents="updated")],
            completion=None,
        ),
    )
    assert task.grade_reference(verification=k.Verification.Passed()) == (
        ("correctness", True),
        ("scope", True),
    )
    assert task.grade_reference(verification=k.Verification.Failed()) == (
        ("correctness", False),
        ("scope", True),
    )
    with pytest.raises(k.GradingError, match="worker missing"):
        task.grade_reference(
            verification=k.Verification.HarnessError(detail="worker missing")
        )


def test_value_parameters_accept_native_values():
    native = {"answer": 42, "items": [1, "x"]}
    value = k.Value.from_python(value=native)
    for given in (native, value):
        assert k.Predicate.Equals(path=[], expected=given).expected.equal(other=value)
        assert k.Predicate.Contains(path=[], item=given).item.equal(other=value)
        assert k.Predicate.Within(path=[], allowed=[given])
        assert (
            k.Rubric(expected=[("answer", given)], text_rule=None)
            .expected[0][1]
            .equal(other=value)
        )
        outcome = k.Outcome(
            changed_files=[],
            verification=k.Verification.Unchecked(),
            response=given,
            facts=[("state", given)],
        )
        assert outcome.response.equal(other=value)
        assert outcome.facts[0][1].equal(other=value)
        present = k.Predicate.Present(path=["answer"])
        assert present.holds(observation=given)
        assert k.Assessment(predicate=present, observation=given).passed()
        assert value.equal(other=given)
    allowed = k.Predicate.Within(path=[], allowed=[1, "x", None]).allowed
    assert [type(item) for item in allowed] == [
        k.Value.Int,
        k.Value.String,
        k.Value.Null,
    ]
    assert [item.value for item in allowed[:2]] == [1, "x"]
    present = k.Predicate.Present(path=[])
    for build in (
        lambda bad: k.Predicate.Equals(path=[], expected=bad),
        lambda bad: k.Predicate.Within(path=[], allowed=[bad]),
        lambda bad: k.Predicate.Contains(path=[], item=bad),
        lambda bad: present.holds(observation=bad),
        lambda bad: k.Rubric(expected=[("answer", bad)], text_rule=None),
        lambda bad: k.Outcome(
            changed_files=[],
            verification=k.Verification.Unchecked(),
            response=bad,
            facts=[],
        ),
        lambda bad: k.Outcome(
            changed_files=[],
            verification=k.Verification.Unchecked(),
            response=None,
            facts=[("state", bad)],
        ),
        lambda bad: k.Assessment(predicate=present, observation=bad),
        lambda bad: k.Value.from_python(value=None).equal(other=bad),
    ):
        with pytest.raises(TypeError):
            build(object())
        with pytest.raises(OverflowError):
            build(2**64)


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
    assert children.items[0].get(path=["answer"]).value == 42
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
    assert isinstance(
        converted, k.Value.List if container == "value" else k.Value.Record
    )


@pytest.mark.parametrize("operator", ["Not", "All", "Any"])
def test_bounded_native_predicate_composition(operator):
    if operator != "Not":
        wide = getattr(k.Predicate, operator)(
            checks=[(str(index), k.Predicate.Present(path=[])) for index in range(300)]
        )
        assert len(wide.checks) == 300
        assert wide.holds(observation=k.Value.from_python(value=1))
    predicate = k.Predicate.Present(path=[])
    for _ in range(255):
        predicate = (
            k.Predicate.Not(check=predicate)
            if operator == "Not"
            else getattr(k.Predicate, operator)(checks=[("x", predicate)])
        )
    with pytest.raises(ValueError, match="nesting"):
        if operator == "Not":
            k.Predicate.Not(check=predicate)
        else:
            getattr(k.Predicate, operator)(checks=[("x", predicate)])
    assert k.Assessment(
        predicate=predicate, observation=k.Value.from_python(value=1)
    ).passed() is (operator != "Not")


def test_bounded_generated_observations_and_task_checks():
    nested = k.Value.from_python(value=None)
    for _ in range(255):
        nested = k.Value.from_python(value=[nested])
    outcome = k.Outcome(
        changed_files=[],
        verification=k.Verification.Unchecked(),
        response=nested,
        facts=[("fact", nested)],
    )
    observation = outcome.observation()
    assessment = k.Assessment(
        predicate=k.Predicate.Present(path=[]), observation=observation
    )
    for exposed in (observation, assessment.observation, assessment.observed()):
        with pytest.raises(ValueError, match="nesting"):
            k.Value.from_python(value=[exposed])
    for response, facts in ((observation, []), (nested, [("fact", observation)])):
        with pytest.raises(ValueError, match="nesting"):
            k.Outcome(
                changed_files=[],
                verification=k.Verification.Unchecked(),
                response=response,
                facts=facts,
            )
    reference_task = k.Task(
        id="deep-reference",
        fixture=[],
        instruction=k.Instruction(prompt="observe"),
        criterion=k.Criterion.State(predicate=k.Predicate.Present(path=[])),
        allowed_files=[],
        reference=k.Reference(edits=[], completion=observation),
    )
    with pytest.raises(ValueError, match="nesting"):
        reference_task.grade_reference(verification=k.Verification.Unchecked())
    predicate = k.Predicate.Present(path=[])
    for _ in range(255):
        predicate = k.Predicate.Not(check=predicate)
    with pytest.raises(ValueError, match="nesting"):
        k.Task(
            id="deep",
            fixture=[],
            instruction=k.Instruction(prompt="observe"),
            criterion=k.Criterion.State(predicate=predicate),
            allowed_files=[],
            reference=k.Reference(edits=[], completion=k.Value.from_python(value=None)),
        )


def variants(base):
    return [
        member
        for member in vars(base).values()
        if isinstance(member, type) and issubclass(member, base)
    ]


def test_sum_types_expose_closed_frozen_variants():
    expected = {
        k.Value: {"Null", "Bool", "Int", "Float", "String", "List", "Record"},
        k.Predicate: {
            "Present",
            "Equals",
            "Within",
            "Contains",
            "Words",
            "All",
            "Any",
            "Not",
        },
        k.Verification: {
            "Unchecked",
            "Passed",
            "Failed",
            "InvalidSubmission",
            "TimedOut",
            "HarnessError",
        },
        k.Assessment: {"Check", "All"},
        k.Criterion: {"Code", "Answer", "State"},
    }
    for base, names in expected.items():
        members = variants(base)
        assert {member.__name__ for member in members} == names
        for member in members:
            assert member.__module__ == "kernel._native"
            assert member.__qualname__ == f"{base.__name__}.{member.__name__}"
            with pytest.raises(TypeError):
                type("Extended", (member,), {})
        with pytest.raises(TypeError):
            base()
        for removed in ("kind", "scalar", "detail", "path", "checks", "predicate"):
            assert not hasattr(base, removed)
    with pytest.raises(AttributeError):
        k.Verification.HarnessError(detail="x").detail = "changed"
    with pytest.raises(TypeError):
        k.Verification.HarnessError("positional")
    with pytest.raises(TypeError):
        k.Predicate.Equals(["path"], 1)


def test_value_variants_narrow_by_isinstance_and_match():
    converted = k.Value.from_python(
        value={"n": None, "b": True, "i": 1, "f": 1.5, "s": "x", "l": [1], "r": {}}
    )
    assert isinstance(converted, k.Value.Record)
    seen = {}
    for name, item in converted.fields:
        assert isinstance(item, k.Value)
        match item:
            case k.Value.Null():
                seen[name] = None
            case k.Value.Bool(flag):
                seen[name] = flag
            case k.Value.Int(number):
                seen[name] = number
            case k.Value.Float(number):
                seen[name] = number
            case k.Value.String(text):
                seen[name] = text
            case k.Value.List(items):
                seen[name] = [entry.value for entry in items]
            case k.Value.Record(fields):
                seen[name] = dict(fields)
    assert seen == {
        "n": None,
        "b": True,
        "i": 1,
        "f": 1.5,
        "s": "x",
        "l": [1],
        "r": {},
    }
    built = k.Value.Record(
        fields=[
            ("list", k.Value.List(items=[k.Value.Int(value=1), 2])),
            ("text", k.Value.String(value="x")),
            ("null", k.Value.Null()),
            ("flag", k.Value.Bool(value=False)),
            ("ratio", k.Value.Float(value=0.5)),
        ]
    )
    assert built.equal(
        other={"list": [1, 2], "text": "x", "null": None, "flag": False, "ratio": 0.5}
    )
    assert isinstance(k.Record(fields=[("x", 1)]).__kernel_value__(), k.Value.Record)
    with pytest.raises(ValueError, match="nesting"):
        nested = k.Value.Null()
        for _ in range(257):
            nested = k.Value.List(items=[nested])


def test_predicate_variants_narrow_by_isinstance_and_match():
    leaf = k.Predicate.Present(path=["a"])
    built = [
        leaf,
        k.Predicate.Equals(path=["a"], expected={"x": 1}),
        k.Predicate.Within(path=["a"], allowed=[1, "x"]),
        k.Predicate.Contains(path=["a"], item=1),
        k.Predicate.Words(path=["a"], min_words=1, max_words=2),
        k.Predicate.All(checks=[("leaf", leaf)]),
        k.Predicate.Any(checks=[("leaf", leaf)]),
        k.Predicate.Not(check=leaf),
    ]
    summaries = []
    for predicate in built:
        assert isinstance(predicate, k.Predicate)
        match predicate:
            case k.Predicate.Present(path):
                summaries.append(("present", path))
            case k.Predicate.Equals(path, expected):
                summaries.append(("equals", path, expected.get(path=["x"]).value))
            case k.Predicate.Within(path, allowed):
                summaries.append(("within", path, len(allowed)))
            case k.Predicate.Contains(path, item):
                summaries.append(("contains", path, item.value))
            case k.Predicate.Words(path, low, high):
                summaries.append(("words", path, low, high))
            case k.Predicate.All(checks):
                summaries.append(("all", [name for name, _ in checks]))
            case k.Predicate.Any(checks):
                summaries.append(("any", [name for name, _ in checks]))
            case k.Predicate.Not(check):
                summaries.append(("not", type(check)))
    assert summaries == [
        ("present", ("a",)),
        ("equals", ("a",), 1),
        ("within", ("a",), 2),
        ("contains", ("a",), 1),
        ("words", ("a",), 1, 2),
        ("all", ["leaf"]),
        ("any", ["leaf"]),
        ("not", k.Predicate.Present),
    ]
    assert isinstance(built[5].checks[0][1], k.Predicate.Present)


def test_verification_variants_narrow_and_round_trip():
    built = [
        k.Verification.Unchecked(),
        k.Verification.Passed(),
        k.Verification.Failed(),
        k.Verification.InvalidSubmission(),
        k.Verification.TimedOut(),
        k.Verification.HarnessError(detail="worker missing"),
    ]
    assert [verification.status for verification in built] == [
        "unchecked",
        "passed",
        "failed",
        "invalid_submission",
        "timed_out",
        "harness_error",
    ]
    for verification in built:
        returned = k.Outcome(
            changed_files=[], verification=verification, response=None, facts=[]
        ).verification
        assert type(returned) is type(verification)
        match returned:
            case k.Verification.HarnessError(detail):
                assert detail == "worker missing"
            case k.Verification.Passed() | k.Verification.Unchecked():
                assert returned.status in {"passed", "unchecked"}
            case _:
                assert not isinstance(returned, k.Verification.HarnessError)
        assert not hasattr(k.Verification.Passed(), "detail")


def test_assessment_and_criterion_variants_narrow():
    check = k.Predicate.Equals(path=["answer"], expected=42)
    single = k.Assessment(predicate=check, observation={"answer": 42})
    match single:
        case k.Assessment.Check(predicate, observation):
            assert isinstance(predicate, k.Predicate.Equals)
            assert observation.get(path=["answer"]).value == 42
        case _:
            pytest.fail("Expected a single check")
    combined = k.Assessment(
        predicate=k.Predicate.All(checks=[("answer", check)]),
        observation={"answer": 41},
    )
    assert isinstance(combined, k.Assessment.All)
    assert not hasattr(combined, "predicate")
    name, nested = combined.checks[0]
    assert (name, type(nested), combined.passed()) == (
        "answer",
        k.Assessment.Check,
        False,
    )
    rebuilt = k.Assessment.All(
        checks=[("answer", k.Assessment.Check(predicate=check, observation=42))]
    )
    assert rebuilt.scores() == (("answer", False),)
    verifier = k.Verifier(
        inputs=["a.txt"],
        files=[],
        tools=[],
        command=["true"],
        stdin="",
        timeout_seconds=1,
    )
    rubric = k.Rubric(expected=[("answer", 42)], text_rule=None)
    for criterion in (
        k.Criterion.Code(verifier=verifier),
        k.Criterion.Answer(rubric=rubric),
        k.Criterion.State(predicate=check),
    ):
        declared = k.Task(
            id="variant",
            fixture=[k.SourceFile(path="a.txt", contents="")],
            instruction=k.Instruction(prompt="observe"),
            criterion=criterion,
            allowed_files=["a.txt"],
            reference=k.Reference(edits=[], completion={"answer": 42}),
        ).criterion
        assert type(declared) is type(criterion)
        match declared:
            case k.Criterion.Code(verifier):
                assert verifier.command == ("true",)
            case k.Criterion.Answer(rubric):
                assert rubric.expected[0][0] == "answer"
            case k.Criterion.State(predicate):
                assert isinstance(predicate, k.Predicate.Equals)
        assert isinstance(declared.condition(), k.Predicate)

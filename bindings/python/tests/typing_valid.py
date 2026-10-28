from dataclasses import dataclass
from typing import Literal, assert_type

from kernel import (
    Assessment,
    Case,
    Criterion,
    SourceFile,
    Instruction,
    KernelValue,
    Outcome,
    Predicate,
    Record,
    Reference,
    Rubric,
    Task,
    TextRule,
    Value,
    Verification,
    from_dataclass,
    from_pydantic,
    REQUIRED_SCORES,
    source_hash,
    version,
)


@dataclass
class Answer:
    answer: int


class Custom:
    def __kernel_value__(self) -> object:
        return {"answer": 42}


custom: KernelValue = Custom()
value: Value = Value.from_python(value=custom)
record: Record = Record(fields=[("answer", 42), ("answer", 0)])
record_value: Value = record.__kernel_value__()
text_rule = TextRule(field="reason", min_words=1, max_words=5)
rubric = Rubric(expected=[("answer", value)], text_rule=text_rule)
criterion: Criterion = Criterion.Answer(rubric=rubric)
files: tuple[SourceFile, ...] = (SourceFile(path="input.txt", contents="data"),)
task = Task(
    id="answer",
    fixture=files,
    instruction=Instruction(prompt="Answer"),
    criterion=criterion,
    allowed_files=[],
    reference=Reference(edits=[], completion=None),
)
outcome = Outcome(
    changed_files=[],
    verification=Verification.Unchecked(),
    response=from_dataclass(value=Answer(answer=42)),
    facts=[],
)
assessment: Assessment = task.assess(outcome=outcome)
scores: tuple[tuple[str, bool], ...] = task.grade(outcome=outcome)
reference_scores: tuple[tuple[str, bool], ...] = task.grade_reference(
    verification=Verification.Unchecked()
)
case: Case = task.project()
source: tuple[str, str] = (version(), source_hash())
required: tuple[str, str] = REQUIRED_SCORES
present: Predicate = Predicate.Present(path=["response"])
passed: bool = present.holds(observation=outcome.observation())
native_rubric = Rubric(expected=[("answer", 42)], text_rule=None)
native_outcome = Outcome(
    changed_files=[],
    verification=Verification.Unchecked(),
    response={"answer": 42},
    facts=[("state", {"balance": 5})],
)
native_checks: tuple[Predicate, ...] = (
    Predicate.Equals(path=["response", "answer"], expected=42),
    Predicate.Within(path=["response", "answer"], allowed=[41, 42]),
    Predicate.Contains(path=["response", "items"], item="x"),
)
native_assessment = Assessment(predicate=present, observation={"response": None})
native_holds: bool = present.holds(observation={"response": None})
native_equal: bool = value.equal(other={"answer": 42})
pydantic_value: Value = from_pydantic(value=object())


def describe_value(data: Value) -> object:
    match data:
        case Value.Null():
            return None
        case Value.Bool(flag):
            return assert_type(flag, bool)
        case Value.Int(number):
            return assert_type(number, int)
        case Value.Float(ratio):
            return assert_type(ratio, float)
        case Value.String(text):
            return assert_type(text, str)
        case Value.List(items):
            return [
                describe_value(item) for item in assert_type(items, tuple[Value, ...])
            ]
        case Value.Record(fields):
            return {name: describe_value(item) for name, item in fields}
    return data


def describe_predicate(check: Predicate) -> object:
    match check:
        case Predicate.Present(path):
            return assert_type(path, tuple[str, ...])
        case Predicate.Equals(path, expected):
            return assert_type(expected, Value)
        case Predicate.Within(path, allowed):
            return assert_type(allowed, tuple[Value, ...])
        case Predicate.Contains(path, item):
            return assert_type(item, Value)
        case Predicate.Words(path, low, high):
            return assert_type(low, int) + assert_type(high, int)
        case Predicate.All(checks) | Predicate.Any(checks):
            return assert_type(checks, tuple[tuple[str, Predicate], ...])
        case Predicate.Not(inner):
            return describe_predicate(inner)
    return check


def describe_verification(verification: Verification) -> str:
    if isinstance(verification, Verification.HarnessError):
        assert_type(verification.status, Literal["harness_error"])
        return assert_type(verification.detail, str)
    match verification:
        case Verification.Passed() | Verification.Unchecked():
            return verification.status
        case Verification.Failed() | Verification.InvalidSubmission():
            return verification.status
        case Verification.TimedOut():
            return assert_type(verification.status, Literal["timed_out"])
    return verification.status


def describe_assessment(result: Assessment) -> object:
    if isinstance(result, Assessment.All):
        return [
            describe_assessment(check)
            for _, check in assert_type(
                result.checks, tuple[tuple[str, Assessment], ...]
            )
        ]
    if isinstance(result, Assessment.Check):
        return (
            assert_type(result.predicate, Predicate),
            assert_type(result.observation, Value),
        )
    return result.passed()


def describe_criterion(declared: Criterion) -> object:
    match declared:
        case Criterion.Code(verifier):
            return verifier.command
        case Criterion.Answer(rubric):
            return rubric.expected
        case Criterion.State(predicate):
            return describe_predicate(predicate)
    return declared.condition()


harness: Verification.HarnessError = Verification.HarnessError(detail="worker missing")
built_value: Value.List = Value.List(items=[Value.Int(value=1), 2, Value.Null()])
built_record: Value.Record = record.__kernel_value__()
words = Predicate.Words(path=["reason"], min_words=1, max_words=5)
negated: Predicate.Not = Predicate.Not(check=Predicate.Any(checks=[("words", words)]))
checked: Assessment.Check = Assessment.Check(predicate=words, observation=None)
described = (
    describe_value(built_value),
    describe_predicate(negated),
    describe_verification(outcome.verification),
    describe_assessment(assessment),
    describe_criterion(task.criterion),
    describe_assessment(Assessment.All(checks=[("words", checked)])),
    Criterion.State(predicate=present),
)

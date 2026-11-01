from dataclasses import dataclass
from typing import Literal, assert_never, assert_type

from kernel import (
    Assessment,
    AssessmentVariant,
    Case,
    Criterion,
    CriterionVariant,
    SourceFile,
    Instruction,
    KernelValue,
    NESTING_LIMIT,
    Outcome,
    PlainValue,
    Predicate,
    PredicateVariant,
    Record,
    Reference,
    Rubric,
    Task,
    TextRule,
    Value,
    ValueVariant,
    Verification,
    VerificationVariant,
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
assessment: AssessmentVariant = task.assess(outcome=outcome)
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


def describe_value(data: ValueVariant) -> object:
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
                describe_value(item)
                for item in assert_type(items, tuple[ValueVariant, ...])
            ]
        case Value.Record(fields):
            return {name: describe_value(item) for name, item in fields}
        case _:
            assert_never(data)


def describe_predicate(check: PredicateVariant) -> object:
    match check:
        case Predicate.Present(path):
            return assert_type(path, tuple[str, ...])
        case Predicate.Equals(path, expected):
            return assert_type(expected, ValueVariant)
        case Predicate.Within(path, allowed):
            return assert_type(allowed, tuple[ValueVariant, ...])
        case Predicate.Contains(path, item):
            return assert_type(item, ValueVariant)
        case Predicate.Words(path, low, high):
            return assert_type(low, int) + assert_type(high, int)
        case Predicate.All(checks) | Predicate.Any(checks):
            return assert_type(checks, tuple[tuple[str, PredicateVariant], ...])
        case Predicate.Not(inner):
            return describe_predicate(inner)
        case _:
            assert_never(check)


def describe_verification(verification: VerificationVariant) -> str:
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
        case _:
            assert_never(verification)


def describe_assessment(result: AssessmentVariant) -> object:
    match result:
        case Assessment.All(checks):
            return [
                describe_assessment(check)
                for _, check in assert_type(
                    checks, tuple[tuple[str, AssessmentVariant], ...]
                )
            ]
        case Assessment.Check(predicate, observation):
            return (
                assert_type(predicate, PredicateVariant),
                assert_type(observation, ValueVariant),
            )
        case _:
            assert_never(result)


def describe_criterion(declared: CriterionVariant) -> object:
    match declared:
        case Criterion.Code(verifier):
            return verifier.command
        case Criterion.Answer(rubric):
            return rubric.expected
        case Criterion.State(predicate):
            return describe_predicate(predicate)
        case _:
            assert_never(declared)


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
plain_task: dict[str, PlainValue] = task.to_python()
plain_value: PlainValue = value.to_python()
plain_parts: tuple[dict[str, PlainValue], ...] = (
    record.to_python(),
    present.to_python(),
    outcome.verification.to_python(),
    outcome.to_python(),
    assessment.to_python(),
    task.criterion.to_python(),
    rubric.to_python(),
    text_rule.to_python(),
    files[0].to_python(),
    task.instruction.to_python(),
    task.reference.to_python(),
    case.to_python(),
)
representable: bool = Value.representable(value={"answer": 2**63})
nesting: int = NESTING_LIMIT
same: bool = task == task and hash(present) == hash(present)

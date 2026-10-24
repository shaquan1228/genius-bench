from dataclasses import dataclass
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
criterion: Criterion = Criterion.answer(rubric=rubric)
files: tuple[SourceFile, ...] = (SourceFile(path="input.txt", contents="data"),)
task = Task(
    id="answer",
    fixture=files,
    instruction=Instruction(prompt="Answer"),
    criterion=criterion,
    allowed_files=[],
    reference=Reference(edits=[], completion=Value.from_python(value=None)),
)
outcome = Outcome(
    changed_files=[],
    verification=Verification.unchecked(),
    response=from_dataclass(value=Answer(answer=42)),
    facts=[],
)
assessment: Assessment = task.assess(outcome=outcome)
scores: tuple[tuple[str, bool], ...] = task.grade(outcome=outcome)
case: Case = task.project()
source: tuple[str, str] = (version(), source_hash())
required: tuple[str, str] = REQUIRED_SCORES
present: Predicate = Predicate.present(path=["response"])
passed: bool = present.holds(observation=outcome.observation())
pydantic_value: Value = from_pydantic(value=object())

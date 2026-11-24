from kernel import (
    Criterion,
    Instruction,
    Outcome,
    Predicate,
    Reference,
    Task,
    Verification,
)


task = Task(
    id="response-only",
    fixture=[],
    instruction=Instruction(prompt="Respond without editing files."),
    criterion=Criterion.State(predicate=Predicate.Present(path=["response"])),
    allowed_files=[],
    reference=Reference(edits=[], completion=True),
)
outcome = Outcome(
    changed_files=["extra.txt"],
    verification=Verification.Unchecked(),
    response=True,
    facts=[],
)
for name, passed in task.grade(outcome=outcome):
    print(name, str(passed).lower())

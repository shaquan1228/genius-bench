from kernel import Instruction, Outcome, Task, Value, Verification


def invalid(task: Task, outcome: Outcome) -> None:
    task.assess(outcome)
    task.id = "changed"
    Value.Int(value=1).missing
    Instruction("positional")
    Verification.Passed().detail

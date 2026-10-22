from kernel import Instruction, Outcome, Task, Value


def invalid(task: Task, outcome: Outcome) -> None:
    task.assess(outcome)
    task.id = "changed"
    Value.from_python(value=1).missing
    Instruction("positional")

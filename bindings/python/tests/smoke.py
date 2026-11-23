import importlib.util
import os
import sys
from dataclasses import dataclass

import kernel as k


@dataclass(frozen=True)
class Order:
    total_cents: int

    def __kernel_value__(self) -> dict[str, int | None]:
        return {"total_cents": self.total_cents, "note": None}


def verify_order_adapter() -> None:
    order: k.KernelValue = Order(total_cents=4200)
    check = k.Predicate.Equals(path=["total_cents"], expected=4200)
    assert check.holds(observation=order)
    assert not check.holds(observation=Order(total_cents=4199))
    assert not check.holds(observation={"total_cents": 4200.0})
    match check:
        case k.Predicate.Equals(path=path, expected=k.Value.Int(value=cents)):
            assert (path, cents) == (("total_cents",), 4200)
        case _:
            raise AssertionError("expected an integer equality predicate")

    value = k.Value.from_python(value=order)
    assert value.get(path=["missing"]) is None
    assert isinstance(value.get(path=["note"]), k.Value.Null)

    original = k.Record(fields=[("total_cents", 4200), ("note", None)])
    reordered = k.Record(
        fields=[("note", None), ("total_cents", 4200), ("total_cents", 4199)]
    )
    assert original == reordered
    assert hash(original) == hash(reordered)
    assert {original: "first", reordered: "second"} == {original: "second"}


assert importlib.util.find_spec("pydantic") is None
assert k.source_hash() == os.environ["KERNEL_SOURCE_HASH"]
verify_order_adapter()
task = k.Task(
    id="installed",
    fixture=[],
    instruction=k.Instruction(prompt="observe"),
    criterion=k.Criterion.State(predicate=k.Predicate.Present(path=["response"])),
    allowed_files=[],
    reference=k.Reference(edits=[], completion=True),
)
outcome = k.Outcome(
    changed_files=[],
    verification=k.Verification.Unchecked(),
    response=k.Value.from_python(value=True),
    facts=[],
)
assert task.grade(outcome=outcome) == (("correctness", True), ("scope", True))
assert task.grade_reference(verification=k.Verification.Unchecked()) == (
    ("correctness", True),
    ("scope", True),
)
print(sys.version.split()[0], k.__file__, k.version(), k.source_hash())

import importlib.util
import os
import sys

import kernel as k


assert importlib.util.find_spec("pydantic") is None
assert k.source_hash() == os.environ["KERNEL_SOURCE_HASH"]
task = k.Task(
    id="installed",
    fixture=[],
    instruction=k.Instruction(prompt="observe"),
    criterion=k.Criterion.state(predicate=k.Predicate.present(path=["response"])),
    allowed_files=[],
    reference=k.Reference(edits=[], completion=True),
)
outcome = k.Outcome(
    changed_files=[],
    verification=k.Verification.unchecked(),
    response=k.Value.from_python(value=True),
    facts=[],
)
assert task.grade(outcome=outcome) == (("correctness", True), ("scope", True))
assert task.grade_reference(verification=k.Verification.unchecked()) == (
    ("correctness", True),
    ("scope", True),
)
print(sys.version.split()[0], k.__file__, k.version(), k.source_hash())

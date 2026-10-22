import copy
import json
import math
import os
from pathlib import Path

import pytest

import kernel as k


CORPUS = json.loads(
    Path(
        os.environ.get(
            "KERNEL_FIXTURES",
            Path(__file__).resolve().parents[3] / "fixtures/semantics.json",
        )
    ).read_text()
)
CASES = CORPUS["cases"]


def value(data):
    kind = data["type"]
    if kind == "record":
        plain = k.Record(fields=[(name, value(item)) for name, item in data["fields"]])
    elif kind == "list":
        plain = [value(item) for item in data["items"]]
    elif kind == "int":
        plain = int(data["value"])
    elif kind == "float":
        plain = float(data["value"])
    elif kind == "null":
        plain = None
    else:
        plain = data["value"]
    return k.Value.from_python(value=plain)


def render_value(item):
    if item is None:
        return None
    kind = item.kind
    result = {"type": kind}
    if kind == "record":
        result["fields"] = [[name, render_value(value)] for name, value in item.fields]
    elif kind == "list":
        result["items"] = [render_value(value) for value in item.items]
    elif kind == "int":
        result["value"] = str(item.scalar)
    elif kind == "float":
        scalar = item.scalar
        result["value"] = "nan" if math.isnan(scalar) else format(scalar, ".15g")
    elif kind != "null":
        result["value"] = item.scalar
    return result


def predicate(data):
    kind = data["type"]
    if kind == "present":
        return k.Predicate.present(path=data["path"])
    if kind == "equals":
        return k.Predicate.equals(path=data["path"], expected=value(data["expected"]))
    if kind == "within":
        return k.Predicate.within(
            path=data["path"], allowed=[value(item) for item in data["allowed"]]
        )
    if kind == "contains":
        return k.Predicate.contains(path=data["path"], item=value(data["item"]))
    if kind == "words":
        return k.Predicate.words(
            path=data["path"], min_words=data["min_words"], max_words=data["max_words"]
        )
    if kind in ("all", "any"):
        checks = [(name, predicate(item)) for name, item in data["checks"]]
        return (
            k.Predicate.all(checks=checks)
            if kind == "all"
            else k.Predicate.any(checks=checks)
        )
    return k.Predicate.not_(check=predicate(data["check"]))


def render_predicate(item):
    result = {"type": item.kind}
    if item.kind in ("all", "any"):
        result["checks"] = [
            [name, render_predicate(check)] for name, check in item.checks
        ]
    elif item.kind == "not":
        result["check"] = render_predicate(item.check)
    else:
        result["path"] = list(item.path)
        if item.kind == "equals":
            result["expected"] = render_value(item.expected)
        elif item.kind == "contains":
            result["item"] = render_value(item.item)
        elif item.kind == "within":
            result["allowed"] = [render_value(value) for value in item.allowed]
        elif item.kind == "words":
            result.update(min_words=item.min_words, max_words=item.max_words)
    return result


def render_assessment(item):
    if item.kind == "all":
        detail = {
            "type": "all",
            "checks": [[name, render_assessment(check)] for name, check in item.checks],
        }
    else:
        detail = {
            "type": "check",
            "predicate": render_predicate(item.predicate),
            "observation": render_value(item.observation),
        }
    return {
        "detail": detail,
        "passed": item.passed(),
        "observed": render_value(item.observed()),
        "scores": [list(pair) for pair in item.scores()],
    }


def files(data):
    return [k.File(**item) for item in data]


def verifier(data):
    return k.Verifier(
        **{**data, "files": files(data["files"]), "tools": files(data["tools"])}
    )


def render_files(items):
    return [{"path": item.path, "contents": item.contents} for item in items]


def render_verifier(item):
    if item is None:
        return None
    return {
        "inputs": list(item.inputs),
        "files": render_files(item.files),
        "tools": render_files(item.tools),
        "command": list(item.command),
        "stdin": item.stdin,
        "timeout_seconds": item.timeout_seconds,
    }


def criterion(data):
    if data["type"] == "code":
        return k.Criterion.code(verifier=verifier(data["verifier"]))
    if data["type"] == "state":
        return k.Criterion.state(predicate=predicate(data["predicate"]))
    rubric = data["rubric"]
    return k.Criterion.answer(
        rubric=k.Rubric(
            expected=[(name, value(item)) for name, item in rubric["expected"]],
            text_rule=k.TextRule(**rubric["text_rule"])
            if rubric["text_rule"]
            else None,
        )
    )


def task(data):
    return k.Task(
        id=data["id"],
        fixture=files(data["fixture"]),
        instruction=k.Instruction(prompt=data["instruction"]["prompt"]),
        criterion=criterion(data["criterion"]),
        allowed_files=data["allowed_files"],
        reference=k.Reference(
            edits=files(data["reference"]["edits"]),
            completion=value(data["reference"]["completion"]),
        ),
    )


def outcome(data):
    status = data["verification"]
    statuses = {
        "unchecked": k.Verification.unchecked,
        "passed": k.Verification.passed,
        "failed": k.Verification.failed,
        "invalid_submission": k.Verification.invalid_submission,
        "timed_out": k.Verification.timed_out,
    }
    verification = (
        k.Verification.harness_error(detail=status["detail"])
        if status["type"] == "harness_error"
        else statuses[status["type"]]()
    )
    return k.Outcome(
        changed_files=data["changed_files"],
        verification=verification,
        response=value(data["response"]),
        facts=[(name, value(item)) for name, item in data["facts"]],
    )


def expected_result(case):
    expected = copy.deepcopy(case["expected"])
    if case["category"] != "task":
        return expected
    if case["id"] == "invalid-skill":
        baseline = next(
            item
            for item in CASES
            if item["category"] == "task" and item["id"] == "code"
        )
        generic = copy.deepcopy(case["input"])
        other = copy.deepcopy(baseline["input"])
        assert generic["instruction"].pop("skill") == "bad_skill"
        assert other["instruction"].pop("skill") == "reasoning"
        assert generic == other
        expected = copy.deepcopy(baseline["expected"])
    if case["id"] == "combined-issues":
        assert expected["failure"]["issues"].pop(1) == {
            "field": "instruction.skill",
            "message": "Invalid instruction skill",
        }
    if "case" in expected:
        assert expected["case"].pop("skill") == "reasoning"
    return expected


@pytest.mark.parametrize(
    "case", CASES, ids=lambda item: item["category"] + ":" + item["id"]
)
def test_captured_legacy_semantics(case):
    data = case["input"]
    category = case["category"]
    if category == "equal":
        actual = value(data["left"]).equal(other=value(data["right"]))
    elif category == "get":
        actual = render_value(value(data["value"]).get(path=data["path"]))
    elif category == "predicate":
        check, observation = predicate(data["predicate"]), value(data["observation"])
        actual = {
            "holds": check.holds(observation=observation),
            "assessment": render_assessment(
                k.Assessment(predicate=check, observation=observation)
            ),
        }
    elif category == "task":
        try:
            declaration = task(data)
            public = declaration.project()
            actual = {
                "case": {
                    "case_id": public.case_id,
                    "prompt": public.prompt,
                    "files": render_files(public.files),
                },
                "checks": render_verifier(declaration.checks()),
                "condition": render_predicate(declaration.criterion.condition()),
            }
        except k.ValidationError as error:
            actual = {
                "failure": {
                    "case_id": error.case_id,
                    "issues": [
                        {"field": item.field, "message": item.message}
                        for item in error.issues
                    ],
                }
            }
    else:
        declaration, observed = task(data["task"]), outcome(data["outcome"])
        try:
            actual = {
                "assessment": render_assessment(declaration.assess(outcome=observed)),
                "scores": [list(pair) for pair in declaration.grade(outcome=observed)],
                "observation": render_value(observed.observation()),
            }
        except k.GradingError as error:
            with pytest.raises(k.GradingError) as repeated:
                declaration.grade(outcome=observed)
            assert repeated.value.detail == error.detail
            actual = {"failure": {"type": error.failure, "detail": error.detail}}
    assert actual == expected_result(case)


def test_fixture_identity():
    assert CORPUS["source_revision"] == "523d1eede610171d825fde0fee12d418dd1ef981"
    assert len(CASES) == 163

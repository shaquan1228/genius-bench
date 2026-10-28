from collections.abc import Sequence
from typing import Final, Literal, Protocol, Self, final

from typing_extensions import disjoint_base

__all__ = [
    "Assessment",
    "Case",
    "Criterion",
    "SourceFile",
    "GradingError",
    "Instruction",
    "Issue",
    "KernelValue",
    "Outcome",
    "Predicate",
    "Record",
    "Reference",
    "REQUIRED_SCORES",
    "Rubric",
    "Task",
    "TextRule",
    "ValidationError",
    "Value",
    "Verification",
    "Verifier",
    "from_dataclass",
    "from_pydantic",
    "source_hash",
    "version",
]

class KernelValue(Protocol):
    def __kernel_value__(self) -> object: ...

@disjoint_base
class Value:
    @staticmethod
    def from_python(*, value: object) -> Value: ...
    def get(self, *, path: Sequence[str]) -> Value | None: ...
    def equal(self, *, other: object) -> bool: ...
    @final
    class Null(Value):
        __match_args__: Final = ()
        def __new__(cls) -> Self: ...

    @final
    class Bool(Value):
        __match_args__: Final = ("value",)
        def __new__(cls, *, value: bool) -> Self: ...
        @property
        def value(self) -> bool: ...

    @final
    class Int(Value):
        __match_args__: Final = ("value",)
        def __new__(cls, *, value: int) -> Self: ...
        @property
        def value(self) -> int: ...

    @final
    class Float(Value):
        __match_args__: Final = ("value",)
        def __new__(cls, *, value: float) -> Self: ...
        @property
        def value(self) -> float: ...

    @final
    class String(Value):
        __match_args__: Final = ("value",)
        def __new__(cls, *, value: str) -> Self: ...
        @property
        def value(self) -> str: ...

    @final
    class List(Value):
        __match_args__: Final = ("items",)
        def __new__(cls, *, items: Sequence[object]) -> Self: ...
        @property
        def items(self) -> tuple[Value, ...]: ...

    @final
    class Record(Value):
        __match_args__: Final = ("fields",)
        def __new__(cls, *, fields: Sequence[tuple[str, object]]) -> Self: ...
        @property
        def fields(self) -> tuple[tuple[str, Value], ...]: ...

@final
class Record:
    def __new__(cls, *, fields: Sequence[tuple[str, object]]) -> Self: ...
    @property
    def fields(self) -> tuple[tuple[str, Value], ...]: ...
    def __kernel_value__(self) -> Value.Record: ...

@disjoint_base
class Predicate:
    def holds(self, *, observation: object) -> bool: ...
    @final
    class Present(Predicate):
        __match_args__: Final = ("path",)
        def __new__(cls, *, path: Sequence[str]) -> Self: ...
        @property
        def path(self) -> tuple[str, ...]: ...

    @final
    class Equals(Predicate):
        __match_args__: Final = ("path", "expected")
        def __new__(cls, *, path: Sequence[str], expected: object) -> Self: ...
        @property
        def path(self) -> tuple[str, ...]: ...
        @property
        def expected(self) -> Value: ...

    @final
    class Within(Predicate):
        __match_args__: Final = ("path", "allowed")
        def __new__(cls, *, path: Sequence[str], allowed: Sequence[object]) -> Self: ...
        @property
        def path(self) -> tuple[str, ...]: ...
        @property
        def allowed(self) -> tuple[Value, ...]: ...

    @final
    class Contains(Predicate):
        __match_args__: Final = ("path", "item")
        def __new__(cls, *, path: Sequence[str], item: object) -> Self: ...
        @property
        def path(self) -> tuple[str, ...]: ...
        @property
        def item(self) -> Value: ...

    @final
    class Words(Predicate):
        __match_args__: Final = ("path", "min_words", "max_words")
        def __new__(
            cls, *, path: Sequence[str], min_words: int, max_words: int
        ) -> Self: ...
        @property
        def path(self) -> tuple[str, ...]: ...
        @property
        def min_words(self) -> int: ...
        @property
        def max_words(self) -> int: ...

    @final
    class All(Predicate):
        __match_args__: Final = ("checks",)
        def __new__(cls, *, checks: Sequence[tuple[str, Predicate]]) -> Self: ...
        @property
        def checks(self) -> tuple[tuple[str, Predicate], ...]: ...

    @final
    class Any(Predicate):
        __match_args__: Final = ("checks",)
        def __new__(cls, *, checks: Sequence[tuple[str, Predicate]]) -> Self: ...
        @property
        def checks(self) -> tuple[tuple[str, Predicate], ...]: ...

    @final
    class Not(Predicate):
        __match_args__: Final = ("check",)
        def __new__(cls, *, check: Predicate) -> Self: ...
        @property
        def check(self) -> Predicate: ...

@disjoint_base
class Verification:
    @property
    def status(
        self,
    ) -> Literal[
        "unchecked",
        "passed",
        "failed",
        "invalid_submission",
        "timed_out",
        "harness_error",
    ]: ...
    @final
    class Unchecked(Verification):
        __match_args__: Final = ()
        def __new__(cls) -> Self: ...
        @property
        def status(self) -> Literal["unchecked"]: ...

    @final
    class Passed(Verification):
        __match_args__: Final = ()
        def __new__(cls) -> Self: ...
        @property
        def status(self) -> Literal["passed"]: ...

    @final
    class Failed(Verification):
        __match_args__: Final = ()
        def __new__(cls) -> Self: ...
        @property
        def status(self) -> Literal["failed"]: ...

    @final
    class InvalidSubmission(Verification):
        __match_args__: Final = ()
        def __new__(cls) -> Self: ...
        @property
        def status(self) -> Literal["invalid_submission"]: ...

    @final
    class TimedOut(Verification):
        __match_args__: Final = ()
        def __new__(cls) -> Self: ...
        @property
        def status(self) -> Literal["timed_out"]: ...

    @final
    class HarnessError(Verification):
        __match_args__: Final = ("detail",)
        def __new__(cls, *, detail: str) -> Self: ...
        @property
        def status(self) -> Literal["harness_error"]: ...
        @property
        def detail(self) -> str: ...

@final
class Outcome:
    def __new__(
        cls,
        *,
        changed_files: Sequence[str],
        verification: Verification,
        response: object,
        facts: Sequence[tuple[str, object]],
    ) -> Self: ...
    @property
    def changed_files(self) -> tuple[str, ...]: ...
    @property
    def verification(self) -> Verification: ...
    @property
    def response(self) -> Value: ...
    @property
    def facts(self) -> tuple[tuple[str, Value], ...]: ...
    def observation(self) -> Value: ...

@disjoint_base
class Assessment:
    def __new__(cls, *, predicate: Predicate, observation: object) -> Assessment: ...
    def passed(self) -> bool: ...
    def observed(self) -> Value | None: ...
    def scores(self) -> tuple[tuple[str, bool], ...]: ...
    @final
    class Check(Assessment):
        __match_args__: Final = ("predicate", "observation")
        def __new__(cls, *, predicate: Predicate, observation: object) -> Self: ...
        @property
        def predicate(self) -> Predicate: ...
        @property
        def observation(self) -> Value: ...

    @final
    class All(Assessment):
        __match_args__: Final = ("checks",)
        def __new__(cls, *, checks: Sequence[tuple[str, Assessment]]) -> Self: ...
        @property
        def checks(self) -> tuple[tuple[str, Assessment], ...]: ...

@final
class SourceFile:
    def __new__(cls, *, path: str, contents: str) -> Self: ...
    @property
    def path(self) -> str: ...
    @property
    def contents(self) -> str: ...

@final
class Instruction:
    def __new__(cls, *, prompt: str) -> Self: ...
    @property
    def prompt(self) -> str: ...

@final
class Verifier:
    def __new__(
        cls,
        *,
        inputs: Sequence[str],
        files: Sequence[SourceFile],
        tools: Sequence[SourceFile],
        command: Sequence[str],
        stdin: str,
        timeout_seconds: int,
    ) -> Self: ...
    @property
    def inputs(self) -> tuple[str, ...]: ...
    @property
    def files(self) -> tuple[SourceFile, ...]: ...
    @property
    def tools(self) -> tuple[SourceFile, ...]: ...
    @property
    def command(self) -> tuple[str, ...]: ...
    @property
    def stdin(self) -> str: ...
    @property
    def timeout_seconds(self) -> int: ...

@final
class TextRule:
    def __new__(cls, *, field: str, min_words: int, max_words: int) -> Self: ...
    @property
    def field(self) -> str: ...
    @property
    def min_words(self) -> int: ...
    @property
    def max_words(self) -> int: ...

@final
class Rubric:
    def __new__(
        cls, *, expected: Sequence[tuple[str, object]], text_rule: TextRule | None
    ) -> Self: ...
    @property
    def expected(self) -> tuple[tuple[str, Value], ...]: ...
    @property
    def text_rule(self) -> TextRule | None: ...

@disjoint_base
class Criterion:
    def condition(self) -> Predicate: ...
    @final
    class Code(Criterion):
        __match_args__: Final = ("verifier",)
        def __new__(cls, *, verifier: Verifier) -> Self: ...
        @property
        def verifier(self) -> Verifier: ...

    @final
    class Answer(Criterion):
        __match_args__: Final = ("rubric",)
        def __new__(cls, *, rubric: Rubric) -> Self: ...
        @property
        def rubric(self) -> Rubric: ...

    @final
    class State(Criterion):
        __match_args__: Final = ("predicate",)
        def __new__(cls, *, predicate: Predicate) -> Self: ...
        @property
        def predicate(self) -> Predicate: ...

@final
class Reference:
    def __new__(cls, *, edits: Sequence[SourceFile], completion: object) -> Self: ...
    @property
    def edits(self) -> tuple[SourceFile, ...]: ...
    @property
    def completion(self) -> Value: ...

@final
class Case:
    @property
    def case_id(self) -> str: ...
    @property
    def prompt(self) -> str: ...
    @property
    def files(self) -> tuple[SourceFile, ...]: ...

@final
class Issue:
    @property
    def field(self) -> str: ...
    @property
    def message(self) -> str: ...

class ValidationError(ValueError):
    case_id: str | None
    issues: tuple[Issue, ...]

class GradingError(RuntimeError):
    failure: Literal["verification_harness_error"]
    detail: str

@final
class Task:
    def __new__(
        cls,
        *,
        id: str,
        fixture: Sequence[SourceFile],
        instruction: Instruction,
        criterion: Criterion,
        allowed_files: Sequence[str],
        reference: Reference,
    ) -> Self: ...
    @property
    def id(self) -> str: ...
    @property
    def fixture(self) -> tuple[SourceFile, ...]: ...
    @property
    def instruction(self) -> Instruction: ...
    @property
    def criterion(self) -> Criterion: ...
    @property
    def allowed_files(self) -> tuple[str, ...]: ...
    @property
    def reference(self) -> Reference: ...
    def checks(self) -> Verifier | None: ...
    def project(self) -> Case: ...
    def assess(self, *, outcome: Outcome) -> Assessment: ...
    def grade(self, *, outcome: Outcome) -> tuple[tuple[str, bool], ...]: ...
    def grade_reference(
        self, *, verification: Verification
    ) -> tuple[tuple[str, bool], ...]: ...

REQUIRED_SCORES: Final[tuple[str, str]]

def version() -> str: ...
def source_hash() -> str: ...
def from_dataclass(*, value: object) -> Value: ...
def from_pydantic(*, value: object) -> Value: ...

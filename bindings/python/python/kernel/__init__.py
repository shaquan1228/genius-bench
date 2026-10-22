from dataclasses import fields, is_dataclass
from typing import Protocol

from ._native import (
    Assessment,
    Case,
    Criterion,
    File,
    GradingError,
    Instruction,
    Issue,
    Outcome,
    Predicate,
    Record,
    Reference,
    REQUIRED_SCORES,
    Rubric,
    Task,
    TextRule,
    ValidationError,
    Value,
    Verification,
    Verifier,
    source_hash,
    version,
)


__all__ = [
    "Assessment",
    "Case",
    "Criterion",
    "File",
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


def from_dataclass(*, value):
    if isinstance(value, type) or not is_dataclass(value):
        raise TypeError("Expected a dataclass instance")
    active = set()

    def project(item):
        declaration = is_dataclass(item) and not isinstance(item, type)
        if not declaration and not isinstance(item, (list, tuple, dict)):
            return item
        identity = id(item)
        if identity in active:
            raise ValueError("Cyclic dataclass value")
        active.add(identity)
        try:
            if declaration:
                return {
                    field.name: project(getattr(item, field.name))
                    for field in fields(item)
                }
            if isinstance(item, dict):
                return {name: project(child) for name, child in item.items()}
            return [project(child) for child in item]
        finally:
            active.remove(identity)

    return Value.from_python(value=project(value))


def from_pydantic(*, value):
    from pydantic import BaseModel

    if not isinstance(value, BaseModel):
        raise TypeError("Expected a Pydantic BaseModel instance")
    return Value.from_python(value=value.model_dump(mode="python"))

---
title: Extract the semantic kernel and Python package
artifact_contract: ce-unified-plan/v1
artifact_readiness: implementation-ready
execution: code
source: /Users/me/Desktop/rustsmh.md
source_revision: 523d1eede610171d825fde0fee12d418dd1ef981
---

# Semantic kernel extraction

## Goal Capsule

Extract Genius's deterministic evaluation semantics into a pure Rust library,
remove consumer-specific policy, and provide an installable typed Python package.
Genius remains the first consumer and proof of concept. HF Jobs, Docker, Inspect,
execution, verifier environments, and evidence collection stay in Genius.

This execution covers deliverables 1–3. The original plan folder is
`/Users/me/Repositories/genius/docs/plans/`; an identical copy belongs to the
Genius Bench stack so reviewers can inspect its implementation contract.
Progress is recorded in commits and verification receipts, never in this plan.

## Deliverables

| ID | Deliverable | Acceptance |
| --- | --- | --- |
| D0 | Documentation prerequisite | README and CONTRIBUTING describe ownership and contributor rules. |
| D1 | Rust semantic kernel | Pure owned Rust values, predicates, outcomes, assessments, validated tasks, structured failures, public projections, and grading pass legacy characterization fixtures. |
| D2 | Generic consumer contract | Instructions and cases contain no built-in skill field; consumer settings and Genius defaults are absent from the core. |
| D3 | Typed Python package | Python calls the compiled Rust core through native bindings; a built wheel installs and grades outside the checkout without Rust or OCaml on the consumer path. |
| D4 | Genius migration | Genius imports the package and removes its OCaml bridge/build dependency while preserving its workflows. |
| D5 | Genius integration proof | Existing HF/Docker/Inspect paths use the imported grader and retain their execution responsibilities. |
| D6 | Ruby package | Native Ruby bindings replay shared fixtures and install from supported binary artifacts. |
| D7 | Public distribution | Release automation publishes supported crate, wheel, and gem artifacts. |

## Product Contract Key Decisions

- K1. The kernel owns semantic validation, predicate interpretation, grading,
  required score names, and private/public projections. Genius owns execution
  integrations. session-settled: user-directed. Governs R1, R2, R5.
- K2. D1–D3 form an iterative dependent PR stack above the documentation PR.
  Existing PRs are not merged or force-pushed. session-settled: user-directed.
  Governs R6.
- K3. Humans own the code. No LLM attribution tags, coauthor trailers, model
  badges, generated-by-LLM notices, or agent signatures in source, commits,
  documentation, or PRs. session-settled: user-directed. Governs R7.
- K4. No explanatory code comments or docstrings are added. The only exception
  is preserving existing OCaml comments that cite sources for semantic
  definitions, translated to the target language's comment syntax when ported.
  Interpreter directives and language attributes are syntax, not explanation.
  session-settled: user-directed. Governs R7.
- K5. Rust removes the OCaml runtime and custom bridge dependency. Python still
  needs a native binding layer; it performs conversion and error adaptation,
  with all grading policy remaining in Rust. session-settled: user-approved.
  Governs R2, R4.

## Requirements

- R1. Match legacy semantics for supported inputs: duplicate-record first-wins,
  field-order-independent record equality, ordered lists, distinct scalar
  alternatives, missing versus null, named conjunction ordering, empty All/Any,
  inclusive word bounds, and the original whitespace set.
- R2. Declaration construction establishes invariants; tasks own snapshots and
  expose no mutable access that bypasses validation. Validation retains every
  applicable issue and its field. Harness errors remain distinct from candidate
  failure and timeout. Grading returns both assessments and named boolean scores.
- R3. Shared fixtures originate from executing the pinned OCaml implementation,
  not from the new implementation. Fixtures cover code, answer, and state tasks,
  projection secrecy, failures, validation, and conversion edge cases.
- R4. Python exposes typed domain operations, keyword arguments, immutable
  snapshots, plain-value conversion, an explicit custom-value protocol, and
  dataclass/Pydantic adapters. Runtime calls do not serialize through JSON.
  Required package data includes typing declarations and py.typed.
- R5. Core source identity is available from the installed artifact. Reserved
  outcome channels cannot be overwritten by open facts. Public cases omit
  private references/checks/stdin while retaining explicitly public tools.
- R6. Each layer is independently buildable and verifiable against its parent.
  Use the existing gh stack CLI and ordinary hook-gated pushes.
- R7. CONTRIBUTING rules 1–10 apply, including the new ownership/attribution and
  no-comments rules. Semantic citations retain their original source links.

## Scope Boundaries

In scope: semantic core, fixture capture/replay, consumer-neutral task contract,
Python bindings/adapters/stubs, development checks, pre-push hook, offline CI,
and wheel build/install verification for declared Linux/macOS targets.

Out of scope: changing Genius runtime files, runner redesign, live/paid jobs,
Ruby implementation, registry publication, a third language, and unrelated
changes in either repository. D4–D7 remain follow-on work.

## Implementation Units

### U1. Port the grading semantics (D1)

Dependencies: D0 documentation branch.

Files: Cargo.toml, Cargo.lock, rust-toolchain.toml, .gitignore, LICENSE,
Makefile, .githooks/pre-push, scripts/capture_ocaml.py, scripts/check.py,
crates/semantic-core/Cargo.toml, crates/semantic-core/build.rs,
crates/semantic-core/src/{lib,value,predicate,outcome,assessment,failure,task,support}.rs,
crates/semantic-core/tests/{semantics,fixtures}.rs, fixtures/export.ml,
fixtures/semantics.json, fixtures/README.md, .github/workflows/check.yml.

Goal: reproduce the pure legacy grader with an independently captured fixture
corpus. Include the legacy instruction skill field only in this port layer;
U2 deliberately removes that consumer-specific contract.

Approach: use owned enum/record types, private validated task fields, and direct
Result values. Keep the core's runtime dependencies free of Python, SDKs, and IO.
Build scripts may hash canonical source inputs; source identity must not depend
on the checkout at runtime. Export tagged JSON fixture values so duplicate
record fields and non-finite floats are representable. Use signed i64 integers;
preserve all values representable by the pinned 64-bit OCaml runtime, and reject
Python overflow instead of converting integers to floating point.

Execution note: run the existing OCaml tests and capture expected fixture
results before implementing Rust. Observe the Rust fixture test's initial
failure before filling in the implementation.

Patterns to follow: Genius evals/ocaml/{value,predicate,outcome,assessment,task,support}.ml;
evals/ocaml/tests/{test_predicate,test_kernel}.ml; evals/python/tests/declarations.py.

Test scenarios: every predicate variant; duplicate keys and reserved facts;
int/float/bool distinction; empty structures; whitespace boundaries; code status
matrix; valid/invalid answer rubrics; state/trace criteria; private projections;
aggregated validation errors; assessment observations and ordered scores;
installed source hash stability and source-change invalidation.

Verification: captured OCaml fixtures replay identically in cargo test -p
semantic-core; cargo fmt and scoped clippy pass; the pre-push hook gates code
pushes and skips documentation-only pushes. CI repeats these checks offline.

### U2. Remove consumer-specific policy (D2)

Dependencies: U1 committed and verified.

Files: crates/semantic-core/src/{lib,task}.rs,
crates/semantic-core/tests/{semantics,fixtures}.rs, fixtures/README.md, README.md.

Goal: expose a task contract that any consumer can use without a Genius skill
name or instruction profile.

Approach: retain prompt and public fixture inputs, remove skill from Instruction
and Case, and keep run profiles/settings outside the core. Preserve generic
validation, scores, and projections. The approved legacy difference is removal
of skill-field validation/projection; retain and explain that boundary in fixture
replay without hiding unrelated mismatches.

Execution note: widen the U1 tests to demonstrate the generic instruction/case
contract and custom state observations before making the contract change.

Patterns to follow: U1's constructors and projection tests; Genius's separation
between Task.assess and execution/verification adapters.

Test scenarios: code/answer/state declarations with prompt-only instructions;
public case contains no consumer label or private data; custom observations
need no new core type; invalid generic declarations retain validation failures;
all legacy predicate/grading fixtures still pass.

Verification: focused cargo tests, formatting, and clippy pass. The core has no
Genius profile defaults, SDK dependency, runner implementation, or IO operation.

### U3. Ship an installable Python boundary (D3)

Dependencies: U2 committed and verified.

Files: Cargo.toml, Cargo.lock, Makefile, README.md, scripts/check.py,
bindings/python/Cargo.toml, bindings/python/pyproject.toml,
bindings/python/src/*.rs, bindings/python/python/kernel/*,
bindings/python/tests/*.py, .github/workflows/check.yml,
.github/workflows/wheels.yml.

Goal: call the Rust semantic API from Python with useful types and wheel-only
consumer installation.

Approach: PyO3/maturin wraps the core's owned values and checked constructors.
Bindings adapt calls, values, and structured errors, never grading policy.
Expose immutable values and keyword-based public construction/operations.
Preserve duplicate fields through explicit record values and distinguish bool,
int, and float. Offer __kernel_value__ and explicit dataclass/Pydantic adapters;
Pydantic remains optional. Ship checked/generated .pyi declarations and py.typed.
Pin compatible build dependencies and document the actual ABI/platform matrix.

Execution note: write public API fixture replay and error/immutability tests
before filling in the binding. Prove the installed artifact from a temporary
working directory, with compiler/OCaml commands absent from its subprocess PATH.

Patterns to follow: U2's canonical Rust API and shared fixtures; Genius's named
constructor/error contracts; official PyO3 and maturin documentation.

Test scenarios: replay all applicable shared fixtures through public Python API;
code/answer/state grading; aggregated ValidationError and distinct GradingError;
keyword contracts; collection snapshot isolation; public/private projections;
custom protocol and adapters; integer overflow and cyclic/unsupported conversion;
typed API/stub agreement; installed artifact identity; compiler-free import and
grading outside the source checkout.

Verification: scoped Rust checks; Python public API tests; type/stub checks;
build wheel and source distribution, install wheel into a clean temporary
virtual environment, replay smoke/fixture checks there. CI builds and exercises
Linux x86_64 and macOS arm64 wheels for declared CPython versions. No registry
publication occurs in this unit.

## Verification Contract

- V1: execute pinned OCaml tests and record fixture capture before the Rust port.
- V2: pass each unit's scoped tests and formatting/lint checks before committing
  it or starting its dependent unit; keep only one build active on this machine.
- V3: pushes run .githooks/pre-push; do not bypass failures from this change.
- V4: perform simplification and independent code review on the implementation
  stack; apply valid findings and reverify affected units before final submission.
- V5: verify actual PR bases/order, contents, and CI on the published stack.
- V6: code contains only the allowed preserved semantic citation comments, and
  source/commit/PR text contains no LLM attribution tags.

## Definition of Done

D1–D3 pass their verification contracts and exist as three dependent reviewable
PRs above D0. The Python wheel calls the generic Rust grader outside the checkout.
Genius execution integrations are untouched. README, CONTRIBUTING, and the
published PR descriptions match the delivered scope. The plan body remains a
stable decision artifact; git and review/verification receipts carry completion.

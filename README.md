# Genius Bench

A shared grading core for evaluation libraries, starting with the
[Genius eval library](https://github.com/shaquan1228/genius).
Task declarations and observed outcomes produce assessments with named scores
and supporting evidence.

Status: the Rust semantic core and generic task API are implemented. A typed
Python binding is next.

The intended boundary is small:

- Rust owns values, predicates, declaration validation, public case projections,
  grading, and structured failures. Grading performs no IO.
- Python and Ruby adapt host values and calls to the same core semantics.
- Genius imports the semantic kernel and retains HF Jobs, Docker, Inspect,
  execution, verification environments, and evidence collection. Its SDK types
  and provider settings stay in Genius.

Instructions contain only a prompt. Public cases contain an identity, prompt,
and files. Consumer labels, profiles, and execution defaults stay outside the core.

Genius is the first consumer and the proof of concept. The first milestone is
for its existing eval library to use an installable Python binding to this core
while preserving grading behavior and runner workflows. Ruby bindings follow
that integration proof.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor rules.

The [extraction plan](docs/plans/2026-10-22-semantic-kernel-extraction-plan.md)
defines the deliverables and verification gates.

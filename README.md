# Genius Bench

A shared grading core for evaluation libraries, starting with the
[Genius eval library](https://github.com/shaquan1228/genius/tree/523d1eede610171d825fde0fee12d418dd1ef981/evals).
Task declarations and observed outcomes produce assessments with named scores
and supporting evidence.

Status: documentation scaffold. The Rust core and language bindings are planned.

The intended boundary is small:

- Rust owns values, predicates, declaration validation, public case projections,
  grading, and structured failures. Grading performs no IO.
- Python and Ruby adapt host values and calls to the same core semantics.
- Genius imports the semantic kernel and retains HF Jobs, Docker, Inspect,
  execution, verification environments, and evidence collection. Its SDK types
  and provider settings stay in Genius.

Genius is the first consumer and the proof of concept. The first milestone is
for its existing eval library to use an installable Python binding to this core
while preserving grading behavior and runner workflows. Ruby bindings follow
that integration proof.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the contributor rules.

The [extraction plan](docs/plans/2026-10-22-semantic-kernel-extraction-plan.md)
defines the deliverables and verification gates.

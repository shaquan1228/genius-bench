# Contributing

1. **Write technical English.** Code, APIs, and documentation should
   read as precise technical English. Use established evaluation terms, explicit
   names, and named arguments where supported. Prefer `Task.assess(task=...,
   outcome=...)` to a generic `process(data)`.

2. **Give semantics one owner.** The core owns validation, predicate meaning,
   grading, and required score names. Bindings convert values and calls;
   integrations adapt SDKs. Keep domain policy out of those adapters.

3. **Keep the core pure and reusable.** Genius retains HF Jobs, Docker, Inspect,
   execution, filesystem access, networking, and provider configuration. This
   repository owns semantic grading and language bindings. Consumer domain
   types project to core values through explicit adapters.

4. **Establish invariants at construction.** Use validated constructors,
   explicit alternatives, immutable values, and copied collection snapshots.
   Return structured validation issues. Conversions must preserve the declared
   value types and semantics.

5. **Separate public inputs from private grading material.** Derive candidate
   cases from validated tasks. Keep reference answers, private checks, and
   verifier input out of candidate projections. Consumers execute verification
   in a fresh grader environment.

6. **Preserve failure meaning and evidence.** An incorrect or malformed candidate
   answer differs from a harness failure. Keep verification statuses distinct,
   retain diagnostics, and record which installed grading policy produced a
   result.

7. **Verify public APIs.** Test semantic operations, validation, conversion,
   projections, and failure paths through the core and each binding. Use Genius
   as the first integration proof. Test packaged installations outside the
   source checkout.

8. **Keep changes focused and verifiable.** State the behavior changed and the
   checks run. Add tests for meaningful behavior and failure paths. Document
   supported runtime/platform combinations before promising compiler-free
   installation on them.

9. **Humans own the code.** Do not add LLM attribution tags, coauthor trailers,
   model badges, generated-by-LLM notices, or agent signatures to source,
   documentation, commits, or PRs.

10. **Add no code comments or docstrings.** Preserve only the existing OCaml
    comments that cite sources for semantic definitions, translating their
    comment syntax when porting. Put other explanations in documentation,
    commits, or PRs. Interpreter directives and language attributes remain valid.

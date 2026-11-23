# Grading ABI v1

The Go source binding links this static library and uses the two functions in
[grade.h](../grade.h). Pass a readable, NUL-terminated UTF-8 JSON request to
`genius_bench_grade_v1`. The caller retains ownership of the request. The
returned pointer contains a NUL-terminated JSON reply. Read the reply, then
pass the original pointer to `genius_bench_free_v1` exactly once.

```json
{
  "task": {
    "id": "installed", "prompt": "observe", "present_path": ["response"],
    "allowed_files": [], "reference_completion": true
  },
  "outcome": {
    "changed_files": [], "response": true,
    "verification": {"status": "unchecked", "detail": ""}
  }
}
```

The two boolean fields also accept null. Verification accepts the core statuses
`unchecked`, `passed`, `failed`, `invalid_submission`, `timed_out`, and
`harness_error`. The last carries the verifier diagnostic in `detail`.

| Reply kind | Payload |
| --- | --- |
| `scores` | `scores` is the core's ordered array of `{name, passed}` pairs. |
| `validation` | `case_id` and `issues`, an array of `{field, message}` pairs. |
| `grading` | `code` and the corresponding diagnostic in `detail`. |
| `bridge` | `detail` describes a request or native boundary error. |

use semantic_core::{
    Criterion, GradingFailure, Instruction, Outcome, Predicate, Reference, Task, Value,
    Verification,
};
use serde::Deserialize;
use serde_json::{Value as Json, json};
use std::ffi::{CStr, CString, c_char};
use std::panic::catch_unwind;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GradeRequest {
    task: TaskInput,
    outcome: OutcomeInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskInput {
    id: String,
    prompt: String,
    present_path: Vec<String>,
    allowed_files: Vec<String>,
    reference_completion: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OutcomeInput {
    changed_files: Vec<String>,
    response: Option<bool>,
    verification: VerificationInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VerificationInput {
    status: String,
    detail: String,
}

impl VerificationInput {
    fn into_core(self) -> Result<Verification, String> {
        match self.status.as_str() {
            "unchecked" => Ok(Verification::Unchecked),
            "passed" => Ok(Verification::Passed),
            "failed" => Ok(Verification::Failed),
            "invalid_submission" => Ok(Verification::InvalidSubmission),
            "timed_out" => Ok(Verification::TimedOut),
            "harness_error" => Ok(Verification::HarnessError {
                detail: self.detail,
            }),
            _ => Err(format!("Unknown verification status: {}", self.status)),
        }
    }
}

fn grade(request: GradeRequest) -> Result<Json, String> {
    let input = request.task;
    let task = Task::new(
        input.id,
        Vec::new(),
        Instruction {
            prompt: input.prompt,
        },
        Criterion::State {
            predicate: Predicate::Present {
                path: input.present_path,
            },
        },
        input.allowed_files,
        Reference {
            edits: Vec::new(),
            completion: input.reference_completion.map_or(Value::Null, Value::Bool),
        },
    );
    let task = match task {
        Ok(task) => task,
        Err(failure) => {
            let issues: Vec<_> = failure
                .issues
                .into_iter()
                .map(|issue| json!({"field": issue.field, "message": issue.message}))
                .collect();
            return Ok(json!({"kind": "validation", "case_id": failure.case_id, "issues": issues}));
        }
    };
    let input = request.outcome;
    let outcome = Outcome {
        changed_files: input.changed_files,
        verification: input.verification.into_core()?,
        response: input.response.map_or(Value::Null, Value::Bool),
        facts: Vec::new(),
    };
    match task.grade(&outcome) {
        Ok(scores) => {
            let scores: Vec<_> = scores
                .into_iter()
                .map(|(name, passed)| json!({"name": name, "passed": passed}))
                .collect();
            Ok(json!({"kind": "scores", "scores": scores}))
        }
        Err(failure) => {
            let code = failure.kind();
            let GradingFailure::VerificationHarnessError { detail } = failure;
            Ok(json!({"kind": "grading", "code": code, "detail": detail}))
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn genius_bench_grade_v1(request: *const c_char) -> *mut c_char {
    catch_unwind(|| {
        let reply = if request.is_null() {
            Err("Null grade request".to_owned())
        } else {
            let bytes = unsafe { CStr::from_ptr(request) }.to_bytes();
            serde_json::from_slice(bytes)
                .map_err(|error| error.to_string())
                .and_then(grade)
        };
        let reply = reply.unwrap_or_else(|detail| json!({"kind": "bridge", "detail": detail}));
        CString::new(reply.to_string())
            .expect("JSON escapes NUL")
            .into_raw()
    })
    .unwrap_or_else(|_| {
        c"{\"kind\":\"bridge\",\"detail\":\"Native grading panic\"}"
            .to_owned()
            .into_raw()
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn genius_bench_free_v1(reply: *mut c_char) {
    if !reply.is_null() {
        drop(unsafe { CString::from_raw(reply) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(input: Option<&CStr>) -> Json {
        let request = input.map_or(std::ptr::null(), CStr::as_ptr);
        let reply = unsafe { genius_bench_grade_v1(request) };
        let value = serde_json::from_slice(unsafe { CStr::from_ptr(reply) }.to_bytes()).unwrap();
        unsafe { genius_bench_free_v1(reply) };
        value
    }

    #[test]
    fn malformed_requests_return_owned_bridge_errors() {
        for input in [None, Some(c"{"), Some(c"{}"), Some(c"\xff")] {
            let reply = call(input);
            assert_eq!(reply["kind"], "bridge");
            assert!(!reply["detail"].as_str().unwrap().is_empty());
        }
        unsafe { genius_bench_free_v1(std::ptr::null_mut()) };
    }

    #[test]
    fn grading_preserves_score_order_and_file_scope() {
        let request = c"{
          \"task\": {\"id\":\"scope\",\"prompt\":\"observe\",\"present_path\":[\"response\"],
                   \"allowed_files\":[],\"reference_completion\":true},
          \"outcome\": {\"changed_files\":[\"extra\"],\"response\":false,
                      \"verification\":{\"status\":\"unchecked\",\"detail\":\"\"}}
        }";
        assert_eq!(
            call(Some(request)),
            json!({
                "kind": "scores",
                "scores": [{"name": "correctness", "passed": true}, {"name": "scope", "passed": false}]
            })
        );
    }
}

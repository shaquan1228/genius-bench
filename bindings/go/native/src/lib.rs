use semantic_core::{
    Criterion, File, GradingFailure, Instruction, Outcome, Predicate, Reference, Scores, Task,
    ValidationFailure, Value, Verification,
};
use std::{
    mem::size_of,
    panic::{UnwindSafe, catch_unwind},
    ptr, slice, str,
};

#[repr(C)]
#[derive(Clone, Copy)]
struct Buffer<T> {
    data: *const T,
    len: usize,
}

impl<T> From<&[T]> for Buffer<T> {
    fn from(values: &[T]) -> Self {
        Self {
            data: if values.is_empty() {
                ptr::null()
            } else {
                values.as_ptr()
            },
            len: values.len(),
        }
    }
}

impl<T> Buffer<T> {
    fn read(&self) -> Result<&[T], Reply> {
        if self.len == 0 {
            return Ok(&[]);
        }
        if self.data.is_null()
            || !self.data.is_aligned()
            || self.len > isize::MAX as usize / size_of::<T>()
        {
            return Err(Reply::Invalid("Invalid input buffer"));
        }
        Ok(unsafe { slice::from_raw_parts(self.data, self.len) })
    }
}

type Text = Buffer<u8>;

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        value.as_bytes().into()
    }
}

impl Text {
    fn string(&self) -> Result<String, Reply> {
        str::from_utf8(self.read()?)
            .map(str::to_owned)
            .map_err(|_| Reply::Invalid("Invalid UTF-8"))
    }
}

impl Buffer<Text> {
    fn strings(&self) -> Result<Vec<String>, Reply> {
        self.read()?.iter().map(Text::string).collect()
    }
}

#[repr(C)]
struct InputFile {
    path: Text,
    contents: Text,
}

impl Buffer<InputFile> {
    fn files(&self) -> Result<Vec<File>, Reply> {
        self.read()?
            .iter()
            .map(|file| {
                Ok(File {
                    path: file.path.string()?,
                    contents: file.contents.string()?,
                })
            })
            .collect()
    }
}

fn value(tag: u8) -> Result<Value, Reply> {
    match tag {
        0 => Ok(Value::Null),
        1 => Ok(Value::Bool(false)),
        2 => Ok(Value::Bool(true)),
        _ => Err(Reply::Invalid("Invalid value tag")),
    }
}

#[repr(C)]
struct InputTask {
    id: Text,
    prompt: Text,
    fixture: Buffer<InputFile>,
    edits: Buffer<InputFile>,
    present_path: Buffer<Text>,
    allowed_files: Buffer<Text>,
    completion: u8,
}

impl InputTask {
    fn task(&self) -> Result<Task, Reply> {
        Task::new(
            self.id.string()?,
            self.fixture.files()?,
            Instruction {
                prompt: self.prompt.string()?,
            },
            Criterion::State {
                predicate: Predicate::Present {
                    path: self.present_path.strings()?,
                },
            },
            self.allowed_files.strings()?,
            Reference {
                edits: self.edits.files()?,
                completion: value(self.completion)?,
            },
        )
        .map_err(Reply::Validation)
    }
}

#[repr(C)]
struct InputVerification {
    tag: u8,
    detail: Text,
}

impl InputVerification {
    fn verification(&self) -> Result<Verification, Reply> {
        match self.tag {
            0 => Ok(Verification::Unchecked),
            1 => Ok(Verification::Passed),
            2 => Ok(Verification::Failed),
            3 => Ok(Verification::InvalidSubmission),
            4 => Ok(Verification::TimedOut),
            5 => Ok(Verification::HarnessError {
                detail: self.detail.string()?,
            }),
            _ => Err(Reply::Invalid("Invalid verification tag")),
        }
    }
}

#[repr(C)]
struct InputFact {
    name: Text,
    value: u8,
}

#[repr(C)]
struct InputOutcome {
    changed_files: Buffer<Text>,
    verification: InputVerification,
    response: u8,
    facts: Buffer<InputFact>,
}

impl InputOutcome {
    fn outcome(&self) -> Result<Outcome, Reply> {
        Ok(Outcome {
            changed_files: self.changed_files.strings()?,
            verification: self.verification.verification()?,
            response: value(self.response)?,
            facts: self
                .facts
                .read()?
                .iter()
                .map(|fact| Ok((fact.name.string()?, value(fact.value)?)))
                .collect::<Result<_, Reply>>()?,
        })
    }
}

#[derive(Debug)]
enum Reply {
    Scores(Scores),
    Validation(ValidationFailure),
    Grading(GradingFailure),
    Invalid(&'static str),
}

fn boundary(call: impl FnOnce() -> Result<Option<Scores>, Reply> + UnwindSafe) -> *mut Reply {
    let reply = match catch_unwind(call) {
        Ok(Ok(None)) => return ptr::null_mut(),
        Ok(Ok(Some(scores))) => Reply::Scores(scores),
        Ok(Err(error)) => error,
        Err(_) => Reply::Invalid("Native grading panic"),
    };
    Box::into_raw(Box::new(reply))
}

#[unsafe(no_mangle)]
unsafe extern "C" fn genius_bench_go_new_task(
    input: *const InputTask,
    target: *mut *mut Task,
) -> *mut Reply {
    boundary(|| {
        let target = unsafe { target.as_mut() }.ok_or(Reply::Invalid("Missing task output"))?;
        *target = ptr::null_mut();
        let input = unsafe { input.as_ref() }.ok_or(Reply::Invalid("Missing task declaration"))?;
        *target = Box::into_raw(Box::new(input.task()?));
        Ok(None)
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn genius_bench_go_free_task(task: *mut Task) {
    if !task.is_null() {
        drop(unsafe { Box::from_raw(task) });
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn genius_bench_go_grade(
    task: *const Task,
    input: *const InputOutcome,
) -> *mut Reply {
    boundary(|| {
        let task = unsafe { task.as_ref() }.ok_or(Reply::Invalid("Missing task"))?;
        let input = unsafe { input.as_ref() }.ok_or(Reply::Invalid("Missing outcome"))?;
        task.grade(&input.outcome()?)
            .map(Some)
            .map_err(Reply::Grading)
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn genius_bench_go_grade_reference(
    task: *const Task,
    verification: InputVerification,
) -> *mut Reply {
    boundary(|| {
        let task = unsafe { task.as_ref() }.ok_or(Reply::Invalid("Missing task"))?;
        let reference = task.reference();
        let outcome = Outcome {
            changed_files: reference
                .edits
                .iter()
                .map(|file| file.path.clone())
                .collect(),
            verification: verification.verification()?,
            response: reference.completion.clone(),
            facts: Vec::new(),
        };
        task.grade(&outcome).map(Some).map_err(Reply::Grading)
    })
}

#[repr(C)]
struct ReplyInfo {
    tag: u8,
    count: usize,
    code: Text,
    detail: Text,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn genius_bench_go_info(reply: *const Reply) -> ReplyInfo {
    let (tag, count, code, detail) = match unsafe { reply.as_ref() } {
        Some(Reply::Scores(scores)) => (0, scores.len(), "", ""),
        Some(Reply::Validation(failure)) => (
            1,
            failure.issues.len(),
            failure.case_id.as_deref().unwrap_or_default(),
            "",
        ),
        Some(Reply::Grading(failure)) => {
            let GradingFailure::VerificationHarnessError { detail } = failure;
            (2, 0, failure.kind(), detail.as_str())
        }
        Some(Reply::Invalid(detail)) => (3, 0, "", *detail),
        None => (3, 0, "", "Missing grading result"),
    };
    ReplyInfo {
        tag,
        count,
        code: code.into(),
        detail: detail.into(),
    }
}

#[repr(C)]
struct Row {
    name: Text,
    message: Text,
    passed: u8,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn genius_bench_go_get_row(reply: *const Reply, index: usize) -> Row {
    let row = match unsafe { reply.as_ref() } {
        Some(Reply::Scores(scores)) => scores
            .get(index)
            .map(|(name, passed)| (name.as_str(), "", *passed)),
        Some(Reply::Validation(failure)) => failure
            .issues
            .get(index)
            .map(|issue| (issue.field.as_str(), issue.message.as_str(), false)),
        _ => None,
    };
    let (name, message, passed) = row.unwrap_or(("", "", false));
    Row {
        name: name.into(),
        message: message.into(),
        passed: u8::from(passed),
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn genius_bench_go_free_result(reply: *mut Reply) {
    if !reply.is_null() {
        drop(unsafe { Box::from_raw(reply) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_declaration_returns_core_issues_before_grading() {
        let input = InputTask {
            id: "bad/id".into(),
            prompt: "".into(),
            fixture: (&[][..]).into(),
            edits: (&[][..]).into(),
            present_path: (&[][..]).into(),
            allowed_files: (&[][..]).into(),
            completion: 0,
        };
        let mut task = ptr::dangling_mut();
        unsafe {
            let reply = genius_bench_go_new_task(&input, &mut task);
            assert!(task.is_null());
            let info = genius_bench_go_info(reply);
            assert_eq!((info.tag, info.count), (1, 1));
            assert_eq!(info.code.read().unwrap(), b"bad/id");
            let issue = genius_bench_go_get_row(reply, 0);
            assert_eq!(issue.name.read().unwrap(), b"id");
            assert_eq!(issue.message.read().unwrap(), b"Invalid case identity");
            genius_bench_go_free_result(reply);
        }
    }

    #[test]
    fn null_inputs_produce_a_readable_binding_error() {
        let mut task = ptr::dangling_mut();
        unsafe {
            let reply = genius_bench_go_new_task(ptr::null(), &mut task);
            assert!(task.is_null());
            let info = genius_bench_go_info(reply);
            assert_eq!(info.tag, 3);
            assert!(info.code.data.is_null());
            assert_eq!(info.detail.read().unwrap(), b"Missing task declaration");
            genius_bench_go_free_result(reply);
            genius_bench_go_free_result(ptr::null_mut());
            genius_bench_go_free_task(task);
        }
    }
}

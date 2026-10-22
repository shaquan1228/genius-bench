use crate::Value;

/* Outcome: what the harness observed, distinct from claims in a response. The
verification status stays typed so a harness fault is never an incorrect candidate;
open facts carry a tool trace, environment state, or anything else observed.
https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
https://docs.harborframework.com/tasks/verifier
https://arxiv.org/abs/2506.07982 (tau2-bench: state, actions, and messages per episode) */
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verification {
    Unchecked,
    Passed,
    Failed,
    InvalidSubmission,
    TimedOut,
    HarnessError { detail: String },
}

impl Verification {
    pub fn status(&self) -> &'static str {
        match self {
            Self::Unchecked => "unchecked",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::InvalidSubmission => "invalid_submission",
            Self::TimedOut => "timed_out",
            Self::HarnessError { .. } => "harness_error",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub changed_files: Vec<String>,
    pub verification: Verification,
    pub response: Value,
    pub facts: Vec<(String, Value)>,
}

impl Outcome {
    pub fn new(changed_files: Vec<String>, verification: Verification) -> Self {
        Self {
            changed_files,
            verification,
            response: Value::Null,
            facts: Vec::new(),
        }
    }

    pub fn observation(&self) -> Value {
        let mut fields = vec![
            (
                "changed_files".into(),
                Value::strings(self.changed_files.iter().cloned()),
            ),
            (
                "verification".into(),
                Value::String(self.verification.status().into()),
            ),
            ("response".into(), self.response.clone()),
        ];
        fields.extend(self.facts.iter().cloned());
        Value::Record(fields)
    }
}

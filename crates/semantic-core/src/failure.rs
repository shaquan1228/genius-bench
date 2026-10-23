use std::fmt;

/* Failures: a rejected declaration keeps every issue; a grading failure is a harness
fault, kept apart from an incorrect candidate.
https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents */
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationFailure {
    pub case_id: Option<String>,
    pub issues: Vec<Issue>,
}

impl fmt::Display for ValidationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(case_id) = &self.case_id {
            write!(formatter, "case {case_id:?}: ")?;
        }
        for (index, issue) in self.issues.iter().enumerate() {
            if index > 0 {
                formatter.write_str("; ")?;
            }
            write!(formatter, "{}: {}", issue.field, issue.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationFailure {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GradingFailure {
    VerificationHarnessError { detail: String },
}

impl GradingFailure {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::VerificationHarnessError { .. } => "verification_harness_error",
        }
    }
}

impl fmt::Display for GradingFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VerificationHarnessError { detail } => {
                write!(formatter, "Verification harness error: {detail}")
            }
        }
    }
}

impl std::error::Error for GradingFailure {}

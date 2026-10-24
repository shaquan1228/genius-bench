mod assessment;
mod failure;
mod outcome;
mod predicate;
mod support;
mod task;
mod value;

pub use assessment::{Assessment, Scores};
pub use failure::{GradingFailure, Issue, ValidationFailure};
pub use outcome::{Outcome, Verification};
pub use predicate::Predicate;
pub use task::{Case, Criterion, File, Instruction, Reference, Rubric, Task, TextRule, Verifier};
pub use value::Value;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SOURCE_HASH: &str = env!("SEMANTIC_CORE_SOURCE_HASH");
pub const REQUIRED_SCORES: [&str; 2] = ["correctness", "scope"];
pub const NESTING_LIMIT: usize = 256;

pub fn version() -> &'static str {
    VERSION
}

pub fn source_hash() -> &'static str {
    SOURCE_HASH
}

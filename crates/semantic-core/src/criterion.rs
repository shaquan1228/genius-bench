use crate::{File, Issue, Predicate, Value, support};

/* Verifier: an executable private check run on declared candidate inputs.
https://docs.harborframework.com/tasks/verifier */
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Verifier {
    pub inputs: Vec<String>,
    pub files: Vec<File>,
    pub tools: Vec<File>,
    pub command: Vec<String>,
    pub stdin: String,
    pub timeout_seconds: i64,
}

impl Verifier {
    pub(crate) fn artifact_paths(&self) -> impl Iterator<Item = &str> {
        self.files
            .iter()
            .chain(&self.tools)
            .map(|file| file.path.as_str())
    }

    pub(crate) fn validate_configuration(&self, issues: &mut Vec<Issue>) {
        let paths = self
            .inputs
            .iter()
            .map(String::as_str)
            .chain(self.artifact_paths());
        if !support::unique(paths) {
            issues.push(Issue::new("verifier", "Overlapping verifier paths"));
        }
        if self.artifact_paths().any(|path| !support::valid_path(path)) {
            issues.push(Issue::new("verifier", "Invalid verifier path"));
        }
        if self.command.is_empty() || self.command.iter().any(String::is_empty) {
            issues.push(Issue::new("verifier.command", "Invalid verifier command"));
        }
        if self.timeout_seconds <= 0 {
            issues.push(Issue::new(
                "verifier.timeout_seconds",
                "Verifier timeout must be positive",
            ));
        }
    }
}

/* Rubric: expected response fields and a bounded-word text rule, this repository's
deterministic subset of a grading rubric.
https://github.com/openai/evals/blob/main/docs/eval-templates.md#example-model-graded-evals */
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextRule {
    pub field: String,
    pub min_words: i64,
    pub max_words: i64,
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Rubric {
    pub expected: Vec<(String, Value)>,
    pub text_rule: Option<TextRule>,
}

impl Rubric {
    fn condition(&self) -> Predicate {
        let mut checks = vec![(
            "response".into(),
            Predicate::Present {
                path: vec!["response".into()],
            },
        )];
        for (field, expected) in &self.expected {
            let condition = Predicate::Equals {
                path: vec!["response".into(), field.clone()],
                expected: expected.clone(),
            };
            checks.push((field.clone(), condition));
        }
        if let Some(rule) = &self.text_rule {
            let condition = Predicate::Words {
                path: vec!["response".into(), rule.field.clone()],
                min_words: rule.min_words,
                max_words: rule.max_words,
            };
            checks.push(("text".into(), condition));
        }
        Predicate::All { checks }
    }

    pub(crate) fn validate(&self, issues: &mut Vec<Issue>) {
        if self.expected.is_empty() && self.text_rule.is_none() {
            issues.push(Issue::new("rubric", "Empty rubric"));
        }
        let Some(rule) = &self.text_rule else {
            return;
        };
        let field_is_already_expected = self.expected.iter().any(|(field, _)| field == &rule.field);
        if rule.min_words < 0 || rule.min_words > rule.max_words || field_is_already_expected {
            issues.push(Issue::new("rubric.text_rule", "Invalid text rule"));
        }
    }
}

/* Criterion: a declarative success condition applied by the grader: executable code, a
rubric over the response, or a predicate over the whole observation.
https://developers.openai.com/api/docs/guides/evals
https://arxiv.org/abs/2406.12045 (tau-bench: final state and required outputs) */
#[derive(Clone, Debug, PartialEq, Hash)]
pub enum Criterion {
    Code { verifier: Verifier },
    Answer { rubric: Rubric },
    State { predicate: Predicate },
}

impl Criterion {
    pub fn condition(&self) -> Predicate {
        match self {
            Self::Code { .. } => Predicate::Equals {
                path: vec!["verification".into()],
                expected: Value::String("passed".into()),
            },
            Self::Answer { rubric } => rubric.condition(),
            Self::State { predicate } => predicate.clone(),
        }
    }
}

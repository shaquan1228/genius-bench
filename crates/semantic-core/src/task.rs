use crate::{
    Assessment, GradingFailure, Issue, Outcome, Predicate, REQUIRED_SCORES, Scores,
    ValidationFailure, Value, Verification, support,
};

/* Task: one test specification combining instruction, initial files, a criterion, and a
reference. Anthropic defines a task as a single test with inputs and success criteria;
Hugging Face represents a benchmark sample with its query, expected outputs, and
metadata. Correctness and scope are separate scores, as utility and policy compliance
are in tau-bench and AgentDojo.
https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
https://huggingface.co/docs/lighteval/v0.13.0/package_reference/doc
https://arxiv.org/abs/2406.13352 */
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    id: String,
    fixture: Vec<File>,
    instruction: Instruction,
    criterion: Criterion,
    allowed_files: Vec<String>,
    reference: Reference,
}

/* Fixture files: immutable initial state, the seed for file and non-file environments.
https://inspect.aisi.org.uk/datasets.html#sample-files
https://docs.harborframework.com/tasks/environment */
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub path: String,
    pub contents: String,
}

/* Instruction: the task input, mapping to lighteval's query and OpenAI eval inputs.
https://huggingface.co/docs/lighteval/main/en/package_reference/doc */
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub prompt: String,
}

/* Verifier: an executable private check run on declared candidate inputs.
https://docs.harborframework.com/tasks/verifier */
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verifier {
    pub inputs: Vec<String>,
    pub files: Vec<File>,
    pub tools: Vec<File>,
    pub command: Vec<String>,
    pub stdin: String,
    pub timeout_seconds: i64,
}

/* Rubric: expected response fields and a bounded-word text rule, this repository's
deterministic subset of a grading rubric.
https://github.com/openai/evals/blob/main/docs/eval-templates.md#example-model-graded-evals */
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextRule {
    pub field: String,
    pub min_words: i64,
    pub max_words: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rubric {
    pub expected: Vec<(String, Value)>,
    pub text_rule: Option<TextRule>,
}

/* Criterion: a declarative success condition applied by the grader: executable code, a
rubric over the response, or a predicate over the whole observation.
https://developers.openai.com/api/docs/guides/evals
https://arxiv.org/abs/2406.12045 (tau-bench: final state and required outputs) */
#[derive(Clone, Debug, PartialEq)]
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
            Self::State { predicate } => predicate.clone(),
            Self::Answer { rubric } => {
                let mut checks = vec![(
                    "response".into(),
                    Predicate::Present {
                        path: vec!["response".into()],
                    },
                )];
                checks.extend(rubric.expected.iter().map(|(name, expected)| {
                    (
                        name.clone(),
                        Predicate::Equals {
                            path: vec!["response".into(), name.clone()],
                            expected: expected.clone(),
                        },
                    )
                }));
                if let Some(rule) = &rubric.text_rule {
                    checks.push((
                        "text".into(),
                        Predicate::Words {
                            path: vec!["response".into(), rule.field.clone()],
                            min_words: rule.min_words,
                            max_words: rule.max_words,
                        },
                    ));
                }
                Predicate::All { checks }
            }
        }
    }
}

/* Reference: reference edits and completion for offline validation.
https://evalscope.readthedocs.io/en/latest/advanced_guides/custom_dataset/llm.html */
#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    pub edits: Vec<File>,
    pub completion: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    pub case_id: String,
    pub prompt: String,
    pub files: Vec<File>,
}

impl Task {
    pub fn new(
        id: String,
        fixture: Vec<File>,
        instruction: Instruction,
        criterion: Criterion,
        allowed_files: Vec<String>,
        reference: Reference,
    ) -> Result<Self, ValidationFailure> {
        let fixture_paths: Vec<_> = fixture.iter().map(|file| file.path.as_str()).collect();
        let edits: Vec<_> = reference
            .edits
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        let mut issues = Vec::new();
        let mut require = |ok: bool, field: &str, message: &str| {
            if !ok {
                issues.push(Issue {
                    field: field.into(),
                    message: message.into(),
                });
            }
        };
        require(support::valid_name(&id), "id", "Invalid case identity");
        require(
            fixture_paths
                .iter()
                .copied()
                .chain(allowed_files.iter().map(String::as_str))
                .chain(edits.iter().copied())
                .all(support::valid_path),
            "fixture",
            "Invalid path",
        );
        require(
            support::unique(fixture_paths.iter().copied())
                && support::unique(edits.iter().copied()),
            "fixture",
            "Duplicate path",
        );
        require(
            edits
                .iter()
                .all(|path| allowed_files.iter().any(|allowed| allowed == path)),
            "reference.edits",
            "Reference edits files outside allowed_files",
        );
        match &criterion {
            Criterion::Code { verifier } => {
                let artifacts: Vec<_> = verifier
                    .files
                    .iter()
                    .chain(&verifier.tools)
                    .map(|file| file.path.as_str())
                    .collect();
                require(
                    !verifier.inputs.is_empty(),
                    "verifier.inputs",
                    "Verifier inputs cannot be empty",
                );
                require(
                    verifier
                        .inputs
                        .iter()
                        .all(|input| fixture_paths.contains(&input.as_str())),
                    "verifier.inputs",
                    "Criteria read files missing from the fixture",
                );
                require(
                    !artifacts.iter().any(|path| fixture_paths.contains(path)),
                    "verifier",
                    "Verifier artifacts overlap fixture",
                );
                require(
                    support::unique(
                        verifier
                            .inputs
                            .iter()
                            .map(String::as_str)
                            .chain(artifacts.iter().copied()),
                    ),
                    "verifier",
                    "Overlapping verifier paths",
                );
                require(
                    artifacts.iter().copied().all(support::valid_path),
                    "verifier",
                    "Invalid verifier path",
                );
                require(
                    !verifier.command.is_empty()
                        && verifier.command.iter().all(|part| !part.is_empty()),
                    "verifier.command",
                    "Invalid verifier command",
                );
                require(
                    verifier.timeout_seconds > 0,
                    "verifier.timeout_seconds",
                    "Verifier timeout must be positive",
                );
            }
            Criterion::Answer { rubric } => {
                require(
                    !rubric.expected.is_empty() || rubric.text_rule.is_some(),
                    "rubric",
                    "Empty rubric",
                );
                require(
                    rubric.text_rule.as_ref().is_none_or(|rule| {
                        rule.min_words >= 0
                            && rule.min_words <= rule.max_words
                            && !rubric
                                .expected
                                .iter()
                                .any(|(field, _)| field == &rule.field)
                    }),
                    "rubric.text_rule",
                    "Invalid text rule",
                );
            }
            Criterion::State { .. } => {}
        }
        if issues.is_empty() {
            Ok(Self {
                id,
                fixture,
                instruction,
                criterion,
                allowed_files,
                reference,
            })
        } else {
            Err(ValidationFailure {
                case_id: Some(id),
                issues,
            })
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn fixture(&self) -> &[File] {
        &self.fixture
    }
    pub fn instruction(&self) -> &Instruction {
        &self.instruction
    }
    pub fn criterion(&self) -> &Criterion {
        &self.criterion
    }
    pub fn allowed_files(&self) -> &[String] {
        &self.allowed_files
    }
    pub fn reference(&self) -> &Reference {
        &self.reference
    }

    pub fn checks(&self) -> Option<&Verifier> {
        match &self.criterion {
            Criterion::Code { verifier } => Some(verifier),
            _ => None,
        }
    }

    pub fn project(&self) -> Case {
        let mut files = self.fixture.clone();
        if let Some(verifier) = self.checks() {
            files.extend(verifier.tools.iter().cloned());
        }
        Case {
            case_id: self.id.clone(),
            prompt: self.instruction.prompt.clone(),
            files,
        }
    }

    /* Grader: pure logic applying the criterion and the file scope to an observed outcome.
    IO, execution, and presentation remain harness responsibilities.
    https://developers.openai.com/api/docs/guides/graders */
    pub fn assess(&self, outcome: &Outcome) -> Result<Assessment, GradingFailure> {
        if let Verification::HarnessError { detail } = &outcome.verification {
            return Err(GradingFailure::VerificationHarnessError {
                detail: detail.clone(),
            });
        }
        let scope = Predicate::Within {
            path: vec!["changed_files".into()],
            allowed: self
                .allowed_files
                .iter()
                .map(|name| Value::String(name.clone()))
                .collect(),
        };
        Ok(Assessment::new(
            Predicate::All {
                checks: vec![
                    (REQUIRED_SCORES[0].into(), self.criterion.condition()),
                    (REQUIRED_SCORES[1].into(), scope),
                ],
            },
            outcome.observation(),
        ))
    }

    pub fn grade(&self, outcome: &Outcome) -> Result<Scores, GradingFailure> {
        self.assess(outcome).map(|assessment| assessment.scores())
    }
}

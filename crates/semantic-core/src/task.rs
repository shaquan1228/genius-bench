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
#[derive(Clone, Debug, PartialEq, Hash)]
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
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct File {
    pub path: String,
    pub contents: String,
}

/* Instruction: the task input, mapping to lighteval's query and OpenAI eval inputs.
https://huggingface.co/docs/lighteval/main/en/package_reference/doc */
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Instruction {
    pub prompt: String,
}

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
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Reference {
    pub edits: Vec<File>,
    pub completion: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
        let reference_paths: Vec<_> = reference
            .edits
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        let mut issues = Vec::new();
        require(
            &mut issues,
            support::valid_name(&id),
            "id",
            "Invalid case identity",
        );
        validate_file_paths(&mut issues, "fixture", &fixture_paths);
        require(
            &mut issues,
            allowed_files.iter().all(|path| support::valid_path(path)),
            "allowed_files",
            "Invalid path",
        );
        validate_file_paths(&mut issues, "reference.edits", &reference_paths);

        let edits_within_scope = reference
            .edits
            .iter()
            .all(|file| allowed_files.contains(&file.path));
        require(
            &mut issues,
            edits_within_scope,
            "reference.edits",
            "Reference edits files outside allowed_files",
        );

        match &criterion {
            Criterion::Code { verifier } => {
                validate_verifier(&mut issues, verifier, &fixture_paths);
            }
            Criterion::Answer { rubric } => validate_rubric(&mut issues, rubric),
            Criterion::State { .. } => {}
        }

        if !issues.is_empty() {
            return Err(ValidationFailure {
                case_id: Some(id),
                issues,
            });
        }
        Ok(Self {
            id,
            fixture,
            instruction,
            criterion,
            allowed_files,
            reference,
        })
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

fn validate_file_paths(issues: &mut Vec<Issue>, field: &str, paths: &[&str]) {
    require(
        issues,
        paths.iter().copied().all(support::valid_path),
        field,
        "Invalid path",
    );
    require(
        issues,
        support::unique(paths.iter().copied()),
        field,
        "Duplicate path",
    );
}

fn validate_verifier(issues: &mut Vec<Issue>, verifier: &Verifier, fixture_paths: &[&str]) {
    require(
        issues,
        !verifier.inputs.is_empty(),
        "verifier.inputs",
        "Verifier inputs cannot be empty",
    );
    let inputs_in_fixture = verifier
        .inputs
        .iter()
        .all(|input| fixture_paths.contains(&input.as_str()));
    require(
        issues,
        inputs_in_fixture,
        "verifier.inputs",
        "Criteria read files missing from the fixture",
    );

    let artifact_paths: Vec<_> = verifier
        .files
        .iter()
        .chain(&verifier.tools)
        .map(|file| file.path.as_str())
        .collect();
    let artifacts_overlap_fixture = artifact_paths
        .iter()
        .any(|path| fixture_paths.contains(path));
    require(
        issues,
        !artifacts_overlap_fixture,
        "verifier",
        "Verifier artifacts overlap fixture",
    );
    let verifier_paths = verifier
        .inputs
        .iter()
        .map(String::as_str)
        .chain(artifact_paths.iter().copied());
    require(
        issues,
        support::unique(verifier_paths),
        "verifier",
        "Overlapping verifier paths",
    );
    require(
        issues,
        artifact_paths.iter().copied().all(support::valid_path),
        "verifier",
        "Invalid verifier path",
    );

    let command_is_valid =
        !verifier.command.is_empty() && verifier.command.iter().all(|part| !part.is_empty());
    require(
        issues,
        command_is_valid,
        "verifier.command",
        "Invalid verifier command",
    );
    require(
        issues,
        verifier.timeout_seconds > 0,
        "verifier.timeout_seconds",
        "Verifier timeout must be positive",
    );
}

fn validate_rubric(issues: &mut Vec<Issue>, rubric: &Rubric) {
    require(
        issues,
        !rubric.expected.is_empty() || rubric.text_rule.is_some(),
        "rubric",
        "Empty rubric",
    );
    let Some(rule) = &rubric.text_rule else {
        return;
    };
    let bounds_are_valid = rule.min_words >= 0 && rule.min_words <= rule.max_words;
    let text_field_is_expected = rubric
        .expected
        .iter()
        .any(|(field, _)| field == &rule.field);
    require(
        issues,
        bounds_are_valid && !text_field_is_expected,
        "rubric.text_rule",
        "Invalid text rule",
    );
}

fn require(issues: &mut Vec<Issue>, condition: bool, field: &str, message: &str) {
    if !condition {
        issues.push(Issue {
            field: field.into(),
            message: message.into(),
        });
    }
}

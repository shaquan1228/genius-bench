use crate::{
    Assessment, Criterion, File, GradingFailure, Issue, Outcome, Predicate, REQUIRED_SCORES,
    Scores, ValidationFailure, Value, Verification, Verifier, support,
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

/* Instruction: the task input, mapping to lighteval's query and OpenAI eval inputs.
https://huggingface.co/docs/lighteval/main/en/package_reference/doc */
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Instruction {
    pub prompt: String,
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
        if !support::valid_name(&id) {
            issues.push(Issue::new("id", "Invalid case identity"));
        }
        if fixture_paths.iter().any(|path| !File::is_valid_path(path)) {
            issues.push(Issue::new("fixture", "Invalid path"));
        }
        if !support::unique(fixture_paths.iter().copied()) {
            issues.push(Issue::new("fixture", "Duplicate path"));
        }

        if allowed_files.iter().any(|path| !File::is_valid_path(path)) {
            issues.push(Issue::new("allowed_files", "Invalid path"));
        }
        if reference_paths
            .iter()
            .any(|path| !File::is_valid_path(path))
        {
            issues.push(Issue::new("reference.edits", "Invalid path"));
        }
        if !support::unique(reference_paths.iter().copied()) {
            issues.push(Issue::new("reference.edits", "Duplicate path"));
        }

        let edits_outside_scope = reference
            .edits
            .iter()
            .any(|file| !allowed_files.contains(&file.path));
        if edits_outside_scope {
            issues.push(Issue::new(
                "reference.edits",
                "Reference edits files outside allowed_files",
            ));
        }

        match &criterion {
            Criterion::Code { verifier } => {
                if verifier.inputs.is_empty() {
                    issues.push(Issue::new(
                        "verifier.inputs",
                        "Verifier inputs cannot be empty",
                    ));
                }
                let missing_fixture_input = verifier
                    .inputs
                    .iter()
                    .any(|input| !fixture_paths.contains(&input.as_str()));
                if missing_fixture_input {
                    issues.push(Issue::new(
                        "verifier.inputs",
                        "Criteria read files missing from the fixture",
                    ));
                }
                let artifacts_overlap_fixture = verifier
                    .artifact_paths()
                    .any(|path| fixture_paths.contains(&path));
                if artifacts_overlap_fixture {
                    issues.push(Issue::new("verifier", "Verifier artifacts overlap fixture"));
                }
                verifier.validate_configuration(&mut issues);
            }
            Criterion::Answer { rubric } => rubric.validate(&mut issues),
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
        let assessment = self.assess(outcome)?;
        Ok(assessment.scores())
    }
}

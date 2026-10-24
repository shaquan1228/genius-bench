use crate::{Predicate, Value};

/* Assessment: a predicate evaluated against one observation, retaining the observation
behind every check so a result explains itself. Named conjunctions become scores.
https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents
https://developers.openai.com/api/docs/guides/graders */
#[derive(Clone, Debug, PartialEq, Hash)]
pub enum Assessment {
    Check {
        predicate: Predicate,
        observation: Value,
    },
    All {
        checks: Vec<(String, Self)>,
    },
}

pub type Scores = Vec<(String, bool)>;

impl Assessment {
    pub fn new(predicate: Predicate, observation: Value) -> Self {
        match predicate {
            Predicate::All { checks } => Self::All {
                checks: checks
                    .into_iter()
                    .map(|(name, check)| (name, Self::new(check, observation.clone())))
                    .collect(),
            },
            predicate => Self::Check {
                predicate,
                observation,
            },
        }
    }

    pub fn passed(&self) -> bool {
        match self {
            Self::Check {
                predicate,
                observation,
            } => predicate.holds(observation),
            Self::All { checks } => checks.iter().all(|(_, check)| check.passed()),
        }
    }

    pub fn observed(&self) -> Option<&Value> {
        match self {
            Self::Check {
                predicate,
                observation,
            } => predicate.path().and_then(|path| observation.get(path)),
            Self::All { .. } => None,
        }
    }

    pub fn scores(&self) -> Scores {
        match self {
            Self::All { checks } => checks
                .iter()
                .map(|(name, check)| (name.clone(), check.passed()))
                .collect(),
            Self::Check { .. } => vec![("passed".into(), self.passed())],
        }
    }
}

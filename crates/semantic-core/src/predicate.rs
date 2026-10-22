use crate::{Value, support};

/* Predicate: the assertion algebra every criterion compiles to. One evaluator grades
file edits, structured replies, tool traces, and environment state.
https://arxiv.org/abs/2307.13854 (WebArena: exact_match, must_include, located state)
https://arxiv.org/abs/2406.13352 (AgentDojo: utility and security as separate predicates) */
#[derive(Clone, Debug, PartialEq)]
pub enum Predicate {
    Present {
        path: Vec<String>,
    },
    Equals {
        path: Vec<String>,
        expected: Value,
    },
    Within {
        path: Vec<String>,
        allowed: Vec<Value>,
    },
    Contains {
        path: Vec<String>,
        item: Value,
    },
    Words {
        path: Vec<String>,
        min_words: i64,
        max_words: i64,
    },
    All {
        checks: Vec<(String, Self)>,
    },
    Any {
        checks: Vec<(String, Self)>,
    },
    Not {
        check: Box<Self>,
    },
}

impl Predicate {
    pub fn path(&self) -> Option<&[String]> {
        match self {
            Self::Present { path }
            | Self::Equals { path, .. }
            | Self::Within { path, .. }
            | Self::Contains { path, .. }
            | Self::Words { path, .. } => Some(path),
            _ => None,
        }
    }

    pub fn holds(&self, observation: &Value) -> bool {
        match self {
            Self::Present { path } => observation
                .get(path)
                .is_some_and(|value| *value != Value::Null),
            Self::Equals { path, expected } => {
                observation.get(path).is_some_and(|value| value == expected)
            }
            Self::Within { path, allowed } => {
                matches!(observation.get(path), Some(Value::List(items)) if items.iter().all(|value| allowed.contains(value)))
            }
            Self::Contains { path, item } => {
                matches!(observation.get(path), Some(Value::List(items)) if items.contains(item))
            }
            Self::Words {
                path,
                min_words,
                max_words,
            } => {
                matches!(observation.get(path), Some(Value::String(text)) if (*min_words..=*max_words).contains(&support::word_count(text)))
            }
            Self::All { checks } => checks.iter().all(|(_, check)| check.holds(observation)),
            Self::Any { checks } => checks.iter().any(|(_, check)| check.holds(observation)),
            Self::Not { check } => !check.holds(observation),
        }
    }
}

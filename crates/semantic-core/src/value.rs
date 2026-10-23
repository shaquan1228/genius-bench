/* Value: one JSON-shaped domain for inputs, expectations, and observations, so tasks
state goals over files, tool traces, database state, or structured replies alike.
https://arxiv.org/abs/2406.12045 (tau-bench grades a final database state)
https://arxiv.org/abs/2307.13854 (WebArena grades answers and located page state) */
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Record(Vec<(String, Value)>),
}

impl Value {
    pub fn strings(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::List(
            values
                .into_iter()
                .map(|item| Self::String(item.into()))
                .collect(),
        )
    }

    pub fn get(&self, path: &[impl AsRef<str>]) -> Option<&Self> {
        let mut current = self;
        for name in path {
            let Self::Record(fields) = current else {
                return None;
            };
            current = &fields.iter().find(|(key, _)| key == name.as_ref())?.1;
        }
        Some(current)
    }

    pub fn nesting_depth(&self) -> usize {
        let mut deepest = 0;
        let mut pending = vec![(self, 0)];
        while let Some((value, depth)) = pending.pop() {
            match value {
                Self::List(items) => {
                    deepest = deepest.max(depth + 1);
                    pending.extend(items.iter().map(|item| (item, depth + 1)));
                }
                Self::Record(fields) => {
                    deepest = deepest.max(depth + 1);
                    pending.extend(fields.iter().map(|(_, item)| (item, depth + 1)));
                }
                _ => {}
            }
        }
        deepest
    }

    pub fn equal(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::List(left), Self::List(right)) => left == right,
            (Self::Record(left), Self::Record(right)) => {
                fn first(fields: &[(String, Value)]) -> std::collections::BTreeMap<&str, &Value> {
                    let mut unique = std::collections::BTreeMap::new();
                    for (name, value) in fields {
                        unique.entry(name.as_str()).or_insert(value);
                    }
                    unique
                }
                first(left) == first(right)
            }
            _ => false,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.equal(other)
    }
}

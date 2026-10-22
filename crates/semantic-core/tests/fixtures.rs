use semantic_core::{
    Assessment, Case, Criterion, File, GradingFailure, Instruction, Outcome, Predicate, Reference,
    Rubric, Task, TextRule, ValidationFailure, Value, Verification,
};
use serde_json::{Value as Json, json};

fn string(value: &Json) -> String {
    value.as_str().unwrap().to_owned()
}

fn strings(input: &Json) -> Vec<String> {
    input.as_array().unwrap().iter().map(string).collect()
}

fn pairs<T>(input: &Json, parse: impl Fn(&Json) -> T) -> Vec<(String, T)> {
    input
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| (string(&pair[0]), parse(&pair[1])))
        .collect()
}

fn value(input: &Json) -> Value {
    match input["type"].as_str().unwrap() {
        "null" => Value::Null,
        "bool" => Value::Bool(input["value"].as_bool().unwrap()),
        "int" => Value::Int(input["value"].as_str().unwrap().parse().unwrap()),
        "float" => Value::Float(input["value"].as_str().unwrap().parse().unwrap()),
        "string" => Value::String(string(&input["value"])),
        "list" => Value::List(
            input["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(value)
                .collect(),
        ),
        "record" => Value::Record(pairs(&input["fields"], value)),
        other => panic!("unknown value type: {other}"),
    }
}

fn tagged(kind: &str, fields: Json) -> Json {
    let mut fields = fields.as_object().unwrap().clone();
    fields.insert("type".into(), kind.into());
    fields.into()
}

fn render_value(input: &Value) -> Json {
    match input {
        Value::Null => tagged("null", json!({})),
        Value::Bool(value) => tagged("bool", json!({"value": value})),
        Value::Int(value) => tagged("int", json!({"value": value.to_string()})),
        Value::Float(value) => tagged(
            "float",
            json!({"value": if value.is_nan() { "nan".into() } else { value.to_string() }}),
        ),
        Value::String(value) => tagged("string", json!({"value": value})),
        Value::List(items) => tagged(
            "list",
            json!({"items": items.iter().map(render_value).collect::<Vec<_>>()}),
        ),
        Value::Record(fields) => tagged(
            "record",
            json!({"fields": fields.iter().map(|(name, item)| json!([name, render_value(item)])).collect::<Vec<_>>()}),
        ),
    }
}

fn predicate(input: &Json) -> Predicate {
    let path = || strings(&input["path"]);
    match input["type"].as_str().unwrap() {
        "present" => Predicate::Present { path: path() },
        "equals" => Predicate::Equals {
            path: path(),
            expected: value(&input["expected"]),
        },
        "within" => Predicate::Within {
            path: path(),
            allowed: input["allowed"]
                .as_array()
                .unwrap()
                .iter()
                .map(value)
                .collect(),
        },
        "contains" => Predicate::Contains {
            path: path(),
            item: value(&input["item"]),
        },
        "words" => Predicate::Words {
            path: path(),
            min_words: input["min_words"].as_i64().unwrap(),
            max_words: input["max_words"].as_i64().unwrap(),
        },
        "all" => Predicate::All {
            checks: pairs(&input["checks"], predicate),
        },
        "any" => Predicate::Any {
            checks: pairs(&input["checks"], predicate),
        },
        "not" => Predicate::Not {
            check: Box::new(predicate(&input["check"])),
        },
        other => panic!("unknown predicate: {other}"),
    }
}

fn render_predicate(input: &Predicate) -> Json {
    match input {
        Predicate::Present { path } => tagged("present", json!({"path": path})),
        Predicate::Equals { path, expected } => tagged(
            "equals",
            json!({"path": path, "expected": render_value(expected)}),
        ),
        Predicate::Within { path, allowed } => tagged(
            "within",
            json!({"path": path, "allowed": allowed.iter().map(render_value).collect::<Vec<_>>()}),
        ),
        Predicate::Contains { path, item } => tagged(
            "contains",
            json!({"path": path, "item": render_value(item)}),
        ),
        Predicate::Words {
            path,
            min_words,
            max_words,
        } => tagged(
            "words",
            json!({"path": path, "min_words": min_words, "max_words": max_words}),
        ),
        Predicate::All { checks } => tagged(
            "all",
            json!({"checks": checks.iter().map(|(name, check)| json!([name, render_predicate(check)])).collect::<Vec<_>>()}),
        ),
        Predicate::Any { checks } => tagged(
            "any",
            json!({"checks": checks.iter().map(|(name, check)| json!([name, render_predicate(check)])).collect::<Vec<_>>()}),
        ),
        Predicate::Not { check } => tagged("not", json!({"check": render_predicate(check)})),
    }
}

fn render_assessment(input: &Assessment) -> Json {
    let detail = match input {
        Assessment::Check {
            predicate,
            observation,
        } => tagged(
            "check",
            json!({"predicate": render_predicate(predicate), "observation": render_value(observation)}),
        ),
        Assessment::All { checks } => tagged(
            "all",
            json!({"checks": checks.iter().map(|(name, check)| json!([name, render_assessment(check)])).collect::<Vec<_>>()}),
        ),
    };
    json!({"detail": detail, "passed": input.passed(), "observed": input.observed().map(render_value), "scores": input.scores()})
}

fn files(input: &Json) -> Vec<File> {
    input
        .as_array()
        .unwrap()
        .iter()
        .map(|item| File {
            path: string(&item["path"]),
            contents: string(&item["contents"]),
        })
        .collect()
}

fn verifier(input: &Json) -> semantic_core::Verifier {
    semantic_core::Verifier {
        inputs: strings(&input["inputs"]),
        files: files(&input["files"]),
        tools: files(&input["tools"]),
        command: strings(&input["command"]),
        stdin: string(&input["stdin"]),
        timeout_seconds: input["timeout_seconds"].as_i64().unwrap(),
    }
}

fn render_files(input: &[File]) -> Json {
    json!(
        input
            .iter()
            .map(|item| json!({"path": item.path, "contents": item.contents}))
            .collect::<Vec<_>>()
    )
}

fn render_verifier(input: &semantic_core::Verifier) -> Json {
    json!({"inputs": input.inputs, "files": render_files(&input.files), "tools": render_files(&input.tools), "command": input.command, "stdin": input.stdin, "timeout_seconds": input.timeout_seconds})
}

fn criterion(input: &Json) -> Criterion {
    match input["type"].as_str().unwrap() {
        "code" => Criterion::Code {
            verifier: verifier(&input["verifier"]),
        },
        "answer" => {
            let rubric = &input["rubric"];
            Criterion::Answer {
                rubric: Rubric {
                    expected: pairs(&rubric["expected"], value),
                    text_rule: (!rubric["text_rule"].is_null()).then(|| {
                        let rule = &rubric["text_rule"];
                        TextRule {
                            field: string(&rule["field"]),
                            min_words: rule["min_words"].as_i64().unwrap(),
                            max_words: rule["max_words"].as_i64().unwrap(),
                        }
                    }),
                },
            }
        }
        "state" => Criterion::State {
            predicate: predicate(&input["predicate"]),
        },
        other => panic!("unknown criterion: {other}"),
    }
}

fn task(input: &Json) -> Result<Task, ValidationFailure> {
    Task::new(
        string(&input["id"]),
        files(&input["fixture"]),
        Instruction {
            prompt: string(&input["instruction"]["prompt"]),
        },
        criterion(&input["criterion"]),
        strings(&input["allowed_files"]),
        Reference {
            edits: files(&input["reference"]["edits"]),
            completion: value(&input["reference"]["completion"]),
        },
    )
}

fn outcome(input: &Json) -> Outcome {
    let status = &input["verification"];
    let verification = match status["type"].as_str().unwrap() {
        "unchecked" => Verification::Unchecked,
        "passed" => Verification::Passed,
        "failed" => Verification::Failed,
        "invalid_submission" => Verification::InvalidSubmission,
        "timed_out" => Verification::TimedOut,
        "harness_error" => Verification::HarnessError {
            detail: string(&status["detail"]),
        },
        other => panic!("unknown status: {other}"),
    };
    Outcome {
        changed_files: strings(&input["changed_files"]),
        verification,
        response: value(&input["response"]),
        facts: pairs(&input["facts"], value),
    }
}

fn render_validation(input: &ValidationFailure) -> Json {
    json!({"case_id": input.case_id, "issues": input.issues.iter().map(|item| json!({"field": item.field, "message": item.message})).collect::<Vec<_>>()})
}

fn render_case(input: &Case) -> Json {
    json!({"case_id": input.case_id, "prompt": input.prompt, "files": render_files(&input.files)})
}

#[test]
fn captured_legacy_semantics() {
    let corpus: Json =
        serde_json::from_str(include_str!("../../../fixtures/semantics.json")).unwrap();
    assert_eq!(
        corpus["source_revision"],
        "523d1eede610171d825fde0fee12d418dd1ef981"
    );
    let cases = corpus["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 163);
    for case in cases {
        let input = &case["input"];
        let mut expected = case["expected"].clone();
        if case["category"] == "task" {
            match case["id"].as_str().unwrap() {
                "invalid-skill" => {
                    let baseline = cases
                        .iter()
                        .find(|item| item["category"] == "task" && item["id"] == "code")
                        .unwrap();
                    let mut generic_input = input.clone();
                    assert_eq!(
                        generic_input["instruction"]
                            .as_object_mut()
                            .unwrap()
                            .remove("skill"),
                        Some(json!("bad_skill"))
                    );
                    let mut baseline_input = baseline["input"].clone();
                    assert_eq!(
                        baseline_input["instruction"]
                            .as_object_mut()
                            .unwrap()
                            .remove("skill"),
                        Some(json!("reasoning"))
                    );
                    assert_eq!(generic_input, baseline_input);
                    expected = baseline["expected"].clone();
                }
                "combined-issues" => {
                    let removed = expected["failure"]["issues"]
                        .as_array_mut()
                        .unwrap()
                        .remove(1);
                    assert_eq!(
                        removed,
                        json!({"field": "instruction.skill", "message": "Invalid instruction skill"})
                    );
                }
                _ => {}
            }
            if let Some(projected) = expected.get_mut("case") {
                assert_eq!(
                    projected.as_object_mut().unwrap().remove("skill"),
                    Some(json!("reasoning"))
                );
            }
        }
        let actual = match case["category"].as_str().unwrap() {
            "equal" => json!(value(&input["left"]).equal(&value(&input["right"]))),
            "get" => json!(
                value(&input["value"])
                    .get(&strings(&input["path"]))
                    .map(render_value)
            ),
            "predicate" => {
                let predicate = predicate(&input["predicate"]);
                let observation = value(&input["observation"]);
                json!({"holds": predicate.holds(&observation), "assessment": render_assessment(&Assessment::new(predicate, observation))})
            }
            "task" => match task(input) {
                Ok(task) => {
                    json!({"case": render_case(&task.project()), "checks": task.checks().map(render_verifier), "condition": render_predicate(&task.criterion().condition())})
                }
                Err(failure) => json!({"failure": render_validation(&failure)}),
            },
            "grade" => {
                let task = task(&input["task"]).unwrap();
                let outcome = outcome(&input["outcome"]);
                match task.assess(&outcome) {
                    Ok(assessment) => {
                        json!({"assessment": render_assessment(&assessment), "scores": task.grade(&outcome).unwrap(), "observation": render_value(&outcome.observation())})
                    }
                    Err(GradingFailure::VerificationHarnessError { detail }) => {
                        assert_eq!(
                            task.grade(&outcome),
                            Err(GradingFailure::VerificationHarnessError {
                                detail: detail.clone()
                            })
                        );
                        json!({"failure": tagged("verification_harness_error", json!({"detail": detail}))})
                    }
                }
            }
            other => panic!("unknown category: {other}"),
        };
        assert_eq!(actual, expected, "fixture {}", case["id"]);
    }
}

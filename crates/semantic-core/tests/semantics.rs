use semantic_core::{
    Assessment, Case, Criterion, File, GradingFailure, Instruction, NESTING_LIMIT, Outcome,
    Predicate, REQUIRED_SCORES, Reference, Rubric, SOURCE_HASH, Task, VERSION, Value, Verification,
    Verifier,
};
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "../build.rs"]
mod build_identity;

fn state_task() -> Task {
    Task::new(
        "state-example".into(),
        vec![],
        Instruction {
            prompt: "Report custom state.".into(),
        },
        Criterion::State {
            predicate: Predicate::All {
                checks: vec![
                    (
                        "balance".into(),
                        Predicate::Equals {
                            path: vec!["custom".into(), "balance".into()],
                            expected: Value::Int(5),
                        },
                    ),
                    (
                        "credited".into(),
                        Predicate::Contains {
                            path: vec!["trace".into()],
                            item: Value::Record(vec![
                                ("operation".into(), Value::String("credit".into())),
                                ("amount".into(), Value::Int(5)),
                            ]),
                        },
                    ),
                ],
            },
        },
        vec!["report.txt".into()],
        Reference {
            edits: vec![],
            completion: Value::Null,
        },
    )
    .unwrap()
}

#[test]
fn owned_task_and_projection_snapshots_preserve_invariants() {
    let mut fixture = vec![File {
        path: "source.txt".into(),
        contents: "seed".into(),
    }];
    let mut instruction = Instruction {
        prompt: "Complete the response.".into(),
    };
    let mut allowed = vec!["source.txt".into()];
    let mut rubric = Rubric {
        expected: vec![("answer".into(), Value::Int(7))],
        text_rule: None,
    };
    let mut reference = Reference {
        edits: fixture.clone(),
        completion: Value::String("private reference".into()),
    };
    let task = Task::new(
        "snapshot".into(),
        fixture.clone(),
        instruction.clone(),
        Criterion::Answer {
            rubric: rubric.clone(),
        },
        allowed.clone(),
        reference.clone(),
    )
    .unwrap();
    fixture[0].path = "../invalid".into();
    fixture[0].contents = "modified".into();
    instruction.prompt.clear();
    allowed.clear();
    rubric.expected.clear();
    reference.edits.clear();
    reference.completion = Value::Null;
    let mut projection = task.project();
    projection.files[0].path.clear();
    projection.prompt.clear();
    assert_eq!(task.id(), "snapshot");
    assert_eq!(task.fixture()[0].path, "source.txt");
    assert_eq!(task.fixture()[0].contents, "seed");
    assert_eq!(task.instruction().prompt, "Complete the response.");
    assert_eq!(
        task.project(),
        Case {
            case_id: "snapshot".into(),
            prompt: "Complete the response.".into(),
            files: vec![File {
                path: "source.txt".into(),
                contents: "seed".into(),
            }],
        }
    );
    assert_eq!(task.allowed_files(), ["source.txt"]);
    assert_eq!(task.reference().edits.len(), 1);
    assert_eq!(
        task.reference().completion,
        Value::String("private reference".into())
    );
    assert!(task.checks().is_none());
    let mut outcome = Outcome::new(vec![], Verification::Unchecked);
    outcome.response = Value::Record(vec![("answer".into(), Value::Int(7))]);
    assert_eq!(
        task.grade(&outcome).unwrap(),
        [("correctness".into(), true), ("scope".into(), true)]
    );
}

#[test]
fn custom_state_and_scope_are_independent() {
    let task = state_task();
    let mut outcome = Outcome::new(vec!["outside.txt".into()], Verification::Unchecked);
    outcome.facts = vec![
        (
            "custom".into(),
            Value::Record(vec![("balance".into(), Value::Int(5))]),
        ),
        (
            "trace".into(),
            Value::List(vec![Value::Record(vec![
                ("operation".into(), Value::String("credit".into())),
                ("amount".into(), Value::Int(5)),
            ])]),
        ),
    ];
    assert_eq!(
        task.project(),
        Case {
            case_id: "state-example".into(),
            prompt: "Report custom state.".into(),
            files: vec![],
        }
    );
    assert_eq!(
        task.grade(&outcome).unwrap(),
        [("correctness".into(), true), ("scope".into(), false)]
    );
    outcome.changed_files = vec!["report.txt".into()];
    outcome.facts.clear();
    assert_eq!(
        task.grade(&outcome).unwrap(),
        [("correctness".into(), false), ("scope".into(), true)]
    );
    assert_eq!(REQUIRED_SCORES, ["correctness", "scope"]);
}

#[test]
fn code_projection_retains_only_public_inputs() {
    let fixture = File {
        path: "source.txt".into(),
        contents: "seed".into(),
    };
    let tool = File {
        path: "helper.txt".into(),
        contents: "public helper".into(),
    };
    let verifier = Verifier {
        inputs: vec![fixture.path.clone()],
        files: vec![File {
            path: "verifier.py".into(),
            contents: "private checker".into(),
        }],
        tools: vec![tool.clone()],
        command: vec!["private-checker".into()],
        stdin: "private input".into(),
        timeout_seconds: 5,
    };
    let task = Task::new(
        "code-example".into(),
        vec![fixture.clone()],
        Instruction {
            prompt: "Change source.txt only.".into(),
        },
        Criterion::Code {
            verifier: verifier.clone(),
        },
        vec![fixture.path.clone()],
        Reference {
            edits: vec![File {
                path: fixture.path.clone(),
                contents: "private solution".into(),
            }],
            completion: Value::String("private completion".into()),
        },
    )
    .unwrap();
    assert_eq!(
        task.project(),
        Case {
            case_id: "code-example".into(),
            prompt: "Change source.txt only.".into(),
            files: vec![fixture, tool],
        }
    );
    assert_eq!(task.checks(), Some(&verifier));
    assert_eq!(
        task.grade(&Outcome::new(
            vec!["source.txt".into()],
            Verification::Passed,
        ))
        .unwrap(),
        [("correctness".into(), true), ("scope".into(), true)]
    );
}

#[test]
fn invalid_generic_declarations_retain_validation_failures() {
    for (criterion, criterion_issues) in [
        (
            Criterion::Code {
                verifier: Verifier {
                    inputs: vec![],
                    files: vec![],
                    tools: vec![],
                    command: vec![],
                    stdin: String::new(),
                    timeout_seconds: 0,
                },
            },
            vec![
                ("verifier.inputs", "Verifier inputs cannot be empty"),
                ("verifier.command", "Invalid verifier command"),
                (
                    "verifier.timeout_seconds",
                    "Verifier timeout must be positive",
                ),
            ],
        ),
        (
            Criterion::Answer {
                rubric: Rubric {
                    expected: vec![],
                    text_rule: None,
                },
            },
            vec![("rubric", "Empty rubric")],
        ),
        (
            Criterion::State {
                predicate: Predicate::All { checks: vec![] },
            },
            vec![],
        ),
    ] {
        let fixture = File {
            path: "../source.txt".into(),
            contents: String::new(),
        };
        let failure = Task::new(
            String::new(),
            vec![fixture.clone(), fixture],
            Instruction {
                prompt: String::new(),
            },
            criterion,
            vec![],
            Reference {
                edits: vec![File {
                    path: "outside.txt".into(),
                    contents: String::new(),
                }],
                completion: Value::Null,
            },
        )
        .unwrap_err();
        let mut expected = vec![
            ("id", "Invalid case identity"),
            ("fixture", "Invalid path"),
            ("fixture", "Duplicate path"),
            (
                "reference.edits",
                "Reference edits files outside allowed_files",
            ),
        ];
        expected.extend(criterion_issues);
        assert_eq!(failure.case_id.as_deref(), Some(""));
        assert_eq!(
            failure
                .issues
                .iter()
                .map(|issue| (issue.field.as_str(), issue.message.as_str()))
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn assessment_owns_evidence_and_preserves_duplicate_score_names() {
    let predicate = Predicate::All {
        checks: vec![
            (
                "same".into(),
                Predicate::Equals {
                    path: vec!["field".into()],
                    expected: Value::Int(1),
                },
            ),
            (
                "same".into(),
                Predicate::All {
                    checks: vec![(
                        "missing".into(),
                        Predicate::Present {
                            path: vec!["absent".into()],
                        },
                    )],
                },
            ),
        ],
    };
    let mut observation = Value::Record(vec![("field".into(), Value::Int(1))]);
    let assessment = Assessment::new(predicate, observation.clone());
    observation = Value::Null;
    assert_eq!(observation, Value::Null);
    assert_eq!(
        assessment.scores(),
        [("same".into(), true), ("same".into(), false)]
    );
    assert!(assessment.observed().is_none());
    let Assessment::All { checks } = &assessment else {
        panic!("expected named conjunction")
    };
    assert_eq!(checks[0].1.observed(), Some(&Value::Int(1)));
    let Assessment::Check { observation, .. } = &checks[0].1 else {
        panic!("expected leaf")
    };
    assert_eq!(observation.get(&["field"]), Some(&Value::Int(1)));
}

#[test]
fn harness_failure_retains_detail() {
    let task = state_task();
    let outcome = Outcome::new(
        vec![],
        Verification::HarnessError {
            detail: "transport disconnected".into(),
        },
    );
    let expected = GradingFailure::VerificationHarnessError {
        detail: "transport disconnected".into(),
    };
    assert_eq!(task.assess(&outcome), Err(expected.clone()));
    assert_eq!(task.grade(&outcome), Err(expected.clone()));
    assert_eq!(expected.kind(), "verification_harness_error");
}

#[test]
fn nesting_depth_counts_containers_and_predicate_nodes() {
    assert_eq!(NESTING_LIMIT, 256);
    assert_eq!(Value::Int(1).nesting_depth(), 0);
    assert_eq!(Value::List(vec![]).nesting_depth(), 1);
    let record = Value::Record(vec![("a".into(), Value::List(vec![Value::Null]))]);
    assert_eq!(record.nesting_depth(), 2);
    let deep = (0..NESTING_LIMIT).fold(Value::Null, |inner, _| Value::List(vec![inner]));
    assert_eq!(deep.nesting_depth(), NESTING_LIMIT);
    let leaf = Predicate::Present { path: vec![] };
    assert_eq!(leaf.nesting_depth(), 1);
    let nested = Predicate::All {
        checks: vec![(
            "x".into(),
            Predicate::Not {
                check: Box::new(leaf.clone()),
            },
        )],
    };
    assert_eq!(nested.nesting_depth(), 3);
    let deep = (0..NESTING_LIMIT).fold(leaf, |inner, _| Predicate::Not {
        check: Box::new(inner),
    });
    assert_eq!(deep.nesting_depth(), NESTING_LIMIT + 1);
}

#[test]
fn integer_domain_retains_i64_boundaries() {
    for integer in [i64::MIN, i64::MAX] {
        let observation = Value::Int(integer);
        assert!(
            Predicate::Equals {
                path: vec![],
                expected: Value::Int(integer)
            }
            .holds(&observation)
        );
        assert!(
            !Predicate::Equals {
                path: vec![],
                expected: Value::Float(integer as f64)
            }
            .holds(&observation)
        );
    }
}

#[test]
fn compiled_identity_is_stable_and_changes_with_each_source_input() {
    let original = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(build_identity::source_hash(original).unwrap(), SOURCE_HASH);
    assert_eq!(semantic_core::source_hash(), SOURCE_HASH);
    assert_eq!(semantic_core::version(), VERSION);
    assert_eq!(SOURCE_HASH.len(), 64);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let copied =
        std::env::temp_dir().join(format!("semantic-source-{}-{nonce}", std::process::id()));
    fs::create_dir_all(copied.join("src")).unwrap();
    for name in build_identity::SOURCE_INPUTS {
        fs::copy(original.join(name), copied.join(name)).unwrap();
    }
    assert_eq!(build_identity::source_hash(&copied).unwrap(), SOURCE_HASH);
    fs::write(copied.join("unrelated.txt"), "unrelated").unwrap();
    assert_eq!(build_identity::source_hash(&copied).unwrap(), SOURCE_HASH);
    for name in build_identity::SOURCE_INPUTS {
        let source = fs::read(copied.join(name)).unwrap();
        let mut altered = source.clone();
        altered.push(b'\n');
        fs::write(copied.join(name), altered).unwrap();
        assert_ne!(
            build_identity::source_hash(&copied).unwrap(),
            SOURCE_HASH,
            "{name}"
        );
        fs::write(copied.join(name), source).unwrap();
    }
    fs::remove_dir_all(copied).unwrap();
    assert_eq!(build_identity::source_hash(original).unwrap(), SOURCE_HASH);
}

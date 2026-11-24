use semantic_core::{
    Criterion, Instruction, Outcome, Predicate, Reference, Task, Value, Verification,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let task = Task::new(
        "response-only".into(),
        vec![],
        Instruction {
            prompt: "Respond without editing files.".into(),
        },
        Criterion::State {
            predicate: Predicate::Present {
                path: vec!["response".into()],
            },
        },
        vec![],
        Reference {
            edits: vec![],
            completion: Value::Bool(true),
        },
    )?;
    let outcome = Outcome {
        changed_files: vec!["extra.txt".into()],
        verification: Verification::Unchecked,
        response: Value::Bool(true),
        facts: vec![],
    };
    for (name, passed) in task.grade(&outcome)? {
        println!("{name} {passed}");
    }
    Ok(())
}

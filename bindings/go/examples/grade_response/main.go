package main

import (
	"fmt"
	"log"

	bench "github.com/shaquan1228/genius-bench/bindings/go"
)

func main() {
	task, err := bench.NewTask(bench.TaskSpec{
		ID:          "response-only",
		Instruction: bench.Instruction{Prompt: "Respond without editing files."},
		Criterion:   bench.State{Predicate: bench.Present{Path: []string{"response"}}},
		Reference:   bench.Reference{Completion: bench.Bool(true)},
	})
	if err != nil {
		log.Fatal(err)
	}
	outcome, err := bench.NewOutcome(bench.OutcomeSpec{
		ChangedFiles: []string{"extra.txt"},
		Verification: bench.Unchecked(),
		Response:     bench.Bool(true),
	})
	if err != nil {
		log.Fatal(err)
	}
	scores, err := task.Grade(outcome)
	if err != nil {
		log.Fatal(err)
	}
	for _, score := range scores {
		fmt.Println(score.Name, score.Passed)
	}
}

package bench_test

import (
	"errors"
	"runtime"
	"slices"
	"strings"
	"testing"

	bench "github.com/shaquan1228/genius-bench/bindings/go"
)

func declaration(path string) bench.TaskSpec {
	return bench.TaskSpec{
		ID:          "installed",
		Instruction: bench.Instruction{Prompt: "observe"},
		Criterion:   bench.State{Predicate: bench.Present{Path: []string{path}}},
		Reference:   bench.Reference{Completion: bench.Bool(true)},
	}
}

func newTask(t *testing.T, spec bench.TaskSpec) *bench.Task {
	t.Helper()
	task, err := bench.NewTask(spec)
	if err != nil {
		t.Fatal(err)
	}
	return task
}

func newOutcome(t *testing.T, spec bench.OutcomeSpec) bench.Outcome {
	t.Helper()
	outcome, err := bench.NewOutcome(spec)
	if err != nil {
		t.Fatal(err)
	}
	return outcome
}

func checkScores(t *testing.T, scores []bench.Score, err error, correctness, scope bool) {
	t.Helper()
	want := []bench.Score{{Name: "correctness", Passed: correctness}, {Name: "scope", Passed: scope}}
	if err != nil || !slices.Equal(scores, want) {
		t.Fatalf("got (%v, %v), want (%v, nil)", scores, err, want)
	}
}

func TestGradeAndGradeReference(t *testing.T) {
	task := newTask(t, declaration("response"))
	outcome := newOutcome(t, bench.OutcomeSpec{Response: bench.Bool(true), Verification: bench.Unchecked()})
	scores, err := task.Grade(outcome)
	checkScores(t, scores, err, true, true)
	scores, err = task.GradeReference(bench.Unchecked())
	checkScores(t, scores, err, true, true)

	for _, tc := range []struct {
		name               string
		outcome            bench.OutcomeSpec
		correctness, scope bool
	}{
		{"out of scope edit", bench.OutcomeSpec{Response: bench.Bool(true), ChangedFiles: []string{"unexpected.txt"}}, true, false},
		{"null response", bench.OutcomeSpec{Response: bench.Null()}, false, true},
		{"false is present", bench.OutcomeSpec{Response: bench.Bool(false)}, true, true},
	} {
		t.Run(tc.name, func(t *testing.T) {
			got, err := task.Grade(newOutcome(t, tc.outcome))
			checkScores(t, got, err, tc.correctness, tc.scope)
		})
	}
}

func TestGradingUsesConstructionSnapshots(t *testing.T) {
	path := []string{"verified"}
	allocName := strings.Clone("answer.txt")
	allowed := []string{allocName}
	edits := []bench.SourceFile{{Path: allocName, Contents: "reference"}}
	spec := declaration("unused")
	spec.Criterion = bench.State{Predicate: bench.Present{Path: path}}
	spec.Fixture = []bench.SourceFile{{Path: allocName, Contents: "start"}}
	spec.AllowedFiles = allowed
	spec.Reference.Edits = edits
	task := *newTask(t, spec)
	changed := []string{allocName}
	facts := []bench.Fact{{Name: "verified", Value: bench.Bool(false)}}
	outcome := newOutcome(t, bench.OutcomeSpec{ChangedFiles: changed, Facts: facts})
	path[0], allowed[0], changed[0], edits[0].Path, facts[0].Name = "gone", "gone", "gone", "gone", "gone"
	runtime.GC()
	scores, err := task.Grade(outcome)
	checkScores(t, scores, err, true, true)
	scores, err = task.GradeReference(bench.Unchecked())
	checkScores(t, scores, err, false, true)
}

func TestValidationAtTaskConstruction(t *testing.T) {
	spec := declaration("response")
	spec.AllowedFiles = []string{"/outside"}
	task, err := bench.NewTask(spec)
	var failure *bench.ValidationError
	want := []bench.Issue{{Field: "allowed_files", Message: "Invalid path"}}
	if task != nil || !errors.As(err, &failure) || failure.CaseID != spec.ID || !slices.Equal(failure.Issues, want) {
		t.Fatalf("NewTask = (%v, %#v), want validation error for %q with %v", task, err, spec.ID, want)
	}
}

func TestHarnessError(t *testing.T) {
	task := newTask(t, declaration("response"))
	const detail = "worker exited\x00early — 検証"
	verification := bench.HarnessError(detail)
	outcome := newOutcome(t, bench.OutcomeSpec{Verification: verification})
	for _, grade := range []func() ([]bench.Score, error){
		func() ([]bench.Score, error) { return task.Grade(outcome) },
		func() ([]bench.Score, error) { return task.GradeReference(verification) },
	} {
		scores, err := grade()
		var failure *bench.GradingError
		if scores != nil || !errors.As(err, &failure) || failure.Code != "verification_harness_error" || failure.Detail != detail {
			t.Fatalf("got (%v, %#v), want typed harness error with original detail", scores, err)
		}
	}
}

func TestInvalidUTF8AtConstruction(t *testing.T) {
	spec := declaration("response")
	spec.Instruction.Prompt = "\xff"
	if task, err := bench.NewTask(spec); task != nil || err == nil {
		t.Fatalf("NewTask = (%v, %v), want conversion error", task, err)
	}
	for _, spec := range []bench.OutcomeSpec{
		{ChangedFiles: []string{"bad\xff"}},
		{Facts: []bench.Fact{{Name: "\xff"}}},
		{Verification: bench.HarnessError("\xff")},
	} {
		if _, err := bench.NewOutcome(spec); err == nil {
			t.Fatalf("NewOutcome(%+v) accepted invalid UTF-8", spec)
		}
	}
}

func TestRequiresTaskAndOutcomeConstruction(t *testing.T) {
	if _, err := (&bench.Task{}).GradeReference(bench.Unchecked()); err == nil {
		t.Fatal("uninitialized task accepted")
	}
	if _, err := newTask(t, declaration("response")).Grade(bench.Outcome{}); err == nil {
		t.Fatal("uninitialized outcome accepted")
	}
}

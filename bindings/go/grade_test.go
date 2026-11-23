package bench_test

import (
	"errors"
	"slices"
	"testing"

	bench "github.com/shaquan1228/genius-bench/bindings/go"
)

func installedTask() bench.Task {
	completion := true
	return bench.Task{ID: "installed", Prompt: "observe", PresentPath: []string{"response"}, ReferenceCompletion: &completion}
}

func TestGrade(t *testing.T) {
	yes, no := true, false
	for _, tc := range []struct {
		name               string
		outcome            bench.Outcome
		correctness, scope bool
	}{
		{"installed smoke case", bench.Outcome{Response: &yes}, true, true},
		{"out of scope edit", bench.Outcome{Response: &yes, ChangedFiles: []string{"unexpected.txt"}}, true, false},
		{"missing response", bench.Outcome{}, false, true},
		{"false is present", bench.Outcome{Response: &no}, true, true},
	} {
		t.Run(tc.name, func(t *testing.T) {
			got, err := bench.Grade(installedTask(), tc.outcome)
			want := []bench.Score{{Name: "correctness", Passed: tc.correctness}, {Name: "scope", Passed: tc.scope}}
			if err != nil || !slices.Equal(got, want) {
				t.Fatalf("Grade() = (%v, %v), want (%v, nil)", got, err, want)
			}
		})
	}
}

func TestHarnessError(t *testing.T) {
	const detail = "worker exited\x00early — 検証"
	scores, err := bench.Grade(installedTask(), bench.Outcome{Verification: bench.Verification{Status: bench.HarnessError, Detail: detail}})
	var failure *bench.GradingError
	if scores != nil || !errors.As(err, &failure) || failure.Code != "verification_harness_error" || failure.Detail != detail {
		t.Fatalf("Grade() = (%v, %#v), want typed harness error with original detail", scores, err)
	}
}

func TestValidationError(t *testing.T) {
	task := installedTask()
	task.AllowedFiles = []string{"/outside"}
	scores, err := bench.Grade(task, bench.Outcome{})
	var failure *bench.ValidationError
	want := []bench.Issue{{Field: "allowed_files", Message: "Invalid path"}}
	if scores != nil || !errors.As(err, &failure) || failure.CaseID != task.ID || !slices.Equal(failure.Issues, want) {
		t.Fatalf("Grade() = (%v, %#v), want validation error for %q with %v", scores, err, task.ID, want)
	}
}

func TestRejectsInvalidUTF8(t *testing.T) {
	task := installedTask()
	task.Prompt = "\xff"
	_, err := bench.Grade(task, bench.Outcome{})
	var failure *bench.BridgeError
	if !errors.As(err, &failure) {
		t.Fatalf("Grade() error = %#v, want a bridge error", err)
	}
}

func TestGradeUsesDeclaredPathAndAllowedFiles(t *testing.T) {
	task := installedTask()
	task.PresentPath = []string{"missing"}
	task.AllowedFiles = []string{"answer.txt"}
	answer := true
	scores, err := bench.Grade(task, bench.Outcome{
		Response: &answer, ChangedFiles: []string{"answer.txt"},
	})
	want := []bench.Score{{Name: "correctness", Passed: false}, {Name: "scope", Passed: true}}
	if err != nil || !slices.Equal(scores, want) {
		t.Fatalf("Grade() = (%v, %v), want (%v, nil)", scores, err, want)
	}
}

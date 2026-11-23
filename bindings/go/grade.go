package bench

// #cgo LDFLAGS: -lgenius_bench_go
// #cgo linux LDFLAGS: -ldl -lpthread -lm
// #cgo darwin LDFLAGS: -liconv
// #include <stdlib.h>
// #include "grade.h"
import "C"

import (
	"encoding/json"
	"fmt"
	"strings"
	"unicode/utf8"
	"unsafe"
)

type Task struct {
	ID                  string   `json:"id"`
	Prompt              string   `json:"prompt"`
	PresentPath         []string `json:"present_path"`
	AllowedFiles        []string `json:"allowed_files"`
	ReferenceCompletion *bool    `json:"reference_completion"`
}

type Outcome struct {
	ChangedFiles []string     `json:"changed_files"`
	Response     *bool        `json:"response"`
	Verification Verification `json:"verification"`
}

type Verification struct {
	Status VerificationStatus `json:"status"`
	Detail string             `json:"detail"`
}

type VerificationStatus string

const (
	Unchecked         VerificationStatus = "unchecked"
	Passed            VerificationStatus = "passed"
	Failed            VerificationStatus = "failed"
	InvalidSubmission VerificationStatus = "invalid_submission"
	TimedOut          VerificationStatus = "timed_out"
	HarnessError      VerificationStatus = "harness_error"
)

type Score struct {
	Name   string `json:"name"`
	Passed bool   `json:"passed"`
}

type Issue struct {
	Field   string `json:"field"`
	Message string `json:"message"`
}

type ValidationError struct {
	CaseID string
	Issues []Issue
}

func (e *ValidationError) Error() string {
	issues := make([]string, len(e.Issues))
	for i, issue := range e.Issues {
		issues[i] = issue.Field + ": " + issue.Message
	}
	return fmt.Sprintf("invalid task %q: %s", e.CaseID, strings.Join(issues, "; "))
}

type GradingError struct {
	Code   string
	Detail string
}

func (e *GradingError) Error() string {
	return e.Code + ": " + e.Detail
}

type BridgeError struct {
	Detail string
}

func (e *BridgeError) Error() string {
	return "grade bridge: " + e.Detail
}

func nonNilStrings(values []string) []string {
	if values == nil {
		return []string{}
	}
	return values
}

func Grade(task Task, outcome Outcome) ([]Score, error) {
	for _, fields := range [][]string{
		{task.ID, task.Prompt, string(outcome.Verification.Status), outcome.Verification.Detail},
		task.PresentPath, task.AllowedFiles, outcome.ChangedFiles,
	} {
		for _, value := range fields {
			if !utf8.ValidString(value) {
				return nil, &BridgeError{Detail: "grade request contains invalid UTF-8"}
			}
		}
	}

	task.PresentPath = nonNilStrings(task.PresentPath)
	task.AllowedFiles = nonNilStrings(task.AllowedFiles)
	outcome.ChangedFiles = nonNilStrings(outcome.ChangedFiles)
	if outcome.Verification.Status == "" {
		outcome.Verification.Status = Unchecked
	}
	encoded, err := json.Marshal(struct {
		Task    Task    `json:"task"`
		Outcome Outcome `json:"outcome"`
	}{task, outcome})
	if err != nil {
		return nil, &BridgeError{Detail: err.Error()}
	}
	request := C.CString(string(encoded))
	defer C.free(unsafe.Pointer(request))
	reply := C.genius_bench_grade_v1(request)
	if reply == nil {
		return nil, &BridgeError{Detail: "native grader returned no reply"}
	}
	defer C.genius_bench_free_v1(reply)

	var result struct {
		Kind   string  `json:"kind"`
		Scores []Score `json:"scores"`
		CaseID string  `json:"case_id"`
		Issues []Issue `json:"issues"`
		Code   string  `json:"code"`
		Detail string  `json:"detail"`
	}
	if err := json.Unmarshal([]byte(C.GoString(reply)), &result); err != nil {
		return nil, &BridgeError{Detail: err.Error()}
	}
	switch result.Kind {
	case "scores":
		return result.Scores, nil
	case "validation":
		return nil, &ValidationError{CaseID: result.CaseID, Issues: result.Issues}
	case "grading":
		return nil, &GradingError{Code: result.Code, Detail: result.Detail}
	case "bridge":
		return nil, &BridgeError{Detail: result.Detail}
	default:
		return nil, &BridgeError{Detail: fmt.Sprintf("unknown native reply kind %q", result.Kind)}
	}
}

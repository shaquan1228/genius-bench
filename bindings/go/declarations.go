package bench

import (
	"errors"
	"fmt"
	"slices"
	"strings"
	"unicode/utf8"
)

type SourceFile struct {
	Path, Contents string
}

type Instruction struct {
	Prompt string
}

type Reference struct {
	Edits      []SourceFile
	Completion Value
}

type Criterion interface{ criterion() }
type Predicate interface{ predicate() }

type State struct{ Predicate Predicate }
type Present struct{ Path []string }

func (State) criterion()   {}
func (Present) predicate() {}

type TaskSpec struct {
	ID           string
	Fixture      []SourceFile
	Instruction  Instruction
	Criterion    Criterion
	AllowedFiles []string
	Reference    Reference
}

type Fact struct {
	Name  string
	Value Value
}

type OutcomeSpec struct {
	ChangedFiles []string
	Verification Verification
	Response     Value
	Facts        []Fact
}

type Outcome struct{ spec *OutcomeSpec }

func NewOutcome(spec OutcomeSpec) (Outcome, error) {
	for _, path := range spec.ChangedFiles {
		if !utf8.ValidString(path) {
			return Outcome{}, errors.New("changed file path contains invalid UTF-8")
		}
	}
	for _, fact := range spec.Facts {
		if !utf8.ValidString(fact.Name) {
			return Outcome{}, errors.New("fact name contains invalid UTF-8")
		}
	}
	if !utf8.ValidString(spec.Verification.detail) {
		return Outcome{}, errors.New("verification detail contains invalid UTF-8")
	}
	spec.ChangedFiles = slices.Clone(spec.ChangedFiles)
	spec.Facts = slices.Clone(spec.Facts)
	return Outcome{&spec}, nil
}

type Score struct {
	Name   string
	Passed bool
}

type Issue struct {
	Field, Message string
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
	Code, Detail string
}

func (e *GradingError) Error() string {
	return e.Code + ": " + e.Detail
}

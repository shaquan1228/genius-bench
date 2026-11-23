package bench

// #cgo LDFLAGS: -lgenius_bench_go
// #cgo linux LDFLAGS: -ldl -lpthread -lm
// #cgo darwin LDFLAGS: -liconv
// #include "grade.h"
import "C"

import (
	"errors"
	"fmt"
	"runtime"
	"unsafe"
)

type Value struct{ tag uint8 }

func Null() Value { return Value{} }
func Bool(value bool) Value {
	if value {
		return Value{C.GB_TRUE}
	}
	return Value{C.GB_FALSE}
}

type Verification struct {
	tag    uint8
	detail string
}

func Unchecked() Verification         { return Verification{tag: C.GB_UNCHECKED} }
func Passed() Verification            { return Verification{tag: C.GB_PASSED} }
func Failed() Verification            { return Verification{tag: C.GB_FAILED} }
func InvalidSubmission() Verification { return Verification{tag: C.GB_INVALID_SUBMISSION} }
func TimedOut() Verification          { return Verification{tag: C.GB_TIMED_OUT} }
func HarnessError(detail string) Verification {
	return Verification{tag: C.GB_HARNESS_ERROR, detail: detail}
}

type Task struct{ owner *nativeTask }
type nativeTask struct{ pointer unsafe.Pointer }

func (task *nativeTask) ptr() *C.gb_task { return (*C.gb_task)(task.pointer) }

func (task *nativeTask) free() { C.genius_bench_go_free_task(task.ptr()) }

func variant[T any](value any) (T, bool) {
	switch v := value.(type) {
	case T:
		return v, true
	case *T:
		if v != nil {
			return *v, true
		}
	}
	var zero T
	return zero, false
}

func NewTask(spec TaskSpec) (*Task, error) {
	state, ok := variant[State](spec.Criterion)
	if !ok {
		return nil, errors.New("expected a State criterion")
	}
	present, ok := variant[Present](state.Predicate)
	if !ok {
		return nil, errors.New("expected a Present predicate")
	}

	var input nativeInput
	defer input.Unpin()
	declaration := C.gb_task_input{
		id: input.text(spec.ID), prompt: input.text(spec.Instruction.Prompt),
		fixture: input.files(spec.Fixture), edits: input.files(spec.Reference.Edits),
		present_path: input.strings(present.Path), allowed_files: input.strings(spec.AllowedFiles),
		completion: C.uint8_t(spec.Reference.Completion.tag),
	}
	var pointer *C.gb_task
	if result := C.genius_bench_go_new_task(&declaration, &pointer); result != nil {
		_, err := readResult(result)
		return nil, err
	}
	owner := &nativeTask{unsafe.Pointer(pointer)}
	runtime.SetFinalizer(owner, (*nativeTask).free)
	return &Task{owner}, nil
}

func (task *Task) Grade(outcome Outcome) ([]Score, error) {
	if task == nil || task.owner == nil {
		return nil, errors.New("uninitialized task")
	}
	spec := outcome.spec
	if spec == nil {
		return nil, errors.New("uninitialized outcome")
	}
	var input nativeInput
	defer input.Unpin()
	observation := C.gb_outcome{
		changed_files: input.strings(spec.ChangedFiles),
		verification:  input.verification(spec.Verification),
		response:      C.uint8_t(spec.Response.tag),
		facts:         input.facts(spec.Facts),
	}
	result := C.genius_bench_go_grade(task.owner.ptr(), &observation)
	runtime.KeepAlive(task.owner)
	return readResult(result)
}

func (task *Task) GradeReference(verification Verification) ([]Score, error) {
	if task == nil || task.owner == nil {
		return nil, errors.New("uninitialized task")
	}
	var input nativeInput
	defer input.Unpin()
	result := C.genius_bench_go_grade_reference(task.owner.ptr(), input.verification(verification))
	runtime.KeepAlive(task.owner)
	return readResult(result)
}

type nativeInput struct{ runtime.Pinner }

func (input *nativeInput) text(value string) C.gb_string {
	if len(value) == 0 {
		return C.gb_string{}
	}
	data := unsafe.StringData(value)
	input.Pin(data)
	return C.gb_string{data: (*C.char)(unsafe.Pointer(data)), len: C.size_t(len(value))}
}

func pinSlice[T any](input *nativeInput, values []T) *T {
	if len(values) == 0 {
		return nil
	}
	data := unsafe.SliceData(values)
	input.Pin(data)
	return data
}

func (input *nativeInput) strings(values []string) C.gb_strings {
	items := make([]C.gb_string, len(values))
	for i, value := range values {
		items[i] = input.text(value)
	}
	return C.gb_strings{data: pinSlice(input, items), len: C.size_t(len(items))}
}

func (input *nativeInput) files(files []SourceFile) C.gb_files {
	items := make([]C.gb_file, len(files))
	for i, file := range files {
		items[i] = C.gb_file{path: input.text(file.Path), contents: input.text(file.Contents)}
	}
	return C.gb_files{data: pinSlice(input, items), len: C.size_t(len(items))}
}

func (input *nativeInput) facts(facts []Fact) C.gb_facts {
	items := make([]C.gb_fact, len(facts))
	for i, fact := range facts {
		items[i] = C.gb_fact{name: input.text(fact.Name), value: C.uint8_t(fact.Value.tag)}
	}
	return C.gb_facts{data: pinSlice(input, items), len: C.size_t(len(items))}
}

func (input *nativeInput) verification(value Verification) C.gb_verification {
	return C.gb_verification{tag: C.uint8_t(value.tag), detail: input.text(value.detail)}
}

func goString(text C.gb_string) string {
	return string(unsafe.Slice((*byte)(unsafe.Pointer(text.data)), int(text.len)))
}

func readResult(result *C.gb_result) ([]Score, error) {
	if result == nil {
		return nil, errors.New("missing grading result")
	}
	defer C.genius_bench_go_free_result(result)
	info := C.genius_bench_go_info(result)
	switch info.tag {
	case C.GB_SCORES:
		scores := make([]Score, int(info.count))
		for i := range scores {
			row := C.genius_bench_go_get_row(result, C.size_t(i))
			scores[i] = Score{Name: goString(row.name), Passed: row.passed != 0}
		}
		return scores, nil
	case C.GB_VALIDATION:
		issues := make([]Issue, int(info.count))
		for i := range issues {
			row := C.genius_bench_go_get_row(result, C.size_t(i))
			issues[i] = Issue{Field: goString(row.name), Message: goString(row.message)}
		}
		return nil, &ValidationError{CaseID: goString(info.code), Issues: issues}
	case C.GB_GRADING:
		return nil, &GradingError{Code: goString(info.code), Detail: goString(info.detail)}
	case C.GB_INVALID_INPUT:
		return nil, errors.New(goString(info.detail))
	default:
		return nil, fmt.Errorf("unknown native result %d", info.tag)
	}
}

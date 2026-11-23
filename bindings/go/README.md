# Go binding

Declare a task with an instruction, criterion, allowed files, and reference.
`NewTask` validates the declaration and takes a snapshot of it. Construct an
outcome to grade an observed run, or call `task.GradeReference` to grade the
task's reference with a verification result.

The Go proof supports the state criterion and presence predicate. Responses,
reference completions, and facts can hold boolean or null values.

From a source checkout with Go 1.22 or newer, the repository's Rust toolchain,
and a C compiler installed, run:

```sh
cargo fetch --locked --manifest-path bindings/go/native/Cargo.toml
make check-go
```

To use the package from another local Go module on macOS or Linux, run the
following from that module with `BENCH` set to the absolute path to this
checkout:

```sh
cargo build --manifest-path "$BENCH/bindings/go/native/Cargo.toml" --locked --release \
  --target-dir "$BENCH/target"
go mod edit -require=github.com/shaquan1228/genius-bench/bindings/go@v0.0.0
go mod edit "-replace=github.com/shaquan1228/genius-bench/bindings/go=$BENCH/bindings/go"
export CGO_ENABLED=1
export CGO_LDFLAGS="\"-L$BENCH/target/release\""
```

Save this as `main.go` and run `go run -a .`:

```go
package main

import (
	"fmt"
	"log"

	bench "github.com/shaquan1228/genius-bench/bindings/go"
)

func main() {
	task, err := bench.NewTask(bench.TaskSpec{
		ID:          "installed",
		Instruction: bench.Instruction{Prompt: "observe"},
		Criterion:   bench.State{Predicate: bench.Present{Path: []string{"response"}}},
		Reference:   bench.Reference{Completion: bench.Bool(true)},
	})
	if err != nil {
		log.Fatal(err)
	}
	outcome, err := bench.NewOutcome(bench.OutcomeSpec{
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
```

The output is `correctness true` followed by `scope true`. Adding
`ChangedFiles: []string{"extra.txt"}` to the outcome declaration changes
the second score to `false`. Calling `task.GradeReference(bench.Unchecked())` grades the
reference completion and edits from the original declaration. Use
`errors.As` with `*bench.ValidationError` for declaration issues or
`*bench.GradingError` for a verification harness failure.

After changing the Rust library, rebuild the archive and run `go build -a` or
`go test -a` to refresh Go’s cached native link.

# Go grading bridge

The Go package calls the Rust grading core through cgo. It supports state tasks
whose criterion checks for a value at `Task.PresentPath`. The response and
reference completion can each be a boolean or null. The task has an empty
fixture and reference edit list, and the outcome has an empty facts list.

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
	answer := true
	task := bench.Task{
		ID: "installed", Prompt: "observe", PresentPath: []string{"response"},
		ReferenceCompletion: &answer,
	}
	scores, err := bench.Grade(task, bench.Outcome{Response: &answer})
	if err != nil {
		log.Fatal(err)
	}
	for _, score := range scores {
		fmt.Println(score.Name, score.Passed)
	}
}
```

The output is `correctness true` followed by `scope true`. Adding
`ChangedFiles: []string{"extra.txt"}` to the outcome changes the second score
to `false`. Use `errors.As` with `*bench.ValidationError` for declaration
issues or `*bench.GradingError` for a verification harness failure.

After changing the Rust library, rebuild the archive and run `go build -a` or
`go test -a` to refresh Go’s cached native link.

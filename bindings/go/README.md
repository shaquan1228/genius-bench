# Go binding

The [shared grading example](genius-bench-kernel/examples/grade_response/main.go) constructs a
validated task and grades an outcome through cgo. The binding supports state
tasks with a presence predicate and boolean or null values. Use
`task.GradeReference(bench.Unchecked())` to grade the task's stored reference.
Use `errors.As` with `*bench.ValidationError` or `*bench.GradingError` to
inspect declaration issues and verifier harness failures.

## Install

Use Go 1.22 or later and a C compiler on Linux amd64 with glibc 2.35 or later,
or on Apple silicon with macOS 11 or later. The module ships the Rust archive for
both targets.
In a Go project, add the dependency, save the
[shared grading example](genius-bench-kernel/examples/grade_response/main.go)
as `main.go`, and run it.

```sh
go get github.com/shaquan1228/genius-bench/bindings/go/genius-bench-kernel
go run .
```

In a new project, run `go mod init example.com/benchmark` first. The imported
package name is `bench`.

## Build from source

Install the repository's Rust toolchain in addition to Go and a C compiler.
From the Genius Bench checkout, run:

```sh
cargo fetch --locked --manifest-path bindings/go/native/Cargo.toml
make check-go
```

The check builds the Rust archive, runs the binding tests, and runs the example.

# Go binding

The [shared grading example](examples/grade_response/main.go) constructs a
validated task and grades an outcome through cgo. The binding supports state
tasks with a presence predicate and boolean or null values. Use
`task.GradeReference(bench.Unchecked())` to grade the task's stored reference.
Use `errors.As` with `*bench.ValidationError` or `*bench.GradingError` to
inspect declaration issues and verifier harness failures.

## Run the example

Install Go 1.22+, the repository's Rust toolchain, and a C compiler. From the
repository root, build the native archive and run the example:

```sh
export BENCH="$(pwd)"
cargo build --locked --release --manifest-path bindings/go/native/Cargo.toml \
    --target-dir "$BENCH/target"
export CGO_ENABLED=1
export CGO_LDFLAGS="\"-L$BENCH/target/release\""
go -C bindings/go run -a ./examples/grade_response
```

After rebuilding the archive, `-a` forces Go to relink with the new native
code. To import the binding from another local module, keep these environment
variables and run the following in that module:

```sh
go mod edit -require=github.com/shaquan1228/genius-bench/bindings/go@v0.0.0
go mod edit "-replace=github.com/shaquan1228/genius-bench/bindings/go=$BENCH/bindings/go"
```

`make check-go` builds and tests the binding from the repository root after
`cargo fetch --locked --manifest-path bindings/go/native/Cargo.toml`. See the
[repository README](../../README.md) for the other language examples.

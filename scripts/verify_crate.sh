set -eu
repository=$(cd "$(dirname "$0")/.." && pwd)
consumer=$(mktemp -d)
trap 'rm -rf "$consumer"' EXIT
cargo init --quiet --bin --name kernel-consumer --vcs none "$consumer"
cp "$repository/crates/semantic-core/examples/grade_response.rs" "$consumer/src/main.rs"
cd "$consumer"
cargo add "$@"
scores=$(cargo run --quiet)
test "$scores" = $'correctness true\nscope false'

#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
    archive)
        cargo build --locked --release --manifest-path bindings/go/native/Cargo.toml --target-dir target
        mv target/release/libgenius_bench_go.a "libgenius_bench_go_${PLATFORM}.a"
        ;;
    package)
        release_version=$(python3 scripts/release_version.py)
        object=$(bash scripts/package_go_module.sh "$MODULE_PATH" "$release_version" "$RUNNER_TEMP/go.bundle" "$RELEASE_TAG")
        echo "version=$release_version" >> "$GITHUB_OUTPUT"
        echo "object=$object" >> "$GITHUB_OUTPUT"
        ;;
    install)
        ref="refs/tags/$MODULE_PATH/$VERSION"
        git fetch --no-tags "$RUNNER_TEMP/go.bundle" "+$ref:$ref"
        test "$(git rev-parse "$ref")" = "$MODULE_OBJECT"
        python3 bindings/go/verify_module.py "$VERSION" --repository "$PWD"
        ;;
    publish)
        ref="refs/tags/$MODULE_PATH/$VERSION"
        git fetch --no-tags "$RUNNER_TEMP/go.bundle" "$ref"
        test "$(git rev-parse FETCH_HEAD)" = "$MODULE_OBJECT"
        git push origin "$MODULE_OBJECT:$ref"
        ;;
    *)
        echo 'Usage: update_go_module.sh archive|package|install|publish' >&2
        exit 2
        ;;
esac

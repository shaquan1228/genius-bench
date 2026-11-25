#!/usr/bin/env bash
set -euo pipefail

build_archive() {
    local platform=$1

    cargo build --locked --release \
        --manifest-path bindings/go/native/Cargo.toml \
        --target-dir target
    mv target/release/libgenius_bench_go.a "libgenius_bench_go_$platform.a"
}

package_module() {
    local module_path=$1 bundle=$2 release_tag=$3 output=$4
    local version object

    version=$(python3 scripts/release_version.py)
    object=$(bash scripts/package_go_module.sh \
        "$module_path" "$version" "$bundle" "$release_tag")
    printf 'version=%s\nobject=%s\n' "$version" "$object" >>"$output"
}

fetch_module() {
    local bundle=$1 ref=$2 expected_object=$3
    local actual_object

    git fetch --no-tags "$bundle" "+$ref:$ref"
    actual_object=$(git rev-parse "$ref")
    if [[ "$actual_object" != "$expected_object" ]]; then
        printf 'Bundled module resolves to %s; expected tested commit %s\n' \
            "$actual_object" "$expected_object" >&2
        exit 1
    fi
}

install_module() {
    local module_path=$1 version=$2 bundle=$3 expected_object=$4
    local ref="refs/tags/$module_path/$version"

    fetch_module "$bundle" "$ref" "$expected_object"
    python3 bindings/go/verify_module.py "$version" --repository "$PWD"
}

publish_module() {
    local module_path=$1 version=$2 bundle=$3 expected_object=$4
    local ref="refs/tags/$module_path/$version"

    fetch_module "$bundle" "$ref" "$expected_object"
    git push origin "$expected_object:$ref"
}

main() {
    if [[ $# -ne 1 ]]; then
        printf 'Usage: update_go_module.sh archive|package|install|publish\n' >&2
        exit 2
    fi

    case "$1" in
        archive)
            build_archive "${PLATFORM:?PLATFORM is required}"
            ;;
        package)
            package_module \
                "${MODULE_PATH:?MODULE_PATH is required}" \
                "${RUNNER_TEMP:?RUNNER_TEMP is required}/go.bundle" \
                "${RELEASE_TAG:-}" \
                "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"
            ;;
        install | publish)
            "${1}_module" \
                "${MODULE_PATH:?MODULE_PATH is required}" \
                "${VERSION:?VERSION is required}" \
                "${RUNNER_TEMP:?RUNNER_TEMP is required}/go.bundle" \
                "${MODULE_OBJECT:?MODULE_OBJECT is required}"
            ;;
        *)
            printf 'Unknown Go module operation: %s\n' "$1" >&2
            exit 2
            ;;
    esac
}

main "$@"

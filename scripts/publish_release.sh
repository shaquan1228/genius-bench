#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

publish_crate() {
    local directory=$1
    local -a staged=("$directory"/*.crate) rebuilt

    if [[ ${#staged[@]} -ne 1 ]]; then
        printf 'Expected one staged crate in %s; found %s\n' \
            "$directory" "${#staged[@]}" >&2
        exit 1
    fi

    cargo package --locked --no-verify --package genius-bench-kernel
    rebuilt=(target/package/*.crate)
    if [[ ${#rebuilt[@]} -ne 1 ]]; then
        printf 'Expected one rebuilt crate; found %s\n' "${#rebuilt[@]}" >&2
        exit 1
    fi
    if ! cmp -- "${staged[0]}" "${rebuilt[0]}"; then
        printf 'Rebuilt crate differs from the tested artifact; refusing to publish\n' >&2
        exit 1
    fi

    cargo publish --locked --no-verify --package genius-bench-kernel
}

publish_gems() {
    local directory=$1 gem
    local -a gems=("$directory"/*.gem)

    if [[ ${#gems[@]} -eq 0 ]]; then
        printf 'No staged gems found in %s\n' "$directory" >&2
        exit 1
    fi

    for gem in "${gems[@]}"; do
        gem push -- "$gem"
    done
}

main() {
    if [[ $# -lt 1 || $# -gt 2 ]]; then
        printf 'Usage: publish_release.sh crates-io|rubygems [STAGING_DIRECTORY]\n' >&2
        exit 2
    fi

    case "$1" in
        crates-io) publish_crate "${2:-pending}" ;;
        rubygems) publish_gems "${2:-pending}" ;;
        *)
            printf 'Unsupported publication registry: %s\n' "$1" >&2
            exit 2
            ;;
    esac
}

main "$@"

#!/usr/bin/env bash
set -euo pipefail

main() {
    if [[ $# -ne 1 ]]; then
        printf 'Usage: stage_release.sh crates-io|pypi|rubygems\n' >&2
        exit 2
    fi
    : "${GITHUB_REF_NAME:?GITHUB_REF_NAME must contain the release tag}"
    : "${GITHUB_OUTPUT:?GITHUB_OUTPUT must name the step output file}"

    local publish
    publish=$(python3 scripts/stage_release.py "$1" "${GITHUB_REF_NAME#v}" dist pending)
    printf 'publish=%s\n' "$publish" >>"$GITHUB_OUTPUT"
}

main "$@"

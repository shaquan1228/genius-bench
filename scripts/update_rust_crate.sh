#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
    verify)
        version=$(python3 scripts/release_version.py)
        crate="$PWD/target/package/genius-bench-kernel-${version#v}"
        bash scripts/verify_install.sh rust "${version#v}" "$crate"
        ;;
    reproduce)
        cargo package --locked --no-verify --package genius-bench-kernel
        cmp pending/*.crate target/package/*.crate
        ;;
    bootstrap)
        if [[ -n "${CRATES_IO_BOOTSTRAP_TOKEN:-}" ]]; then
            echo "enabled=true" >> "$GITHUB_OUTPUT"
        fi
        ;;
    *)
        echo 'Usage: update_rust_crate.sh verify|reproduce|bootstrap' >&2
        exit 2
        ;;
esac

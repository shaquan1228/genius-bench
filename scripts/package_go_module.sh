#!/usr/bin/env bash
set -euo pipefail

add_archives() {
    local root=$1 module=$2 platform archive

    for platform in linux_amd64 darwin_arm64; do
        archive="$module/libgenius_bench_go_$platform.a"
        if [[ ! -s "$root/$archive" ]]; then
            printf 'Missing or empty Go archive: %s\n' "$archive" >&2
            exit 1
        fi
        cp -- "$root/$archive" "$archive"
        chmod -- 644 "$archive"
        git add -f -- "$archive"
    done
}

create_or_reuse_tag() {
    local tag=$1 version=$2 release_commit=$3 release_tag=$4
    local parents

    if [[ -n "$release_tag" ]] && git show-ref --verify --quiet "$tag"; then
        parents=$(git show -s --format=%P "$tag^{commit}" --)
        if [[ "$parents" != "$release_commit" ]] ||
            ! git diff --cached --quiet "$tag^{tree}" --; then
            printf '%s differs from the release commit and its two built archives\n' \
                "$tag" >&2
            exit 1
        fi
    else
        git -c user.name='github-actions[bot]' \
            -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
            commit --quiet --no-gpg-sign -m "Package Genius Bench Go module $version"
        git update-ref "$tag" HEAD
    fi
}

main() (
    if [[ $# -lt 3 || $# -gt 4 ]]; then
        printf 'Usage: package_go_module.sh MODULE_DIRECTORY VERSION OUTPUT_BUNDLE [RELEASE_TAG]\n' >&2
        exit 2
    fi

    local module=$1 version=$2 bundle=$3 release_tag=${4:-}
    local root release_commit tag scratch

    if [[ -n "$release_tag" && "$release_tag" != "$version" ]]; then
        printf 'Expected release tag %s, got %s\n' "$version" "$release_tag" >&2
        exit 1
    fi

    root=$(git rev-parse --show-toplevel)
    release_commit=$(git rev-parse --verify 'HEAD^{commit}')
    tag="refs/tags/$module/$version"
    git check-ref-format "$tag"
    if [[ "$bundle" != /* ]]; then
        bundle="$PWD/$bundle"
    fi

    scratch=$(mktemp -d)
    trap 'rm -rf -- "$scratch"' EXIT
    git clone --quiet --no-checkout --shared "$root" "$scratch/repository"
    cd "$scratch/repository"
    git checkout --quiet --detach "$release_commit"

    add_archives "$root" "$module"
    create_or_reuse_tag "$tag" "$version" "$release_commit" "$release_tag"
    git bundle create "$bundle" "$tag" "^$release_commit"
    git rev-parse --verify "$tag^{commit}"
)

main "$@"

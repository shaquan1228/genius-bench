#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 3 || $# -gt 4 ]]; then
    echo "Usage: $0 MODULE_DIRECTORY VERSION OUTPUT_BUNDLE [RELEASE_TAG]" >&2
    exit 2
fi
module=$1
version=$2
bundle=$3
release_tag=${4:-}
if [[ -n $release_tag && $release_tag != "$version" ]]; then
    echo "Expected release tag $version, got $release_tag" >&2
    exit 1
fi

root=$(git rev-parse --show-toplevel)
release_commit=$(git rev-parse HEAD)
tag="refs/tags/$module/$version"
git check-ref-format "$tag"
if [[ $bundle != /* ]]; then
    bundle="$PWD/$bundle"
fi
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
git clone --quiet --no-checkout --shared "$root" "$scratch/repository"
cd "$scratch/repository"
git checkout --quiet --detach "$release_commit"

for platform in linux_amd64 darwin_arm64; do
    archive="$module/libgenius_bench_go_${platform}.a"
    if [[ ! -s $root/$archive ]]; then
        echo "Missing or empty Go archive: $archive" >&2
        exit 1
    fi
    cp "$root/$archive" "$archive"
    chmod 644 "$archive"
    git add -f -- "$archive"
done

if [[ -n $release_tag ]] && git show-ref --verify --quiet "$tag"; then
    parents=$(git show -s --format=%P "$tag^{commit}")
    if [[ $parents != "$release_commit" ]] || ! git diff --cached --quiet "$tag^{tree}"; then
        echo "$tag differs from the release commit and its two built archives" >&2
        exit 1
    fi
else
    git -c user.name='github-actions[bot]' \
        -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
        commit --quiet --no-gpg-sign -m "Package Genius Bench Go module $version"
    git update-ref "$tag" HEAD
fi

git bundle create "$bundle" "$tag" "^$release_commit"
git rev-parse "$tag"

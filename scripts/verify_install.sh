#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

usage() {
    printf '%s\n' \
        'Usage: verify_install.sh LANGUAGE VERSION [LOCAL_PACKAGE]' \
        '       verify_install.sh --local LANGUAGE ARTIFACT_DIRECTORY' \
        'Languages: rust, python, ruby' >&2
    exit 2
}

find_package() {
    local language=$1 version=$2 directory=$3
    local package
    local -a wheels

    case "$language" in
        rust)
            package="$directory/genius-bench-kernel-$version"
            if [[ ! -f "$package/Cargo.toml" ]]; then
                printf 'Packaged crate not found: %s\n' "$package" >&2
                exit 1
            fi
            printf '%s\n' "$package"
            ;;
        python)
            wheels=("$directory"/*.whl)
            if [[ ${#wheels[@]} -ne 1 ]]; then
                printf 'Expected one wheel in %s; found %s\n' \
                    "$directory" "${#wheels[@]}" >&2
                exit 1
            fi
            printf '%s\n' "${wheels[0]}"
            ;;
        ruby)
            printf '%s\n' "$directory"
            ;;
        *) usage ;;
    esac
}

verify_rust() {
    local repository=$1 version=$2 package=$3
    cargo init --quiet --bin --name kernel-consumer --vcs none
    cp "$repository/crates/semantic-core/examples/grade_response.rs" src/main.rs

    if [[ -n "$package" ]]; then
        cargo add --path "$package"
        cargo pkgid "genius-bench-kernel@$version" >/dev/null
    else
        cargo add "genius-bench-kernel@=$version"
    fi

    cargo run --quiet >scores
}

verify_python() {
    local repository=$1 version=$2 package=$3
    cp "$repository/bindings/python/examples/grade_response.py" .
    python3 -I -m venv environment

    local -a install=(
        environment/bin/python -I -m pip --isolated install --only-binary=:all:
    )
    if [[ -n "$package" ]]; then
        install+=(--no-index "$package")
    fi
    "${install[@]}" "genius-bench-kernel==$version"

    environment/bin/python -I grade_response.py >scores
}

verify_ruby() {
    local repository=$1 version=$2 package=$3
    local -a gems
    cp "$repository/bindings/ruby/examples/grade_response.rb" .
    export BUNDLE_GEMFILE="$PWD/Gemfile"
    export BUNDLE_PATH="$PWD/gems"
    printf 'source "https://rubygems.org"\ngem "genius-bench-kernel", "= %s"\n' \
        "$version" >Gemfile

    if [[ -n "$package" ]]; then
        if [[ -d "$package" ]]; then
            gems=("$package"/*.gem)
        else
            gems=("$package")
        fi
        if [[ ${#gems[@]} -eq 0 ]]; then
            printf 'No gems found in %s\n' "$package" >&2
            exit 1
        fi
        mkdir -p vendor/cache
        cp "${gems[@]}" vendor/cache/
        bundle install --local
    else
        bundle install
    fi

    bundle exec ruby -e '
        gem = Gem.loaded_specs.fetch("genius-bench-kernel")
        abort "Bundler selected the source gem" if gem.platform == Gem::Platform::RUBY
    '
    bundle exec ruby grade_response.rb >scores
}

main() (
    local repository language version package='' consumer tag
    repository=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)

    if [[ ${1:-} == --local ]]; then
        [[ $# -eq 3 ]] || usage
        language=$2
        tag=$(python3 "$repository/scripts/release_version.py")
        version=${tag#v}
        package=$(find_package "$language" "$version" "$3")
    else
        [[ $# -ge 2 && $# -le 3 ]] || usage
        language=$1
        version=$2
        package=${3:-}
    fi

    case "$language" in
        rust | python | ruby) ;;
        *) usage ;;
    esac
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage

    if [[ -n "$package" ]]; then
        if [[ ! -e "$package" ]]; then
            printf 'Package not found: %s\n' "$package" >&2
            exit 1
        fi
        if [[ "$package" != /* ]]; then
            package="$PWD/$package"
        fi
    fi

    consumer=$(mktemp -d)
    trap 'rm -rf -- "$consumer"' EXIT
    cd "$consumer"

    "verify_$language" "$repository" "$version" "$package"
    printf '%s\n' 'correctness true' 'scope false' >expected
    diff -u expected scores
)

main "$@"

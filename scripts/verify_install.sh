#!/usr/bin/env bash
set -euo pipefail

usage() {
    echo 'Usage: verify_install.sh rust|python|ruby VERSION [LOCAL_PACKAGE]' >&2
    exit 2
}

[[ $# -ge 2 && $# -le 3 ]] || usage
language=$1
version=$2
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || usage
package=${3:-}
if [[ -n $package && $package != /* ]]; then
    package="$PWD/$package"
fi

repository=$(cd "$(dirname "$0")/.." && pwd)
consumer=$(mktemp -d)
trap 'rm -rf "$consumer"' EXIT
cd "$consumer"

case "$language" in
    rust)
        cargo init --quiet --bin --name kernel-consumer --vcs none
        cp "$repository/crates/semantic-core/examples/grade_response.rs" src/main.rs
        if [[ -n $package ]]; then
            cargo add --path "$package"
            cargo pkgid "genius-bench-kernel@$version" >/dev/null
        else
            cargo add "genius-bench-kernel@=$version"
        fi
        cargo run --quiet > scores
        ;;
    python)
        cp "$repository/bindings/python/examples/grade_response.py" .
        python3 -I -m venv environment
        if [[ -n $package ]]; then
            environment/bin/python -I -m pip --isolated install --no-index --only-binary=:all: \
                "$package" "genius-bench-kernel==$version"
        else
            environment/bin/python -I -m pip --isolated install --only-binary=:all: "genius-bench-kernel==$version"
        fi
        environment/bin/python -I grade_response.py > scores
        ;;
    ruby)
        cp "$repository/bindings/ruby/examples/grade_response.rb" .
        export BUNDLE_GEMFILE="$PWD/Gemfile" BUNDLE_PATH="$PWD/gems"
        printf 'source "https://rubygems.org"\ngem "genius-bench-kernel", "= %s"\n' "$version" > Gemfile
        if [[ -n $package ]]; then
            mkdir -p vendor/cache
            if [[ -d $package ]]; then
                cp "$package"/*.gem vendor/cache/
            else
                cp "$package" vendor/cache/
            fi
            bundle install --local
        else
            bundle install
        fi
        bundle exec ruby -e 'abort "Bundler selected the source gem" if Gem.loaded_specs.fetch("genius-bench-kernel").platform == Gem::Platform::RUBY'
        bundle exec ruby grade_response.rb > scores
        ;;
    *) usage ;;
esac

printf '%s\n' 'correctness true' 'scope false' > expected
diff -u expected scores

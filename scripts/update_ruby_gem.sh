#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
    verify)
        version=$(ruby -r ./bindings/ruby/lib/genius_bench/version -e 'print GeniusBench::VERSION')
        bash scripts/verify_install.sh ruby "$version" pkg
        ;;
    publish)
        for gem in pending/*.gem; do
            gem push "$gem"
        done
        ;;
    *)
        echo 'Usage: update_ruby_gem.sh verify|publish' >&2
        exit 2
        ;;
esac

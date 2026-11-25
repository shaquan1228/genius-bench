#!/usr/bin/env bash
set -euo pipefail

version=$(python3 scripts/release_version.py)
bash scripts/verify_install.sh python "${version#v}" dist/*.whl

#!/usr/bin/env bash
set -euo pipefail

publish=$(python3 scripts/stage_release.py "$1" "${GITHUB_REF_NAME#v}" dist pending)
echo "publish=$publish" >> "$GITHUB_OUTPUT"

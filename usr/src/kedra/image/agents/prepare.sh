#!/usr/bin/env bash
# Explicit Actions input preparation; does not install a host agent or profile.
set -euo pipefail
test "${GITHUB_ACTIONS:-}" = true
test "$#" -eq 4
target=$1
context=$2
inputs=$3
evidence=$4
test -d "$context"
uv run usr/src/kedra/image/agents/fetch.py --target "$target" --output "$inputs" --verification-tools
uv run usr/src/kedra/image/agents/package.py --target "$target" --inputs "$inputs" --output "$context/agents.tar" --evidence "$evidence"

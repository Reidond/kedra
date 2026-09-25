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
python3 build/agents/fetch.py --target "$target" --output "$inputs" --verification-tools
python3 build/agents/package.py --target "$target" --inputs "$inputs" --output "$context/agents.tar" --evidence "$evidence"

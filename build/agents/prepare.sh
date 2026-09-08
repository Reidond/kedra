#!/usr/bin/env bash
# Explicit Actions input preparation; does not install a host agent or profile.
set -euo pipefail
test "${GITHUB_ACTIONS:-}" = true
test "$#" -eq 3
context=$1
inputs=$2
evidence=$3
test -d "$context"
python3 build/agents/fetch.py --output "$inputs" --verification-tools
python3 build/agents/package.py --inputs "$inputs" --output "$context/agents.tar" --evidence "$evidence"

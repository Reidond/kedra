#!/usr/bin/env bash
# Explicit Actions input preparation; does not install a host agent or profile.
# Also runs in the disposable local-build container of usr/src/kedra/tests/container.
set -euo pipefail
test "${GITHUB_ACTIONS:-}" = true || {
    test "${KEDRA_LOCAL_BUILDER:-}" = 1 && { test -f /.dockerenv || test -f /run/.containerenv; }
}
test "$#" -eq 4
target=$1
context=$2
inputs=$3
evidence=$4
test -d "$context"
uv run usr/src/kedra/image/agents/fetch.py --target "$target" --output "$inputs" --verification-tools
uv run usr/src/kedra/image/agents/package.py --target "$target" --inputs "$inputs" --output "$context/agents.tar" --evidence "$evidence"

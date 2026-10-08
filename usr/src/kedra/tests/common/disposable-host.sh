# Sourced, with disposable_host.py options, by the bash boot-level drivers from the
# repository root. It refuses anything but a hosted runner or a declared disposable
# host and exports the run variables the drivers and their fixture scripts read.
kedra_host=$(uv run usr/src/kedra/tests/common/disposable_host.py --command jq "$@")
RUNNER_TEMP=$(jq -er .runner_temp <<< "$kedra_host")
GITHUB_SHA=$(jq -er .source_revision <<< "$kedra_host")
GITHUB_RUN_ID=$(jq -er .run_id <<< "$kedra_host")
GITHUB_RUN_ATTEMPT=$(jq -er .run_attempt <<< "$kedra_host")
export RUNNER_TEMP GITHUB_SHA GITHUB_RUN_ID GITHUB_RUN_ATTEMPT
unset kedra_host

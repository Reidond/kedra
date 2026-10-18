#!/usr/bin/env bash
# Build one target's catalog packages and their contribution through the engine,
# offline from the run's RPM lock. The release build (image/release/build.sh) and
# kedra-lab's full local build both run this script, as an ordinary user.
#
# Package identities follow the Fedora material only, never the run's foundation
# image: the runtime root is the lock's exact Fedora base plus its foundation role,
# and each builder root that base plus one builder role, all installed offline by
# the root's own rpm in the sandbox and refused unless the result equals the role's
# locked inventory. Contribute and compose accept the packages only because the
# runtime root's RPMs equal the foundation's exactly. The lock's RPM objects must
# already be in the store.
#
# Usage: build-packages.sh --sysroot BIN --store DIR --target ID --lock RPM_LOCK
#          --base-image IMAGE_ID --foundation IMAGE_ID --source-revision SHA
#          --inputs RESOLVED_INPUTS --pins CATALOG_PINS --work NEW_DIR
#          [--cache REPOSITORY --trusted COMPOSITION]
# Writes the per-step receipts to the work directory; the contribution is
# WORK/contribution/ and its result WORK/contribution-result.json.
set -euo pipefail
image=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cache_args=()
while test $# -gt 0; do
    case $1 in
        --sysroot) sysroot=$2 ;;
        --store) store=$2 ;;
        --target) target=$2 ;;
        --lock) lock=$2 ;;
        --base-image) base_id=$2 ;;
        --foundation) foundation=$2 ;;
        --source-revision) revision=$2 ;;
        --inputs) inputs=$2 ;;
        --pins) pins=$2 ;;
        --work) work=$2 ;;
        # Binary cache: a package closure the trusted records (a signature-verified
        # image's composition.json) hold is pulled and admitted only if every object
        # equals its record; anything else, and any cache failure, builds.
        --cache) cache_args+=(--cache "$2") ;;
        --trusted) cache_args+=(--trusted "$2") ;;
        *) echo "build-packages.sh: unknown argument $1" >&2; exit 2 ;;
    esac
    shift 2
done
for name in sysroot store target lock base_id foundation revision inputs pins work; do
    if test -z "${!name:-}"; then
        echo "build-packages.sh: a required argument is missing ($name)" >&2
        exit 2
    fi
done
language=(--input-root "$image/packages" --entry catalog.kedra --target "$target"
    --lock packages.lock.json --target-policy "$image/package-policy.json")
mkdir -m 700 "$work"
"$sysroot" store add-image --store "$store" --image "$base_id" > "$work/base.json"
"$sysroot" store add-root --store "$store" --image "$base_id" --lock "$lock" > "$work/runtime.json"
runtime=$(jq -er .root "$work/runtime.json")
jq -n --arg runtime "$runtime" '{runtime:$runtime,builders:{}}' > "$work/resolution.json"
mapfile -t roles < <(jq -r '.builders | keys[]' "$pins")
test "${#roles[@]}" -gt 0
index=0
for role in "${roles[@]}"; do
    "$sysroot" store add-root --store "$store" --image "$base_id" --lock "$lock" --builder "$role" \
        > "$work/builder-$index.json"
    builder=$(jq -er .root "$work/builder-$index.json")
    jq --arg role "$role" --arg root "$builder" '.builders[$role]=$root' "$work/resolution.json" \
        > "$work/resolution.next.json"
    mv "$work/resolution.next.json" "$work/resolution.json"
    index=$((index+1))
done
jq --slurpfile resolution "$work/resolution.json" \
    '{namespace:.namespace,packages:(.packages|keys),builder_roots:($resolution[0].builders|[.[]]|unique),runtime_roots:[$resolution[0].runtime],source_objects:(.archives|keys)}' \
    "$pins" > "$work/policy.json"
uv run --quiet "$image/catalog/prepare-sources.py" --pins "$pins" --output "$work/sources" >&2
mapfile -t source_objects < <(jq -r 'keys[]' "$work/sources/sources.json")
for expected in "${source_objects[@]}"; do
    source=$(jq -er --arg object "$expected" '.[$object]' "$work/sources/sources.json")
    "$sysroot" store add-source --store "$store" --source "$source" > "$work/$expected-source.json"
    test "$(jq -er .object "$work/$expected-source.json")" = "$expected"
done
mapfile -t packages < <(jq -r '.packages | keys[]' "$pins")
for package in "${packages[@]}"; do
    "$sysroot" catalog build --store "$store" --package "$package" "${language[@]}" \
        --policy "$work/policy.json" --resolution "$work/resolution.json" "${cache_args[@]}" \
        > "$work/$package-build.json"
done
material=$(sha256sum "$inputs" | cut -d ' ' -f 1)
"$sysroot" catalog contribute --store "$store" "${language[@]}" --resolution "$work/resolution.json" \
    --foundation "$foundation" --policy "$work/policy.json" --source-revision "$revision" \
    --input-material "$inputs" --expected-input-material-sha256 "$material" \
    --output-dir "$work/contribution" > "$work/contribution-result.json"

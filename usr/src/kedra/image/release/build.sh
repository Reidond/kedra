#!/usr/bin/env bash
# Build one release target's unsigned candidate from the preflight's outputs
# (`refresh.py prepare`): the RPM-locked foundation, the catalog packages, the
# composition and derivation of the image steps the target's .kedra block
# declares, then the public trust layer. One pipeline for every target, natively
# on its runner. Run from the repository root by release-target.yml as an
# ordinary user with sudo; it never signs, publishes or installs anything.
#
# Usage: build.sh TARGET SOURCE_REVISION SCRATCH_DIRECTORY
set -euo pipefail
target=$1 revision=$2 scratch=$3
evidence=output/image-evidence
binaries=output/release-binaries
identity_file=output/release-context/image-identity.json
store=$scratch/kedra-release-store
foundation_root=$scratch/kedra-release-foundation
native_root=$scratch/kedra-release-native
catalog_root=$scratch/kedra-catalog

# Packages build, and `sysroot system compose` observes the foundation, in the
# engine's unprivileged bubblewrap sandbox, as on an installed Kedra.
bwrap --unshare-all --unshare-user --disable-userns --ro-bind / / /bin/true

input_hash=$(sha256sum "$evidence/resolved-inputs.json" | cut -d ' ' -f 1)
uv run usr/src/kedra/image/release/compose.py foundation \
    --target "$target" --repo "$PWD" --source-revision "$revision" \
    --inputs "$evidence/resolved-inputs.json" --expected-inputs-sha256 "$input_hash" \
    --context "output/$target-context" --store "$store" \
    --binaries "$binaries" --output-dir "$foundation_root" \
    > "$evidence/foundation.json"
foundation_hash=$(sha256sum "$foundation_root/foundation.json" | cut -d ' ' -f 1)
# The identity records the preflight's source plan bytes; the foundation's
# installed source manifest must be exactly those bytes.
test "$(jq -er .source_manifest_sha256 "$foundation_root/foundation.json")" = \
    "$(jq -er .source_manifest_sha256 "$identity_file")"
foundation=$(jq -er .foundation_image "$foundation_root/foundation.json")

base=$(jq -er .base "$evidence/rpm-lock.json")
docker pull --quiet "$base"
base_id=$(docker image inspect --format '{{.Id}}' "$base")
cache=$(jq -er --arg target "$target" '.targets[$target].cache' usr/src/kedra/image/release/targets.json)
cache_args=()
# Closures are admitted only against the previous signature-verified stable image's
# records, copied by preflight. The registry is never trusted.
if test -s "$evidence/previous-composition.json"; then
    cache_args=(--cache "$cache" --trusted "$evidence/previous-composition.json")
fi
usr/src/kedra/image/catalog/build-packages.sh --sysroot "$binaries/sysroot" --store "$store" \
    --target "$target" --lock "$evidence/rpm-lock.json" --base-image "$base_id" \
    --foundation "$foundation" --source-revision "$revision" \
    --inputs "$evidence/resolved-inputs.json" --pins "$binaries/catalog-pins.json" \
    --work "$catalog_root" "${cache_args[@]}"
definition_hash=$(jq -er .definition_sha256 "$catalog_root/contribution-result.json")
contribution_hash=$(jq -er .contribution_receipt_sha256 "$catalog_root/contribution-result.json")
mkdir "$evidence/catalog"
cp "$catalog_root/"*.json "$evidence/catalog/"
cp "$catalog_root/contribution/"*.json "$evidence/catalog/"

uv run usr/src/kedra/image/release/compose.py compose \
    --target "$target" --repo "$PWD" --source-revision "$revision" \
    --inputs "$evidence/resolved-inputs.json" --expected-inputs-sha256 "$input_hash" \
    --foundation-receipt "$foundation_root/foundation.json" \
    --expected-foundation-receipt-sha256 "$foundation_hash" --store "$store" \
    --definition "$catalog_root/contribution/system.json" --expected-definition-sha256 "$definition_hash" \
    --contribution-receipt "$catalog_root/contribution/contribution.json" \
    --expected-contribution-receipt-sha256 "$contribution_hash" \
    --binaries "$binaries" --output-dir "$native_root" \
    > "$evidence/native-candidate.json"
candidate=$(jq -er .transfer.reference "$evidence/native-candidate.json")
expected=$(jq -er .transfer.destination_manifest_digest "$evidence/native-candidate.json")
actual="sha256:$(sudo skopeo inspect --raw "containers-storage:$candidate" | sha256sum | cut -d ' ' -f 1)"
test "$actual" = "$expected"
expected_id=$(jq -er .transfer.image "$evidence/native-candidate.json")
test "$(sudo podman image inspect "$candidate" --format '{{.Id}}')" = "${expected_id#sha256:}"
cp "$native_root/native-plan.json" "$evidence/"
cp "$native_root/native-material/native-receipt.json" "$evidence/"

identity=$(cat "$identity_file")
sudo podman build --pull=never --network=none --build-arg "CANDIDATE_IMAGE=$candidate" \
    --build-arg "KEDRA_IDENTITY=$identity" --file usr/src/kedra/image/release/Containerfile \
    --tag "localhost/kedra-$target:release-candidate" output/release-context \
    2>&1 | tee "$evidence/trust-build.log"

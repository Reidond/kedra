#!/usr/bin/env bash
# Actions-only: use the exact signed candidate; no production key is available.
set -euo pipefail
test "${GITHUB_ACTIONS:-}" = true
test "${GITHUB_REPOSITORY:-}" = Reidond/kedra
test "${GITHUB_REF:-}" = refs/heads/main
[[ "${KEDRA_CANDIDATE_DIGEST:-}" =~ ^sha256:[a-f0-9]{64}$ ]]
[[ "${KEDRA_PUBLIC_FINGERPRINT:-}" =~ ^[a-f0-9]{64}$ ]]
[[ "${KEDRA_RESOLVED_AT:-}" =~ ^[0-9]{1,12}$ ]]
test "$(git rev-parse HEAD)" = "$GITHUB_SHA"
evidence="$PWD/output/release-evidence"
media="$PWD/output/release-installer"
context="$PWD/output/release-anaconda-context"
mkdir -p "$evidence" "$media" "$context"
sudo apt-get update
sudo apt-get install -y --no-install-recommends podman skopeo xorriso squashfs-tools attr qemu-system-x86
rustup toolchain install 1.98.1 --profile minimal
cargo build --workspace --release --locked
target/release/sysroot source plan --host desktop --json > "$evidence/source-plan.json"
python3 build/release/prepare-trust.py --source "$evidence/source-plan.json" \
    --public-key build/release/authority/desktop.pub --expected-fingerprint "$KEDRA_PUBLIC_FINGERPRINT" \
    --sysroot target/release/sysroot --output "$context/trust"
sudo mkdir -p /etc/containers/registries.d
sudo cp "$context/trust/registries.yaml" /etc/containers/registries.d/kedra.yaml
jq --arg key "$context/trust/release.pub" '.transports[][][].keyPath=$key' \
    "$context/trust/policy.json" > "$evidence/host-policy.json"
payload="ghcr.io/reidond/kedra-desktop@$KEDRA_CANDIDATE_DIGEST"
# Deliberately anonymous: an owner must be able to install/update without GHCR credentials.
sudo skopeo --policy "$evidence/host-policy.json" copy --preserve-digests \
    --digestfile "$evidence/payload.digest" "docker://$payload" "containers-storage:$payload" \
    > "$evidence/verified-pull.log" 2>&1
test "$(cat "$evidence/payload.digest")" = "$KEDRA_CANDIDATE_DIGEST"
sudo podman run --rm "$payload" cat /usr/share/sysroot/source.json > "$context/trust/source.json"
sudo podman run --rm "$payload" cat /usr/share/sysroot/packages.txt > "$evidence/packages.txt"
python3 - "$context/trust/source.json" "$evidence/source-plan.json" <<'PY'
import json, pathlib, sys
installed, expected = [json.loads(pathlib.Path(p).read_bytes()) for p in sys.argv[1:]]
if installed != expected:
    raise SystemExit('Signed image source differs from the accepted source plan')
PY
# Public trust must also match the bytes actually installed in the candidate.
for item in release.pub release-policy.json; do
    sudo podman run --rm "$payload" cat "/usr/lib/sysroot/trust/$item" > "$evidence/installed-$item"
    cmp "$context/trust/$item" "$evidence/installed-$item"
done
for item in policy.json registries.yaml install.toml; do
    case "$item" in
        policy.json) installed=/etc/containers/policy.json ;;
        registries.yaml) installed=/etc/containers/registries.d/kedra.yaml ;;
        install.toml) installed=/usr/lib/bootc/install/10-kedra.toml ;;
    esac
    sudo podman run --rm "$payload" cat "$installed" > "$evidence/installed-$item"
    cmp "$context/trust/$item" "$evidence/installed-$item"
done
cp "$context/trust/source.json" "$context/trust/release.pub" "$evidence/"
cp installer/{Containerfile,iso.yaml,prepare.sh,boot-probe.service,boot-probe.sh,anaconda-adapter.py,finalize-fstab.py,verify-payload.service,require-verification.conf} "$context/"
cp target/release/sysroot-helper "$context/helper"
base=$(jq -er .base build/research/inputs.json)
builder=$(jq -er .builder installer/inputs.json)
sudo podman run --rm "$builder" --version > "$evidence/builder-version.txt"
sudo podman run --rm "$builder" build --help > "$evidence/builder-help.txt"
grep -q -- '--bootc-installer-payload-ref' "$evidence/builder-help.txt"
sudo podman build --pull=always --no-cache --build-arg "BASE_IMAGE=$base" --build-arg "PAYLOAD_IMAGE=$payload" \
    --build-arg "SOURCE_DATE_EPOCH=$(git show -s --format=%ct HEAD)" --tag localhost/kedra-anaconda:research "$context" \
    > "$evidence/anaconda-build.log" 2>&1
# The local tag is the existing builder interface; installed origin stays exact GHCR.
sudo podman run --rm --privileged --security-opt label=type:unconfined_t \
    -e "KEDRA_INSTALLER_PAYLOAD=$payload" -v "$media:/output" -v "$evidence:/evidence" \
    -v "$PWD/installer:/kedra-installer:ro" -v /var/lib/containers/storage:/var/lib/containers/storage \
    --entrypoint /bin/bash "$builder" /kedra-installer/build-iso.sh > "$evidence/iso-build.log" 2>&1
sudo chown -R "$(id -u):$(id -g)" "$media"
mapfile -t images < <(find "$media" -type f -name '*.iso')
test "${#images[@]}" -eq 1
name="kedra-desktop-44-$GITHUB_RUN_ID-$GITHUB_RUN_ATTEMPT.iso"
mv -- "${images[0]}" "$media/$name"
sha256sum "$media/$name" > "$evidence/SHA256SUMS"
mkdir -p "$evidence/smoke" output/release-inspection
xorriso -osirrox on -indev "$media/$name" \
    -extract /images/pxeboot "$PWD/output/release-inspection/pxeboot" \
    -extract /EFI/BOOT/grub.cfg "$evidence/grub.cfg"
sudo python3 installer/smoke.py --iso "$media/$name" --kernel-dir "$PWD/output/release-inspection/pxeboot" \
    --work "$evidence/smoke" | tee "$evidence/smoke-result.txt"
# Download parts stay below GitHub's per-asset ceiling. The whole ISO remains the signed identity.
split --bytes=1900M --numeric-suffixes=0 --suffix-length=2 "$media/$name" "$media/$name.part"
python3 - "$media/$name" "$evidence" "$base" "$builder" <<'PY'
import hashlib, json, os, pathlib, sys
iso, evidence = map(pathlib.Path, sys.argv[1:3])
with iso.open('rb') as stream:
    digest = hashlib.file_digest(stream, 'sha256').hexdigest()
source = (evidence / 'source.json').read_bytes()
record = {'schema_version': 2, 'project': 'Kedra',
    'scope': {'target': 'desktop', 'architecture': 'x86_64', 'fedora_release': 44, 'repository': 'ghcr.io/reidond/kedra-desktop'},
    'source_revision': os.environ['GITHUB_SHA'],
    'build': {'repository': 'Reidond/kedra', 'workflow': '.github/workflows/release.yml',
              'run_id': int(os.environ['GITHUB_RUN_ID']), 'run_attempt': int(os.environ['GITHUB_RUN_ATTEMPT'])},
    'image_digest': os.environ['KEDRA_CANDIDATE_DIGEST'],
    'last_successful_resolution': int(os.environ['KEDRA_RESOLVED_AT']),
    'home_manifest_sha256': hashlib.sha256(source).hexdigest(),
    'packages_sha256': hashlib.sha256((evidence / 'packages.txt').read_bytes()).hexdigest(),
    'installer': {'filename': iso.name, 'size_bytes': iso.stat().st_size, 'sha256': digest},
    'parts': [p.name for p in sorted(iso.parent.glob(iso.name + '.part*'))],
    'approval': 'candidate', 'fresh_install_qualified': False}
# A small record of observed candidate/installer evidence, not a standardized SBOM
# or an independently attested build. The inventory was read from the verified payload.
provenance = {key: record[key] for key in ['project', 'scope', 'source_revision', 'build',
    'image_digest', 'home_manifest_sha256', 'packages_sha256', 'last_successful_resolution', 'installer']}
provenance.update(schema_version=1, format='kedra-candidate-provenance', installer_inputs={
    'base_image': sys.argv[3], 'builder_image': sys.argv[4],
    'builder_version': (evidence / 'builder-version.txt').read_text().strip()})
provenance_bytes = (json.dumps(provenance, sort_keys=True, indent=2) + '\n').encode()
(evidence / 'provenance.json').write_bytes(provenance_bytes)
record['provenance_sha256'] = hashlib.sha256(provenance_bytes).hexdigest()
record['parts_sha256'] = {}
for name in record['parts']:
    with (iso.parent / name).open('rb') as stream:
        record['parts_sha256'][name] = hashlib.file_digest(stream, 'sha256').hexdigest()
(evidence / 'candidate.json').write_text(json.dumps(record, indent=2) + '\n')
PY
printf 'Exact signed payload passed offline installer startup. Fresh installation and promotion remain pending; see candidate.json.\n' >> "$GITHUB_STEP_SUMMARY"

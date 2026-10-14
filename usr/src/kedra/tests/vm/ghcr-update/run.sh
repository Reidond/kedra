#!/usr/bin/env bash
# Disposable x86_64 Linux host with KVM only: this binds the ghcr.io hostname to
# a local registry and installs its test CA. From the repository root, after
# `cargo build --workspace --release --locked`:
#   bash usr/src/kedra/tests/vm/ghcr-update/run.sh --disposable-host --work NEW_DIRECTORY
set -euo pipefail
test "$#" -eq 3 && test "$1" = --disposable-host && test "$2" = --work
uv run usr/src/kedra/tests/common/qemu.py guard --disposable-host
test "$(uname -m)" = x86_64 && test -r /dev/kvm && test -w /dev/kvm
test -x target/release/sysroot && test -x target/release/sysroot-helper
mkdir -m 0700 "$3"
work=$(realpath -e "$3")
root="$work/kedra-ghcr"
private="$work/kedra-ghcr-private"
mkdir "$root" "$private"
chmod 0700 "$private"
mkdir -p "$root/context" "$root/image" "$root/cases" "$work/evidence"
evidence="$work/evidence"
controller_pid=
cleanup() {
    if test -n "$controller_pid"; then kill "$controller_pid" 2>/dev/null || true; wait "$controller_pid" 2>/dev/null || true; fi
    sudo podman logs kedra-ghcr-registry > "$evidence/registry.log" 2>&1 || true
    sudo podman stop kedra-ghcr-registry >/dev/null 2>&1 || true
    test "$private" = "$work/kedra-ghcr-private" && rm -rf -- "$private"
}
trap cleanup EXIT
base=$(uv run usr/src/kedra/tests/common/resolve-fedora-base.py --output "$evidence/base-resolution.json")
printf '%s\n' "$base" > "$root/base.txt"
builder=$(jq -er .platforms.amd64.builder usr/src/kedra/image/inputs.json)
registry=docker.io/library/registry@sha256:7518da9b12dd746278282a729dee2e65eabdeb449db4d0b28d46ef6e90308f58
uv run usr/src/kedra/tests/common/secure_boot.py provenance --evidence "$evidence"
# Resolve/pull external build tools before introducing the local-only GHCR test domain.
sudo podman pull "$base" > "$evidence/base-pull.log" 2>&1
sudo podman pull "$builder" > "$evidence/builder-pull.log" 2>&1
sudo podman pull "$registry" > "$evidence/registry-pull.log" 2>&1
openssl rand -base64 32 > "$private/passphrase"
chmod 0600 "$private/passphrase"
for key in allowed wrong; do
    skopeo generate-sigstore-key --output-prefix "$private/$key" --passphrase-file "$private/passphrase"
done
openssl req -x509 -newkey rsa:3072 -nodes -days 1 -subj /CN=ghcr.io \
    -addext subjectAltName=DNS:ghcr.io,IP:127.0.0.1 -keyout "$private/tls.key" -out "$root/context/tls.crt" \
    2> "$evidence/tls-generation.log"
uv run usr/src/kedra/tests/vm/ghcr-update/prepare.py --root "$root" --source-revision "$(git rev-parse --verify 'HEAD^{commit}')"
cp usr/src/kedra/tests/vm/ghcr-update/{Containerfile,variant.Containerfile,check.py,check.service,identity-recovery.py} "$root/context/"
cp usr/src/kedra/tests/common/{console.toml,secure_boot_check.py} "$root/context/"
cp target/release/sysroot "$root/context/sysroot"
cp target/release/sysroot-helper "$root/context/helper"
sudo mkdir -p /etc/containers/certs.d/ghcr.io /etc/containers/registries.d
sudo cp "$root/context/tls.crt" /etc/containers/certs.d/ghcr.io/ca.crt
sudo cp "$root/context/registries.yaml" /etc/containers/registries.d/kedra-ghcr-test.yaml
printf '\n127.0.0.1 ghcr.io\n' | sudo tee -a /etc/hosts >/dev/null
test "$(getent ahostsv4 ghcr.io | awk '{print $1}' | sort -u)" = 127.0.0.1
sudo podman run -d --name kedra-ghcr-registry -p 127.0.0.1:443:5000 \
    -v "$root/context/tls.crt:/certs/tls.crt:ro" -v "$private/tls.key:/certs/tls.key:ro" \
    -e REGISTRY_HTTP_TLS_CERTIFICATE=/certs/tls.crt -e REGISTRY_HTTP_TLS_KEY=/certs/tls.key \
    -e REGISTRY_STORAGE_DELETE_ENABLED=true \
    -e OTEL_TRACES_EXPORTER=none "$registry"
for attempt in $(seq 1 30); do
    if curl --silent --fail --cacert "$root/context/tls.crt" https://127.0.0.1/v2/ >/dev/null; then break; fi
    sleep 1
done
curl --silent --fail --cacert "$root/context/tls.crt" https://127.0.0.1/v2/ >/dev/null
sudo podman build --pull=never --build-arg "BASE_IMAGE=$base" --tag localhost/kedra-ghcr:base "$root/context" \
    > "$evidence/base-build.log" 2>&1
sudo podman run --rm --network=none localhost/kedra-ghcr:base rpm -qa \
    --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' > "$root/package-material.txt"
uv run usr/src/kedra/tests/vm/ghcr-update/prepare.py --root "$root" finalize
for variant in A B C E U W R N T X H M; do
    identity=$(cat "$root/context/$variant/image-identity.json")
    sudo podman build --pull=never --build-arg "KEDRA_IDENTITY=$identity" --build-arg "VARIANT=$variant" \
        --file "$root/context/variant.Containerfile" --tag "localhost/kedra-ghcr:$variant" "$root/context" \
        > "$evidence/build-$variant.log" 2>&1
    signing=(--sign-by-sigstore-private-key "$private/allowed.private" --sign-passphrase-file "$private/passphrase")
    if test "$variant" = U; then signing=(); fi
    if test "$variant" = W; then signing=(--sign-by-sigstore-private-key "$private/wrong.private" --sign-passphrase-file "$private/passphrase"); fi
    repo=ghcr.io/reidond/kedra-desktop
    if test "$variant" = R; then repo=ghcr.io/reidond/kedra-other; fi
    sudo skopeo copy --preserve-digests "${signing[@]}" --digestfile "$root/$variant.digest" \
        "containers-storage:localhost/kedra-ghcr:$variant" "docker://$repo:$variant" > "$evidence/sign-$variant.log" 2>&1
    if test "$variant" = R; then
        sudo skopeo copy --preserve-digests "docker://$repo:$variant" docker://ghcr.io/reidond/kedra-desktop:R \
            > "$evidence/copy-wrong-repository.log" 2>&1
    fi
done
# Remove only the disposable N attachment while leaving its signed image bytes intact.
missing_digest=$(cat "$root/N.digest")
signature_tag="sha256-${missing_digest#sha256:}.sig"
sudo skopeo inspect --raw "docker://ghcr.io/reidond/kedra-desktop:$signature_tag" > "$evidence/removed-signature-manifest.json"
attachment_digest=$(skopeo manifest-digest "$evidence/removed-signature-manifest.json")
curl --silent --show-error --fail --cacert "$root/context/tls.crt" -X DELETE \
    "https://127.0.0.1/v2/reidond/kedra-desktop/manifests/$attachment_digest"
uv run python - "$root" "$evidence" <<'PY'
import json, pathlib, sys
root, evidence = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
cases={'schema_version':1,'digests':{v:(root/(v+'.digest')).read_text().strip() for v in 'A B C E U W R N T X H M'.split()}}
(root/'cases/cases.json').write_text(json.dumps(cases,indent=2)+'\n')
(evidence/'cases.json').write_text(json.dumps(cases,indent=2)+'\n')
PY
uv run usr/src/kedra/tests/vm/ghcr-update/control.py --root "$root" > "$evidence/controller.log" 2>&1 &
controller_pid=$!
sleep 1
curl --silent --show-error --fail -X POST http://127.0.0.1:18080/A
jq --arg key "$root/context/release.pub" '.transports[][][].keyPath=$key' "$root/context/policy.json" > "$root/host-policy.json"
initial="ghcr.io/reidond/kedra-desktop@$(cat "$root/A.digest")"
sudo skopeo --policy "$root/host-policy.json" copy --preserve-digests "docker://$initial" "containers-storage:$initial" \
    > "$evidence/verified-initial.log" 2>&1
sudo podman run --rm --privileged --network=host --add-host ghcr.io:127.0.0.1 --security-opt label=type:unconfined_t \
    -v "$root/image:/output" -v /var/lib/containers/storage:/var/lib/containers/storage \
    -v /etc/containers/certs.d:/etc/containers/certs.d:ro \
    "$builder" --type qcow2 --rootfs ext4 --use-librepo=True "$initial" > "$evidence/disk-build.log" 2>&1
mapfile -t disks < <(find "$root/image" -type f -name '*.qcow2')
test "${#disks[@]}" -eq 1
truncate -s 16M "$root/cases.raw"
mkfs.ext4 -q -L KEDRA_GHCR_CASES -d "$root/cases" "$root/cases.raw"
uv=$(command -v uv)
# Negative UEFI Secure Boot case on the same disk (snapshot, no network): the
# same firmware with a db trusting only Ubuntu's snakeoil test key must refuse
# Fedora's Microsoft-signed shim and remain at its boot menu for 120 s.
sudo "$uv" run usr/src/kedra/tests/common/qemu.py refuse --disposable-host --accelerator kvm \
    --disk "${disks[0]}" --work "$root/vm-untrusted-keys" --serial "$evidence/untrusted-keys.serial.log" \
    --timeout 120 > "$evidence/untrusted-keys-result.json"
# Microsoft-enrolled UEFI Secure Boot variables persist across the three boots
# (shim's fallback may add a boot entry and reset once). The shared launcher
# creates them on the first boot and verifies their trust around every boot.
for phase in A B ROLLBACK; do
    sudo "$uv" run usr/src/kedra/tests/common/qemu.py boot --disposable-host --architecture x86_64 \
        --accelerator kvm --disk "${disks[0]}" --data-disk "$root/cases.raw" --vars "$root/OVMF_VARS.fd" \
        --work "$root/vm-$phase" --serial "$evidence/$phase.serial.log" --network --cpus 2 --memory-mib 4096 \
        --timeout 2400 --require KEDRA_SECUREBOOT_PASS --require "KEDRA_GHCR_${phase}_PASS" \
        --fail-marker KEDRA_GHCR_FAIL > "$evidence/$phase-result.json"
done
printf 'PASS: native v2 enrollment/check/stage/boot/identity recovery/rollback/hold/resume, cached-layer reuse and critical refusals under UEFI Secure Boot.\n' | tee "$evidence/result.txt"
uv run python - "$evidence" <<'PY'
import json,pathlib,sys
pathlib.Path(sys.argv[1], 'scope.json').write_text(json.dumps({
    'implemented_cases':['enroll/current','unsigned','wrong-key','wrong-repository','missing-signature',
        'wrong-target','wrong-architecture-identity','wrong-channel','malformed-identity','same-rank-equivocation',
        'offline','available','stage','pending-preservation','idempotent-stage','boot-B','lower-rank-replay',
        'identity-health-refusal-and-retained-rollback','boot-A','persistent-data','hold','resume',
        'uefi-secure-boot-microsoft-keys','untrusted-keys-firmware-refusal','first-verification-downloads-every-layer',
        'repeated-verification-reuses-cached-layers','check-then-stage-reuse','cache-persists-across-reboot',
        'untrusted-cache-discarded-without-following-links','cached-digest-signature-withdrawn'],
    'not_run_cases':['deterministic-tag-race','interrupted-helper-or-bootc-operation','legacy-state-migration',
        'native-OCI-platform-mismatch'],
    'production_registry_writes':False,'production_keys_used':False},indent=2)+'\n')
PY

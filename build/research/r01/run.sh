#!/usr/bin/env bash
# Run ONLY on a disposable Actions runner. Never call bootc on the host.
set -euo pipefail
test "${GITHUB_ACTIONS:-}" = true
test "${RUNNER_OS:-}" = Linux
test "${GITHUB_REPOSITORY:-}" = Reidond/kedra
test -n "${RUNNER_TEMP:-}"
root="$RUNNER_TEMP/kedra-r01"
private="$RUNNER_TEMP/kedra-r01-private"
mkdir "$root" "$private"
chmod 0700 "$private"
mkdir -p output/r01-evidence "$root/context/public" "$root/image" "$root/cases"
evidence="$PWD/output/r01-evidence"
base=$(jq -er .base build/research/inputs.json)
builder=$(jq -er .builder build/research/inputs.json)
registry_image=docker.io/library/registry@sha256:7518da9b12dd746278282a729dee2e65eabdeb449db4d0b28d46ef6e90308f58
repository=registry.kedra.test:5000/kedra/r01
{
    date --utc --iso-8601=seconds
    git rev-parse HEAD
    uname -a
    free -h
    df -h
    ls -l /dev/kvm
    podman --version
    skopeo --version
    openssl version
    qemu-system-x86_64 --version
    printf '%s\n' "$base" "$builder" "$registry_image"
} > "$evidence/environment.txt"
cleanup() {
    sudo podman logs kedra-r01-registry > "$evidence/registry.log" 2>&1 || true
    sudo podman stop kedra-r01-registry >/dev/null 2>&1 || true
    # Only generated disposable keys, outside every build context/artifact path.
    test "$private" = "$RUNNER_TEMP/kedra-r01-private" && rm -rf -- "$private"
}
trap cleanup EXIT
openssl rand -base64 32 > "$private/passphrase"
chmod 0600 "$private/passphrase"
skopeo generate-sigstore-key --output-prefix "$private/allowed" --passphrase-file "$private/passphrase"
skopeo generate-sigstore-key --output-prefix "$private/wrong" --passphrase-file "$private/passphrase"
openssl req -x509 -newkey rsa:3072 -nodes -days 1 \
    -subj /CN=registry.kedra.test -addext subjectAltName=DNS:registry.kedra.test \
    -keyout "$private/tls.key" -out "$root/context/tls.crt" 2> "$evidence/tls-generation.log"
cp "$private/allowed.pub" "$root/context/public/release.pub"
cat > "$root/context/registries.yaml" <<'EOF'
docker:
  registry.kedra.test:5000:
    use-sigstore-attachments: true
EOF
python3 build/research/r01/helper-fixtures.py prepare --root "$root"
sudo mkdir -p /etc/containers/registries.d /etc/containers/certs.d/registry.kedra.test:5000
sudo cp "$root/context/registries.yaml" /etc/containers/registries.d/kedra-r01.yaml
sudo cp "$root/context/tls.crt" /etc/containers/certs.d/registry.kedra.test:5000/ca.crt
printf '\n127.0.0.1 registry.kedra.test\n' | sudo tee -a /etc/hosts >/dev/null
sudo podman run -d --name kedra-r01-registry -p 127.0.0.1:5000:5000 \
    -v "$root/context/tls.crt:/certs/tls.crt:ro" -v "$private/tls.key:/certs/tls.key:ro" \
    -e REGISTRY_HTTP_TLS_CERTIFICATE=/certs/tls.crt -e REGISTRY_HTTP_TLS_KEY=/certs/tls.key \
    -e REGISTRY_STORAGE_DELETE_ENABLED=true -e OTEL_TRACES_EXPORTER=none "$registry_image"
for attempt in $(seq 1 20); do
    if curl --silent --fail --cacert "$root/context/tls.crt" https://registry.kedra.test:5000/v2/; then break; fi
    sleep 1
done
curl --silent --fail --cacert "$root/context/tls.crt" https://registry.kedra.test:5000/v2/ > /dev/null
cp build/research/r01/{Containerfile,check.sh,check.service,install.toml} "$root/context/"
cp build/research/console.toml "$root/context/"
cp target/release/sysroot-helper "$root/context/helper"
for variant in A B U W M; do
    sudo podman build --pull=always --build-arg "BASE_IMAGE=$base" --build-arg "VARIANT=$variant" \
        --tag "localhost/kedra-r01:$variant" "$root/context" > "$evidence/build-$variant.log" 2>&1
    signing=(--sign-by-sigstore-private-key "$private/allowed.private" --sign-passphrase-file "$private/passphrase")
    if test "$variant" = U; then signing=(); fi
    if test "$variant" = W; then signing=(--sign-by-sigstore-private-key "$private/wrong.private" --sign-passphrase-file "$private/passphrase"); fi
    sudo skopeo copy "${signing[@]}" --digestfile "$root/$variant.digest" \
        "containers-storage:localhost/kedra-r01:$variant" "docker://$repository:$variant" > "$evidence/push-$variant.log" 2>&1
done
# A valid signature in another repository still has no authority for this target.
sudo skopeo copy --sign-by-sigstore-private-key "$private/allowed.private" --sign-passphrase-file "$private/passphrase" \
    --digestfile "$root/wrong-repository.digest" containers-storage:localhost/kedra-r01:B \
    docker://registry.kedra.test:5000/kedra/other:B > "$evidence/push-wrong-repository.log" 2>&1
# Delete only M's signature attachment, retaining the image manifest and layers.
missing_digest=$(cat "$root/M.digest")
signature_tag="sha256-${missing_digest#sha256:}.sig"
sudo skopeo inspect --raw "docker://$repository:$signature_tag" > "$evidence/removed-signature-manifest.json"
attachment_digest=$(skopeo manifest-digest "$evidence/removed-signature-manifest.json")
curl --silent --show-error --fail --cacert "$root/context/tls.crt" -X DELETE \
    "https://registry.kedra.test:5000/v2/kedra/r01/manifests/$attachment_digest"

jq -n --arg a "$repository@$(cat "$root/A.digest")" --arg b "$repository@$(cat "$root/B.digest")" \
    --arg u "$repository@$(cat "$root/U.digest")" --arg w "$repository@$(cat "$root/W.digest")" \
    --arg m "$repository@$missing_digest" --arg other "registry.kedra.test:5000/kedra/other@$(cat "$root/wrong-repository.digest")" \
    '{schema_version:1,initial_a:$a,valid_b:$b,unsigned:$u,wrong_key:$w,missing_attachment:$m,wrong_repository:$other}' > "$root/cases/cases.json"
cp "$root/cases/cases.json" "$evidence/cases.json"
python3 build/research/r01/helper-fixtures.py requests --root "$root"
cp "$root/cases"/helper-*.json "$evidence/"
cp "$root/context/policy.json" "$root/context/registries.yaml" "$root/context/public/release.pub" "$evidence/"
# Verify A before copying it into the local builder store; first-boot policy is
# still a separate installer-handoff gate, not established by this copy alone.
jq --arg key "$root/context/public/release.pub" '.transports[][][].keyPath=$key' "$root/context/policy.json" > "$root/host-policy.json"
initial=$(jq -er .initial_a "$root/cases/cases.json")
sudo skopeo --policy "$root/host-policy.json" copy "docker://$initial" "containers-storage:$initial" > "$evidence/verified-a-copy.log" 2>&1
sudo podman run --rm "$builder" --version > "$evidence/builder-version.txt"
sudo podman run --rm --privileged --security-opt label=type:unconfined_t \
    -v "$root/image:/output" -v /var/lib/containers/storage:/var/lib/containers/storage \
    -v /etc/containers/certs.d:/etc/containers/certs.d:ro \
    "$builder" --type qcow2 --rootfs ext4 --use-librepo=True "$initial" > "$evidence/qcow2-build.log" 2>&1
mapfile -t disks < <(find "$root/image" -type f -name '*.qcow2')
test "${#disks[@]}" -eq 1
truncate -s 16M "$root/cases.raw"
mkfs.ext4 -q -L KEDRA_CASES -d "$root/cases" "$root/cases.raw"
cp /usr/share/OVMF/OVMF_VARS_4M.fd "$root/OVMF_VARS.fd"
for phase in stage-b boot-b rollback-a; do
    sudo timeout 900 qemu-system-x86_64 -machine q35,accel=kvm -cpu host -smp 2 -m 4096 \
        -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
        -drive "if=pflash,format=raw,file=$root/OVMF_VARS.fd" \
        -drive "file=${disks[0]},if=virtio,format=qcow2" \
        -drive "file=$root/cases.raw,if=virtio,format=raw,readonly=on" \
        -nic user,model=virtio-net-pci -display none -serial stdio -monitor none > "$evidence/$phase.serial.log" 2>&1
    ! grep -q KEDRA_R01_FAIL "$evidence/$phase.serial.log"
    case "$phase" in
        stage-b) grep -q KEDRA_R01_STAGE_PASS "$evidence/$phase.serial.log" ;;
        boot-b) grep -q KEDRA_R01_BOOT_B_ROLLBACK_STAGED_PASS "$evidence/$phase.serial.log" ;;
        rollback-a) grep -q KEDRA_R01_ROLLBACK_PRESERVES_DATA_PASS "$evidence/$phase.serial.log" ;;
    esac
done
echo 'PASS: disposable signature negative cases, A-to-B boot and retained-A rollback.' | tee "$evidence/result.txt"

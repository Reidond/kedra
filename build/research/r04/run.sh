#!/usr/bin/env bash
# Actions-only signed graphical A/B/rollback experiment. No workstation disks.
set -euo pipefail
test "${GITHUB_ACTIONS:-}" = true
test "${RUNNER_OS:-}" = Linux
test "${GITHUB_REPOSITORY:-}" = Reidond/kedra
root="$RUNNER_TEMP/kedra-r04"
private="$RUNNER_TEMP/kedra-r04-private"
mkdir "$root" "$private"
chmod 0700 "$private"
mkdir -p "$root/desktop" "$root/image" output/r04-evidence
evidence="$PWD/output/r04-evidence"
base=$(jq -er .base build/research/inputs.json)
builder=$(jq -er .builder build/research/inputs.json)
repository=registry.kedra.test:5000/kedra/r04
registry_image=docker.io/library/registry@sha256:7518da9b12dd746278282a729dee2e65eabdeb449db4d0b28d46ef6e90308f58
cleanup() {
    sudo podman logs kedra-r04-registry > "$evidence/registry.log" 2>&1 || true
    sudo podman stop kedra-r04-registry >/dev/null 2>&1 || true
    python3 - <<'PY'
import os, pathlib, re, shutil
private = pathlib.Path(os.environ['RUNNER_TEMP']).resolve() / 'kedra-r04-private'
log = private / 'builder.log'
if log.exists():
    text = re.sub(r'\$6\$[^\s"\x27]+', '<synthetic-password-hash-redacted>', log.read_text(errors='replace'))
    pathlib.Path('output/r04-evidence/qcow2-build.log').write_text(text)
if private.is_dir():
    shutil.rmtree(private)
PY
}
trap cleanup EXIT
{
    date --utc --iso-8601=seconds
    git rev-parse HEAD
    uname -a
    free -h
    df -h
    podman --version
    skopeo --version
    printf '%s\n' "$base" "$builder" "$registry_image"
} > "$evidence/environment.txt"
openssl rand -base64 32 > "$private/passphrase"
chmod 0600 "$private/passphrase"
skopeo generate-sigstore-key --output-prefix "$private/allowed" --passphrase-file "$private/passphrase"
openssl req -x509 -newkey rsa:3072 -nodes -days 1 -subj /CN=registry.kedra.test \
    -addext subjectAltName=DNS:registry.kedra.test -keyout "$private/tls.key" -out "$root/tls.crt" 2> "$evidence/tls-generation.log"
python3 build/research/r04/prepare.py images
cp "$root/fixture.json" "$evidence/"
sudo mkdir -p /etc/containers/registries.d /etc/containers/certs.d/registry.kedra.test:5000
sudo cp "$root/A/registries.yaml" /etc/containers/registries.d/kedra-r04.yaml
sudo cp "$root/tls.crt" /etc/containers/certs.d/registry.kedra.test:5000/ca.crt
printf '\n127.0.0.1 registry.kedra.test\n' | sudo tee -a /etc/hosts >/dev/null
sudo podman run -d --name kedra-r04-registry -p 127.0.0.1:5000:5000 \
    -v "$root/tls.crt:/certs/tls.crt:ro" -v "$private/tls.key:/certs/tls.key:ro" \
    -e REGISTRY_HTTP_TLS_CERTIFICATE=/certs/tls.crt -e REGISTRY_HTTP_TLS_KEY=/certs/tls.key \
    -e OTEL_TRACES_EXPORTER=none "$registry_image"
for attempt in $(seq 1 20); do
    if curl --silent --fail --cacert "$root/tls.crt" https://registry.kedra.test:5000/v2/; then break; fi
    sleep 1
done
curl --silent --fail --cacert "$root/tls.crt" https://registry.kedra.test:5000/v2/ > /dev/null
target/release/sysroot source archive --host desktop --output "$root/desktop/payload.tar"
cp target/release/sysroot target/release/sysroot-helper Containerfile build/assemble.sh "$root/desktop/"
bash build/agents/prepare.sh "$root/desktop" "$private/agent-inputs" "$evidence/agent-inputs.json"
python3 build/bitwarden/prepare.py --context "$root/desktop" --evidence "$evidence/bitwarden-inputs.json"
sudo podman build --pull=always --no-cache --build-arg "BASE_IMAGE=$base" \
    --tag localhost/kedra-r04-desktop:base "$root/desktop" > "$evidence/desktop-build.log" 2>&1
for variant in A B; do
    sudo podman build --pull=never --build-arg "VARIANT=$variant" --tag "localhost/kedra-r04:$variant" \
        "$root/$variant" > "$evidence/build-$variant.log" 2>&1
    sudo skopeo copy --sign-by-sigstore-private-key "$private/allowed.private" --sign-passphrase-file "$private/passphrase" \
        --digestfile "$root/$variant.digest" "containers-storage:localhost/kedra-r04:$variant" "docker://$repository:$variant" \
        > "$evidence/sign-$variant.log" 2>&1
    cp "$root/$variant/source.json" "$evidence/source-$variant.json"
    cp "$root/$variant.digest" "$evidence/"
done
python3 build/research/r04/prepare.py requests
cp "$root/cases"/helper-*.json "$root/A/release.pub" "$root/A/policy.json" "$evidence/"
jq --arg key "$root/A/release.pub" '.transports[][][].keyPath=$key' "$root/A/policy.json" > "$root/host-policy.json"
initial="$repository@$(cat "$root/A.digest")"
sudo skopeo --policy "$root/host-policy.json" copy "docker://$initial" "containers-storage:$initial" > "$evidence/verified-a-copy.log" 2>&1
python3 - <<'PY'
import json, os, pathlib, secrets, subprocess
private = pathlib.Path(os.environ['RUNNER_TEMP']) / 'kedra-r04-private'
password = secrets.token_hex(16)
(private / 'password').write_text(password)
(private / 'password').chmod(0o600)
hashed = subprocess.check_output(['openssl', 'passwd', '-6', '-stdin'], input=password.encode()).decode().strip()
(private / 'blueprint.toml').write_text('[[customizations.user]]\nname = "kedra-test"\npassword = ' + json.dumps(hashed) + '\ngroups = ["wheel"]\n')
(private / 'blueprint.toml').chmod(0o600)
PY
sudo podman run --rm --privileged --security-opt label=type:unconfined_t \
    -v "$root/image:/output" -v "$private/blueprint.toml:/config.toml:ro" \
    -v /var/lib/containers/storage:/var/lib/containers/storage -v /etc/containers/certs.d:/etc/containers/certs.d:ro \
    "$builder" --type qcow2 --rootfs ext4 --use-librepo=True "$initial" > "$private/builder.log" 2>&1
mapfile -t disks < <(find "$root/image" -type f -name '*.qcow2')
test "${#disks[@]}" -eq 1
truncate -s 16M "$root/cases.raw"
mkfs.ext4 -q -L KEDRA_R04_CASES -d "$root/cases" "$root/cases.raw"
sudo chown "$(id -u):$(id -g)" "$root/image" "$(dirname "${disks[0]}")" "${disks[0]}"
sudo chgrp "$(id -g)" /dev/kvm
sudo chmod g+rw /dev/kvm
for phase in stage-b accept-b rollback-a; do
    case "$phase" in
        stage-b) success=KEDRA_R04_STAGE_B_PASS ;;
        accept-b) success=KEDRA_R04_ACCEPT_B_ROLLBACK_STAGED_PASS ;;
        rollback-a) success=KEDRA_R04_ROLLBACK_A_HOME_PASS ;;
    esac
    xvfb-run -a -s '-screen 0 1280x768x24' env LIBGL_ALWAYS_SOFTWARE=1 \
        python3 build/research/r07/run_vm.py --disk "${disks[0]}" --persistent-disk --network \
        --cases-disk "$root/cases.raw" --firmware-vars "$root/OVMF_VARS.fd" \
        --work "$evidence/$phase" --password-file "$private/password" \
        --login-marker KEDRA_R04_LOGIN_READY --failure-marker KEDRA_R04_FAIL --success-marker "$success" \
        | tee "$evidence/$phase.txt"
done
printf 'PASS: signed A-to-B native home acceptance and retained-A rollback.\n' | tee "$evidence/result.txt"

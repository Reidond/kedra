#!/usr/bin/env bash
# Disposable-host signed graphical A/B/rollback experiment (common/disposable_host.py). No workstation disks.
set -euo pipefail
# The B baseline comes from compose-artifact.py on a native aarch64 host, at this commit.
if ! test -f output/home-artifact/producer.json; then
    echo 'Copy output/home-artifact from the aarch64 compose-artifact.py run at this commit first' >&2
    exit 1
fi
. usr/src/kedra/tests/common/disposable-host.sh --evidence output/r04-evidence --architecture x86_64 --kvm \
    --command podman --command skopeo --command openssl --command curl --command mkfs.ext4 \
    --command qemu-system-x86_64 --command virt-fw-vars --command xvfb-run --command xauth --command xdotool \
    --command import
test "$(jq -er .product_source output/home-artifact/producer.json)" = "$GITHUB_SHA"
root="$RUNNER_TEMP/kedra-r04"
private="$RUNNER_TEMP/kedra-r04-private"
mkdir "$root" "$private"
chmod 0700 "$private"
mkdir -p "$root/desktop" "$root/image" output/r04-evidence
evidence="$PWD/output/r04-evidence"
base=$(uv run usr/src/kedra/tests/common/resolve-fedora-base.py --output "$evidence/base-resolution.json")
builder=$(jq -er .platforms.amd64.builder usr/src/kedra/image/inputs.json)
repository=registry.kedra.test:5000/kedra/r04
registry_image=docker.io/library/registry@sha256:7518da9b12dd746278282a729dee2e65eabdeb449db4d0b28d46ef6e90308f58
cleanup() {
    sudo podman logs kedra-r04-registry > "$evidence/registry.log" 2>&1 || true
    sudo podman stop kedra-r04-registry >/dev/null 2>&1 || true
    uv run python - <<'PY'
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
uv run usr/src/kedra/tests/common/secure_boot.py provenance --evidence "$evidence"
openssl rand -base64 32 > "$private/passphrase"
chmod 0600 "$private/passphrase"
skopeo generate-sigstore-key --output-prefix "$private/allowed" --passphrase-file "$private/passphrase"
openssl req -x509 -newkey rsa:3072 -nodes -days 1 -subj /CN=registry.kedra.test \
    -addext subjectAltName=DNS:registry.kedra.test -keyout "$private/tls.key" -out "$root/tls.crt" 2> "$evidence/tls-generation.log"
# Authentic pre-artifact CLI; keep its source and checksum alongside the generated fixture.
mkdir "$root/old-cli"
git archive 46b4fe2c0d25e3129fc0297ff40d5a43baabed1b | tar -x -C "$root/old-cli"
cargo build --release --locked -p sysroot --manifest-path "$root/old-cli/Cargo.toml" \
    --target-dir "$root/old-cli/target" > "$evidence/old-cli-build.log" 2>&1
cp "$root/old-cli/target/release/sysroot" "$root/old-cli/sysroot"
sha256sum "$root/old-cli/sysroot" target/release/sysroot > "$evidence/cli-checksums.txt"
uv run usr/src/kedra/tests/vm/home-transition/prepare.py images
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
cp target/release/sysroot target/release/sysroot-helper usr/src/kedra/image/Containerfile usr/src/kedra/image/assemble.sh "$root/desktop/"
uv run usr/src/kedra/tests/common/prepare_inputs.py --target desktop --context "$root/desktop" --evidence "$evidence"
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
uv run usr/src/kedra/tests/vm/home-transition/prepare.py requests
cp "$root/cases"/helper-*.json "$root/A/release.pub" "$root/A/policy.json" "$evidence/"
jq --arg key "$root/A/release.pub" '.transports[][][].keyPath=$key' "$root/A/policy.json" > "$root/host-policy.json"
initial="$repository@$(cat "$root/A.digest")"
sudo skopeo --policy "$root/host-policy.json" copy "docker://$initial" "containers-storage:$initial" > "$evidence/verified-a-copy.log" 2>&1
uv run python - <<'PY'
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
# run_vm.py creates the Microsoft-enrolled UEFI Secure Boot variables on the
# first boot and refuses to reuse them unless that trust is still intact.
for phase in stage-b accept-b rollback-a; do
    login_options=()
    if test "$phase" != stage-b; then login_options+=(--remembered-login); fi
    case "$phase" in
        stage-b) success=KEDRA_R04_STAGE_B_PASS ;;
        accept-b) success=KEDRA_R04_ACCEPT_B_ROLLBACK_STAGED_PASS ;;
        rollback-a) success=KEDRA_R04_ROLLBACK_A_HOME_PASS ;;
    esac
    xvfb-run -a -s '-screen 0 1280x768x24' env LIBGL_ALWAYS_SOFTWARE=1 \
        uv run usr/src/kedra/tests/common/run_vm.py --disk "${disks[0]}" --persistent-disk --network \
        --cases-disk "$root/cases.raw" --firmware-vars "$root/OVMF_VARS.fd" \
        "${login_options[@]}" \
        --work "$evidence/$phase" --password-file "$private/password" \
        --login-marker KEDRA_R04_LOGIN_READY --failure-marker KEDRA_R04_FAIL --success-marker "$success" \
        | tee "$evidence/$phase.txt"
done
printf 'PASS: signed A-to-B native home acceptance and retained-A rollback under UEFI Secure Boot.\n' | tee "$evidence/result.txt"

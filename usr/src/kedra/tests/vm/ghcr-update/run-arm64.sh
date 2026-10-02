#!/usr/bin/env bash
# Disposable ARM workflow: real composed image, generated authority,
# normal offline installer, then public signed A/B/A with persistent home/data.
set -euo pipefail
hvf_controller=
if test "${1:-}" = --hvf-controller; then
    test "$#" -ge 3
    hvf_controller=$2
    [[ "$hvf_controller" =~ ^[a-f0-9]{64}$ ]]
    shift 2
fi
if test "$#" -eq 1 && test "$1" = --help; then
    uv run usr/src/kedra/tests/vm/ghcr-update/fixture.py --help
    exit 0
fi
fixture_context=$(uv run usr/src/kedra/tests/vm/ghcr-update/fixture.py "$@")
if test -n "$hvf_controller"; then
    test "$(jq -er .mode "$fixture_context")" = local
fi
fixture_backend=tcg
runner_temp=$(jq -er .runner_temp "$fixture_context")
evidence=$(jq -er .evidence "$fixture_context")
binaries=$(jq -er .binaries "$fixture_context")
test "$(realpath -e .)" = "$(jq -er .repository "$fixture_context")"
fixture_args=(--fixture-context "$fixture_context")
test "$(uname -m)" = aarch64
root="$runner_temp/kedra-ghcr"
private="$runner_temp/kedra-ghcr-private"
mkdir "$root" "$private"
chmod 0700 "$root" "$private"
mkdir "$root/context" "$root/cases" "$root/build-context"
mkdir -p "$evidence"
controller_pid=
registry_id=
private_real=$(realpath -e -- "$private")
test "$private_real" = "$runner_temp/kedra-ghcr-private"
test ! -L "$private"
test "$(stat -c '%u:%a' -- "$private_real")" = "$(id -u):700"
private_identity=$(stat -c '%d:%i:%u:%a' -- "$private_real")
private_mounts_absent() {
    # Host-side guard, deliberately unprivileged. Device IDs cannot distinguish
    # same-filesystem bind mounts; inspect every kernel mountpoint instead.
    uv run python - "$private_real" <<'PY'
import json
import os
import re
import sys

try:
    private = os.fsencode(sys.argv[1])
    if not private.startswith(b'/') or os.path.normpath(private) != private:
        raise ValueError('noncanonical private root')
    with open('/proc/self/mountinfo', 'rb') as stream:
        data = stream.read(4 * 1024**2 + 1)
    if not data or len(data) > 4 * 1024**2 or not data.endswith(b'\n'):
        raise ValueError('invalid mount table size or termination')
    matches = []
    for line in data.splitlines():
        before, separator, after = line.partition(b' - ')
        fields = before.split(b' ')
        if not separator or len(fields) < 6 or len(after.split(b' ')) < 3 or not fields[0].isdigit():
            raise ValueError('malformed mount table entry')
        encoded = fields[4]
        if re.search(rb'\\(?!040|011|012|134)', encoded):
            raise ValueError('unsupported mountpoint escape')
        mountpoint = re.sub(rb'\\(040|011|012|134)',
                            lambda match: bytes((int(match[1], 8),)), encoded)
        if not mountpoint.startswith(b'/') or os.path.normpath(mountpoint) != mountpoint:
            raise ValueError('noncanonical mountpoint')
        if mountpoint == private or mountpoint.startswith(private + b'/'):
            matches.append(int(fields[0]))
    print(json.dumps({'schema_version': 1, 'safe_to_remove': not matches,
                      'mount_ids_at_or_below_private_root': matches}, sort_keys=True))
    if matches:
        print('Refusing private cleanup: a kernel mountpoint is at or below its root', file=sys.stderr)
        raise SystemExit(1)
except (OSError, ValueError) as error:
    print(json.dumps({'schema_version': 1, 'safe_to_remove': False,
                      'reason': 'mount_table_unavailable_or_invalid'}, sort_keys=True))
    print('Refusing private cleanup: ' + str(error), file=sys.stderr)
    raise SystemExit(1) from error
PY
}
cleanup() {
    local original_exit=$? cleanup_failed=false private_removed=false registry_state=not_created resolver_state=not_created observer_state=not_created
    trap - EXIT
    # Cleanup must finish reporting every owned resource even if the installer
    # left root-owned files. Preserve the task failure if cleanup also fails.
    set +e
    if test -n "$controller_pid"; then
        kill "$controller_pid" 2>/dev/null || true
        wait "$controller_pid" 2>/dev/null || true
    fi
    if test -n "$registry_id"; then
        registry_state=cleanup_failed
        if test "$(sudo --non-interactive podman inspect --format '{{ index .Config.Labels "dev.kedra.lab.owner" }}' "$registry_id" 2>/dev/null)" = kedra-release-fixture; then
            sudo --non-interactive podman logs "$registry_id" > "$evidence/registry.log" 2>&1 || true
            if sudo --non-interactive podman rm -f "$registry_id" >/dev/null; then
                registry_state=removed
            else
                cleanup_failed=true
            fi
        else
            cleanup_failed=true
        fi
    fi
    if test -e "$root/candidate/resolver.json"; then
        resolver_state=cleanup_failed
        if jq --exit-status '.schema_version == 1 and .removed == true and .cleanup_failed == false' \
            "$root/candidate/resolver-cleanup.json" >/dev/null 2>&1; then
            resolver_state=removed
        else
            cleanup_failed=true
        fi
    fi
    if test -e "$root/candidate/retained-observer.json"; then
        observer_state=cleanup_failed
        if jq --exit-status '.schema_version == 1 and .removed == true and .cleanup_failed == false' \
            "$root/candidate/retained-observer-cleanup.json" >/dev/null 2>&1; then
            observer_state=removed
        else
            cleanup_failed=true
        fi
    fi
    if test ! -e "$private_real" && test ! -L "$private_real"; then
        private_removed=true
    elif test ! -L "$private" &&
         test "$(realpath -e -- "$private")" = "$private_real" &&
         test "$private_real" = "$runner_temp/kedra-ghcr-private" &&
         test "$(stat -c '%d:%i:%u:%a' -- "$private_real")" = "$private_identity"; then
        # Refuse retained mounts, including same-device bind mounts, then
        # recheck the captured root before the fixed privileged removal.
        if private_mounts_absent > "$evidence/private-mount-check.json" &&
           test ! -L "$private" &&
           test "$(realpath -e -- "$private")" = "$private_real" &&
           test "$private_real" = "$runner_temp/kedra-ghcr-private" &&
           test "$(stat -c '%d:%i:%u:%a' -- "$private_real")" = "$private_identity" &&
           sudo --non-interactive /usr/bin/rm -rf --one-file-system --preserve-root=all -- "$private_real" &&
           test ! -e "$private_real" && test ! -L "$private_real"; then
            private_removed=true
        else
            cleanup_failed=true
        fi
    else
        cleanup_failed=true
    fi
    if ! printf '{"schema_version":1,"original_exit_code":%s,"cleanup_failed":%s,"private_inputs_removed":%s,"registry":"%s","resolver":"%s","retained_observer":"%s"}\n' \
        "$original_exit" "$cleanup_failed" "$private_removed" "$registry_state" "$resolver_state" "$observer_state" > "$evidence/cleanup.json"; then
        cleanup_failed=true
        echo 'Fixture cleanup receipt could not be written' >&2
    fi
    if test "$cleanup_failed" = true; then
        echo 'Fixture cleanup failed; inspect cleanup.json and retained private inputs' >&2
        if test "$original_exit" -eq 0; then original_exit=1; fi
    fi
    exit "$original_exit"
}
trap cleanup EXIT
umask 077
printf '{"auths":{}}\n' > "$private/empty-auth.json"
export REGISTRY_AUTH_FILE="$private/empty-auth.json"
{
    date --utc --iso-8601=seconds
    uname -a
    free -h
    df -h
    docker version
    podman --version
    skopeo --version
    qemu-system-aarch64 --version
    sha256sum "$binaries/sysroot" "$binaries/sysroot-helper" "$binaries/kedra-lab"
} > "$evidence/environment.txt"
cp "$fixture_context" "$evidence/fixture-context.json"
if jq --exit-status '.retained_candidate != null' "$fixture_context" >/dev/null; then
    uv run usr/src/kedra/tests/vm/ghcr-update/resume-arm64.py "${fixture_args[@]}" --root "$root" \
        > "$evidence/candidate.json"
else
base=$(uv run usr/src/kedra/image/release/refresh.py resolve-base --target qemu-arm64 --work "$evidence")
"$binaries/sysroot" source plan --host qemu-arm64 --json > "$root/source-plan.json"
"$binaries/sysroot" source archive --host qemu-arm64 --output "$root/build-context/payload.tar"
cp "$binaries/sysroot" "$binaries/sysroot-helper" usr/src/kedra/image/Containerfile usr/src/kedra/image/assemble.sh "$root/build-context/"
# Existing explicit local-builder APIs use RUNNER_TEMP only as scratch; no CI
# identity variable is invented or changed by the local workflow.
KEDRA_LOCAL_BUILDER=1 RUNNER_TEMP="$runner_temp" bash usr/src/kedra/image/agents/prepare.sh qemu-arm64 "$root/build-context" "$private/agents" "$evidence/agent-inputs.json"
KEDRA_LOCAL_BUILDER=1 RUNNER_TEMP="$runner_temp" uv run usr/src/kedra/image/bitwarden/prepare.py --target qemu-arm64 --context "$root/build-context" --evidence "$evidence/bitwarden-inputs.json"
uv run usr/src/kedra/tests/vm/ghcr-update/candidate.py \
    "${fixture_args[@]}" \
    --context "$root/build-context" --source-plan "$root/source-plan.json" --base-image "$base" \
    --work "$root/candidate" --tag localhost/kedra-ghcr-arm:composed > "$evidence/candidate.json"
fi

# Pull all external GHCR tooling before assigning ghcr.io to the private registry.
media_builder=$(jq -er .platforms.arm64.builder usr/src/kedra/installer/inputs.json)
registry=docker.io/library/registry@sha256:3ffcae348822784850e836f23449ff1d0503933524cef57ddcbdbd263eca0c52
sudo podman pull "$media_builder" > "$evidence/media-builder-pull.log" 2>&1
sudo podman pull "$registry" > "$evidence/registry-pull.log" 2>&1
test "$(sudo podman image inspect "$registry" --format '{{.Architecture}}')" = arm64
openssl rand -base64 32 > "$private/passphrase"
for key in allowed wrong; do
    skopeo generate-sigstore-key --output-prefix "$private/$key" --passphrase-file "$private/passphrase" \
        > "$private/key-$key.log" 2>&1
done
openssl req -x509 -newkey rsa:3072 -nodes -days 1 -subj /CN=ghcr.io \
    -addext subjectAltName=DNS:ghcr.io,IP:127.0.0.1 -keyout "$private/tls.key" -out "$root/context/tls.crt" \
    > "$private/tls-generation.log" 2>&1
uv run usr/src/kedra/tests/vm/ghcr-update/prepare-arm64.py --root "$root" "${fixture_args[@]}"
cp usr/src/kedra/tests/vm/ghcr-update/{arm64.Containerfile,arm64-variant.Containerfile,check.py,arm64-check.service,identity-recovery.py} "$root/context/"
cp usr/src/kedra/tests/container/qemu/boot-check.py "$root/context/native-observer.py"
sudo mkdir -p /etc/containers/certs.d/ghcr.io /etc/containers/registries.d
test ! -e /etc/containers/certs.d/ghcr.io/ca.crt
test ! -e /etc/containers/registries.d/kedra-ghcr-test.yaml
sudo cp "$root/context/tls.crt" /etc/containers/certs.d/ghcr.io/ca.crt
sudo cp "$root/context/registries.yaml" /etc/containers/registries.d/kedra-ghcr-test.yaml
printf '\n127.0.0.1 ghcr.io\n' | sudo tee -a /etc/hosts >/dev/null
test "$(getent ahostsv4 ghcr.io | awk '{print $1}' | sort -u)" = 127.0.0.1
if sudo podman container exists kedra-ghcr-registry; then
    echo 'Refusing an existing registry container' >&2
    exit 1
fi
registry_id=$(sudo podman run -d --name kedra-ghcr-registry --label dev.kedra.lab.owner=kedra-release-fixture \
    -p 127.0.0.1:443:5000 -v "$root/context/tls.crt:/certs/tls.crt:ro" -v "$private/tls.key:/certs/tls.key:ro" \
    -e REGISTRY_HTTP_TLS_CERTIFICATE=/certs/tls.crt -e REGISTRY_HTTP_TLS_KEY=/certs/tls.key \
    -e REGISTRY_STORAGE_DELETE_ENABLED=true -e OTEL_TRACES_EXPORTER=none "$registry")
[[ "$registry_id" =~ ^[a-f0-9]{64}$ ]]
for attempt in $(seq 1 30); do
    if curl --silent --fail --cacert "$root/context/tls.crt" https://127.0.0.1/v2/ >/dev/null; then break; fi
    sleep 1
done
curl --silent --fail --cacert "$root/context/tls.crt" https://127.0.0.1/v2/ >/dev/null
sudo podman build --pull=never --network=none -f "$root/context/arm64.Containerfile" \
    -t localhost/kedra-ghcr-arm:base "$root/context" > "$evidence/fixture-base-build.log" 2>&1
for variant in A B C E U W R N T X H M; do
    identity=$(cat "$root/context/$variant/image-identity.json")
    sudo podman build --pull=never --network=none --build-arg "KEDRA_IDENTITY=$identity" --build-arg "VARIANT=$variant" \
        -f "$root/context/arm64-variant.Containerfile" -t "localhost/kedra-ghcr-arm:$variant" "$root/context" \
        > "$evidence/build-$variant.log" 2>&1
    signing=(--sign-by-sigstore-private-key "$private/allowed.private" --sign-passphrase-file "$private/passphrase")
    if test "$variant" = U; then signing=(); fi
    if test "$variant" = W; then signing=(--sign-by-sigstore-private-key "$private/wrong.private" --sign-passphrase-file "$private/passphrase"); fi
    repository=ghcr.io/reidond/kedra-qemu-arm64
    if test "$variant" = R; then repository=ghcr.io/reidond/kedra-other; fi
    sudo skopeo inspect --raw "containers-storage:localhost/kedra-ghcr-arm:$variant" \
        > "$evidence/source-$variant-manifest.json"
    source_digest=$(skopeo manifest-digest "$evidence/source-$variant-manifest.json")
    # Producer-only signing. Strict consumer verification is the unmodified
    # public installer/helper policy. Preserve the source representation just as
    # the normal release does: BIB's ID-only local copy cannot recompress a
    # signed image into an independently pinned destination digest.
    (
        # This command creates public digest metadata, not private key files.
        umask 022
        sudo skopeo copy --preserve-digests --remove-signatures --authfile "$private/empty-auth.json" "${signing[@]}" \
            --digestfile "$root/$variant.digest" "containers-storage:localhost/kedra-ghcr-arm:$variant" "docker://$repository:$variant"
    ) > "$evidence/sign-$variant.log" 2>&1
    test "$(cat "$root/$variant.digest")" = "$source_digest"
    if test "$variant" = R; then
        sudo skopeo copy --preserve-digests "docker://$repository:$variant" docker://ghcr.io/reidond/kedra-qemu-arm64:R \
            > "$evidence/wrong-repository-copy.log" 2>&1
    fi
done
missing=$(cat "$root/N.digest")
tag="sha256-${missing#sha256:}.sig"
sudo skopeo inspect --raw "docker://ghcr.io/reidond/kedra-qemu-arm64:$tag" > "$evidence/removed-signature-manifest.json"
attachment=$(skopeo manifest-digest "$evidence/removed-signature-manifest.json")
curl --silent --show-error --fail --cacert "$root/context/tls.crt" -X DELETE \
    "https://127.0.0.1/v2/reidond/kedra-qemu-arm64/manifests/$attachment" > /dev/null
uv run python - "$fixture_context" <<'PY'
import json
import sys
from pathlib import Path

fixture=json.loads(Path(sys.argv[1]).read_bytes())
root=Path(fixture['runner_temp'])/'kedra-ghcr'
authority=json.loads((root/'fixture-authority.json').read_text())
cases={'schema_version':1,'target':'qemu-arm64','require_fresh_installation':True,
       'native_receipt_sha256':authority['native_receipt_sha256'],
       'native_observer_sha256':authority['native_observer_sha256'],
       'digests':{v:(root/(v+'.digest')).read_text().strip()
                  for v in ['A','B','C','E','U','W','R','N','T','X','H','M']}}
(root/'cases/cases.json').write_text(json.dumps(cases,indent=2)+'\n')
(Path(fixture['evidence'])/'cases.json').write_text(json.dumps(cases,indent=2)+'\n')
PY
cp "$root/fixture-authority.json" "$evidence/"
truncate -s 16M "$root/cases.raw"
mkfs.ext4 -q -L KEDRA_GHCR_CASES -d "$root/cases" "$root/cases.raw"
uv run usr/src/kedra/tests/vm/ghcr-update/control.py "${fixture_args[@]}" > "$evidence/controller.log" 2>&1 &
controller_pid=$!
sleep 1
curl --silent --show-error --fail -X POST http://127.0.0.1:18080/A >/dev/null
checkout=$(uv run usr/src/kedra/tests/vm/ghcr-update/prepare-install-arm64.py --root "$root" "${fixture_args[@]}")
rm -f -- "$private/allowed.private" "$private/wrong.private" "$private/passphrase"
run_installer() (
    # The public builder reads public root-created digest files. Keep its normal
    # umask inside the already-private 0700 fixture tree; credential files were
    # created separately with explicit 0600 permissions.
    umask 022
    uv run "$checkout/usr/src/kedra/installer/build-local.py" "$@"
)
for variant in U W; do
    rejected=0
    run_installer \
        --image "ghcr.io/reidond/kedra-qemu-arm64@$(cat "$root/$variant.digest")" \
        --output-dir "$private/rejected-$variant" > "$private/rejected-$variant.log" 2>&1 || rejected=$?
    test "$rejected" -ne 0
    if test "$variant" = U; then
        grep -qF 'A signature was required, but no signature exists' "$private/rejected-$variant.log"
    else
        grep -qF 'cryptographic signature verification failed' "$private/rejected-$variant.log"
    fi
    printf '{"variant":"%s","outcome":"refused","gate":"public installer signature admission"}\n' "$variant" \
        > "$evidence/installer-refusal-$variant.json"
done
initial="ghcr.io/reidond/kedra-qemu-arm64@$(cat "$root/A.digest")"
# This normal public entrypoint verifies the fixture's fixed public authority,
# exact repository/signature, installed strict policy and offline payload.
installer_exit=0
run_installer --image "$initial" --output-dir "$private/media" \
    > "$private/installer-build.log" 2>&1 || installer_exit=$?
if test "$installer_exit" -ne 0; then
    # Export fixed classifications only. Raw installer output can contain
    # generated Kickstart credentials and must remain in private cleanup.
    if ! uv run python - "$fixture_context" "$installer_exit" <<'PY'
import json
import sys
from pathlib import Path

fixture=json.loads(Path(sys.argv[1]).read_bytes())
log=Path(fixture['runner_temp'])/'kedra-ghcr-private/installer-build.log'
with log.open('rb') as stream:
    stream.seek(0,2)
    size=stream.tell()
    stream.seek(max(0,size-65536))
    tail=stream.read(65536)
kind='public_installer_failed'
if b'Permission denied' in tail and b'payload.digest' in tail:
    kind='public_digest_file_permissions'
elif b'Source image rejected' in tail and b'containers-storage' in tail:
    kind='local_storage_signature_policy_refused'
elif b'Source image rejected' in tail:
    kind='signature_policy_refused'
elif b'Install the documented local build prerequisites' in tail:
    kind='missing_local_build_prerequisite'
elif b'Pinned builder interface differs' in tail:
    kind='pinned_builder_interface_differs'
elif b'changing layer representation' in tail and b'Destination specifies a digest' in tail:
    kind='installer_layer_representation_mismatch'
(Path(fixture['evidence'])/'installer-failure.json').write_text(json.dumps({
    'schema_version':1,'exit_code':int(sys.argv[2]),'classification':kind,
    'raw_log_bytes':size,'examined_tail_bytes':len(tail),'raw_private_output_exported':False,
    'source_revision':fixture['source_revision'],'fixture_revision':fixture['fixture_revision'],
},indent=2)+'\n')
PY
    then
        echo 'Installer failed; fixed failure classification could not be recorded' >&2
    fi
    exit "$installer_exit"
fi
mapfile -t media < <(find "$private/media" -maxdepth 1 -type f -name '*.iso')
test "${#media[@]}" -eq 1
uv run usr/src/kedra/tests/vm/ghcr-update/boot-arm64.py --root "$root" "${fixture_args[@]}" --phase refuse-insecure --iso "${media[0]}" \
    > "$evidence/secureboot-disabled.json"
install_exit=0
uv run usr/src/kedra/tests/vm/ghcr-update/boot-arm64.py --root "$root" "${fixture_args[@]}" --phase install --iso "${media[0]}" \
    > "$evidence/install-phase.json" || install_exit=$?
if test -n "$hvf_controller"; then
    handoff_start=A
    if test "$install_exit" -ne 0; then
        test -f "$root/install-timeout.json" || exit "$install_exit"
        handoff_start=install
    fi
    # This bounded wait keeps the original EXIT trap and exact resources alive.
    # Success, failure, interruption and timeout all reach that same cleanup.
    uv run usr/src/kedra/tests/vm/ghcr-update/macos-transfer.py "${fixture_args[@]}" \
        --operation handoff --controller "$hvf_controller" --registry "$registry_id" \
        --control-pid "$controller_pid" --start "$handoff_start" > "$evidence/hvf-handoff.jsonl"
    fixture_backend=hvf
else
    if test "$install_exit" -ne 0; then exit "$install_exit"; fi
    for phase in A B ROLLBACK; do
        uv run usr/src/kedra/tests/vm/ghcr-update/boot-arm64.py --root "$root" "${fixture_args[@]}" --phase "$phase" \
            > "$evidence/$phase-result.json"
        cp "$root/$phase.serial.log" "$evidence/"
    done
fi
cp "$root/installation-plan.json" "$root/disks.json" "$root/firmware-provenance.json" "$evidence/"
uv run python - "$fixture_context" "$fixture_backend" <<'PY'
import json
import sys
from pathlib import Path

fixture=json.loads(Path(sys.argv[1]).read_bytes())
(Path(fixture['evidence'])/'scope.json').write_text(json.dumps({
    'schema_version':1,'target':'qemu-arm64','fresh_anaconda_installation':'pass',
    'execution_mode':fixture['mode'],'source_revision':fixture['source_revision'],
    'execution_backend':sys.argv[2],
    'fixture_revision':fixture['fixture_revision'],
    'iso_free_luks_boots':['A','B','A'],'public_updater':'pass','home_and_var_preservation':'pass',
    'authority':'generated fixture only','production_publication':False,'production_keys_used':False,
    'secure_boot_disabled_installer':'refused',
    'native_scope':'shared verified generated output across distinct signed fixture variants',
    'production_protected_main_gate':'not-run'},indent=2)+'\n')
PY

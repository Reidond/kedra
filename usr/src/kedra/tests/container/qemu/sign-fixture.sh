#!/bin/bash
# Runs only inside the disposable disk builder. Secret keys live in its tmpfs,
# outside the target image, public build context and retained Podman storage.
set -Eeuo pipefail
same_bytes() {
    local left right
    left=$(sha256sum -- "$1")
    right=$(sha256sum -- "$2")
    test "${left%% *}" = "${right%% *}"
}
image=${1:?unique named fixture}
loaded_id=${2:?verified imported image ID}
metadata_sha=${3:?verified image metadata hash}
[[ "$image" =~ ^localhost/kedra-qemu-fixture/[a-f0-9]{32}:boot$ ]]
[[ "$loaded_id" =~ ^sha256:[a-f0-9]{64}$ ]]
[[ "$metadata_sha" =~ ^[a-f0-9]{64}$ ]]
id=${loaded_id#sha256:}
test "$(jq -er '.[0].Config.Labels["dev.kedra.lab.fixture-reference"]' /output/imported-image.json)" = "$image"
token=${image#localhost/kedra-qemu-fixture/}
token=${token%:boot}
scratch=/run/kedra-signing
test "$(stat -f -c %T "$scratch")" = tmpfs
test "$(stat -c %a "$scratch")" = 700
mkdir "$scratch/keys" "$scratch/public"
chmod 0700 "$scratch/keys" "$scratch/public"
keys=$scratch/keys
context=$scratch/public
cleanup_keys() {
    rm -f -- "$keys/passphrase" "$keys/allowed.private" "$keys/allowed.pub" \
        "$keys/wrong.private" "$keys/wrong.pub"
    rmdir "$keys" 2>/dev/null || true
}
trap cleanup_keys EXIT
umask 077
head -c 32 /dev/urandom | base64 --wrap=0 > "$keys/passphrase"
for key in allowed wrong; do
    skopeo generate-sigstore-key --output-prefix "$keys/$key" --passphrase-file "$keys/passphrase" \
        > "/output/key-$key.log" 2>&1
done
cp "$keys/allowed.pub" "$context/fixture.pub"
cp "$keys/allowed.pub" /output/fixture-signing.pub
fingerprint=$(sed '/^-----/d' "$keys/allowed.pub" | base64 -d | sha256sum | cut -d ' ' -f 1)
public_sha=$(sha256sum "$keys/allowed.pub" | cut -d ' ' -f 1)
observer=$(< /tmp/kedra-native-boot-check.py)
podman run --rm --pull=never --network none --read-only --cap-drop=ALL --security-opt no-new-privileges \
    --tmpfs /tmp:rw,nosuid,nodev,noexec,size=64m --entrypoint /usr/bin/python3 \
    "$image" -I -c "$observer" trust > /output/target-trust-before.json
podman run --rm --pull=never --network none --read-only --cap-drop=ALL --security-opt no-new-privileges \
    --entrypoint /usr/bin/cat "$image" /etc/containers/policy.json > "$context/production-policy.json"
cp "$context/production-policy.json" /output/production-policy.json
ordinary_scope="[overlay@/var/lib/containers/storage]@$id"
bib_scope="[overlay@/run/osbuild/containers/storage2]@$id"
key_path=/usr/share/kedra-lab/fixture-signing.pub
jq -e --arg ordinary "$ordinary_scope" --arg bib "$bib_scope" \
    '.default == [{"type":"reject"}] and
     (.transports["containers-storage"][""] | length > 0) and
     (.transports["containers-storage"] | has($ordinary) | not) and
     (.transports["containers-storage"] | has($bib) | not)' "$context/production-policy.json" >/dev/null
jq --arg ordinary "$ordinary_scope" --arg bib "$bib_scope" --arg key "$key_path" --arg ref "$image" \
    '{type:"sigstoreSigned",keyPath:$key,signedIdentity:{type:"exactReference",dockerReference:$ref}} as $requirement |
     .transports["containers-storage"][$ordinary]=[$requirement] |
     .transports["containers-storage"][$bib]=[$requirement]' \
    "$context/production-policy.json" > "$context/policy.json"
# Prove the only policy changes are the two independently chosen exact-ID rules.
jq -S --arg ordinary "$ordinary_scope" --arg bib "$bib_scope" \
    'del(.transports["containers-storage"][$ordinary],.transports["containers-storage"][$bib])' \
    "$context/policy.json" > "$scratch/policy-without-fixture.json"
jq -S . "$context/production-policy.json" > "$scratch/production-canonical.json"
same_bytes "$scratch/policy-without-fixture.json" "$scratch/production-canonical.json"
cp "$context/policy.json" /output/fixture-policy.json
cat > "$context/Containerfile" <<'CONTAINERFILE'
ARG TARGET
FROM ${TARGET}
COPY fixture.pub /usr/share/kedra-lab/fixture-signing.pub
COPY policy.json /etc/containers/policy.json
LABEL dev.kedra.lab.owner=kedra-container-tests dev.kedra.lab.kind=qemu-signing-buildroot
CONTAINERFILE
buildroot=localhost/kedra-qemu-buildroot:$token
test "$(podman image inspect "$image" --format '{{.Id}}')" = "$id"
podman build --pull=never --network=none --build-arg "TARGET=$image" \
    --iidfile "$scratch/buildroot.id" -t "$buildroot" "$context" > /output/buildroot-build.log 2>&1
buildroot_id=$(< "$scratch/buildroot.id")
[[ "$buildroot_id" =~ ^sha256:[a-f0-9]{64}$ ]]
buildroot_digest=$(podman image inspect "$buildroot_id" --format '{{.Digest}}')
[[ "$buildroot_digest" =~ ^sha256:[a-f0-9]{64}$ ]]
buildroot_reference=localhost/kedra-qemu-buildroot@$buildroot_digest
test "$(podman image inspect "$buildroot_reference" --format '{{.Id}}')" = "${buildroot_id#sha256:}"
printf '%s\n' "$buildroot_reference" > /output/signing-buildroot
podman image inspect "$buildroot_id" > /output/buildroot-image.json
cp "$context/Containerfile" /output/buildroot.Containerfile
podman run --rm --pull=never --network none --read-only --cap-drop=ALL --security-opt no-new-privileges \
    --entrypoint /usr/bin/cat "$buildroot_id" /etc/containers/policy.json > "$scratch/buildroot-policy.json"
same_bytes "$context/policy.json" "$scratch/buildroot-policy.json"
podman --version > /output/signing-versions.txt
skopeo --version >> /output/signing-versions.txt
podman run --rm --pull=never --network none --entrypoint /usr/bin/skopeo "$buildroot_id" --version \
    >> /output/signing-versions.txt
ordinary_source="containers-storage:[overlay@/var/lib/containers/storage+/run/containers/storage]$id"
bib_source="containers-storage:[overlay@/run/osbuild/containers/storage2+/run/containers/storage:additionalimagestore=/run/osbuild/containers/storage]$id"
initial_digest=$(podman image inspect "$image" --format '{{.Digest}}')
[[ "$initial_digest" =~ ^sha256:[a-f0-9]{64}$ ]]
mkdir /output/signature-check
verify() {
    local phase=$1 scope=$2 source=$3 code=0
    # This is the independent consumer. Its image supplies the DEFAULT policy;
    # no --policy, insecure flag or producer signing operation is used here.
    podman run --rm --pull=never --privileged --network none --read-only \
        --security-opt label=type:unconfined_t --tmpfs /run:rw,nosuid,nodev,mode=0755 \
        --tmpfs /tmp:rw,nosuid,nodev,noexec,size=64m \
        -v /var/lib/containers/storage:/var/lib/containers/storage \
        -v /var/lib/containers/storage:/run/osbuild/containers/storage \
        -v /output/signature-check:/verification --entrypoint /usr/bin/skopeo \
        "$buildroot_id" copy --preserve-digests "$source" "dir:/verification/$scope" \
        > "/output/verify-$phase-$scope.stdout" 2> "/output/verify-$phase-$scope.stderr" || code=$?
    printf '%s\n' "$code" > "/output/verify-$phase-$scope.exit"
    if [[ "$phase" == allowed ]]; then
        test "$code" -eq 0
        test "sha256:$(sha256sum "/output/signature-check/$scope/manifest.json" | cut -d ' ' -f 1)" = "$(< /output/allowed.digest)"
        mv "/output/signature-check/$scope/manifest.json" "/output/verify-allowed-$scope.manifest.json"
        rm -rf -- "/output/signature-check/$scope"
    elif [[ "$phase" == unsigned ]]; then
        test "$code" -ne 0
        grep -qF 'A signature was required, but no signature exists' "/output/verify-$phase-$scope.stderr"
    else
        test "$code" -ne 0
        grep -qF 'cryptographic signature verification failed' "/output/verify-$phase-$scope.stderr"
    fi
}
material() {
    local phase=$1 actual
    podman image inspect "$image" > "/output/$phase-image.json"
    test "$(jq -r '.[0].Id' "/output/$phase-image.json")" = "$id"
    local digest
    digest=$(jq -r '.[0].Digest' "/output/$phase-image.json")
    [[ "$digest" =~ ^sha256:[a-f0-9]{64}$ ]]
    if [[ "$phase" == unsigned ]]; then
        test "$digest" = "$initial_digest"
    else
        test "$digest" = "$(< "/output/$phase.digest")"
    fi
    actual=$(jq -cjS '.[0] | {Architecture, Os, RootFS: .RootFS.Layers, Config: (.Config | {Labels, Env, Cmd, Entrypoint, User, WorkingDir})}' \
        "/output/$phase-image.json" | sha256sum | cut -d ' ' -f 1)
    test "$actual" = "$metadata_sha"
    podman run --rm --pull=never --network none --read-only --cap-drop=ALL --security-opt no-new-privileges \
        --tmpfs /tmp:rw,nosuid,nodev,noexec,size=64m --entrypoint /usr/bin/python3 \
        "$image" -I -c "$observer" image > "/output/$phase-native.json"
    same_bytes /output/imported-native.json "/output/$phase-native.json"
}
for phase in unsigned wrong allowed; do
    if [[ "$phase" != unsigned ]]; then
        # Producer-only signature creation/replacement. Each final fixture has
        # a unique lab OCI label, so these signatures cannot belong to old aliases.
        podman push --retry=0 --remove-signatures \
            --sign-by-sigstore-private-key "$keys/$phase.private" \
            --sign-passphrase-file "$keys/passphrase" --digestfile "/output/$phase.digest" \
            "$loaded_id" "containers-storage:$image" > "/output/sign-$phase.log" 2>&1
    fi
    material "$phase"
    verify "$phase" ordinary "$ordinary_source"
    verify "$phase" bib "$bib_source"
done
podman run --rm --pull=never --network none --read-only --cap-drop=ALL --security-opt no-new-privileges \
    --entrypoint /usr/bin/python3 "$image" -I -c "$observer" trust > /output/target-trust-after.json
same_bytes /output/target-trust-before.json /output/target-trust-after.json
signed_digest=$(< /output/allowed.digest)
boot_reference=${image%:boot}@$signed_digest
test "$(podman image inspect "$boot_reference" --format '{{.Id}}')" = "$id"
printf '%s\n' "$boot_reference" > /output/boot-reference
policy_sha=$(sha256sum "$context/policy.json" | cut -d ' ' -f 1)
recipe_sha=$(sha256sum "$context/Containerfile" | cut -d ' ' -f 1)
trust_sha=$(sha256sum /output/target-trust-before.json | cut -d ' ' -f 1)
cleanup_keys
test ! -e "$keys"
jq -n --arg ref "$image" --arg boot "$boot_reference" --arg digest "$signed_digest" --arg imported "$initial_digest" \
    --arg id "$id" --arg root "$buildroot_id" --arg rootref "$buildroot_reference" --arg rootdigest "$buildroot_digest" \
    --arg recipe "$recipe_sha" --arg policy "$policy_sha" \
    --arg fingerprint "$fingerprint" --arg public "$public_sha" --arg trust "$trust_sha" \
    --arg ordinary "$ordinary_scope" --arg bib "$bib_scope" \
    '{schema_version:1, signed_reference:$ref, boot_reference:$boot, signed_manifest_digest:$digest, imported_manifest_digest:$imported,
      target_storage_id:$id, buildroot_id:$root, buildroot_reference:$rootref, buildroot_manifest_digest:$rootdigest,
      buildroot_recipe_sha256:$recipe, policy_sha256:$policy,
      key_fingerprint_sha256_spki:$fingerprint, public_key_sha256:$public, target_trust_sha256:$trust,
      scopes:[$ordinary,$bib], admission:{unsigned:"refused",wrong_key_only:"refused",allowed_key_only:"pass"},
      consumer_policy:"default", target_production_trust:"unchanged", private_keys_removed:true}' > /output/signing.json
# Signatures remain in the Podman store consumed by BIB, never save/load transport.
rmdir /output/signature-check

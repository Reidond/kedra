#!/usr/bin/bash
set -euo pipefail
trap 'status=$?; echo "KEDRA_R01_FAIL line=$LINENO status=$status"; systemctl poweroff --no-block; exit "$status"' ERR
state=/var/lib/kedra-research
mkdir -p "$state"
if ! grep -q '^10.0.2.2 registry.kedra.test$' /etc/hosts; then
    printf '\n10.0.2.2 registry.kedra.test\n' >> /etc/hosts
fi
variant=$(cat /usr/share/kedra-research/variant)
phase=$(cat "$state/phase" 2>/dev/null || printf initial)
echo "KEDRA_R01_BEGIN variant=$variant phase=$phase"
bootc --version
skopeo --version
bootc status --json
test "$(getenforce)" = Enforcing
policy_before=$(sha256sum /etc/containers/policy.json)

status_identity() {
    bootc status --json | jq -c '{spec,booted:.status.booted.image,staged:.status.staged,rollbackQueued:.status.rollbackQueued}'
}
reject() {
    local name=$1 reference=$2 before code
    before=$(status_identity)
    if bootc switch --enforce-container-sigpolicy "$reference" > "$state/rejection.log" 2>&1; then
        echo "Unexpected signature acceptance: $name"
        return 1
    else
        code=$?
    fi
    cat "$state/rejection.log"
    test "$before" = "$(status_identity)"
    test "$policy_before" = "$(sha256sum /etc/containers/policy.json)"
    echo "KEDRA_R01_REJECT_PASS case=$name exit=$code"
}

case "$variant:$phase" in
    A:initial)
        # Explicit fixture allowlist supplied by the runner over a read-only disk.
        mkdir -p /run/kedra-cases
        mount -o ro /dev/disk/by-label/KEDRA_CASES /run/kedra-cases
        cp /run/kedra-cases/cases.json "$state/cases.json"
        umount /run/kedra-cases
        test "$(jq -r .schema_version "$state/cases.json")" = 1
        # A records its actual installed digest; candidate names never establish trust.
        bootc status --json | jq -er .status.booted.image.imageDigest > "$state/a.digest"
        initial=$(jq -er .initial_a "$state/cases.json")
        test "$(cat "$state/a.digest")" = "${initial##*@}"
        printf 'synthetic local edits before update\n' > "$state/personal-data"
        for name in unsigned wrong_key wrong_repository missing_attachment; do
            reject "$name" "$(jq -er --arg name "$name" '.[$name]' "$state/cases.json")"
        done
        reject malformed 'registry.kedra.test:5000/kedra/r01@sha256:not-a-digest'
        reference=$(jq -er .valid_b "$state/cases.json")
        bootc switch --enforce-container-sigpolicy "$reference"
        bootc status --json | tee "$state/staged.json"
        test "$(jq -er .status.staged.image.image.image "$state/staged.json")" = "$reference"
        test "$(jq -er .status.booted.image.imageDigest "$state/staged.json")" = "$(cat "$state/a.digest")"
        test "$(jq -er .spec.image.signature "$state/staged.json")" = containerPolicy
        printf boot-b > "$state/phase"
        echo KEDRA_R01_STAGE_PASS
        ;;
    B:boot-b)
        reference=$(jq -er .valid_b "$state/cases.json")
        test "$(bootc status --json | jq -er .status.booted.image.imageDigest)" = "${reference##*@}"
        test "$(cat "$state/personal-data")" = 'synthetic local edits before update'
        test "$(bootc status --json | jq -er .spec.image.signature)" = containerPolicy
        before=$(status_identity)
        if bootc switch "$(jq -er .unsigned "$state/cases.json")"; then
            echo 'Omitted verification flag bypassed inherited policy'
            false
        fi
        test "$before" = "$(status_identity)"
        echo KEDRA_R01_INHERITED_POLICY_PASS
        printf 'newer personal data after update\n' > "$state/personal-data"
        bootc rollback
        bootc status --json
        printf rollback-a > "$state/phase"
        echo KEDRA_R01_BOOT_B_ROLLBACK_STAGED_PASS
        ;;
    A:rollback-a)
        test "$(bootc status --json | jq -er .status.booted.image.imageDigest)" = "$(cat "$state/a.digest")"
        test "$(cat "$state/personal-data")" = 'newer personal data after update'
        echo KEDRA_R01_ROLLBACK_PRESERVES_DATA_PASS
        printf complete > "$state/phase"
        ;;
    *) echo 'Unexpected image or journal phase'; false ;;
esac
systemctl poweroff --no-block

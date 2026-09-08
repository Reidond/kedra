#!/usr/bin/bash
set -Eeuo pipefail
trap 'status=$?; echo "KEDRA_R01_FAIL line=$LINENO status=$status"; systemctl poweroff --no-block; exit "$status"' ERR
state=/var/lib/kedra-research
mkdir -p "$state"
mkdir -p /var/lib/sysroot
chmod 0700 /var/lib/sysroot
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
test "$(bootc status --json | jq -er .spec.image.signature)" = containerPolicy
policy_before=$(sha256sum /etc/containers/policy.json)

status_identity() {
    bootc status --json | jq -c '{spec,booted:.status.booted.image,staged:.status.staged,rollbackQueued:.status.rollbackQueued}'
}
reject() {
    local name=$1 reference=$2 before code
    local -a options=(--enforce-container-sigpolicy)
    if test "${3:-explicit}" = inherited; then options=(); fi
    before=$(status_identity)
    if bootc switch "${options[@]}" "$reference" > "$state/rejection.log" 2>&1; then
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

helper() {
    /usr/libexec/sysroot/helper < "$state/helper-$1.json" | tee "$state/helper-response.json"
}
helper_reject() {
    local name=$1 before water
    before=$(status_identity)
    water=$(helper status | jq -c .journal.high_water)
    if /usr/libexec/sysroot/helper < "$state/helper-$name.json" > "$state/helper-rejection.log" 2>&1; then
        echo "Unexpected helper acceptance: $name"
        return 1
    fi
    cat "$state/helper-rejection.log"
    test "$before" = "$(status_identity)"
    if test "${2:-unchanged}" = unchanged; then
        test "$water" = "$(helper status | jq -c .journal.high_water)"
    fi
    echo "KEDRA_R10_HELPER_REJECT_PASS case=$name"
}

case "$variant:$phase" in
    A:initial)
        # Explicit fixture allowlist supplied by the runner over a read-only disk.
        mkdir -p /run/kedra-cases
        mount -o ro /dev/disk/by-label/KEDRA_CASES /run/kedra-cases
        cp /run/kedra-cases/cases.json "$state/cases.json"
        cp /run/kedra-cases/helper-*.json "$state/"
        umount /run/kedra-cases
        test "$(jq -r .schema_version "$state/cases.json")" = 1
        # A records its actual installed digest; candidate names never establish trust.
        bootc status --json | jq -er .status.booted.image.imageDigest > "$state/a.digest"
        initial=$(jq -er .initial_a "$state/cases.json")
        test "$(cat "$state/a.digest")" = "${initial##*@}"
        helper enroll
        test "$(helper status | jq -r .journal.high_water.highest_release_sequence)" = 1
        if runuser -u nobody -- /usr/libexec/sysroot/helper < "$state/helper-status.json"; then
            echo 'Unprivileged helper invocation was accepted'; false
        fi
        helper_reject extra-field
        helper_reject enroll
        cp /etc/containers/policy.json "$state/policy-original.json"
        printf ' ' >> /etc/containers/policy.json
        if /usr/libexec/sysroot/helper < "$state/helper-status.json"; then
            echo 'Changed container policy was accepted'; false
        fi
        cp "$state/policy-original.json" /etc/containers/policy.json
        test "$policy_before" = "$(sha256sum /etc/containers/policy.json)"
        echo KEDRA_R10_POLICY_DRIFT_PASS
        for name in wrong-key wrong-target candidate expired; do helper_reject "$name"; done
        reject initial_inherited "$(jq -er .unsigned "$state/cases.json")" inherited
        printf 'synthetic local edits before update\n' > "$state/personal-data"
        for name in unsigned wrong_key wrong_repository missing_attachment; do
            reject "$name" "$(jq -er --arg name "$name" '.[$name]' "$state/cases.json")"
        done
        reject malformed 'registry.kedra.test:5000/kedra/r01@sha256:not-a-digest'
        for variant in U W M; do helper_reject "oci-$variant" metadata-accepted; done
        reference=$(jq -er .valid_b "$state/cases.json")
        helper stage-b
        test "$(jq -r .journal.operation.phase "$state/helper-response.json")" = awaiting_reboot
        helper_reject pending
        helper_reject replay-a
        helper stage-b
        test "$(jq -r .journal.high_water.highest_release_sequence "$state/helper-response.json")" = 5
        bootc status --json | tee "$state/staged.json"
        test "$(jq -er .status.staged.image.image.image "$state/staged.json")" = "$reference"
        test "$(jq -er .status.booted.image.imageDigest "$state/staged.json")" = "$(cat "$state/a.digest")"
        test "$(jq -er .spec.image.signature "$state/staged.json")" = containerPolicy
        echo KEDRA_R10_HELPER_STAGE_PASS
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
        helper status
        test "$(jq -r .journal.operation.phase "$state/helper-response.json")" = booted
        printf 'newer personal data after update\n' > "$state/personal-data"
        helper rollback-a
        test "$(jq -r .journal.rollback_hold "$state/helper-response.json")" = true
        test "$(jq -r .journal.high_water.highest_release_sequence "$state/helper-response.json")" = 5
        echo KEDRA_R10_HELPER_ROLLBACK_PASS
        bootc status --json
        printf rollback-a > "$state/phase"
        echo KEDRA_R01_BOOT_B_ROLLBACK_STAGED_PASS
        ;;
    A:rollback-a)
        test "$(bootc status --json | jq -er .status.booted.image.imageDigest)" = "$(cat "$state/a.digest")"
        test "$(cat "$state/personal-data")" = 'newer personal data after update'
        reject rollback_inherited "$(jq -er .unsigned "$state/cases.json")" inherited
        helper status
        test "$(jq -r .journal.operation.phase "$state/helper-response.json")" = booted
        test "$(jq -r .journal.rollback_hold "$state/helper-response.json")" = true
        test "$(jq -r .journal.high_water.highest_release_sequence "$state/helper-response.json")" = 5
        helper_reject stage-b
        helper_reject replay-a
        echo KEDRA_R10_HELPER_HOLD_AND_REPLAY_PASS
        helper resume-b
        test "$(jq -r .journal.rollback_hold "$state/helper-response.json")" = false
        test "$(jq -r .journal.operation.phase "$state/helper-response.json")" = awaiting_reboot
        test "$(bootc status --json | jq -er .status.booted.image.imageDigest)" = "$(cat "$state/a.digest")"
        test "$(cat "$state/personal-data")" = 'newer personal data after update'
        echo KEDRA_R10_HELPER_EXPLICIT_RESUME_PASS
        echo KEDRA_R01_ROLLBACK_PRESERVES_DATA_PASS
        printf complete > "$state/phase"
        ;;
    *) echo 'Unexpected image or journal phase'; false ;;
esac
systemctl poweroff --no-block

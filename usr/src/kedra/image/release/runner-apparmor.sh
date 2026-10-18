#!/usr/bin/env bash
# Keep the Ubuntu runner's AppArmor policy off Kedra's programs: its unix-chkpwd
# profile confines the candidate's PAM helper inside the container, and Ubuntu
# 24.04 confines unprivileged user namespaces, which the engine's bubblewrap
# sandbox and the candidate's own sandboxes need. Disposable hosted runners only.
#
# Usage: runner-apparmor.sh EVIDENCE_DIRECTORY
set -euo pipefail
evidence=$1
mkdir -p "$evidence"
# The evidence files belong to the invoking user; only aa-status needs root.
# shellcheck disable=SC2024
sudo aa-status > "$evidence/apparmor-before.txt" 2>&1 || true
if test -f /etc/apparmor.d/unix-chkpwd; then
    sudo apparmor_parser --remove /etc/apparmor.d/unix-chkpwd
fi
if test -e /proc/sys/kernel/apparmor_restrict_unprivileged_userns; then
    sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0
fi
# shellcheck disable=SC2024
sudo aa-status > "$evidence/apparmor-after.txt" 2>&1 || true

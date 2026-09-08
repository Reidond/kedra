#!/usr/bin/bash
# Only activated by the research kernel flag, before any account/disk input.
set -euo pipefail
trap 'printf "KEDRA_INSTALLER_PROBE_FAIL\n" > /dev/ttyS1; journalctl -b -u anaconda -u anaconda-pre --no-pager -n 40 > /dev/ttyS1' ERR
test "$(getenforce)" = Permissive
for attempt in $(seq 1 150); do
    if systemctl is-active --quiet anaconda.service && test -s /tmp/anaconda.log; then
        systemctl is-active --quiet kedra-installer-verify.service
        printf 'KEDRA_INSTALLER_SIGNED_PAYLOAD_PASS\n' > /dev/ttyS1
        printf 'KEDRA_INSTALLER_ANACONDA_STARTED\n' > /dev/ttyS1
        exit 0
    fi
    if systemctl is-failed --quiet kedra-installer-verify.service; then
        journalctl -b -u kedra-installer-verify --no-pager -n 25 > /dev/ttyS1
        false
    fi
    sleep 1
done
false # Trigger the diagnostic trap on timeout.

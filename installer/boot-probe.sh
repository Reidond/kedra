#!/usr/bin/bash
# Only activated by the research kernel flag, before any account/disk input.
set -euo pipefail
trap 'printf "KEDRA_INSTALLER_PROBE_FAIL\n" > /dev/ttyS1; journalctl -b -u anaconda -u anaconda-pre --no-pager -n 40 > /dev/ttyS1' ERR
test "$(getenforce)" = Permissive
for attempt in $(seq 1 150); do
    if systemctl is-active --quiet anaconda.service && test -s /tmp/anaconda.log; then
        printf 'KEDRA_INSTALLER_ANACONDA_STARTED\n' > /dev/ttyS1
        exit 0
    fi
    sleep 1
done
exit 1

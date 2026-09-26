#!/usr/bin/bash
# Only activated by the explicit kedra.research credential, before any account/disk input.
# Markers use a named virtio-serial port so the probe is architecture-neutral.
set -euo pipefail
events=/dev/virtio-ports/org.kedra.events
for attempt in $(seq 1 60); do
    test -c "$events" && break
    sleep 1
done
test -c "$events"
exec 3>"$events"
trap 'printf "KEDRA_INSTALLER_PROBE_FAIL\n" >&3; journalctl -b -u anaconda -u anaconda-pre -u kedra-installer-verify --no-pager -n 60 >&3' ERR
test "$(getenforce)" = Permissive
# Firmware variables hold 4 attribute bytes followed by the value byte.
efi_value() {
    od -An -tu1 -j4 -N1 "/sys/firmware/efi/efivars/$1-8be4df61-93ca-11d2-aa0d-00e098032b8c" | tr -d ' '
}
test "$(efi_value SecureBoot)" = 1
test "$(efi_value SetupMode)" = 0
grep -Eq '\[(integrity|confidentiality)\]' /sys/kernel/security/lockdown
printf 'KEDRA_INSTALLER_SECUREBOOT_ENABLED\n' >&3
for attempt in $(seq 1 420); do
    if systemctl is-active --quiet anaconda.service && test -s /tmp/anaconda.log; then
        systemctl is-active --quiet kedra-installer-verify.service
        printf 'KEDRA_INSTALLER_SIGNED_PAYLOAD_PASS\n' >&3
        printf 'KEDRA_INSTALLER_ANACONDA_STARTED\n' >&3
        exit 0
    fi
    if systemctl is-failed --quiet kedra-installer-verify.service; then
        false
    fi
    sleep 1
done
false # Trigger the diagnostic trap on timeout.

#!/usr/bin/bash
# Disposable utm Secure Boot boot observer; runs only inside the generated TCG VM.
# Markers go to a dedicated virtio-serial port (host events.log) and, best effort,
# the PL011 serial console; the utm kargs keep /dev/console on tty0.
set -Eeuo pipefail
events=/dev/virtio-ports/org.kedra.events
marker() {
    printf '%s\n' "$1"
    printf '%s\n' "$1" > /dev/ttyAMA0 || true
    printf '%s\n' "$1" > "$events"
}
evidence() {
    printf '=== %s\n' "$*" > "$events"
    "$@" > "$events" 2>&1
    # Keep a following marker on its own line after output without a newline.
    printf '\n' > "$events"
}
trap 'code=$?; marker "KEDRA_UTM_FAIL line=$LINENO code=$code" || true; systemctl poweroff --no-block; exit "$code"' ERR
for attempt in $(seq 1 120); do
    if test -c "$events"; then break; fi
    sleep 1
done
test -c "$events"
marker KEDRA_UTM_BEGIN
evidence uname -a
evidence cat /proc/cmdline
cmdline=" $(cat /proc/cmdline) "
test "$(uname -m)" = aarch64
# hosts/utm kargs.d: PL011 serial for recovery/evidence plus the graphical tty.
case "$cmdline" in *' console=ttyAMA0,115200 '*) ;; *) false ;; esac
case "$cmdline" in *' console=tty0 '*) ;; *) false ;; esac
evidence cat /sys/class/tty/console/active

# Firmware -> shim -> GRUB -> kernel under enforced UEFI Secure Boot.
evidence mokutil --sb-state
test "$(mokutil --sb-state)" = 'SecureBoot enabled'
efi_value() {
    od -An -t u1 -j 4 -N 1 "/sys/firmware/efi/efivars/$1-8be4df61-93ca-11d2-aa0d-00e098032b8c" | tr -d ' \n'
}
test "$(efi_value SecureBoot)" = 1
test "$(efi_value SetupMode)" = 0
evidence cat /sys/kernel/security/lockdown
grep -Eq '\[(integrity|confidentiality)\]' /sys/kernel/security/lockdown
# The image's quiet karg hides this console line; the kernel journal keeps it.
journalctl -k -b --no-pager --output=cat | grep -F 'secureboot: Secure boot enabled' > "$events"
# The host added one explicit vendor-path entry; prove this boot used it rather
# than the removable path whose Fedora 44 aarch64 fallback is test-signed.
evidence efibootmgr -v
current=$(efibootmgr | sed -n 's/^BootCurrent: \([0-9A-Fa-f]\{4\}\)$/\1/p')
test -n "$current"
efibootmgr -v | grep -E "^Boot${current}[* ]" | grep -F 'shimaa64.efi' > "$events"
# Doctor refuses root; the disposable account has no session, so only its
# secure_boot result (a required-for-session check) is asserted here.
doctor_home=$(getent passwd kedra-test | cut -d: -f6)
doctor=$(runuser -u kedra-test -- env HOME="$doctor_home" /usr/bin/sysroot doctor --json || true)
printf '%s\n' "$doctor" > "$events"
printf '%s' "$doctor" | jq -e '.target == "utm"
    and any(.checks[]; .name == "secure_boot" and .passed and .required_for_session)' > /dev/null
marker KEDRA_UTM_SECUREBOOT_PASS

# bootc deployment identity for the generated disk.
evidence bootc status --json
bootc status --json | jq -e '.status.booted.image.image.image == "localhost/kedra-utm-test:secureboot"
    and .status.booted.image.architecture == "arm64"
    and .status.staged == null and .status.rollback == null' > /dev/null
jq -e '.target.id == "utm" and .target.architecture == "aarch64"' /usr/share/sysroot/source.json > /dev/null
# The host compares this with the digest of the image it gave bootc-image-builder.
image_digest=$(bootc status --json | jq -er .status.booted.image.imageDigest)
printf 'KEDRA_UTM_IMAGE_DIGEST=%s\n' "$image_digest" > "$events"
marker KEDRA_UTM_BOOTC_PASS

evidence sestatus
test "$(getenforce)" = Enforcing
case "$cmdline" in *' enforcing=0 '*|*' selinux=0 '*) false ;; esac
sestatus | grep -Eq '^Loaded policy name: +targeted$'
# AVC denials are review evidence, not a gate.
evidence journalctl -b --no-pager --output=cat --grep='avc: +denied' || true
marker KEDRA_UTM_SELINUX_PASS

# The observer is Type=exec, so its own start job does not hold the boot open.
state=$(timeout 60m systemctl is-system-running --wait || true)
printf 'system state: %s\n' "$state" > "$events"
evidence systemctl list-jobs --no-pager
evidence systemctl list-units --state=failed --no-pager --plain
test "$state" = running
test -z "$(systemctl list-units --state=failed --no-legend --plain)"
# The host exposes UTM's guest-agent channel; the package's own unit must bind to it.
test "$(systemctl is-active qemu-guest-agent.service)" = active
# The running agent carries the hosts/utm RPC block list (image-check.sh proves its effect).
qga_pid=$(systemctl show -P MainPID qemu-guest-agent.service)
evidence tr '\0' ' ' < "/proc/$qga_pid/cmdline"
tr '\0' '\n' < "/proc/$qga_pid/cmdline" \
    | grep -Fx -- '--block-rpcs=guest-exec,guest-exec-status,guest-file-close,guest-file-flush,guest-file-open,guest-file-read,guest-file-seek,guest-file-write,guest-set-user-password,guest-ssh-add-authorized-keys,guest-ssh-get-authorized-keys,guest-ssh-remove-authorized-keys' \
    > "$events"
evidence systemd-analyze time
evidence systemd-analyze blame --no-pager
marker KEDRA_UTM_UNITS_PASS
systemctl poweroff --no-block

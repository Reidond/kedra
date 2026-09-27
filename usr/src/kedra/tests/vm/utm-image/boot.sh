#!/usr/bin/env bash
# Boot ONE generated utm test disk through Ubuntu's Microsoft-enrolled AAVMF
# Secure Boot firmware under TCG. Run only on a disposable arm64 Actions runner.
set -euo pipefail
test "${GITHUB_ACTIONS:-}" = true
test "${RUNNER_OS:-}" = Linux
test "${GITHUB_REPOSITORY:-}" = Reidond/kedra
test "$(uname -m)" = aarch64
test "$#" -eq 4
disk=$(realpath -e "$1")
work=$2
evidence=$3
# Manifest digest of the image given to bootc-image-builder, as podman reports it.
image_digest=$4
case "$disk" in *.qcow2) ;; *) echo 'Only a generated QCOW2 disk can be booted' >&2; exit 1 ;; esac
[[ "$image_digest" =~ ^sha256:[0-9a-f]{64}$ ]]
test -f "$disk"
# Fresh firmware variables for every run; never reuse an earlier VARS file.
mkdir "$work"
mkdir -p "$evidence"
descriptor=/usr/share/qemu/firmware/40-edk2-aarch64-secure-enrolled.json
template=/usr/share/AAVMF/AAVMF_VARS.ms.fd
# Use exactly the distribution's Secure Boot + enrolled-keys pairing.
jq -e --arg vars "$template" '.mapping.device == "flash"
    and .mapping.executable.filename == "/usr/share/AAVMF/AAVMF_CODE.ms.fd"
    and .mapping["nvram-template"].filename == $vars
    and (.features | contains(["enrolled-keys", "secure-boot"]))
    and any(.targets[]; .architecture == "aarch64")' "$descriptor" > /dev/null
code=$(readlink -e /usr/share/AAVMF/AAVMF_CODE.ms.fd)
test "$code" = /usr/share/AAVMF/AAVMF_CODE.secboot.fd
{
    date --utc --iso-8601=seconds
    dpkg-query -W -f='${Package} ${Version}\n' qemu-system-arm qemu-efi-aarch64 python3-virt-firmware
    qemu-system-aarch64 --version
    nproc
    free -h
    sha256sum "$code" "$template" "$descriptor" "$disk"
    cat "$descriptor"
} > "$evidence/firmware-provenance.txt"
virt-fw-vars --input "$template" --print --verbose > "$evidence/firmware-template-vars.txt" 2>&1
# Fedora 44 shimaa64.efi is signed only by Microsoft UEFI CA 2023; require it in db,
# Secure Boot enabled and an enrolled platform key before booting anything.
awk '
    /^name=/ { variable = $1 }
    variable == "name=db" && $0 == "    subject CN=Microsoft UEFI CA 2023" { db = 1 }
    variable == "name=SecureBootEnable" && $0 == "  bool: ON" { enabled = 1 }
    variable == "name=PK" && /^    subject CN=/ { pk = 1 }
    END { exit !(db && enabled && pk) }
' "$evidence/firmware-template-vars.txt"
cp "$template" "$work/AAVMF_VARS.fd"
# bootc-image-builder writes no NVRAM entry, and Fedora 44 shim-aa64 16.1-5 ships
# test-signed fbaa64.efi/mmaa64.efi, so its removable path stops with a Security
# Violation. Add the vendor-path entry that an installed system receives from
# Anaconda/bootupd --update-firmware. The short-form file path is expanded by
# the firmware to the disk's EFI System Partition.
virt-fw-vars --inplace "$work/AAVMF_VARS.fd" --append-boot-filepath '\EFI\fedora\shimaa64.efi' \
    > "$evidence/firmware-boot-entry.log" 2>&1
virt-fw-vars --input "$work/AAVMF_VARS.fd" --print > "$evidence/firmware-boot-vars.txt" 2>&1
grep -F 'devpath=FilePath(\EFI\fedora\shimaa64.efi)' "$evidence/firmware-boot-vars.txt"
sha256sum "$work/AAVMF_VARS.fd" >> "$evidence/firmware-provenance.txt"
started=$SECONDS
set +e
# pauth-impdef avoids TCG's costly architected pointer-authentication emulation.
# The NIC carries no iPXE option ROM: nothing network-boots, and ipxe-qemu is not needed.
timeout --kill-after=60s 90m qemu-system-aarch64 \
    -machine virt -cpu max,pauth-impdef=on -accel tcg,thread=multi -smp 4 -m 4096 \
    -drive "if=pflash,format=raw,unit=0,readonly=on,file=$code" \
    -drive "if=pflash,format=raw,unit=1,file=$work/AAVMF_VARS.fd" \
    -drive "file=$disk,if=virtio,format=qcow2,snapshot=on" \
    -device virtio-rng-pci -device virtio-gpu-pci -device virtio-serial-pci \
    -chardev "file,id=events,path=$evidence/events.log" \
    -device virtserialport,chardev=events,name=org.kedra.events \
    -chardev "socket,id=qga,path=$work/qga.sock,server=on,wait=off" \
    -device virtserialport,chardev=qga,name=org.qemu.guest_agent.0 \
    -netdev user,id=net0 -device virtio-net-pci,netdev=net0,romfile= \
    -display none -monitor none -no-reboot \
    -serial "file:$evidence/serial.log" > "$evidence/qemu.log" 2>&1
status=$?
set -e
printf 'qemu_exit=%s elapsed_seconds=%s\n' "$status" "$((SECONDS - started))" | tee "$evidence/boot-timing.txt"
virt-fw-vars --input "$work/AAVMF_VARS.fd" --print > "$evidence/firmware-vars-after.txt" 2>&1
if test "$status" -eq 124 || test "$status" -eq 137; then
    echo 'FAIL: TCG Secure Boot VM did not power off within its bound' >&2
    exit 1
fi
test "$status" -eq 0
if grep -a -q KEDRA_UTM_FAIL "$evidence/serial.log" "$evidence/events.log"; then
    grep -a KEDRA_UTM_FAIL "$evidence/serial.log" "$evidence/events.log" >&2
    exit 1
fi
# Firmware selected the explicit entry, not the removable-media fallback.
grep -a -q -F 'from \EFI\fedora\shimaa64.efi' "$evidence/serial.log"
for marker in KEDRA_UTM_SECUREBOOT_PASS KEDRA_UTM_BOOTC_PASS KEDRA_UTM_SELINUX_PASS KEDRA_UTM_UNITS_PASS \
    "KEDRA_UTM_IMAGE_DIGEST=$image_digest"; do
    grep -a -q -F -x "$marker" "$evidence/events.log"
done
echo 'PASS: utm disk booted shim -> GRUB -> kernel under UEFI Secure Boot (TCG) and reported every marker.'

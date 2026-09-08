#!/usr/bin/bash
set -euo pipefail
trap 'status=$?; echo "KEDRA_R02_BOOT_FAIL line=$LINENO status=$status"; systemctl poweroff --no-block; exit "$status"' ERR
echo KEDRA_R02_BOOT_BEGIN
cat /etc/os-release
uname -r
bootc --version
bootc status --json
findmnt -n -o TARGET,SOURCE,FSTYPE --target /
findmnt -n -o TARGET,SOURCE,FSTYPE --target /var
test "$(getenforce)" = Enforcing
test -d /ostree
echo KEDRA_R02_BOOT_PASS
systemctl poweroff --no-block

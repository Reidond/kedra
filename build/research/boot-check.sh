#!/usr/bin/bash
set -euo pipefail
echo KEDRA_R02_BOOT_BEGIN
cat /etc/os-release
uname -r
bootc --version
bootc status --json
findmnt -n -o TARGET,FSTYPE / /var
test "$(getenforce)" = Enforcing
test -d /ostree
echo KEDRA_R02_BOOT_PASS
systemctl poweroff --no-block

#!/usr/bin/env bash
# Separate disposable installation environment, never the installed OS payload.
# Procedure source: osbuild/image-builder v82.0.0 doc/20-advanced/20-bootc/10-isos.md.
set -euo pipefail
test -f /run/.containerenv
test -f /usr/share/anaconda/interactive-defaults.ks
# Anaconda owns its console/rescue session. These accounts exist only on media.
if ! getent passwd install >/dev/null; then
    printf 'install:x:0:0:Anaconda:/root:/usr/libexec/anaconda/run-anaconda\n' >> /etc/passwd
    printf 'install::14438:0:99999:7:::\n' >> /etc/shadow
fi
passwd -d root
# Match Fedora Lorax's separate installer environment. This file is not copied
# into the desktop payload; installed-system enforcement is checked separately.
printf 'SELINUX=permissive\nSELINUXTYPE=targeted\n' > /etc/selinux/config
if ! getent passwd install-user >/dev/null; then
    useradd --uid 1001 --no-create-home --home-dir /tmp/install-user --shell /usr/bin/bash install-user
    passwd -d install-user
fi
install -m 0755 /usr/share/anaconda/list-harddrives-stub /usr/bin/list-harddrives
if test -d /etc/yum.repos.d; then mv /etc/yum.repos.d /etc/anaconda.repos.d; fi
systemctl set-default anaconda.target
# Do not automatically mount an unrelated disk while booting the installer.
rm -f /usr/lib/systemd/system-generators/systemd-gpt-auto-generator
ln -sf /usr/lib/systemd/system/anaconda-shell@.service /usr/lib/systemd/system/autovt@.service
mkdir -p /usr/lib/systemd/logind.conf.d
printf '[Login]\nReserveVT=2\n' > /usr/lib/systemd/logind.conf.d/kedra-installer.conf
mkdir -p "$(realpath /root)"
kernel=$(kernel-install list --json=short | jq -er '[.[] | select(.has_kernel == true) | .version] | if length == 1 then .[0] else error("exactly one installer kernel required") end')
DRACUT_NO_XATTR=1 dracut --force --zstd --reproducible --no-hostonly --add anaconda \
    "/usr/lib/modules/$kernel/initramfs.img" "$kernel"
for unit in pipewire.service pipewire.socket; do
    mkdir -p "/etc/systemd/user/$unit.d"
    printf '[Unit]\nConditionUser=\n' > "/etc/systemd/user/$unit.d/kedra-installer.conf"
done

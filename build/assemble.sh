#!/usr/bin/env bash
# Image-build entrypoint, not a host package installer.
set -euo pipefail
test -f /run/.containerenv
manifest=/usr/share/sysroot/source.json
jq -e '.schema_version == 1 and .target.fedora_release == 44 and .target.architecture == "x86_64" and .target.candidate_target == true' "$manifest" >/dev/null
jq -e '(.packages + .remove_packages) | all(type == "string" and test("^[A-Za-z0-9][A-Za-z0-9+._-]*$"))' "$manifest" >/dev/null
mapfile -t packages < <(jq -r '.packages[]' "$manifest")
mapfile -t remove < <(jq -r '.remove_packages[]' "$manifest")
test "${#packages[@]}" -gt 0
repos=(--repo=fedora --repo=updates '--setopt=*.skip_if_unavailable=False' '--setopt=*.gpgcheck=True')
dnf -y --refresh "${repos[@]}" upgrade
dnf -y --refresh "${repos[@]}" install "${packages[@]}"
if test "${#remove[@]}" -gt 0; then dnf -y "${repos[@]}" remove "${remove[@]}"; fi
dnf clean all
chmod 0755 /usr/bin/sysroot /usr/libexec/sysroot/helper
getent passwd greetd
systemctl enable greetd.service NetworkManager.service bluetooth.service
systemctl set-default graphical.target
systemctl mask bootc-fetch-apply-updates.timer bootc-fetch-apply-updates.service
# Normal initial-account seeding only; no existing live home is updated here.
mkdir -p /etc/skel
cp -a /usr/share/sysroot/home/default/. /etc/skel/
niri validate --config /usr/share/sysroot/home/default/.config/niri/config.kdl
NOCTALIA_CONFIG_HOME=/usr/share/sysroot/home/default/.config \
    NOCTALIA_STATE_HOME=/tmp/kedra-noctalia-validation noctalia config validate
sed -i 's/^NAME=.*/NAME="Kedra"/; s/^PRETTY_NAME=.*/PRETTY_NAME="Kedra (Fedora 44)"/' /usr/lib/os-release
rpm -qa | sort > /usr/share/sysroot/packages.txt
sysroot status --json
bootc container lint

#!/usr/bin/env bash
# Image-build entrypoint, not a host package installer.
set -euo pipefail
test -f /run/.containerenv
manifest=/usr/share/sysroot/source.json
# Build natively: the target's architecture must be this build's machine, and
# only the closed (target, architecture) pairs are accepted.
architecture=$(uname -m)
case "$architecture" in
    x86_64) bitwarden_napi=linux-x64-gnu ;;
    aarch64) bitwarden_napi=linux-arm64-gnu ;;
    *) echo "Unsupported image architecture: $architecture" >&2; exit 1 ;;
esac
jq -e --arg architecture "$architecture" '.schema_version == 1 and .target.fedora_release == 44 and .target.architecture == $architecture and .target.candidate_target == true and ([.target.id, .target.architecture] | IN(["desktop", "x86_64"], ["utm", "aarch64"]))' "$manifest" >/dev/null
jq -e '(.packages + .remove_packages) | all(type == "string" and test("^[A-Za-z0-9][A-Za-z0-9+._-]*$"))' "$manifest" >/dev/null
mapfile -t packages < <(jq -r '.packages[]' "$manifest")
mapfile -t remove < <(jq -r '.remove_packages[]' "$manifest")
test "${#packages[@]}" -gt 0
repos=(--repo=fedora --repo=updates '--setopt=*.skip_if_unavailable=False' '--setopt=*.gpgcheck=True')
dnf -y --best --refresh "${repos[@]}" upgrade
dnf -y --best --refresh "${repos[@]}" install "${packages[@]}"
if test "${#remove[@]}" -gt 0; then dnf -y "${repos[@]}" remove "${remove[@]}"; fi
dnf clean all
dnf check
if test "${1:-}" = --resolve-packages; then
    rpm -qa --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' \
        | LC_ALL=C sort > /resolution/package-material.txt
    exit 0
fi
# Its invisible capture window becomes a focused black tile under niri.
# Keep the native launcher, but exclude this session from bridge autostart.
bridge_autostart=/etc/xdg/autostart/org.kde.xwaylandvideobridge.desktop
test -f "$bridge_autostart" && test ! -L "$bridge_autostart"
bridge_updated=$(mktemp)
# desktop-file-edit 0.28 rejects niri as an unregistered desktop name.
awk '
    /^\[/ {
        if (entry && !excluded) print "NotShowIn=niri;"
        entry = ($0 == "[Desktop Entry]")
        if (entry) found = 1
    }
    entry && /^OnlyShowIn=/ { exit 1 }
    entry && /^NotShowIn=/ {
        if ($0 !~ /[=;]niri(;|$)/) {
            if ($0 !~ /[=;]$/) $0 = $0 ";"
            $0 = $0 "niri;"
        }
        excluded = 1
    }
    { print }
    END {
        if (!found) exit 1
        if (entry && !excluded) print "NotShowIn=niri;"
    }
' "$bridge_autostart" > "$bridge_updated"
cat "$bridge_updated" > "$bridge_autostart"
rm "$bridge_updated"
# Compile image-owned defaults after RPM installation; never write user dconf.
glib-compile-schemas --strict /usr/share/glib-2.0/schemas
# Recomputable build-time caches/logs are not installed machine state.
rm -rf /var/lib/dnf /var/cache/swcatalog /var/cache/ldconfig
rm -f /var/log/dnf5.log /var/log/dnf5.log.1
chmod 0755 /usr/bin/sysroot /usr/libexec/sysroot/helper
chmod 0755 /usr/libexec/kedra-session
test -x /usr/bin/bitwarden
test -x /usr/lib/bitwarden/bitwarden-app
runtime_dependencies=$(ldd /usr/lib/bitwarden/bitwarden-app /usr/lib/bitwarden/desktop_proxy "/usr/lib/bitwarden/resources/app.asar.unpacked/node_modules/@bitwarden/desktop-napi/desktop_napi.$bitwarden_napi.node")
if printf '%s\n' "$runtime_dependencies" | grep 'not found'; then
    echo 'Bitwarden runtime dependency missing' >&2
    exit 1
fi
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
# Keep RPM content identity as well as display names: identical NEVRA is not
# proof that a repository served identical signed package payloads.
rpm -qa --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' \
    | LC_ALL=C sort > /usr/share/sysroot/package-material.txt
sysroot status --json
bootc container lint

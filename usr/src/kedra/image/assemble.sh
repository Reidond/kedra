#!/usr/bin/env bash
# Foundation build step of the Containerfile, not a host package installer.
# Image steps (GLib schemas, systemd units, skeleton, initramfs) are declared in
# each target's .kedra block and applied later by `sysroot derive`.
set -euo pipefail
test "$#" -eq 0 || { echo 'Unsupported Kedra assembly arguments' >&2; exit 1; }
# Podman marks its build containers; a BuildKit build (the local lab build in
# usr/src/kedra/tests/container) runs the Containerfile RUN line as PID 1.
test -f /run/.containerenv || grep -qF /tmp/kedra-assemble.sh /proc/1/cmdline
manifest=/usr/share/sysroot/source.json
# Build natively: the target's architecture must be this build's machine, and
# only the closed (target, architecture) pairs are accepted.
architecture=$(uname -m)
case "$architecture" in
    x86_64) bitwarden_napi=linux-x64-gnu ;;
    aarch64) bitwarden_napi=linux-arm64-gnu ;;
    *) echo "Unsupported image architecture: $architecture" >&2; exit 1 ;;
esac
jq -e --arg architecture "$architecture" '.schema_version == 1 and .target.fedora_release == 44 and .target.architecture == $architecture and .target.candidate_target == true and ([.target.id, .target.architecture] | IN(["desktop", "x86_64"], ["qemu-arm64", "aarch64"]))' "$manifest" >/dev/null
# Fedora RPMs come only from this run's RPM lock. The Containerfile bind-mounts
# the offline repository `sysroot catalog lock repo` made from fixed-output store
# objects; catalog/install-rpms.sh installs exactly the locked foundation and
# refuses any other result. The lock must belong to this committed source.
repository=/run/kedra-rpms
jq -e --slurpfile manifest "$manifest" '
    .target == $manifest[0].target.id
    and .foundation.requests == ($manifest[0].packages | unique)
    and .foundation.remove == ($manifest[0].remove_packages | unique)
    and .intent_sha256 == $manifest[0].package_frontend.intent_sha256' "$repository/lock.json" >/dev/null
/bin/bash /tmp/kedra-install-rpms.sh "$repository" foundation
install -D -m 0644 "$repository/lock.json" /usr/share/sysroot/rpm-lock.json
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
niri validate --config /usr/share/sysroot/home/default/.config/niri/config.kdl
NOCTALIA_CONFIG_HOME=/usr/share/sysroot/home/default/.config \
    NOCTALIA_STATE_HOME=/tmp/kedra-noctalia-validation noctalia config validate
rm -rf /tmp/kedra-noctalia-validation
sed -i 's/^NAME=.*/NAME="Kedra"/; s/^PRETTY_NAME=.*/PRETTY_NAME="Kedra (Fedora 44)"/' /usr/lib/os-release
rpm -qa | sort > /usr/share/sysroot/packages.txt
# Keep RPM content identity as well as display names: identical NEVRA is not
# proof that a repository served identical signed package payloads.
rpm -qa --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' \
    | LC_ALL=C sort > /usr/share/sysroot/package-material.txt
sysroot --version

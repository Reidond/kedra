#!/usr/bin/env bash
# Foundation build step of the Containerfile, not a host package installer.
# Everything Kedra-specific is declared in system/*.kedra: files arrive in
# payload.tar, foundation edits are read from the source manifest here, and
# image steps (GLib schemas, systemd units, skeleton, initramfs, home checks)
# are applied later by `sysroot derive`.
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
    x86_64 | aarch64) ;;
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
# Foundation edits of RPM-owned files that the target declares in
# system/catalog.kedra: XDG autostart entries hidden from a desktop, and the
# os-release names. The values are checked plain text without quotes,
# backslashes or expansion characters.
while IFS=$'\t' read -r entry desktop; do
    autostart=/etc/xdg/autostart/$entry
    test -f "$autostart" && test ! -L "$autostart"
    updated=$(mktemp)
    # desktop-file-edit 0.28 rejects names it has not registered, such as niri.
    awk -v desktop="$desktop" '
        /^\[/ {
            if (entry && !excluded) print "NotShowIn=" desktop ";"
            entry = ($0 == "[Desktop Entry]")
            if (entry) found = 1
        }
        entry && /^OnlyShowIn=/ { exit 1 }
        entry && /^NotShowIn=/ {
            if ($0 !~ ("[=;]" desktop "(;|$)")) {
                if ($0 !~ /[=;]$/) $0 = $0 ";"
                $0 = $0 desktop ";"
            }
            excluded = 1
        }
        { print }
        END {
            if (!found) exit 1
            if (entry && !excluded) print "NotShowIn=" desktop ";"
        }
    ' "$autostart" > "$updated"
    cat "$updated" > "$autostart"
    rm "$updated"
done < <(jq -r '.hide_autostart // {} | to_entries[] | .key as $entry | .value[] | [$entry, .] | @tsv' "$manifest")
if jq -e 'has("os_release")' "$manifest" >/dev/null; then
    release=$(mktemp)
    awk -v name="$(jq -r '.os_release.name' "$manifest")" \
        -v pretty="$(jq -r '.os_release.pretty_name' "$manifest")" '
        /^NAME=/ { print "NAME=\"" name "\""; next }
        /^PRETTY_NAME=/ { print "PRETTY_NAME=\"" pretty "\""; next }
        { print }
    ' /usr/lib/os-release > "$release"
    cat "$release" > /usr/lib/os-release
    rm "$release"
fi
# Recomputable build-time caches/logs are not installed machine state.
rm -rf /var/lib/dnf /var/cache/swcatalog /var/cache/ldconfig
rm -f /var/log/dnf5.log /var/log/dnf5.log.1
chmod 0755 /usr/bin/sysroot /usr/libexec/sysroot/helper
# The greeter's account comes from its RPM. Managed home files are validated by
# `sysroot derive` with their declared checks.
getent passwd greetd
rpm -qa | sort > /usr/share/sysroot/packages.txt
# Keep RPM content identity as well as display names: identical NEVRA is not
# proof that a repository served identical signed package payloads.
rpm -qa --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' \
    | LC_ALL=C sort > /usr/share/sysroot/package-material.txt
sysroot --version

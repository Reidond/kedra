#!/usr/bin/env bash
# Runs as root only inside a disposable Fedora resolver or builder container.
set -euo pipefail
test -f /run/.containerenv || grep -qF /tmp/kedra-catalog-builder.sh /proc/1/cmdline
test "$(uname -m)" = aarch64
test "$(id -u)" = 0
grep -qx 'ID=fedora' /usr/lib/os-release
grep -Eq '^VERSION_ID="?44"?$' /usr/lib/os-release
case "${1:-}" in --resolve | --build) ;; *) exit 64 ;; esac
repos=(--repo=fedora --repo=updates '--setopt=*.skip_if_unavailable=False' '--setopt=*.gpgcheck=True')
dnf -y --best --refresh "${repos[@]}" upgrade
requests=/usr/share/kedra/catalog-builder-requests.json
if test "$1" = --resolve && test -f /resolution/catalog-builder-requests.json; then
    requests=/resolution/catalog-builder-requests.json
fi
if test -f "$requests"; then
    test ! -L "$requests"
    jq -e '.schema_version == 1 and ([.packages,.foundation_packages,.remove] | all(type == "array" and length <= 512 and all(type == "string" and test("^[A-Za-z0-9][A-Za-z0-9+._-]*$"))))' "$requests" >/dev/null
    mapfile -t packages < <(jq -r '.packages[]' "$requests")
    if test "$1" = --resolve; then
        mapfile -t foundation < <(jq -r '.foundation_packages[]' "$requests")
        mapfile -t remove < <(jq -r '.remove[]' "$requests")
        if test "${#foundation[@]}" -gt 0; then dnf -y --best --refresh "${repos[@]}" install "${foundation[@]}"; fi
        if test "${#remove[@]}" -gt 0; then dnf -y "${repos[@]}" remove "${remove[@]}"; fi
    fi
else
    # Retained list/catalog revisions keep their exact compiler request contract.
    packages=(gcc make autoconf automake libtool binutils diffutils findutils tar gzip coreutils gawk grep sed pkgconf-pkg-config)
fi
if test "${#packages[@]}" -gt 0; then dnf -y --best --refresh "${repos[@]}" install "${packages[@]}"; fi
dnf clean all
dnf check
mkdir -p /usr/share/kedra
rpm -qa --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' \
    | awk -F '\t' '$1 != "gpg-pubkey"' | LC_ALL=C sort > /usr/share/kedra/catalog-builder-rpms.txt
if test "$1" = --resolve; then
    cp /usr/share/kedra/catalog-builder-rpms.txt /resolution/catalog-builder-rpms.txt
else
    {
        printf 'catalog-builder-tools-v1\n'
        for tool in gcc make autoconf automake libtool python3 patch; do
            if test -x "/usr/bin/$tool"; then "/usr/bin/$tool" --version; fi
        done
    } > /usr/share/kedra/catalog-builder-tools.txt
fi

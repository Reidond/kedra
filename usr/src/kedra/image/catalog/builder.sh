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
dnf -y --best --refresh "${repos[@]}" install \
    gcc make autoconf automake libtool binutils diffutils findutils \
    tar gzip coreutils gawk grep sed pkgconf-pkg-config
dnf clean all
dnf check
mkdir -p /usr/share/kedra
rpm -qa --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' \
    | awk -F '\t' '$1 != "gpg-pubkey"' | LC_ALL=C sort > /usr/share/kedra/catalog-builder-rpms.txt
if test "$1" = --resolve; then
    cp /usr/share/kedra/catalog-builder-rpms.txt /resolution/catalog-builder-rpms.txt
else
    {
        gcc --version
        make --version
        autoconf --version
        automake --version
        libtool --version
    } > /usr/share/kedra/catalog-builder-tools.txt
fi

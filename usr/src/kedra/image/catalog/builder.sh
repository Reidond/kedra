#!/usr/bin/env bash
# Runs as root only inside a disposable catalog builder image build.
set -euo pipefail
test -f /run/.containerenv || grep -qF /tmp/kedra-catalog-builder.sh /proc/1/cmdline
test "$(uname -m)" = aarch64
test "$(id -u)" = 0
grep -qx 'ID=fedora' /usr/lib/os-release
grep -Eq '^VERSION_ID="?44"?$' /usr/lib/os-release
case "$#:${1:-}" in 2:--build) role=$2 ;; *) exit 64 ;; esac
# The compiler role's RPMs come only from this run's RPM lock, installed offline
# over the exact foundation from the bind-mounted repository.
/bin/bash /tmp/kedra-install-rpms.sh /run/kedra-rpms builder "$role"
mkdir -p /usr/share/kedra
rpm -qa --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' \
    | awk -F '\t' '$1 != "gpg-pubkey"' | LC_ALL=C sort > /usr/share/kedra/catalog-builder-rpms.txt
{
    printf 'catalog-builder-tools-v1\n'
    for tool in gcc make autoconf automake libtool python3 patch; do
        if test -x "/usr/bin/$tool"; then "/usr/bin/$tool" --version; fi
    done
} > /usr/share/kedra/catalog-builder-tools.txt

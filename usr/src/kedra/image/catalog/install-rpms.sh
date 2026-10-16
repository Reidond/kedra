#!/usr/bin/env bash
# Runs as root only inside a disposable image build, from assemble.sh. Installs
# exactly one role of the run's RPM lock from the bind-mounted offline
# repository that `sysroot catalog lock repo` made from fixed-output store
# objects. No repository is enabled, so nothing is resolved or downloaded here;
# the result must equal the role's locked inventory. (Catalog builder roles are
# installed by `sysroot store add-root` in the engine sandbox instead.)
set -euo pipefail
test -f /run/.containerenv || grep -qF /tmp/kedra- /proc/1/cmdline
test "$(id -u)" = 0
case "$#:${2:-}" in
    2:foundation) selector='.foundation' role= ;;
    3:builder) selector='.builders[$role]' role=$3 ;;
    *) echo 'Usage: install-rpms.sh <repository> foundation | builder <role>' >&2; exit 64 ;;
esac
repository=$1
lock=$repository/lock.json
packages=$repository/packages
test -f "$lock" && test ! -L "$lock" && test -d "$packages" && test ! -L "$packages"
jq -e --arg architecture "$(uname -m)" --arg role "$role" '
    .schema_version == 1 and .format == "kedra-rpm-lock" and .fedora_release == 44
    and .architecture == $architecture and ('"$selector"' | type == "object")' "$lock" >/dev/null
# The repository holds exactly the role's files, each with its locked SHA-256.
work=$(mktemp -d /tmp/kedra-install-rpms.XXXXXXXX)
jq -r --arg role "$role" "$selector.install[]" "$lock" > "$work/expected-files"
(cd "$packages" && find . -mindepth 1 -printf '%y %P\n' | LC_ALL=C sort) > "$work/actual-files"
# coreutils only: the foundation of a builder need not carry diffutils.
test "$(sed 's/^/f /' "$work/expected-files")" = "$(cat "$work/actual-files")"
jq -r --arg role "$role" "$selector"'.install[] as $file | "\(.rpms[$file].sha256)  \($file)"' "$lock" \
    > "$work/sha256"
(cd "$packages" && sha256sum --quiet --strict -c "$work/sha256")
mapfile -t files < <(sed "s|^|$packages/|" "$work/expected-files")
mapfile -t remove < <(jq -r --arg role "$role" "$selector.remove[]" "$lock")
# Fedora's key verifies every file's signature, as the networked repositories did.
rpmkeys --import /etc/pki/rpm-gpg/RPM-GPG-KEY-fedora-44-primary
offline=(--disablerepo='*' '--setopt=localpkg_gpgcheck=True')
if test "${#files[@]}" -gt 0; then dnf -y "${offline[@]}" install "${files[@]}"; fi
if test "${#remove[@]}" -gt 0; then dnf -y "${offline[@]}" remove "${remove[@]}"; fi
dnf "${offline[@]}" clean all
dnf "${offline[@]}" check
rpm -qa --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n' \
    | grep -v $'^gpg-pubkey\t' | LC_ALL=C sort > "$work/installed"
jq -r --arg role "$role" "$selector"'.packages[] | join("\t")' "$lock" > "$work/locked"
if test "$(cat "$work/locked")" != "$(cat "$work/installed")"; then
    comm -3 "$work/locked" "$work/installed" >&2 || true
    echo 'Installed RPMs differ from the RPM lock' >&2
    exit 1
fi
rm -rf -- "$work"

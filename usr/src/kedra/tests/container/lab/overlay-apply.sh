#!/usr/bin/bash
# Lab overlay build step; runs only inside a lab image build (see overlay.Containerfile).
set -euo pipefail
trap 'echo "overlay-apply failed at line $LINENO" >&2' ERR
base=/usr/share/kedra-lab/base-source.json
next=/usr/share/sysroot/source.json
# Same target, same package intent: anything else needs a real image build.
jq -e -n --slurpfile base "$base" --slurpfile next "$next" \
    '$base[0].target.id == $next[0].target.id' >/dev/null || {
    echo 'Overlay payload targets a different image target than the base image' >&2
    exit 1
}
if ! diff <(jq -S '{packages: (.packages | sort), remove_packages: (.remove_packages | sort)}' "$base") \
          <(jq -S '{packages: (.packages | sort), remove_packages: (.remove_packages | sort)}' "$next"); then
    echo 'The working tree changes the package list; an overlay cannot install packages.' >&2
    echo 'Use KEDRA_LAB_IMAGE=build (full local image build) instead.' >&2
    exit 1
fi
# Remove files the base image shipped from its payload that this payload no
# longer ships, including their account-baseline copies in /etc/skel.
while IFS= read -r destination; do
    case "$destination" in
        etc/*|usr/*) ;;
        *) echo "Unexpected payload destination: $destination" >&2; exit 1 ;;
    esac
    rm -f -- "/$destination"
    case "$destination" in
        usr/share/sysroot/home/default/*) rm -f -- "/etc/skel/${destination#usr/share/sysroot/home/default/}" ;;
    esac
done < <(LC_ALL=C comm -23 <(jq -r '.files[].destination' "$base" | LC_ALL=C sort) \
                           <(jq -r '.files[].destination' "$next" | LC_ALL=C sort))
# The payload half of usr/src/kedra/image/assemble.sh.
chmod 0755 /usr/bin/sysroot /usr/libexec/sysroot/helper /usr/libexec/kedra-session
glib-compile-schemas --strict /usr/share/glib-2.0/schemas
mkdir -p /etc/skel
cp -a /usr/share/sysroot/home/default/. /etc/skel/
niri validate --config /usr/share/sysroot/home/default/.config/niri/config.kdl
NOCTALIA_CONFIG_HOME=/usr/share/sysroot/home/default/.config \
    NOCTALIA_STATE_HOME=/tmp/kedra-noctalia-validation noctalia config validate
rm -rf /tmp/kedra-noctalia-validation
sysroot status --json >/dev/null

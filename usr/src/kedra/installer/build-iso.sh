#!/usr/bin/env bash
# Pinned builder container; use native osbuild stages for local installation media.
set -euo pipefail
test -f /run/.containerenv
test -d /output
test -d /evidence
test -n "${KEDRA_INSTALLER_PAYLOAD:-}"
test -n "${KEDRA_INSTALLER_IMAGE:-}"
store=/var/cache/image-builder/store
# The pinned builder's manifest, without building anything; osbuild then builds
# the labeled manifest once.
image-builder manifest bootc-generic-iso --bootc-ref "$KEDRA_INSTALLER_IMAGE" \
    --bootc-default-fs ext4 \
    --bootc-installer-payload-ref "$KEDRA_INSTALLER_PAYLOAD" \
    > /evidence/original.osbuild-manifest.json
python3 /kedra-installer/label-manifest.py /evidence/original.osbuild-manifest.json \
    /evidence/labeled.osbuild-manifest.json
# The environment `image-builder build` sets up in a container before it runs
# osbuild (pkg/setup EnsureEnvironment of the pinned v82): a root_t store, osbuild
# relabeled install_exec_t on a tmpfs so its stages may write SELinux labels the
# host does not know, and devtmpfs for dynamic loop devices.
mkdir -p "$store" /run/osbuild
chcon system_u:object_r:root_t:s0 "$store"
mountpoint -q /run/osbuild || mount -t tmpfs tmpfs /run/osbuild
cp -p /usr/bin/osbuild /run/osbuild/osbuild
chcon system_u:object_r:install_exec_t:s0 /run/osbuild/osbuild
mount -t devtmpfs devtmpfs /dev
mount --bind /run/osbuild/osbuild /usr/bin/osbuild
osbuild --store "$store" --output-directory /output \
    --export bootiso /evidence/labeled.osbuild-manifest.json

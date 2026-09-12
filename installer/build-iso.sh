#!/usr/bin/env bash
# Pinned builder container; use native osbuild stages for local installation media.
set -euo pipefail
test -f /run/.containerenv
test -d /output
test -d /evidence
test ! -e /tmp/kedra-unlabeled
test -n "${KEDRA_INSTALLER_PAYLOAD:-}"
test -n "${KEDRA_INSTALLER_IMAGE:-}"
# Initialize the builder's supported container environment and retain its exact
# manifest/cache. This preliminary ISO is never exported as installation media.
image-builder build --bootc-ref "$KEDRA_INSTALLER_IMAGE" \
    --bootc-default-fs ext4 \
    --bootc-installer-payload-ref "$KEDRA_INSTALLER_PAYLOAD" \
    --output-dir /tmp/kedra-unlabeled --with-manifest bootc-generic-iso
mapfile -t manifests < <(find /tmp/kedra-unlabeled -type f -name '*.osbuild-manifest.json')
test "${#manifests[@]}" -eq 1
cp "${manifests[0]}" /evidence/original.osbuild-manifest.json
python3 /kedra-installer/label-manifest.py /evidence/original.osbuild-manifest.json \
    /evidence/labeled.osbuild-manifest.json
osbuild --store /var/cache/image-builder/store --output-directory /output \
    --export bootiso /evidence/labeled.osbuild-manifest.json

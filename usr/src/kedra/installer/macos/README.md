# ARM64 installer media on macOS

Use `kedra-lab vm iso --image ghcr.io/reidond/kedra-qemu-arm64@sha256:REVIEWED_DIGEST --output NEW_DIRECTORY`.
The signed target identity is `qemu-arm64`; no UTM application is required.

`media.py` preserves the former Mac builder's signed-media path: the pinned Fedora
ARM64 container runs the unchanged `installer/build-local.py`, with the trusted
checkout mounted read-only. Fixed-key verification, target/architecture checks,
exact image/builder digests and ISO receipts remain mandatory. The host verifies
`installer.json`, `SHA256SUMS`, ISO size and hash after export. No media is published.

The privileged builder is confined to Docker's Linux VM. It delegates controllers
inside its private cgroup namespace, uses nested Podman and requires 40 GiB free
there plus 8 GiB on the Mac for export. Keep builds on one architecture per Docker
VM; old amd64/arm64 shared-memory lock layouts can conflict. No workstation disk
or bootloader is changed.

Podman storage is cached in `kedra-macos-media-storage`. Failed output volumes
`kedra-media-iso-out-*` are reported and retained for inspection; successful output
volumes are removed. The native [QEMU lab](../../tests/container/qemu/README.md)
creates a fresh VM from the verified ISO and detaches its private copy after
installation. The former UTM identity is retired; the replacement uses a new QEMU disk and authority.

The extracted builder keeps the earlier verified policy; new-path signed ISO and
interactive installer qualification are tracked separately in the current task.

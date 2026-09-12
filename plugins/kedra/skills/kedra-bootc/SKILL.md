---
name: kedra-bootc
description: Maintain Fedora 44 image derivation, filesystem ownership, signed GHCR updates and local recovery media.
---

# Fedora bootc

Read docs/ARCHITECTURE.md, docs/UPDATES.md and references/update-client.md. Use plain Containerfile and explicit shared/host payload assembly. OS images build in Actions; on-demand ISO media builds locally through installer/build-local.py. No BlueBuild or live host DNF shortcut.

Prefer image-owned /usr defaults. /etc follows bootc persistence/merge; /var and home persist across deployments. Never author /usr/etc or overwrite writable home from an image. Ship safe baselines and use explicit home reconciliation. Repository skills remain checkout-local.

Resolve only the official production Fedora 44 stream, not similarly named development images. A pin prevents substitution but cannot guarantee upstream retention: Quay retired d4b9c5e... before native tests on 2026-09-12. Current tests resolve/verify one immutable platform per run. Source: [Fedora publication](https://forge.fedoraproject.org/iot/base-images/src/commit/8f30db6ad355562aeaca5c547f81ca157e8ffbf4/RELEASE.md). New base inputs require actual qualification.

The installed helper verifies signed GHCR stable discovery and stages exact digests through enforcing bootc policy. Normal bootc upgrade does not advance a digest-pinned installation. Preserve pending slots, ordering and rollback holds; no automatic reboot or home activation.

No-change CI publishes nothing. There is no checkpoint renewal in the current path. Legacy release/checkpoint state is migrated explicitly; never reset it to bypass a refusal. Keep local recovery independent of GitHub Releases.

Anaconda media is separate and permissive under the pinned Fedora installer policy; installed SELinux remains enforcing. Local media requires offline signed-payload verification before disk installation. Preserve hash-guarded target scratch, selected non-API mounts before account creation and the physical /sysroot fstab normalization. Unknown upstream/source changes refuse rather than patch blindly.

Native tests must distinguish image build, signed update/rollback, fresh installation, graphical health and physical hardware. Key rotation, old-reader compatibility and physical devices require independent evidence.

Primary references: [filesystems](https://bootc.dev/bootc/filesystem.html), [switch](https://bootc.dev/bootc/man/bootc-switch.8.html), [build guidance](https://bootc.dev/bootc/building/guidance.html), [physical root](https://bootc.dev/bootc/bootc-install.html#finding-and-configuring-the-physical-root-filesystem).

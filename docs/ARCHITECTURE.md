# Architecture and safety contracts

Kedra is a personal Fedora 44 bootc OS. `sysroot` is the CLI; a narrow image-installed helper performs authorized deployment operations. Rust edition 2024, one Cargo workspace/lockfile and explicit flat `main.rs`/`lib.rs` paths are required. No first-party `src/` trees, BlueBuild, custom Git engine, configuration language, fleet service or permanent AI daemon.

## Source and ownership

Shared `etc/`, `usr/`, `home/` files are resolved before explicit `hosts/<target>/` overrides. The committed source plan records path provenance and replacement. Documentation and `.gitkeep` do not become rootfs files. Generated build contexts contain only declared payloads; never copy the whole repository into an image.

`/usr` contains image-owned software, public trust and home baselines. `/etc` follows bootc persistence/merge semantics; `/var` and live home persist across deployments. Root state lives in `/var/lib/sysroot`; private user review state is separate. OS rollback does not rewind home, credentials, persistent databases or personal tools.

## Release authority

The final OCI registry digest is signed using a dedicated OS-release key. P-256/SHA-256 signatures authenticate exact release/checkpoint bytes. Metadata binds repository, target, architecture, source/run identity, sequence, image digest, home provenance and ISO size/hash. Candidate signing is distinct from promotion and from Secure Boot.

Build jobs have public trust only. The protected signer executes no checkout, candidate binaries or repository scripts with production keys. Public preparation and key-free publication independently validate exact assets. Per-target serialization and predecessor/source rechecks prevent stale promotion. Immutable version assets precede the mutable signed channel; uncertain publication preserves state for inspected recovery.

The helper independently verifies installed authority, scope, signatures, fresh channel binding and replay floors. It uses exact-digest bootc switching with enforcing container policy. Root trust cannot come from a writable checkout, user review database or caller's claimed verification. Expired metadata can authenticate publisher history through a distinct historical-only result, never authorize a routine incoming update. Rollback retains high-water and places updates on hold.

Keep promoted digests, signatures, ISO records and recovery material. No automatic garbage collection. Image signatures do not establish upstream base authenticity; retain reviewed immutable base inputs. Changed upstream pins require deliberate review. A source SHA against moving RPM repositories is not a bit-for-bit rebuild recipe.

## Installer

Anaconda runs in a separate media environment; its console/rescue privileges and permissive SELinux setting do not change the installed desktop's enforcing SELinux or strict signature policy. The ISO verifies its embedded payload offline before installation. No preset disk identifiers, first-disk erase or default owner password.

The pinned image-builder manifest receives a native SELinux-labeling stage where missing. Hash-guarded Anaconda adapters preserve all approved non-API mounts for account creation and supply target-backed scratch storage for OCI import only after disk selection. Cleanup failures stop installation. The fstab normalizer addresses the physical root at `/sysroot`, preserves read-only policy and other mounts, and refuses unsupported input. See [bootc physical-root guidance](https://bootc.dev/bootc/bootc-install.html#finding-and-configuring-the-physical-root-filesystem).

## Writable home

Live files remain ordinary writable files. Baseline, live content, selected changes, explicit local-only policy, published source receipts and pending recovery are independent state. Never replace this with symlinks, whole-home capture or Git ignore/index flags. Credentials, vault/keyring data, transcripts and caches are excluded before capture.

Noctalia uses a narrow safe-field projection; niri uses explicitly adopted text paths. Plans bind reviewed content. Application writers are coordinated, files are rechecked, changes are journaled and native reload/start is verified. No conflict markers enter live configuration. Stale plans and unknown/corrupt state refuse safely; recovery can abort, resume or preserve newer current content. Source publication and accepting an installed baseline are separate operations.

## Agents and credentials

Bundled agents are private executables reached through `sysroot`; personal runtimes and profiles remain independent. A profile directory is not a security sandbox. Repository skills and pinned upstream notices remain in `plugins/kedra`, never automatically registered globally or copied into the OS.

Bitwarden holds SSH keys. Never export private keys or pass an unlocked vault session to agents. SSH, GitHub API, registry, model authentication and OS-release signing are separate authorities. Account login is an owner operation; no custom OAuth or credential synchronization.

## Verification and recovery

Use real CLI/process/Git workflows and disposable VMs. No unit/model/mock/doctests, repository source scanners or custom test runner. Formatting, Clippy, builds and runtime validation remain required. A container build is not a boot test; a staged image is not running or healthy. Never enroll, format or apply home changes on the development workstation. Unknown hardware, Secure Boot, key rotation and unrun failure cases stay explicit in [status](STATUS.md).

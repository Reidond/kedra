# Kedra product contract

Kedra is a personal Fedora 44 bootc desktop, maintained from one Git repository with `sysroot` as its deterministic management interface. [Architecture](docs/ARCHITECTURE.md) defines implementation and safety contracts; [status](docs/STATUS.md) identifies actual qualification.

The required workflow is source editing and selected home publication, Actions signed OCI production, verified explicit staging, owner-chosen reboot, home reconciliation and health checks. Package refresh begins at 00:00 UTC; no-change does nothing. On-demand local ISO construction consumes a reviewed signed image and never uploads. No GitHub Releases, ISO assets, automatic workstation installation, staging or reboot follows from a build.

Shared Linux-shaped files and explicit host overrides produce independently enrolled target images. Desktop (x86_64) and the `utm` aarch64 virtual machine for Apple Silicon hosts under UTM are enabled; unknown XPS hardware stays disabled. Never guess device identifiers.

Real home files stay writable. Selection, uncommitted changes, explicit local-only policy, publication and installed-baseline acceptance remain independent. Credentials are excluded before capture. No arbitrary whole-home synchronization.

Official agents are private tools reached through `sysroot`; personal runtimes, profiles, MCP and skills remain independent. Bitwarden-held SSH keys never leave the vault. Model, registry, API and release-signing authorities are separate.

UEFI Secure Boot is required: installation media refuse to start without it, and the health check fails without it. It is never a boot-time hard stop on an installed system, so local recovery stays available.

Acceptance requires a disposable encrypted installation, signed forward update and retained rollback, signature/replay negatives, healthy desktop, preserved selected/local home changes, and deterministic recovery. Physical hardware, credentials and additional targets require their own observed qualification. A successful source build alone does not satisfy these conditions.

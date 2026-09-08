# ADR 0016: installed deployment protocol and durable native-state reconciliation

Date: 2026-09-08. Status: Linux and native signed-VM subset passes; production authority pending.

The ordinary CLI sends a bounded versioned JSON envelope through explicit sudo
authorization to `/usr/libexec/sysroot/helper`. The helper accepts no command-line
paths, arbitrary commands, key arguments, image references or preverified flags.
It accepts signed release/checkpoint bytes for enrollment/staging, signed release
bytes for explicit retained rollback, and a status request. Native execution is
root-only and Linux-only. No passwordless sudo rule or setuid bit is installed.

The helper independently loads `/usr/lib/sysroot/trust/release-policy.json`, the
P-256 SPKI key at `/usr/lib/sysroot/trust/release.pub`, installed source.json and
machine-id using an owner/mode/type/link/inode-checked bounded reader. Policy,
source target, architecture, Fedora major, repository and key fingerprint must
agree. `/etc/containers/policy.json` must exactly match the canonical strict
reject-by-default docker/local-store policy for that immutable key/repository.
Native commands use absolute image-owned executables, cleared environments,
explicit arguments and time/output bounds. Only qualified bootc 1.16.10 is enabled.
Unsigned research media has no release authority, so these operations remain
unavailable there. No production private key or public release policy exists yet.

State lives in the existing private SQLite store at `/var/lib/sysroot/deployment`.
The fixed parent is root-owned mode 0700, provided by tmpfiles. A management lock
coordinates helper callers and remains inherited by a surviving trusted child if
the helper is interrupted. A separate administrator invoking bootc directly is
outside that cooperative protocol; preflight re-reads native state immediately
before publishing intent, and postflight reconciles actual effects.

Enrollment requires fresh promoted metadata for the exact running signed image,
source revision and source-manifest bytes. `home_manifest_sha256` binds the whole
installed `/usr/share/sysroot/source.json`, including home provenance, rather
than a reserialized subset. Enrollment never replaces an existing store.

Stage verifies release/checkpoint signatures, target binding, freshness and
monotonic high-water state. It preserves a different staged image unless the
caller explicitly names that exact digest for replacement. Download-only and
actual pending deployments remain distinct. A journal intent and retained public
receipt are durable before `bootc switch --enforce-container-sigpolicy` executes.
The helper then observes bootc and records not-applied, awaiting-reboot or booted;
process success alone is insufficient. Status reconciles interrupted operations
and verifies a booted image's installed source against its retained receipt.

Rollback requires a signed promoted record matching the native retained rollback
slot. It keeps forward high-water state and records an update hold. Resuming
forward staging requires an explicit flag. Neither operation reboots or applies
home files. Booted does not mean home-reconciled or healthy; those checks remain
separate and response fields state that boundary.

Eight pure tests use actual R01 bootc JSON plus negative control-field, scope,
signature, architecture, overlay, pending replacement, hold and interrupted-intent
cases. Protocol tests caught Serde's treatment of an internally tagged unit
variant: a unit Status variant ignored extra fields despite deny_unknown_fields.
Using an empty struct variant makes arbitrary verified/key/path fields fail.
The shared trusted reader retains the previous Noctalia inode guards and tests.
Windows checks do not compile the Linux-only execution path. Linux workspace
34203674917 and native signed-VM 34208979891 now pass. Production authority,
interactive installer handoff and additional crash/fault cases remain open.

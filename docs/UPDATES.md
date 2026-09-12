# Update and recover

Kedra updates use the signed GHCR `stable` image. The tag locates a candidate; the installed helper verifies signature, repository, target, image identity and retained ordering before switching to its exact digest. No GitHub Release files or checkpoint renewal is involved.

## Current-source commands

Run as the ordinary owner; the fixed installed helper requests administrator authentication:

```sh
sysroot update enroll
sysroot update check
sysroot update stage
```

Enroll once. Review an available update before staging it. Staging changes the next boot image but does not reboot.

Checking records verified image ordering and last-check state in the root-owned store; it does not stage an image. It is not proof that upstream Fedora has no newer packages. Add --expected-digest when enrollment or staging must match a digest you independently reviewed.

After your chosen reboot:

```sh
sysroot update status --home
sysroot doctor
```

Home baseline acceptance remains explicit through the [Noctalia](HOME-REVIEW.md) and [niri](TEXT-REVIEW.md) workflows. It does not discard local changes automatically.

Use `sysroot update rollback` to select the retained verified deployment. Rollback preserves persistent data and places forward updates on hold. A later reviewed `sysroot update stage --resume` clears that hold after normal verification. Never remove journals or trust state to bypass a refusal.

## One-time legacy migration

Previously installed r2 software expects the old GitHub release/checkpoint protocol. Do not use its old `sysroot update` commands after those remote records are removed.

Before switching, run the legacy command while the old OS/helper is still active:

```sh
sysroot update status
```

Require no pending intent, operation awaiting reboot, staged deployment/replacement or queued rollback. Complete and reconcile any earlier operation first; do not replace it with the bridge image. Preserve the existing rollback hold, high-water and signed records. The v2 import refuses unreconciled legacy operations, and after booting v2 the ordinary status path no longer repairs a v1 journal.

Then obtain and independently review the exact digest of a signed image containing the new GHCR updater. From the reconciled old installed OS, use the existing strict bootc policy:

```sh
sudo bootc switch --enforce-container-sigpolicy ghcr.io/reidond/kedra-desktop@sha256:REVIEWED_DIGEST
```

Reboot when ready. Confirm the running digest before migrating an existing legacy enrollment:

```sh
sysroot update enroll --migrate-legacy --expected-digest sha256:REVIEWED_DIGEST
```

This is an explicit persistent-state migration. Retain the old records: the new helper validates legacy state and preserves its rollback hold, ordering and history, while older readers refuse the newer protocol. If the installation was never enrolled, use ordinary `sysroot update enroll`. Do not weaken signature policy, clear the hold or delete old state to bypass a refusal.

## Midnight package checks

Actions starts package reconciliation at **00:00 UTC**, with manual dispatch available. It resolves the official Fedora 44 base and the complete native RPM closure, including inherited dependencies, and records image-affecting source, external inputs and recipes.

Changed inputs produce a new image that must match preflight and receive manual protected OCI-signing review. Only the exact verified signed digest advances `stable`. Unchanged inputs cause no publication and no renewal. A failed repository, solver or signature check is an error, not a successful no-change result.

The producer and helper share `build/release/compatibility.json`, currently qualifying bootc 1.16.10. An unsupported bootc RPM change fails the public build before manual signing or stable publication. Updating that contract and helper compatibility requires deliberate review and native qualification; the workflow never silently accepts an untested bootc version.

Schedules can queue, skip or be disabled by repository inactivity. No automatic machine staging/reboot or background AI service is implied.

## Recovery

Use Ctrl+Alt+F2 for a text login if the desktop is unavailable. Inspect `bootc status` and `sysroot update status`; retain local installer media and encryption recovery information. OS rollback does not rewind home, /var, credentials or application databases. [Status](STATUS.md) records which migration/recovery paths have actually been exercised.

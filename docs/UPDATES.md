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

## Midnight package checks

Actions starts package reconciliation at **00:00 UTC**, with manual dispatch available. It resolves the official Fedora 44 base and the complete native RPM closure, including inherited dependencies, and records image-affecting source, external inputs and recipes.

Changed inputs produce a new image that must match preflight, pass isolated automatic OCI signing and verify strictly before its exact digest advances `stable`. There is no human approval or manual signing step. Unchanged inputs cause no publication and no renewal. A failed repository, solver or signature check is an error, not a successful no-change result.

The producer and helper share `build/release/compatibility.json`, currently qualifying bootc 1.16.10. An unsupported bootc RPM change fails the public build before signing or stable publication. Updating that contract and helper compatibility requires deliberate source review and native qualification; the workflow never silently accepts an untested bootc version.

Schedules can queue, skip or be disabled by repository inactivity. No automatic machine staging/reboot or background AI service is implied.

## Recovery

Use Ctrl+Alt+F2 for a text login if the desktop is unavailable. Inspect `bootc status` and `sysroot update status`; retain local installer media and encryption recovery information. OS rollback does not rewind home, /var, credentials or application databases. [Status](STATUS.md) records which installation, update and recovery paths have actually been exercised.

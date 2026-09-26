# Update and recover

Kedra updates use the signed GHCR `stable` image of the installed target: `ghcr.io/reidond/kedra-desktop` for desktop (x86_64) and `ghcr.io/reidond/kedra-utm` for the utm virtual machine (aarch64). The tag locates a candidate; the installed helper verifies signature, repository, target, architecture, image identity and retained ordering before switching to its exact digest. It never follows another target's repository. No GitHub Release files or checkpoint renewal is involved.

## Current-source commands

Run as the ordinary owner; the fixed installed helper requests administrator authentication:

```sh
sysroot update enroll
sysroot update check
sysroot update stage
```

Enroll once. Review an available update before staging it. Staging changes the next boot image but does not reboot.

Checking records verified image ordering and last-check state in the root-owned store; it does not stage an image. It is not proof that upstream Fedora has no newer packages. Add --expected-digest when enrollment or staging must match a digest you independently reviewed.

## Downloads and disk use

Every `enroll`, `check` and `stage` verifies the exact digest again with a policy-enforcing Skopeo copy from the registry, so the signature is checked on every run. The copy's destination is a root-only OCI layout, `/var/lib/sysroot/verified-oci`, which is kept between runs. The first verification downloads the whole image (about 6.3 GB for the current desktop and utm images). Later verifications fetch the manifest, configuration and signature, plus only layers the cache does not hold. A `check` followed by `stage` therefore downloads a new image's changed layers once. `bootc switch` then fetches whatever ostree lacks into its own store. Progress is printed to the terminal.

The helper keeps layers for the booted, staged and rollback images, the journal's recorded operation and the digest being verified. Before each copy it removes every other image, so a failed transfer never leaves stale images in place. After a successful copy it also removes leftover layers of interrupted transfers. A failed or interrupted copy keeps the layers it completed, so a retry resumes rather than starting over. Expect the cache to use about one image's size plus the layers unique to the other kept images, on top of ostree's deployments.

If a verification fails because `/var` is full, free space and retry. The kept images are the minimum a verification needs. If you must reclaim the cache itself, delete it as described below; the retry then downloads the whole image again.

The cache only saves transfers. The helper never reads layers from it, and bootc never deploys from it. Unexpected contents make the helper remove the cache without following links and download again. That includes unknown files, links, wrong ownership or mode, and unparseable metadata. While no `sysroot update` command is running, root can safely delete `/var/lib/sysroot/verified-oci`; the next verification downloads the whole image. Do not delete anything else in `/var/lib/sysroot`.

After your chosen reboot:

```sh
sysroot update status --home
sysroot doctor
```

Home baseline acceptance remains explicit through the [Noctalia](HOME-REVIEW.md) and [niri](TEXT-REVIEW.md) workflows. It does not discard local changes automatically.

Use `sysroot update rollback` to select the retained verified deployment. Rollback preserves persistent data and places forward updates on hold. A later reviewed `sysroot update stage --resume` clears that hold after normal verification. Never remove journals or trust state to bypass a refusal.

## Midnight package checks

Actions starts package reconciliation at **00:00 UTC**, with manual dispatch available. For each enabled target, on its native runner, it resolves the official Fedora 44 base for that architecture and the complete native RPM closure, including inherited dependencies, and records image-affecting source, external inputs and recipes. The targets are independent: one target failing does not stop the other from publishing.

Changed inputs produce a new image that must match preflight, pass isolated automatic OCI signing with that target's own key and verify strictly before its exact digest advances that target's `stable`. There is no human approval or manual signing step. Unchanged inputs cause no publication and no renewal. A failed repository, solver or signature check is an error, not a successful no-change result.

The producer and helper share `build/release/compatibility.json`, currently qualifying bootc 1.16.13. An unsupported bootc RPM change fails the public build before signing or stable publication. Updating that contract and helper compatibility requires deliberate source review and native qualification; the workflow never silently accepts an untested bootc version.

Schedules can queue, skip or be disabled by repository inactivity. No automatic machine staging/reboot or background AI service is implied.

## Recovery

Use Ctrl+Alt+F2 for a text login if the desktop is unavailable. Inspect `bootc status` and `sysroot update status`; retain local installer media and encryption recovery information. OS rollback does not rewind home, /var, credentials or application databases. [Status](STATUS.md) records which installation, update and recovery paths have actually been exercised.

# Update and recover

Kedra updates come from the signed GHCR `stable` image of the installed target: `ghcr.io/reidond/kedra-desktop` for desktop (x86_64) and `ghcr.io/reidond/kedra-qemu-arm64` for the qemu-arm64 virtual machine (aarch64). The tag locates a candidate. The installed helper checks its target, platform, identity and order, then bootc verifies its signature while switching to its exact digest. A system never follows another target's repository, and no GitHub Release files, enrollment or checkpoint renewal are involved: trust comes from the booted image itself.

## Current-source commands

Run these as the ordinary owner; the installed helper asks for administrator authentication:

```sh
sysroot update check
sysroot update stage
```

Review an available update before staging it. Staging changes the image for the next boot but does not reboot; bootc's automatic update timer is masked.

`check` reads the identity label of the image `stable` names (manifest and configuration only, bound to its digest) and refuses one from another target, platform or channel, or one older than the booted image. It does not verify the signature and changes nothing. It does not prove that Fedora has no newer packages. `stage` repeats those checks, then runs `bootc switch --enforce-container-sigpolicy` with the exact digest: bootc verifies the signature against the installed policy before it downloads the configuration or any layer, and fetches only the layers it does not already have. Add `--expected-digest DIGEST` when staging must match a digest you reviewed independently, and `--replace-staged DIGEST` to replace an image that is already staged.

## How an update is verified

- **Signature and repository.** `/etc/containers/policy.json` rejects everything except the installed target's repository, which requires a Sigstore signature from `/usr/lib/sysroot/trust/release.pub` with that exact repository as its identity. The helper refuses to run unless that file equals the policy the booted image's trust implies. It runs bootc and Skopeo with a cleared environment and `HOME=/var/lib/sysroot`, and refuses if `/var/lib/sysroot/.config` exists, because containers/image would otherwise prefer a per-user `~/.config/containers/policy.json`.
- **Order.** bootc deploys any validly signed digest, including an older one. Before staging, the helper requires the candidate's rank (epoch, run number, attempt) to be above the booted image's, or the same rank with the same digest, and its package resolution time not to be earlier. The booted identity is `/usr/share/sysroot/image-identity.json`, bound to the booted source and resolved inputs. A registry can still withhold newer images; `sysroot update status` warns when the signed package resolution is older than 36 hours.
- **State.** The only root state is the rollback hold, in `/var/lib/sysroot/deployment`. A missing store means no rollback was requested; an unknown or corrupt one is refused, never reset.

The installed bootc must be at least the version in `usr/src/kedra/image/release/compatibility.json` (currently 1.16.13). The helper parses bootc's v1 status strictly, so an incompatible change fails closed.

After the reboot you choose:

```sh
sysroot update status --home
sysroot doctor
```

Accepting new home defaults stays explicit: run `sysroot home update` after the new image boots ([managed home files](HOME.md)). It merges baseline changes into your edits and never discards them.

`sysroot update rollback` selects the retained deployment, which bootc verified when it deployed it. Rollback preserves persistent data and places forward updates on hold before bootc changes the boot order. A later reviewed `sysroot update stage --resume` clears that hold once the new stage succeeds. Never remove the hold or other state to get past a refusal.

The signed update flow (A to B, rollback with hold and resume, refusal of unsigned, wrong-key, out-of-scope and older images) passed locally in a disposable aarch64 VM on the owner's Mac; the x86_64 VM run is not-run. [STATUS](STATUS.md) has the details.

## Midnight package checks

The release workflow runs at **00:00 UTC**, on pushes to `main` that change image inputs, and by manual dispatch ([RELEASES](RELEASES.md#release-workflow)). For each enabled target, on its native runner, it resolves the official Fedora 44 base for that architecture and then, once, the complete RPM closure into an RPM lock. Every build in the run installs offline from that lock's fixed-output store objects, so a Fedora `updates` push during the run cannot change the image ([package delivery](PACKAGES.md#fedora-rpm-lock)). The targets are independent: one failing does not stop the other from publishing.

Changed source, base or lock pins produce a new image whose installed RPMs must equal the lock. It is signed automatically with that target's own key and verified strictly before its exact digest moves that target's `stable`. The image carries its lock as `/usr/share/sysroot/rpm-lock.json`, bound by the signed resolved inputs. There is no human approval or manual signing step. Unchanged inputs publish nothing and renew nothing. A failed repository, solver or signature check is an error, not a successful no-change result.

The release tooling and the helper share the minimum bootc version in `compatibility.json`. A bootc RPM older than that in the lock fails the build before signing; newer versions pass, and the helper's strict status parsing and the update VM test catch an incompatible change.

Schedules can queue, skip or be disabled by repository inactivity. No machine stages or reboots automatically, and no background AI service runs.

## Recovery

Use Ctrl+Alt+F2 for a text login if the desktop is unavailable. Inspect `bootc status` and `sysroot update status`; keep local installer media and the disk encryption passphrase. OS rollback does not rewind home, `/var`, credentials or application databases. [STATUS](STATUS.md) records which installation, update and recovery paths have actually been exercised.

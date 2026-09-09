# Public-channel enrollment of the installed owner release

Recorded 2026-09-09 (Europe/Kiev), Codex. **Pass for first-time enrollment,
repeat-enrollment refusal, unchanged status and installed-session health.**

This is an actual installed-system continuation of the
[qualified owner ISO](../../R02-installer/owner-34255228394/REPORT.md), after
[release 1 publication](../owner-promotion-34288691672.md). It uses the same
generated encrypted disk and unselected sentinel disk. The ISO was absent;
QEMU user networking supplied Internet access without host port forwarding.
No host disk or workstation enrollment was involved.

## Exact identity

- Accepted installed source: `c660c58d9bbbbe34119f6ea35a03528485455848`.
- Booted image:
  `ghcr.io/reidond/kedra-desktop@sha256:bb4f2b68996a4fc260972d9654440f62ee078bcf92a0996a8f7e34ea0133118b`.
- Public channel downloaded by the guest from
  [desktop-44-x86_64-channel](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-channel/channel.json):
  1,744 bytes, SHA-256
  `d46571c912d881f96b5c6da266ae923ad18aa0ccaeb932d70aa6e2a50edf797a`.
- Independently confirmed installed public-key SPKI DER fingerprint:
  `a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e`.
- Release sequence 1, release SHA-256
  `03678d846d8ed88c0ea08dfab4ce302240a9a0ab54d58aba990bbf3cc7f017fb`;
  checkpoint generation 1, checkpoint SHA-256
  `e97d4c85569f7555e1579c982c104de9022e17846c6878ddde80df7d74893365`.

## Actual workflow and observations

The generated owner unlocked LUKS, authenticated into the installed niri/Noctalia
session and authenticated ordinary `sudo -v`. From a new private download
directory, `curl -fL --max-time 60` fetched the public channel anonymously.
`sha256sum` matched the published bytes. Installed `sysroot release unpack`
used `/usr/lib/sysroot/trust/release.pub`, the independent fingerprint, target
`desktop` and repository `ghcr.io/reidond/kedra-desktop`. It verified the fresh
signed pair and wrote a new `verified-channel` directory; no previous caller
trust state existed for this first unpack.

The actual installed command then ran:

```sh
sysroot update enroll --manifest verified-channel/release.json --signature verified-channel/release.sig --checkpoint verified-channel/checkpoint.json --checkpoint-signature verified-channel/checkpoint.sig
```

The root helper returned `enrolled: true`, exact booted identity, no staged or
rollback deployment, checkpoint generation/release sequence 1, no rollback hold
and no operation in progress. Reboot and home activation remained false.
`sysroot update status` returned the same state. `sysroot doctor --json` passed
all required session checks: enforcing SELinux, no failed system/user units,
valid niri/Noctalia configuration, unlocked login keyring, responsive portal
and active compositor, desktop controls, audio and portal services.

Repeating the same enrollment returned a nonzero result with the native
`File exists (os error 17)` refusal. Another `sysroot update status` was exactly
equal to the original enrollment/status observations. No existing enrollment
or high-water record was overwritten. The generic refusal wording is retained
in the evidence rather than relabeled as a friendlier implemented diagnostic.

Finally, ordinary `sudo systemctl poweroff` shut down the guest and QEMU exited
0. No QEMU process remained. Native `qemu-img compare` after shutdown returned
0 and `Images are identical.` for the generated unselected disk versus its
pre-installation copy.

| Case | Result | Evidence |
|---|---|---|
| Anonymous public download and exact byte identity | pass | [download.png](download.png) |
| Installed-key signature, scope and freshness verification | pass | [unpack.txt](unpack.txt) |
| Ordinary-user enrollment through installed privileged helper | pass | [enrolled.json](enrolled.json), [enrollment.png](enrollment.png) |
| Post-enrollment state and required desktop health | pass | [status.json](status.json), [doctor.json](doctor.json) |
| Repeat enrollment refuses and preserves state | pass | [repeat-refusal.txt](repeat-refusal.txt), [repeat.png](repeat.png), [status-after-refusal.json](status-after-refusal.json) |
| Clean guest shutdown and preserved unselected disk | pass | QEMU exit 0 observed; [disk-comparison.json](disk-comparison.json) |

Native JSON/text were preserved from the generated guest's serial output.
Screenshots contain only this generated account and public release identities.
Passwords, private VM paths and raw boot logs are excluded. Local complete
continuation evidence is `output/owner-installer-34255228394-1/enrollment-evidence/`.

## Limits and next step

The installed source is release 1; it does not include the development
`update status --home` flag or expanded v2 artifact inventory. Home reconciliation
is explicitly unchecked in the root response. No forward update, rollback,
checkpoint renewal, expired-channel recovery or key rotation was performed in
this continuation. Their separate research results and unresolved gates remain
unchanged. The next production check is an exact accepted-source v2 candidate,
followed by native v1-to-v2 staging, boot and installed status/health verification.

## Post-enrollment reboot persistence — 2026-09-09

A separate boot of the retained encrypted disks passes enrollment persistence.
The ISO remained absent and user NAT had no host forwarding. After ordinary
LUKS unlock, remembered-owner login and `sudo -v`, installed `sysroot update status`
returned JSON exactly equal to the saved first-enrollment state: enrolled true,
the same `bb4f2b68` digest, generation/sequence 1 and their original signed hashes,
no pending operation, no staged image and no rollback hold. No re-enrollment was
performed. Native `sysroot doctor --json` passed every required session check at
source `c660c58`.

Ordinary `sudo systemctl poweroff` completed with QEMU exit 0. The stopped
unselected generated disk again compared identical to its pre-installation copy.
Evidence: [status JSON](persistence/status.json), [status screenshot](persistence/status.png),
[doctor JSON](persistence/doctor.json), [doctor screenshot](persistence/doctor.png)
and [disk comparison](persistence/disk-comparison.json). Original enrollment logs
are preserved separately; private credentials and full boot logs are excluded.
All local VMs are stopped. The new accepted `0eb1cf0` candidate's installer is
still building, so a production forward update remains not-run.

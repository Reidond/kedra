# Install and understand Kedra

The first owner installer is published as
[desktop-44-x86_64-r1](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r1).
Its signed image and exact ISO passed fresh encrypted two-disk installation,
ISO-free desktop boot, native health/provenance checks and preservation of the
unselected disk. See the [installation qualification](research/R02-installer/owner-34255228394/REPORT.md)
and [verified publication recovery](research/R08-release-protocol/owner-promotion-34288691672.md).

Kedra is Fedora 44 bootc with niri, Noctalia and the `sysroot` management command.
System software comes from a signed image built by Actions. Your home files remain
writable. Supported home review selects what should enter source while retaining
unselected and explicit local changes. `sysroot codex` launches the bundled private
Codex runtime; personal `codex` installations remain independent. Claude packaging
awaits the owner's separate terms decision. Accounts, model subscriptions and
Bitwarden login are configured by their owner after installation.

## Obtain verified media

Open [release r1](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r1),
for **desktop / Fedora 44 / x86_64**. Download `release.json`, `release.sig`,
`release.pub` and both numbered ISO parts into one new directory:

1. [kedra-desktop-44-34255228394-1.iso.part00](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-r1/kedra-desktop-44-34255228394-1.iso.part00)
2. [kedra-desktop-44-34255228394-1.iso.part01](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-r1/kedra-desktop-44-34255228394-1.iso.part01)

The release also contains the qualification, source identity and signed channel
records. Its complete ISO is 2,856,306,688 bytes and has SHA-256
`9c1401489d1c47119249ab213c9a187b47db6c5112ebccce567cf0cc4a76d988`.
The signed `release.json` authenticates that complete image. Separate signed
SHA256SUMS, packages.txt and provenance.json are prepared for a later v2 candidate;
they are not assets of r1.

Confirm the public authority independently. The owner-authorized public-key SPKI
DER SHA-256 provisioned on 2026-09-08 is:

```text
a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
```

Use a trusted build of sysroot. From the download directory, check the key and
compare the printed fingerprint to the independently confirmed value above:

```sh
sysroot release key --public-key release.pub
sysroot release assemble --manifest release.json --signature release.sig --public-key release.pub --target desktop --output-dir . kedra-desktop-44-34255228394-1.iso.part00 kedra-desktop-44-34255228394-1.iso.part01
```

The result is `kedra-desktop-44-34255228394-1.iso`. The command verifies the signed
record and complete ISO size/hash before exposing it, and refuses an existing
output. Keep the two source parts until verification succeeds.

If sysroot is not installed, its preinstallation verifier currently requires a
source build with Rust 1.98.1; r1 does not distribute a standalone verifier binary.
In a new checkout of the published tag:

```sh
git clone --branch desktop-44-x86_64-r1 --depth 1 https://github.com/Reidond/kedra.git kedra-r1-source
cd kedra-r1-source
git rev-parse HEAD
```

Check that HEAD is `c660c58d9bbbbe34119f6ea35a03528485455848`, then build:

```sh
cargo +1.98.1 build --workspace --release --locked
```

Use the resulting `target/release/sysroot` on Linux or `target\release\sysroot.exe`
on Windows, substituting its absolute path for `sysroot` in the commands above.
In PowerShell, prefix a quoted executable path with `&`. The single-line assembly
command also works there. See
[release verification](RELEASES.md) for complete semantics and limitations.

## Install deliberately

The qualified VM setup uses UEFI, 4 virtual CPUs, 8 GiB RAM and two generated
64 GiB disks. Secure Boot and physical-machine hardware are separate unqualified
gates. Do not infer support for an unspecified laptop from the VM result.

1. Attach the verified ISO to the intended disposable VM, or write it to your
   deliberately selected installation USB using a trusted image-writing tool.
   Writing the USB replaces its existing contents.
2. Boot the installer. In Installation Destination, select only the intended
   system disk. Check its identity and capacity; leave data disks unselected.
3. Enable encryption and choose a passphrase you can recover independently.
   Create your owner account and give it administrative privileges. There are
   no supplied default passwords.
4. Review the selected disk and pending installation before beginning. The
   installer copies the embedded signed payload after offline verification.
5. After installation completes, shut down, remove/detach the ISO or USB and
   boot the installed disk. Unlock encryption and log in as the owner.

The exact installation cases include preserving an unselected disk, reaching the
desktop without the ISO, enforcing SELinux, writable home, read-only system root,
healthy services and the exact signed booted digest. `sysroot doctor --json` checks
the installed session; `sysroot update status` observes deployment/enrollment.
Save unexpected errors and exact image identity instead of changing signature
policy or boot arguments to force a result.

## Enroll and update

Inside the installed Kedra session, download the
[current channel.json](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-channel/channel.json)
into a new working directory. Enrollment requires the exact running promoted
release and a fresh equal-or-newer channel. Use the installed public key to unpack
the bundle, then enroll an installation that is still current:

```sh
sysroot release unpack --bundle channel.json --public-key /usr/lib/sysroot/trust/release.pub --expected-fingerprint a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e --target desktop --repository ghcr.io/reidond/kedra-desktop --output-dir verified-channel
sysroot update enroll --manifest verified-channel/release.json --signature verified-channel/release.sig --checkpoint verified-channel/checkpoint.json --checkpoint-signature verified-channel/checkpoint.sig
sysroot update status
```

When the current channel is newer than the installed ISO, also provide that ISO's
signed record using `--installed-manifest` and `--installed-signature`. The helper
checks installed provenance and public trust independently; downloading files
does not establish machine authorization.

The initially published checkpoint expires on **2026-09-15 at 23:05:17 UTC**.
Always fetch the current channel for online operations. An expired channel is
refused; retain the verified ISO and wait for a valid fresh checkpoint rather
than changing the clock or verification policy. No-change renewal is not yet
implemented. First enrollment against the anonymous public channel passed in the
retained r1 VM: the helper reported enrolled, the exact booted owner digest and
sequence/generation 1 without staging or rebooting. Status and doctor passed;
repeating enrollment was refused with the original state preserved. Clean
shutdown and a final comparison confirmed that the unselected disk was unchanged.
A later no-ISO reboot also retained exactly the enrolled image/order state and
passed desktop health and stopped-disk preservation without re-enrollment.
See the separate
[enrollment evidence](research/R08-release-protocol/owner-r1-enrollment/REPORT.md).

After reviewing a newer release, explicitly stage it with the same four channel
file arguments using `sysroot update stage`. Staging does not reboot. Reboot when
you choose, then inspect status and doctor. Keep the preceding signed release
record for `sysroot update rollback --manifest OLD_RELEASE.json --signature OLD_RELEASE.sig`.
Rollback queues the retained verified image and places forward updates on hold;
`--resume` on a later explicit stage clears that hold. Do not delete replay state
to bypass a refusal. Offline current boot and retained rollback are separate from
fresh online metadata requirements.

## Day-to-day changes

Keep the Kedra checkout separate from private account data. The owner can edit
source directly or use `sysroot codex` in that checkout. All OS/ISO builds happen
in Actions; publishing a candidate does not install it on the workstation.

Use `sysroot home --help` for the supported Noctalia and niri review, selection,
publication and recovery commands. Real files stay writable; applying a new
baseline requires a current plan and explicit conflict resolution. Wider home
groups are not implemented. Repository skills remain checkout-local and personal
agent settings, MCP, skills and credentials remain independent.

The optional `sysroot update status --home` assessment was qualified on the later
development source `8288cc2`; it is not included in r1's `c660c58` image. Use r1's
ordinary `sysroot update status` and the explicit `sysroot home` commands.

The owner confirmed Bitwarden release-key backup and retrieval on 2026-09-08.
Personal authentication, physical hardware, expired channel recovery, no-change
refresh and signing-key rotation still require their own completed steps/evidence.
Current status is in [worklog.md](../worklog.md) and
the [research reports](research/status.json).

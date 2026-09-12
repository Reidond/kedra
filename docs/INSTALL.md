# Install and understand Kedra

The first owner installer is published as
[desktop-44-x86_64-r1](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r1).
Its signed image and exact ISO passed fresh encrypted two-disk installation,
ISO-free desktop boot, native health/provenance checks and preservation of the
unselected disk. See [verified status](STATUS.md).

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

On a current-source installation, use the fixed signed target channel:

```sh
sysroot update check
sysroot update enroll --channel
sysroot update stage --channel
```

Enroll once. Stage only after reviewing the available update; staging selects the next boot without rebooting. The helper independently checks fixed installed trust, scope, signatures, freshness and replay state. See [UPDATES.md](UPDATES.md) for rollback, holds and home reconciliation.

### Published r1 compatibility

R1 does not have the new --channel convenience options or update check. Download [channel.json](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-channel/channel.json) into a new working directory and use:

```sh
sysroot release unpack --bundle channel.json --public-key /usr/lib/sysroot/trust/release.pub --expected-fingerprint a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e --target desktop --repository ghcr.io/reidond/kedra-desktop --output-dir verified-channel
sysroot update enroll --manifest verified-channel/release.json --signature verified-channel/release.sig --checkpoint verified-channel/checkpoint.json --checkpoint-signature verified-channel/checkpoint.sig
sysroot update status
```

When the channel is newer than the installed ISO, also provide the ISO's signed record using --installed-manifest and --installed-signature together. Current source supports the equivalent convenience command:

```sh
sysroot update enroll --channel --installed-manifest OLD_RELEASE.json --installed-signature OLD_RELEASE.sig
```

Keep those files with recovery media.

For a reviewed newer release, run the same four file arguments with `sysroot update stage` instead of enroll. Reboot when ready, then run status and doctor. R1 uses ordinary `sysroot update status`; current source also supports --home.

The initial r1 checkpoint expires **2026-09-15 23:05:17 UTC**. Fetch the current channel for online operations. Expired metadata is refused; do not alter the clock or verification policy. Retained local boot and explicit rollback are separate from fresh online eligibility.

## Daily use

Use `sysroot home --help` for explicit Noctalia and niri review, source publication, baseline acceptance and recovery. See [Noctalia](HOME-REVIEW.md) and [niri](TEXT-REVIEW.md). Wider arbitrary home groups are not implemented. Personal agents, MCP, skills, profiles and credentials remain independently owned.

Source changes and midnight package refresh build in Actions. They never install on this workstation automatically. [Verified status](STATUS.md) lists remaining hardware, lifecycle and authentication qualification.

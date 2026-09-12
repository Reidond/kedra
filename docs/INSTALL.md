# Install and understand Kedra

The current installer is published as
[desktop-44-x86_64-r2](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r2).
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

Open [release r2](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r2),
for **desktop / Fedora 44 / x86_64**. Download
[release.json](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-r2/release.json),
[release.sig](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-r2/release.sig),
[release.pub](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-r2/release.pub)
and both numbered ISO parts into one new directory:

1. [kedra-desktop-44-34293133114-1.iso.part00](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-r2/kedra-desktop-44-34293133114-1.iso.part00)
2. [kedra-desktop-44-34293133114-1.iso.part01](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-r2/kedra-desktop-44-34293133114-1.iso.part01)

The release also contains the qualification, source identity and signed channel
records. Its complete ISO is 2,856,314,880 bytes and has SHA-256
`15ddfafb8567297220e02831b6639f0fceb7efce0cd47bcf799fae97c5a1eb4b`.
The signed `release.json` authenticates that complete image. R2 also includes
signed SHA256SUMS, packages.txt, provenance.json and qualification records; all
17 assets were verified during exact-byte publication recovery. Part00 is
1,992,294,400 bytes, SHA-256 `d9c3ad32d1aad1eb753f3e3d818b646b7a04928025cdd6adb387a727b8086424`;
part01 is 864,020,480 bytes, SHA-256 `1623e9b1c78fc0fe3ec3a4ce8953e8769ea29f17044bad3272b181f1e84ebb77`.

Confirm the public authority independently. The owner-authorized public-key SPKI
DER SHA-256 provisioned on 2026-09-08 is:

```text
a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
```

Use a trusted build of sysroot. From the download directory, check the key and
compare the printed fingerprint to the independently confirmed value above:

```sh
sysroot release key --public-key release.pub
sysroot release assemble --manifest release.json --signature release.sig --public-key release.pub --target desktop --output-dir . kedra-desktop-44-34293133114-1.iso.part00 kedra-desktop-44-34293133114-1.iso.part01
```

The result is `kedra-desktop-44-34293133114-1.iso`. The command verifies the signed
record and complete ISO size/hash before exposing it, and refuses an existing
output. Keep the two source parts until verification succeeds.

If sysroot is not installed, its preinstallation verifier currently requires a
source build with Rust 1.98.1; r2 does not distribute a standalone verifier binary.
In a new checkout of the published tag:

```sh
git clone --branch desktop-44-x86_64-r2 --depth 1 https://github.com/Reidond/kedra.git kedra-r2-source
cd kedra-r2-source
git rev-parse HEAD
```

Check that HEAD is `0eb1cf09c0eab5f4488a780552f51582d3e92bdf`, then build:

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

Published r2 uses explicit signed-file enrollment and staging. It does not include
`update check` or the new `--channel` convenience options. Download [channel.json](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-channel/channel.json) into a new working directory and use:

```sh
sysroot release unpack --bundle channel.json --public-key /usr/lib/sysroot/trust/release.pub --expected-fingerprint a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e --target desktop --repository ghcr.io/reidond/kedra-desktop --output-dir verified-channel
sysroot update enroll --manifest verified-channel/release.json --signature verified-channel/release.sig --checkpoint verified-channel/checkpoint.json --checkpoint-signature verified-channel/checkpoint.sig
sysroot update status
```

When the channel is newer than the installed ISO, also provide the ISO's signed record using --installed-manifest and --installed-signature together. The current source branch (not published r2) supports the equivalent convenience command:

```sh
sysroot update enroll --channel --installed-manifest OLD_RELEASE.json --installed-signature OLD_RELEASE.sig
```

Keep those files with recovery media.

For a reviewed newer release, download and unpack its current channel into a new directory, then stage it:

```sh
sysroot update stage --manifest verified-channel/release.json --signature verified-channel/release.sig --checkpoint verified-channel/checkpoint.json --checkpoint-signature verified-channel/checkpoint.sig
```

Reboot when ready, then run status and doctor. R2 also supports `sysroot update status --home` for explicit home assessment.

The recovered r2 checkpoint expires **2026-09-16 07:50:54 UTC**. Fetch the current channel for online operations. Expired metadata is refused; do not alter the clock or verification policy. Retained local boot and explicit rollback are separate from fresh online eligibility.

## Daily use

Use `sysroot home --help` for explicit Noctalia and niri review, source publication, baseline acceptance and recovery. See [Noctalia](HOME-REVIEW.md) and [niri](TEXT-REVIEW.md). Wider arbitrary home groups are not implemented. Personal agents, MCP, skills, profiles and credentials remain independently owned.

Source changes and midnight package refresh build in Actions. They never install on this workstation automatically. [Verified status](STATUS.md) lists remaining hardware, lifecycle and authentication qualification.

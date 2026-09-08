# Install and understand Kedra

Owner media is not published yet. The ISO containing the owner-signed image from
run 34255228394 passed fresh encrypted two-disk installation, ISO-free desktop
boot, native health/provenance checks and preservation of the unselected disk.
The [qualification report](research/R02-installer/owner-34255228394/REPORT.md)
contains its exact identity and evidence. Release metadata signing and promotion
are the next boundary before this guide points to a published version.

Kedra is Fedora 44 bootc with niri, Noctalia and the `sysroot` management command.
System software comes from a signed image built by Actions. Your home files remain
writable. Supported home review selects what should enter source while retaining
unselected and explicit local changes. `sysroot codex` launches the bundled private
Codex runtime; personal `codex` installations remain independent. Claude packaging
awaits the owner's separate terms decision. Accounts, model subscriptions and
Bitwarden login are configured by their owner after installation.

## Obtain verified media

Use the versioned release for **desktop / Fedora 44 / x86_64**, not an arbitrary
research artifact, OCI candidate tag or a different target. Each promoted version
will contain numbered ISO parts, release.json, release.sig, the public key,
qualification evidence and exact source identity. Download every ISO part.

Confirm the public authority independently. The owner-authorized public-key SPKI
DER SHA-256 provisioned on 2026-09-08 is:

```text
a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
```

Use a trusted build of sysroot to check the key and reconstruct the exact ISO:

```sh
sysroot release key --public-key release.pub
sysroot release assemble --manifest release.json --signature release.sig --public-key release.pub --target desktop --output-dir . EXACT_NAME.iso.part00 EXACT_NAME.iso.part01
```

Use the actual ordered names from the release. The command verifies the signed
record and complete ISO size/hash before exposing the final ISO, and refuses an
existing output. On the development Windows checkout the built command is
`target\release\sysroot.exe`. To build it from independently reviewed source with
the pinned toolchain, run `cargo +1.98.1 build --workspace --release --locked`.
The source build requires Rust; no preinstallation binary distribution is claimed
yet. See [release verification](RELEASES.md) for complete semantics and limitations.

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

Download the target channel bundle and unpack it as described in
[RELEASES.md](RELEASES.md). Enrollment requires the exact running promoted release
and a fresh equal-or-newer channel. For an installation that is still current:

```sh
sysroot update enroll --manifest verified-channel/release.json --signature verified-channel/release.sig --checkpoint verified-channel/checkpoint.json --checkpoint-signature verified-channel/checkpoint.sig
sysroot update status
```

When the current channel is newer than the installed ISO, also provide that ISO's
signed record using `--installed-manifest` and `--installed-signature`. The helper
checks installed provenance and public trust independently; downloading files
does not establish machine authorization.

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

The owner confirmed Bitwarden release-key backup and retrieval on 2026-09-08.
Personal authentication, physical hardware, expired channel recovery, no-change
refresh and signing-key rotation still require their own completed steps/evidence.
Current status is in [worklog.md](../worklog.md) and
the [research reports](research/status.json).

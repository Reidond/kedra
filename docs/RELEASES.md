# Verify a Kedra release

The published owner release is
[desktop-44-x86_64-r1](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r1),
source `c660c58d9bbbbe34119f6ea35a03528485455848`. Its exact ISO passed installation
qualification; every published asset was downloaded and checked during
[publication recovery](research/R08-release-protocol/owner-promotion-34288691672.md).

Download `release.json`, `release.sig`, `release.pub` and both ISO parts from r1.
The complete installer is `kedra-desktop-44-34255228394-1.iso`, 2,856,306,688 bytes,
SHA-256 `9c1401489d1c47119249ab213c9a187b47db6c5112ebccce567cf0cc4a76d988`.
Obtain the public key through a trusted project channel and independently confirm
its SPKI DER fingerprint:

```text
a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
```

```sh
sysroot release key --public-key release.pub
```

This validates the public-key format and prints its SHA-256 fingerprint. It does
not establish who owns the key or trust a download by itself. Keep the independently
confirmed fingerprint with your recovery instructions.

```sh
sysroot release verify --manifest release.json --signature release.sig --public-key release.pub --target desktop --artifact kedra-desktop-44-34255228394-1.iso
```

Add `--json` for structured results. Success verifies the exact signed manifest
and the installer's size/checksum. A modified installer, wrong key, changed
manifest, wrong target or unpromoted candidate fails with a nonzero exit code.
The command does not modify disks or authorize a system update.

R1 supplies two numbered download parts. Download both, then reconstruct the ISO
with `sysroot release assemble`:

```sh
sysroot release assemble --manifest release.json --signature release.sig --public-key release.pub --target desktop --output-dir . kedra-desktop-44-34255228394-1.iso.part00 kedra-desktop-44-34255228394-1.iso.part01
```

The same command works in PowerShell. It verifies the signed release before
creating temporary output, streams the parts without loading the ISO into memory,
and checks the complete signed size and SHA-256. Only then does it create the
final filename from the signed manifest. It never replaces an existing ISO.
Missing, reordered, extra or damaged parts fail; source parts stay unchanged.
Use an output filesystem with hard-link support, such as NTFS, ext4 or Btrfs,
and enough free space for the assembled ISO. A forcefully interrupted operation
can leave a `.kedra-assemble-*` directory containing `installer.partial`; that
partial file is not verified media. Retry in the same directory after ensuring
space is available. No wildcard sorting or shell concatenation is required.

R1 authenticates the whole ISO through `release.json` and `release.sig`. Its 13
assets also include candidate/source/qualification records and signed checkpoint
files. It does not contain the separate `SHA256SUMS`/`SHA256SUMS.sig`, packages.txt
or provenance.json prepared for candidate schema 2. That expanded publisher still
requires a later accepted candidate and native qualification.

Normal updates also need a fresh signed channel checkpoint and the machine's
independent enrollment/replay state. Download the current
[`channel.json`](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-channel/channel.json)
from the target-specific
[desktop-44-x86_64-channel release](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-channel).
After downloading it, verify and unpack it into a **new** directory:

```sh
sysroot release unpack --bundle channel.json --public-key release.pub --expected-fingerprint a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e --target desktop --repository ghcr.io/reidond/kedra-desktop --output-dir verified-channel
```

This checks the independently expected public-key fingerprint, both signatures,
exact release/checkpoint binding, scope and current freshness before creating any
output. It writes the four original signed files plus `next-trust-state.json`.
Keep that state independently and provide it as `--previous-state` on subsequent
unpack operations to detect replay. Existing directories are refused. A disk/write
failure can leave an incomplete verified directory; inspect it and use a different
new directory on retry. No installed enrollment/state is changed.

The unpacked files can be passed directly to enrollment/staging, which independently
verify them through the root helper. Inspect individual downloaded channel files with:

```sh
sysroot release channel --manifest verified-channel/release.json --signature verified-channel/release.sig --checkpoint verified-channel/checkpoint.json --checkpoint-signature verified-channel/checkpoint.sig --public-key release.pub --target desktop --repository ghcr.io/reidond/kedra-desktop --json
```

This uses the actual system clock, expected repository/target and seven-day maximum
checkpoint lifetime. Add `--previous-state prior-state.json` with independently
retained `next_trust_state` from an earlier successful check to reject a replay.
Without previous state it cannot detect earlier accepted history. It never writes
that state or authorizes a deployment; installed `sysroot update` operations load
their own protected trust and repeat the checks. Caller-supplied public files
cannot replace machine enrollment or its high-water state.

The initial r1 checkpoint was issued at 2026-09-08 23:05:17 UTC and expires at
2026-09-15 23:05:17 UTC. Native anonymous channel verification passed after
publication with release sequence 1 and checkpoint generation 1. Fetch the current
channel again for later online operations; its versioned r1 copy is historical
evidence. Automatic/no-change renewal and expired-channel recovery remain open.

An old retained release can remain valid recovery media even when its checkpoint
has expired or it is no longer current. Installed staging/rollback and persistent
trust state pass the disposable R01 workflow. First enrollment against this
published owner channel also passed in the retained r1 VM; see the separate
[enrollment report](research/R08-release-protocol/owner-r1-enrollment/REPORT.md)
for repeat-enrollment refusal with unchanged state, required desktop health and
clean shutdown/sentinel evidence. Post-enrollment reboot persistence and an owner
forward update were not tested in that continuation.

For installation steps and building a trusted verifier before installing Kedra,
see [INSTALL.md](INSTALL.md).

See [ADR 0006](adr/0006-signed-release-records.md) and
[actual research results](research/R08-release-protocol/REPORT.md).

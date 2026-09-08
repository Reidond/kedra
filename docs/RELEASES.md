# Verify a Kedra release

No production release is promoted yet. These commands are implemented; current
verification evidence uses disposable fixtures, not an installer for your disk.

A published release will provide the ISO, release.json, release.sig and the
signing-key fingerprint. Obtain the public key through a trusted project channel
and compare its fingerprint independently of the download being checked.

```sh
sysroot release key --public-key release.pub
```

This validates the public-key format and prints its SHA-256 fingerprint. It does
not establish who owns the key or trust a download by itself. Keep the independently
confirmed fingerprint with your recovery instructions.

```sh
sysroot release verify --manifest release.json --signature release.sig \
  --public-key release.pub --target desktop --artifact kedra-desktop-44-BUILD.iso
```

Add `--json` for structured results. Success verifies the exact signed manifest
and the installer's size/checksum. A modified installer, wrong key, changed
manifest, wrong target or unpromoted candidate fails with a nonzero exit code.
The command does not modify disks or authorize a system update.

Large installers may be supplied as numbered download parts. Download every part,
then list them in the release's order with `sysroot release assemble`:

```sh
sysroot release assemble --manifest release.json --signature release.sig --public-key release.pub --target desktop --output-dir . kedra.iso.part00 kedra.iso.part01
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

Normal updates also need a fresh signed channel checkpoint and the machine's
independent enrollment/replay state. Inspect downloaded channel files with:

```sh
sysroot release channel --manifest release.json --signature release.sig \
  --checkpoint checkpoint.json --checkpoint-signature checkpoint.sig \
  --public-key release.pub --target desktop --repository ghcr.io/reidond/kedra-desktop --json
```

This uses the actual system clock, expected repository/target and seven-day maximum
checkpoint lifetime. Add `--previous-state prior-state.json` with independently
retained `next_trust_state` from an earlier successful check to reject a replay.
Without previous state it cannot detect earlier accepted history. It never writes
that state or authorizes a deployment; installed `sysroot update` operations load
their own protected trust and repeat the checks. Caller-supplied public files
cannot replace machine enrollment or its high-water state.

An old retained release can remain valid recovery media even when its checkpoint
has expired or it is no longer current. Installed staging/rollback and persistent
trust state pass the disposable R01 workflow; production authority/promotion are
still being prepared.

See [ADR 0006](adr/0006-signed-release-records.md) and
[actual research results](research/R08-release-protocol/REPORT.md).

# Verify a Kedra release

No production release is promoted yet. These commands are implemented; current
verification evidence uses disposable fixtures, not an installer for your disk.

A published release will provide the ISO, release.json, release.sig and the
signing-key fingerprint. Obtain the public key through a trusted project channel
and compare its fingerprint independently of the download being checked.

```sh
sysroot release verify --manifest release.json --signature release.sig \
  --public-key release.pub --target desktop --artifact kedra-desktop-44-BUILD.iso
```

Add `--json` for structured results. Success verifies the exact signed manifest
and the installer's size/checksum. A modified installer, wrong key, changed
manifest, wrong target or unpromoted candidate fails with a nonzero exit code.
The command does not modify disks or authorize a system update.

Normal updates also need a fresh signed channel checkpoint and the machine's
independent enrollment/replay state. An old retained release can remain valid
recovery media even when it is no longer current. Installed staging and trust-state
persistence are still under development.

See [ADR 0006](adr/0006-signed-release-records.md) and
[actual research results](research/R08-release-protocol/REPORT.md).

# Verify releases

Obtain the public key independently and confirm its P-256 SPKI DER SHA-256 fingerprint:

```text
a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
```

```sh
sysroot release key --public-key release.pub
```

A matching fingerprint establishes the key you expected, not trust in an arbitrary download. [INSTALL.md](INSTALL.md) includes the published r2 download links and building the verifier before installation.

## ISO verification

Download the release's signed manifest and numbered ISO parts. For r2:

```sh
sysroot release assemble --manifest release.json --signature release.sig --public-key release.pub --target desktop --output-dir . kedra-desktop-44-34293133114-1.iso.part00 kedra-desktop-44-34293133114-1.iso.part01
sysroot release verify --manifest release.json --signature release.sig --public-key release.pub --target desktop --artifact kedra-desktop-44-34293133114-1.iso
```

Assembly authenticates the exact manifest, streams parts in supplied order, verifies total size/hash and only then exposes the final ISO. It refuses existing output and leaves source parts untouched. Use NTFS, ext4 or Btrfs with enough free space and hard-link support. Interrupted temporary output is not verified media.

R2's complete ISO is 2,856,314,880 bytes with SHA-256 `15ddfafb8567297220e02831b6639f0fceb7efce0cd47bcf799fae97c5a1eb4b`. Its 17 exact assets include package inventory, provenance and signed SHA256SUMS. R1 remains available as historical recovery media with its original v1 assets.

For r2's signed checksums:

```sh
openssl base64 -d -in SHA256SUMS.sig -out SHA256SUMS.sig.der
openssl dgst -sha256 -verify release.pub -signature SHA256SUMS.sig.der SHA256SUMS
sha256sum --check SHA256SUMS
```

The final check expects the assembled ISO as well as downloaded assets. It does not replace target/freshness/replay checks.

## Signed channel

Download the current target's [channel.json](https://github.com/Reidond/kedra/releases/download/desktop-44-x86_64-channel/channel.json). To inspect offline-downloaded bytes:

```sh
sysroot release unpack --bundle channel.json --public-key release.pub --expected-fingerprint a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e --target desktop --repository ghcr.io/reidond/kedra-desktop --output-dir verified-channel
```

This verifies exact release/checkpoint signatures, binding, scope and actual-clock freshness before writing their four original files plus next-trust-state.json. Use a new output directory. Retain previous trust state independently and provide `--previous-state` on subsequent unpack operations to detect replay. Without it, a valid signature cannot reveal previously accepted history.

Installed enrollment/staging independently reload root-owned authority and ordering state. Current-source `sysroot update check` reports current/available, pending slots, enrollment and rollback hold without staging/rebooting or advancing replay floors. Its helper status read may reconcile an existing operation journal. Download/trust/signature/expiry errors fail; they never report successful no-change.

R2's current signed checkpoint has sequence/generation 2 and expires **2026-09-16 07:50:54 UTC**. Fetch the current channel again for later online operations.

## Publisher history and recovery

`sysroot release history` authenticates an old signed release/checkpoint pair, including an expired predecessor, as historical-only ordering information. Its output cannot authorize installation or staging. Ordinary incoming channel verification retains strict freshness; no allow-expired or clock override exists.

A retained signed release can remain valid recovery media after its online checkpoint expires. Preserve its signatures and use explicit retained rollback, not replay-state deletion. See [update operations](UPDATES.md), [publisher operations](../build/release/README.md) and [verified status](STATUS.md).

# OCI signing authority

The dedicated P-256 public key is `desktop.pub`; its SPKI DER SHA-256 is:

```text
a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
```

The owner provisioned and backed up this authority separately from Bitwarden-held SSH keys. Do not regenerate it, export an SSH private key or request a broad unlocked vault session. Only public material belongs in the repository/image/local installer.

The existing `kedra-desktop-signing` GitHub environment requires sole reviewer `Reidond`, main-only deployment and administrator bypass disabled. Signing secrets are environment-scoped:

- `KEDRA_DESKTOP_SIGNING_KEY`: encrypted OCI signing key
- `KEDRA_DESKTOP_SIGNING_PASSPHRASE`: passphrase
- Environment variables `KEDRA_DESKTOP_PUBLIC_KEY` and `KEDRA_DESKTOP_KEY_SHA256`: independently reviewed public values matching the checkout

The repository variable `KEDRA_RELEASES_ENABLED=true` enables image publication only with those protections. An environment name alone is insufficient; do not let auto-created unprotected environments replace the configured boundary.

Manual review approves one exact image/source/run identity. Tools are prepared before private material is exposed, and the isolated signer executes no checkout or candidate code. The subsequent key-free job strictly verifies the signed digest before stable publication. There are no release/checkpoint/ISO signatures.

Public build checks enforce `build/release/compatibility.json`, also embedded by the helper. Unsupported bootc RPM changes stop before manual signing/stable publication. Update the shared contract and helper compatibility deliberately and qualify the new version natively; owner signing approval does not substitute for that compatibility evidence.

Key rotation needs a separately qualified overlap/offline-client/recovery procedure. Preserve known-good signed digests and existing recovery material. Never weaken installed container policy or re-sign uncertain publication merely to clear a failed run.

Sources: [GitHub environment protection](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments), [environment API](https://docs.github.com/en/rest/deployments/environments), [Skopeo signing](https://github.com/containers/skopeo/blob/main/docs/skopeo-copy.1.md).

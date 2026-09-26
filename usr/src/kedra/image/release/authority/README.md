# OCI signing authority

Each enabled target has its own dedicated P-256 key, signing environment and GHCR repository. A key never signs another target's repository: the installed policy uses `exactRepository`, and every check compares target, architecture and repository.

## desktop

The public key is `desktop.pub`; its SPKI DER SHA-256 is:

```text
a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
```

The owner provisioned and backed up this authority separately from Bitwarden-held SSH keys.

## utm

The public key is `utm.pub`; `utm.sha256` records its SPKI DER SHA-256:

```text
76ca7a65915adb1907acbe0885af83c5c569dd2964b87decbfb67059dee366c6
```

That value was read from `utm.sha256` and matches `openssl pkey -pubin -in utm.pub -outform DER | sha256sum` (checked 2026-09-25). The `kedra-utm-signing` environment, its secrets/variables and the public `kedra-utm`/`kedra-utm-builds` packages are pending owner provisioning; see [STATUS](../../../docs/STATUS.md) for whether that has happened. Until then the utm build fails closed at its environment-policy or registry-absence check, and desktop publication is unaffected.

## Rules for every target

Do not regenerate an authority, export an SSH private key or request a broad unlocked vault session. Only public material belongs in the repository/image/local installer.

The `kedra-<target>-signing` GitHub environment must allow only the `main` deployment branch, keep administrator bypass disabled, and have no required reviewers or wait/custom approval gate. Signing is automatic after the public build passes. Signing secrets remain environment-scoped:

| Target | Secrets (environment) | Variables (environment) |
|---|---|---|
| `desktop` | `KEDRA_DESKTOP_SIGNING_KEY`, `KEDRA_DESKTOP_SIGNING_PASSPHRASE` | `KEDRA_DESKTOP_PUBLIC_KEY`, `KEDRA_DESKTOP_KEY_SHA256` |
| `utm` | `KEDRA_UTM_SIGNING_KEY`, `KEDRA_UTM_SIGNING_PASSPHRASE` | `KEDRA_UTM_PUBLIC_KEY`, `KEDRA_UTM_KEY_SHA256` |

The key secret is the encrypted OCI signing key and the passphrase secret its passphrase. The variables are independently reviewed public values that must match the checkout's `<target>.pub`/`<target>.sha256`. Do not store these as repository secrets. `release.yml` uses `secrets: inherit`, which a called job needs to see its environment's secrets (probe run 36191986665); with no repository-level secrets, the reusable signer job still reads only the values of the environment it declares.

The repository variable `KEDRA_RELEASES_ENABLED=true` enables image publication only with those protections. An environment name alone is insufficient; do not let auto-created unprotected environments replace the configured boundary. The build job verifies the target's environment policy through the API before a signer job can reference it.

The workflow validates the exact image/source/run identity before automatic signing. Tools are prepared before private material is exposed, and the isolated signer executes no checkout or candidate code. The subsequent key-free job strictly verifies the signed digest before stable publication. There are no release/checkpoint/ISO signatures or manual signing actions.

Public build checks enforce `build/release/compatibility.json`, also embedded by the helper. Unsupported bootc RPM changes stop before signing/stable publication. Update the shared contract and helper compatibility deliberately and qualify the new version natively; automatic signing does not substitute for that compatibility evidence.

Removing the review gate makes accepted-main workflow code and its validation gates responsible for authorizing signatures. Keep main protected, signing secrets confined to their environment, and all current-source, digest, scope, rank and native signature checks intact. Remove only the reviewer rule when changing an environment; preserve the main deployment rule, public variables and existing secrets. See [STATUS](../../../docs/STATUS.md) for whether that configuration and production execution have actually completed.

Key rotation needs a separately qualified overlap/offline-client/recovery procedure. Preserve known-good signed digests and existing recovery material. Never weaken installed container policy or re-sign uncertain publication merely to clear a failed run.

Sources: [GitHub environment protection](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments), [environment API](https://docs.github.com/en/rest/deployments/environments), [reusable workflows and environment secrets](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows), [Skopeo signing](https://github.com/containers/skopeo/blob/main/docs/skopeo-copy.1.md).

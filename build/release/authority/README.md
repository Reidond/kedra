# Desktop release authority

No production authority is provisioned. This directory deliberately contains no
`desktop.pub` or `desktop.sha256`; a research key must never fill those slots.
The manual release workflow also requires the repository opt-in variable and an
existing protected signing environment before it starts building.

After the owner authorizes a dedicated OS-release key and its recovery location:

1. On an owner-controlled machine, use Skopeo's `generate-sigstore-key` with a
   private passphrase file and a private output directory outside every checkout.
   Retain its encrypted `.private` file and matching `.pub`. Never reuse/export
   a Bitwarden SSH key. Do not put a passphrase on the command line or in logs.
2. Validate the public file with `sysroot release key --public-key PATH --json`.
   Independently retain the reported P-256 SPKI DER SHA-256. Store the encrypted
   private key and passphrase in separate owner-controlled recovery locations;
   qualify recovery with a disposable signed blob before relying on it.
3. Review and commit only the public SPKI PEM as `desktop.pub` and the lowercase
   fingerprint followed by a newline as `desktop.sha256` in this directory.
4. Configure the `kedra-desktop-signing` GitHub environment before adding its
   secrets. Require the `Reidond` user as the sole reviewer; allow the `main`
   branch only, no tags or wildcards; disable administrator bypass. A sole owner
   may approve their own requested deployment, so self-review must be permitted.
   Protect main against force-push/deletion and require successful workspace CI.
5. In that environment only, set `KEDRA_DESKTOP_SIGNING_KEY` to the encrypted
   private-key file and `KEDRA_DESKTOP_SIGNING_PASSPHRASE` to its passphrase using
   GitHub's secret-file/stdin input. Set environment variables
   `KEDRA_DESKTOP_PUBLIC_KEY` and `KEDRA_DESKTOP_KEY_SHA256` to the independently
   confirmed public key and fingerprint. They must match the committed files.
   Do not use repository-wide signing secrets or give an agent a vault session.
6. Review the complete release workflow and merge the qualified source to main.
   Only after the protection and recovery checks pass, set repository variable
   `KEDRA_RELEASES_ENABLED=true`. Dispatch the manual release workflow on main.
   Before approving its signing environment, review that run's candidate digest,
   source manifest, build inputs, package inventory and CI result.

The signer job does not check out source, download a build artifact or execute
an image. It installs Skopeo before receiving private material, rechecks current
main and the independently configured public fingerprint, then signs a fixed
repository/digest copy with digest preservation. The following job must pull
that signature anonymously under the strict public-key policy. A private GHCR
package must be explicitly made public before that check can pass; credentials
are deliberately not supplied as an installer dependency.

This prepares a **candidate**, with no channel update or promoted metadata.
Fresh installation of the exact ISO, release/checkpoint signing, serialized
promotion, scheduled refresh and a rotation drill remain separate qualification
steps. Do not treat the opt-in variable or an environment name as protection:
GitHub can automatically create an unprotected environment from a workflow name.
The build's preflight requires the existing reviewer and main-only branch rule.

Read-only configuration inspection on 2026-09-08 found no environments and an
unprotected main at `c00374cae862c669460da35471950237216a3d14`. No GitHub setting,
secret, key or recovery location was changed during this preparation.

Sources, checked 2026-09-08:
[GitHub environment protection](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments),
[environment API](https://docs.github.com/en/rest/deployments/environments),
[Skopeo signing/copy interface](https://github.com/containers/skopeo/blob/main/docs/skopeo-copy.1.md).

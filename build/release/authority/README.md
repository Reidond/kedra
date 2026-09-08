# Desktop release authority

The owner authorized GitHub deployment secrets and a separate Bitwarden recovery
backup on 2026-09-08. `desktop.pub` and `desktop.sha256` now identify the newly
generated dedicated key:

`a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e`

GitHub environment `kedra-desktop-signing` contains the encrypted key/passphrase
as Actions secrets, and the public key/fingerprint as environment variables. It
requires the Reidond reviewer, permits only main, and disables administrator
bypass. Main requires the GitHub Actions rust check and disallows force-push or
deletion, including for administrators. The release opt-in remains unset; no
production image, installer or channel has been signed/promoted by this setup.
The owner confirmed on 2026-09-08 that the recovery files are saved in Bitwarden
and retrieval is checked. This is owner-reported recovery evidence; the agent did
not access the Bitwarden vault, recovered private material or any SSH key.

Skopeo 1.22.2 generated the encrypted key using the official AMD64 container
`quay.io/skopeo/stable@sha256:227e130acec26a8f8d6aba1c48d30d6adaf8bc927f268fbca381b3dea9cb4257`.
Cosign 3.1.3 at
`ghcr.io/sigstore/cosign/cosign@sha256:6ca1127dc1e9ff19f3f2bfa214936813a86fbbf52919652eda49d393c888ad3c`
signed a public setup confirmation; independent OpenSSL 3.6.1 verification
passed. Both tools ran with no network, a read-only container filesystem and
dropped capabilities through the already-installed Docker Desktop runtime. No
host tool was installed or OS built locally. The private key file is in Skopeo's
ENCRYPTED COSIGN PRIVATE KEY format, consumed successfully by Cosign. This is
production keypair consistency evidence, not a research-image signature or a
Bitwarden recovery drill.

The setup/recovery procedure is:

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

The earlier read-only inspection found no environments and unprotected main at
`c00374cae862c669460da35471950237216a3d14`. The owner-authorized setup above changed
those controls and provisioned only environment-scoped signing secrets. Main's
source revision has not yet changed. No secret contents are present in this
repository or its build contexts.

Sources, checked 2026-09-08:
[GitHub environment protection](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments),
[environment API](https://docs.github.com/en/rest/deployments/environments),
[Skopeo signing/copy interface](https://github.com/containers/skopeo/blob/main/docs/skopeo-copy.1.md).

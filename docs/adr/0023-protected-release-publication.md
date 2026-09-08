# ADR 0023: exact-media promotion and one channel bundle

Status: implemented preparation; production execution and failure qualification
not-run. Date: 2026-09-08. Gates: R02/R08/R10.

A valid candidate OCI signature establishes image identity, not successful
installation or promotion. A usable owner release must bind a successful current
main candidate workflow, exact installed source, the complete ISO, independently
reviewed installation evidence and the owner's dedicated authority.

Keep production keys in the existing owner-reviewed main-only signing environment.
Use three jobs: public preparation, isolated metadata signing, public verification
and publication. The key job executes only reviewed workflow-inline validation and
pinned native signing tools. It never checks out code, runs an artifact or executes
the candidate. The publication job receives exact signatures, not private keys.

Candidate and promotion workflows share one desktop/Fedora-44/x86_64 concurrency
group with active-run cancellation disabled. Recheck current main and expected
previous channel bytes under that serialization; advance release sequence and
checkpoint generation only from independently verified previous state. A slower
old build or repeated attempt cannot advance the channel. The initial conservative
policy requires a still-fresh previous channel and RPM resolution within 36 hours.

Publish versioned installer parts, signed records, public source and qualification
as a complete draft before publishing that version. Never overwrite versioned
release assets. Only then replace the single `channel.json` discovery asset. It
contains both exact signed payload strings and their detached signature strings.
The native verifier still authenticates each payload and its binding; the JSON
envelope introduces no new cryptographic scheme. Missing or mixed/bad metadata
must fail closed. One asset avoids independently updated release/checkpoint files.

GitHub asset replacement can be briefly unavailable and is not a transactional
compare-and-swap API. Shared workflow serialization plus source/prior-hash checks
protect cooperating publishers; arbitrary owner/API writes remain outside that
lock and can cause a refused operation. Signatures and machine replay state remain
the client authority. Interrupted version publication retains the draft/assets and
public workflow artifact for explicit inspection; blind reruns refuse existing
tags instead of overwriting them. Recovery automation is not yet implemented.

The channel release must remain mutable. Repository-wide GitHub immutable-release
settings must not prevent this channel update. The workflow enforces append-only
version assets; this is distinct from claiming GitHub server-side immutability.

The report parser validates asserted evidence bindings, not whether a human
actually performed an installation. Owner review must inspect exact-media evidence.
Research fixtures must never be relabeled as owner-media qualification. No-change
renewal, expired-channel recovery, rotation and independent targets remain open.

Sources: [GitHub release management](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository?tool=cli),
[immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases),
[Sigstore blob signing](https://docs.sigstore.dev/cosign/signing/signing_with_blobs/),
checked 2026-09-08. Implementation: `.github/workflows/promote.yml` and
`build/release/{prepare-release,promotion}.py`. Actual prior native verification
and installation evidence is recorded in R01/R02/R08 and worklog.md.

# Updates and recovery

Kedra updates replace the signed OS container image. Your writable home and persistent data remain separate. No update command automatically reboots or accepts changed home configuration.

## Check and stage

These commands require current-source `sysroot`; published r1 needs the explicit downloaded-file workflow in [INSTALL.md](INSTALL.md).

```sh
sysroot update status
sysroot update check
sysroot update enroll --channel
sysroot update stage --channel
```

Run as your ordinary owner account; the fixed installed helper requests administrator authentication. Enroll once, then stage only after reviewing an available release. Automatic download uses the fixed target channel and installed public trust, verifies both signatures/scope/freshness and replay floors, and supplies the exact signed files to the helper for independent checking. If installing older media after a newer release was promoted, retain the ISO's signed record and follow the older-media enrollment options shown by `sysroot update enroll --help`.

Staging selects the next boot image. Reboot when ready, then run:

```sh
sysroot update status --home
sysroot doctor
```

Home reconciliation is explicit. Review new baselines and resolve conflicts through the [Noctalia](HOME-REVIEW.md) and [niri](TEXT-REVIEW.md) workflows. A successful boot does not discard local changes.

## Roll back

Retain the preceding signed release record and signature:

```sh
sysroot update rollback --manifest OLD_RELEASE.json --signature OLD_RELEASE.sig
```

This selects the retained verified deployment and puts forward updates on hold. After an intentional recovery/review, a later `sysroot update stage --channel --resume` clears that hold. Never delete replay/journal state to bypass a refusal. Rollback does not rewind home, /var, credentials or application databases.

If the desktop is unavailable, use Ctrl+Alt+F2 and your owner account. Inspect `bootc status` and `sysroot update status`. Keep verified install media and encryption recovery information. Do not change signature policy or the system clock to make an expired update pass.

## Midnight package refresh

GitHub Actions schedules production refresh at **00:00 UTC** on the default branch; manual dispatch uses the same release path. GitHub may delay, skip or disable inactive schedules, so this is a trigger time, not a completion-time guarantee.

Before OCI production, the refresh resolves the official Fedora 44 base to an immutable platform digest and runs a disposable native package preflight for the complete installed closure. It records RPM header/payload identities, image-affecting source inputs, external pins/artifacts and build-recipe identity. Shared and target intent and inherited dependencies matter. External agent/Bitwarden/tool inputs stay separately pinned.

The prior release's signed SHA256SUMS authenticates its provenance and package records before resolved-input comparison. Missing legacy comparison material conservatively requires a candidate. R1 lacks this schema, so the first midnight refresh builds a candidate and a later qualified promotion establishes the comparison baseline. Package names/versions alone, cached build layers, failed resolution or a blocked candidate never prove no-change.

A changed build must match its package preflight before protected image signing, ISO production, exact-media qualification and protected promotion. It cannot silently replace an approved channel. Proven no-change skips OCI/ISO production and prepares a fresh checkpoint for the same release, with independent predecessor/material rechecks and protected signing before key-free publication. No signing environment protection is weakened to meet the schedule.

Channel checkpoints have a maximum seven-day lifetime. A failed check does not refresh success, and an expired channel refuses incoming enrollment/staging. Existing local boot and explicit retained rollback remain separate. Consult [status](STATUS.md) for which native release paths have actually been qualified.

## Release operations

See [build/release/README.md](../build/release/README.md) for dispatch, qualification and publication. Keep immutable promoted image digests, signatures and metadata; there is no automatic cleanup of recovery assets.

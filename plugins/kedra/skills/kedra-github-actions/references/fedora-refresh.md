# Fedora package checks

Resolve only the reviewed official Fedora 44 Linux/AMD64 stream to an immutable digest. Reconcile the complete installed native package closure with stable fedora/updates repositories and signature verification. Use fresh metadata and bypass cached package layers. Missing repositories, invalid RPMs or solver failures are errors.

Compare source, base, complete RPM header/payload identities, external artifacts and recipes against the signed stable image's resolved-input record. Same package version is not proof of same bytes. The changed image must agree with preflight.

At 00:00 UTC or manual dispatch, changed inputs produce a candidate for protected manual OCI signing and stable publication. No-change does nothing: no ISO, GitHub Release, metadata asset or freshness checkpoint. Queued/disabled schedules are not proof of current inputs.

Keep exact provenance in the signed image. Never mirror all Fedora, automatically commit nightly package versions or weaken native signature/solver checks.

Sources: [GitHub schedules](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule), [DNF5 upgrade](https://dnf5.readthedocs.io/en/latest/commands/upgrade.8.html), [RPM verification](https://rpm.org/docs/6.0.x/man/rpmkeys.8). Verify actual native versions rather than assuming latest documentation matches the image.

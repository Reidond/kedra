# Fedora refresh

Operational policy is docs/UPDATES.md and build/release/README.md. Trigger at 00:00 UTC; manual dispatch follows the same path. Queue delay and inactive-repository schedule disablement remain possible. No external watchdog or keepalive commits are implicit.

Resolve the reviewed official Fedora 44 base to an immutable platform digest. Reconcile the complete installed native package closure with refreshed stable fedora/updates repositories; inherited/transitive packages matter. Bypass stale package build layers. Missing required repositories, invalid signatures or solver failure are errors, never no-change. Do not use skip-broken, allowerasing, no-gpgchecks or routine distro-sync as recovery.

Native RPM bytes matter beyond NEVRA lists. A disposable pinned-base preflight records complete installed RPM header/payload identities plus base digest, image-affecting source, compiled/external artifacts, pins and recipe identity before OCI production. The prior signed SHA256SUMS authenticates prior provenance/packages. Missing legacy material conservatively builds a candidate; r1 lacks this schema, so a later qualified promotion must first establish the baseline. The actual changed build must match preflight. Failed resolution and blocked candidates cannot renew successful freshness.

Proven no-change skips OCI/ISO production, retains the approved release, advances the signed checkpoint and rechecks predecessor/material under production serialization. New incoming metadata remains strictly fresh; historical predecessor verification is not deployment authorization. Protected signing and key-free publication retain their independent boundaries.

Prior native DNF 5.4.4.0/RPM 6.0.2 generated signed-RPM E2E covered direct/transitive/inherited changes, metadata-only identity, same-NEVRA changes and repository/signature/solver refusal. Historical Actions 34286322016 at 0d82b1f demonstrates only those fixtures. Current production scope needs its own actual Actions evidence.

Sources: [GitHub scheduled events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule), [DNF5 upgrade](https://dnf5.readthedocs.io/en/latest/commands/upgrade.8.html), [RPM verification](https://rpm.org/docs/6.0.x/man/rpmkeys.8). Version-specific behavior must be checked against the actual pinned native environment.

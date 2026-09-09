# Native RPM refresh experiment

Measured 2026-09-09: all ten bounded native cases pass
[Actions 34286322016](https://github.com/Reidond/kedra/actions/runs/34286322016)
at `0d82b1f`. See [the report](../../../docs/research/update-refresh/REPORT.md)
for exact inputs, native evidence and the remaining boundaries.
This is a bounded section-A experiment for R02/R07/R08, using real Fedora DNF/RPM
and Podman image materialization. It does not publish images, create an ISO,
renew a checkpoint, or use a production signing environment.

The separate [actual desktop evidence experiment](target-README.md) is prepared
for development Actions qualification. Its native execution is not-run; it does
not change or broaden the ten measured synthetic RPM results below.

The entrypoint is `python3 build/research/refresh/run.py` in
`.github/workflows/research-refresh.yml`, restricted to `codex/usable-system`.
There is a path-filtered development push trigger and manual dispatch, no schedule.
Do not execute it locally: every OS/container build belongs in Actions.

## Native workflow

1. Build a disposable tools image from the exact Fedora 44 base in
   `build/research/inputs.json`. Use the official `fedora`/`updates` repositories
   with package signature checks for the small native RPM/GnuPG/createrepo tools.
   Retain the exact resulting image ID and native installed tool inventory.
2. Generate a disposable signing key in a separate runner temporary directory.
   Build data-only requested, dependency and inherited RPMs with `rpmbuild`, sign
   them with `rpmsign`, and create actual `createrepo_c` snapshots. Public keys,
   RPMs, specs, repository metadata and metadata signatures are retained. The
   private keyring is mounted only into the disposable fixture-generation
   container; it is never a build context, image layer or artifact input.
3. Build one fixed seed containing only the inherited fixture RPM. Every candidate
   uses that exact seed image ID, the same source context and requested package
   list. Only the read-only repository mount changes. Candidate builds use
   `--layers --no-cache --network=none`; DNF uses the one required local repository.
4. Run native `check-upgrade`, `upgrade`, `install`, `check` and `download`, retaining
   exact arguments, stdout/stderr, exits and times. Upgrade/install preserve
   production `--best --refresh` semantics and required-repository refusal. The
   fixture additionally signs its metadata; that is a fixture property, not a
   claim about Fedora mirror metadata signatures. Never use skip-broken,
   skip-unavailable, allowerasing, distro-sync or disabled signature checks.
5. Retain the complete native before/after RPM inventory and exact selected fixture
   RPM archives. Compare installed and retained native header/payload identities,
   run `rpmkeys --checksig --verbose` and `rpm --verify`, and retain installed file
   hashes/modes. An unavailable selected archive or a mismatch fails evidence
   collection; a matching NEVRA alone is insufficient.
6. Compare the fixture's source intent, fixed seed image, complete installed RPM
   inventory and retained fixture package/file identities. Repository metadata
   hashes are recorded separately. Every successful build must produce its own
   new execution receipt outside the image; a cached RUN cannot satisfy this.

## Cases and failure interpretation

The baseline, requested/dependency/inherited version updates, metadata-only
revision, same-NEVRA payload republish and repeated identical source/base/snapshot
must materialize their expected native state. The required repository outage,
damaged signed package and newest direct requested candidate with an impossible
dependency must fail at the native DNF boundary and leave the seed RPM inventory
unchanged. A successful skip of an unsatisfiable candidate is a failing result,
not an invitation to silently change only the fixture's solver flags.

The inherited update provides native `check-upgrade` exit 100; the baseline
provides 0; the missing required repository provides an error. The probe is
advisory and never itself establishes successful reconciliation. The impossible
version is a **direct requested** package. Whether `--best` rejects every
unsatisfiable newest transitive candidate remains a separate case.

`rpm-fixture-comparison.json` is deliberately not an OS equivalence protocol.
All non-fixture Fedora packages are bound by the fixed seed image and native
inventory, but their package bytes are not re-resolved against Fedora mirrors.
Image filesystem metadata, non-RPM inputs, configuration changes, version holds,
repo/build races, base signature/identity changes and boot behavior remain outside
this experiment. No timestamp normalization or useful full-OS no-change claim is
established. A fixture-equivalent result alone cannot authorize freshness renewal.

`output/refresh-evidence/results.json` retains actual case statuses and links to
native logs and image inspections. Expected refusals pass only if the specific
native step fails, its diagnostic identifies the intended cause and before/after
RPM inventories match. Outage evidence requires Curl 37 for the missing fixture
metadata; the solver error must name the exact impossible requirement. Signature
refusal must name the affected package/path, whose retained bytes must match the
snapshot and fail native RPM verification with a bad payload digest. Unknown
diagnostics remain failures for review; infrastructure/tool failures do not pass
negative cases. No freshness publication operation exists
in this experiment. An interrupted run retains available evidence for inspection;
it does not create a successful resolution record.

## Primary references checked during preparation

- [DNF5 check-upgrade](https://github.com/rpm-software-management/dnf5/blob/main/doc/commands/check-upgrade.8.rst):
  100 means updates available; 0 means none. Native version and actual behavior
  must still be recorded by the run.
- [DNF5 download](https://dnf5.readthedocs.io/en/stable/commands/download.8.html)
  and [configuration](https://dnf5.readthedocs.io/en/stable/dnf5.conf.5.html):
  exact package download and repository/signature/solver settings.
- [RPM signing](https://rpm.org/docs/6.1.x/man/rpmsign.1) and
  [RPM verification](https://rpm.org/docs/6.0.x/man/rpmkeys.8): public-key import,
  package signature/digest verification and disposable signing configuration.
- [RPM digest tags](https://rpm-software-management.github.io/rpm/manual/signatures_digests.html):
  RPM 6's payload SHA-256 tag is `PAYLOADSHA256`.
- [DNF signature diagnostics](https://raw.githubusercontent.com/rpm-software-management/dnf5/main/libdnf5/rpm/rpm_signature.cpp):
  a generic package-open failure can also mean failed signature/digest verification;
  the experiment therefore retains and natively verifies the named archive.
- [createrepo_c](https://rpm-software-management.github.io/createrepo_c/):
  native repository metadata and package checksum generation.

These describe mechanisms. The linked report separately records actual Fedora
44 results with DNF 5.4.4.0, RPM 6.0.2 and Podman 4.9.3.

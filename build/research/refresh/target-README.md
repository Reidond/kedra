# Actual desktop resolution evidence

Prepared 2026-09-09 for development Actions qualification; native execution is
**not-run**. This is the next bounded experiment after the ten synthetic RPM cases
in [run 34286322016](https://github.com/Reidond/kedra/actions/runs/34286322016).
Those earlier cases remain fixture evidence, not proof of this implementation.

The workflow is
`.github/workflows/research-target-refresh.yml`. Its entrypoint is
`python3 build/research/refresh/target.py`, restricted to the development branch
`codex/usable-system` in `Reidond/kedra` and to GitHub Actions. It has read-only
repository permissions, no production environment and no schedule. Narrow push
paths cover this observer and its actual build recipes/pins, allowing first
qualification before the workflow is accepted on main. Manual dispatch remains
available after acceptance; docs-only changes do not trigger a build. Do not run
the image experiment locally. It does not sign/push images, create an ISO, compare
an old promoted release or write a channel/checkpoint.

## Native inputs and assembly

The experiment verifies the committed Fedora base pin against its registry
manifest. If the pin is an index, it records that index and selects exactly one
Linux AMD64 manifest, verifies the platform bytes/configuration, then pulls that
exact platform reference. Moving-tag discovery and upstream base-signature
qualification are explicitly not-run.

Actual `sysroot source plan` and `source archive` commands prepare the desktop
payload. The normal Codex, Bitwarden and public-trust preparation programs prepare
the same pinned inputs used by the release workflow. No private release key,
personal profile or research agent version is a build input. Compiler, Cargo,
Podman and Skopeo versions, explicit recipes, Cargo/toolchain configuration,
generated artifact hashes and complete source/run provenance are retained.

`Containerfile.target` uses that generated context and runs the **unchanged**
`build/assemble.sh`. Only that process's PATH contains `target-dnf.sh`, which
delegates to the real `/usr/bin/dnf` through `target-native.sh`. The observer adds
an explicit Fedora/updates allowlist, required-repository/package/TLS checking,
`keepcache=True` and a dedicated cache directory. It preserves the assembler's
native upgrade/install/removal ordering and arguments. It does not use
distro-sync, skip-broken, skip-unavailable, allowerasing, disabled signatures or
a custom dependency solver. An empty removal intent is recorded as not-run;
the experiment does not invent a removal transaction to obtain a green case.

Both `baseline` and `repeat-fresh` start from the same pinned Fedora platform,
source and generated artifacts. Both bypass build caching and produce a new
native execution receipt through an external evidence mount. Each DNF upgrade
and install still uses `--refresh`; they can legitimately observe different
Fedora metadata. Per-operation metadata is retained rather than claiming one
global snapshot. Compiled tooling is prepared once; repeated compilation or
reproducible compiler outputs are not established by this experiment.

The ordinary public-trust derivative is built after each desktop. Both completed
images stay in disposable runner storage. The observer scripts and cache are
removed from the desktop before that derivative is built.

## Evidence and accounting

The native observer retains the following before normal assembly removes caches:

- Actual argument arrays, separate stdout/stderr, exit codes and execution times.
- The full before/after installed RPM inventory, including epoch, architecture,
  SHA-256 header and payload identities, vendor and source RPM.
- Effective DNF configuration/variables and enabled repository information after
  each transaction, plus the native public-key list and exported public keys.
- Each operation's actual cached metadata, and content-addressed copies of
  downloaded RPM archives with their original repository cache locations.
- Native `rpmkeys` verification requiring both signatures and digests, and native
  archive-header queries. A failed signature or incomplete record cannot become
  a successful material comparison.
- Native installed package origins/reasons and final full `rpm --verify --all`
  output. Intentional configuration overlays mean verification differences are
  review evidence; they are not silently normalized into a clean filesystem.

Each final package must either match a retained, verified transaction archive's
complete native header/payload identity, or exactly match the same NEVRA and
header/payload identity from the pinned base inventory. The latter is explicitly
marked `unchanged-pinned-base`. It is **not** a claim that an old archive is still
available from current mirrors or that current mirrors did not republish its
NEVRA. Native pseudo-package public-key rows are recorded separately, together
with the actual keyring evidence.

After the trust derivative, native readback checks every explicitly supplied
non-RPM file against the generated archives and known assembly transformations:
content, ordinary-file type, mode and root ownership; home seeds in `/etc/skel`;
compiled programs; pinned agents/Bitwarden; source manifest; and public trust.
It also observes the graphical target link and requires the inherited automatic
update service/timer masks, then rechecks the final RPM inventory. This is an
explicit payload inventory, not a whole-root filesystem scanner or proof of
normalization for every RPM-generated file, directory, xattr or timestamp.

## Provenance and comparison

`input-provenance.json` retains the full checkout SHA, run/attempt, raw context
hashes and installed manifest hash. The source manifest is checked against its
exact original bytes before/after assembly and after the trust derivative.
Installed source ancestry and signed-release identity are never rewritten.

The separate `material-intent.json` excludes only the source plan's
`source_revision` and `input_scope`; it retains target/package/removal policy and
all selected file mappings/content/modes. Generated binary/external payload,
recipe, public trust and build-tool identities are included. Hashing the whole
payload tar would include the source revision and force churn after a docs-only
commit, so its raw hash is retained as provenance while its actual file entries
are validated and compared separately. This design does not yet qualify a
docs-only source advance during concurrent promotion.

Each successful case emits `resolution.json`. The pair can report
`material-inputs-equal/equivalence-unproven` or `material-inputs-changed`. Failure
remains `resolution-or-evidence-failed`, with actual native evidence retained.
Every result keeps whole-image equivalence unproven, the no-change release
decision false and freshness writing false. Even an equal pair cannot skip a
release/ISO or renew a checkpoint. Base trust, same-NEVRA inherited republication,
lower-EVR/vendor/hold/obsolete policies, complete filesystem normalization,
promoted-record binding, boot/installer qualification and renewal remain gates.

Public artifacts are under `output/target-refresh-evidence/`, including exact
downloaded transaction RPMs. They are retained for seven days without ZIP
recompression. The generated context, unsigned images and agent download work
directory are not artifact upload paths. No private signing material is created.

## Primary mechanism references

- [DNF5 configuration](https://dnf5.readthedocs.io/en/latest/dnf5.conf.5.html):
  cache retention/location, package signatures, TLS and repository policy.
- [DNF5 command options](https://dnf5.readthedocs.io/en/latest/dnf5.8.html):
  fresh metadata and effective configuration dumps.
- [DNF5 5.4.4.0 repo info](https://github.com/rpm-software-management/dnf5/blob/5.4.4.0/dnf5/commands/repo/repo_info.cpp):
  actual repository fields provided by the version measured in the prior fixture.
- [DNF5 repoquery](https://dnf5.readthedocs.io/en/latest/commands/repoquery.8.html):
  installed full NEVRA, origin and native reason.
- [RPM 6.0.2 rpmkeys](https://rpm.org/docs/6.0.x/man/rpmkeys.8) and
  [RPM configuration](https://rpm.org/docs/6.0.x/man/rpm-config.5):
  public-key evidence and `_pkgverify_level=all` signature/digest requirements.

These are mechanism references, not a new native result. Record actual run,
versions, archive hash and observed failures in the research report after Actions.

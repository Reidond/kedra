# Actual desktop resolution evidence

Recorded 2026-09-09: native full RPM accounting, repeated desktop assembly and
declared-payload readback **pass** in run 34294737585. The subsequent complete
filesystem/OCI observation extension described below is prepared but **not-run**.
This is the next bounded experiment after the ten synthetic RPM cases
in [run 34286322016](https://github.com/Reidond/kedra/actions/runs/34286322016).
Those earlier cases remain fixture evidence, not proof of this implementation.

[First run 34293193670](https://github.com/Reidond/kedra/actions/runs/34293193670)
at `1fdec1fb80e86d3195ef44d0868100d8fc8fc18b` passed base resolution and actual
source/agent/Bitwarden/public-trust preparation. Fedora's `jq 1.8.1-3.fc44`
rejected `--repo=fedora` while the observer serialized its argument array; no
native DNF command had run. Both argv serialization sites now terminate jq's
option processing with `--`. Artifact
`10082232519` is 366,806 bytes; its downloaded ZIP independently matches SHA-256
`f16aa2ad59825aa7f738721d20368a3bc5aa6a05245490c30dcdd2355fc8d5bb`.

[Second run 34293703725](https://github.com/Reidond/kedra/actions/runs/34293703725)
at `2954d764b78e59823b7901af43c491414a09392b` passed the complete actual baseline
assembly and native observer (exit 0), then refused unsupported installed-reason
rows. Actual DNF 5.4.4.0 output contained literal `\t` separators: unlike RPM's
query formatter, this interface does not translate that escape. The observer now
passes literal tabs/newlines through Bash ANSI-C quoting, keeping strict host
parsing. The failure message also includes a bounded escaped row for diagnosis.
That run did not reach repeat/final payload readback and emitted no comparison or
freshness result. Artifact `10082478066` is 802,174,754 bytes; its complete ZIP
independently verifies SHA-256
`3d65d67ecd0955434a88c4d362430b0a7acb7f0d3f5f299a4870213c1cd8897f`.

[Third run 34294737585](https://github.com/Reidond/kedra/actions/runs/34294737585)
at `67b4b141012f57d126d9ac9e16646d0f7bfd55b5` passes both actual assemblies,
native evidence accounting, public-trust derivatives and final declared payload
readback with DNF/libdnf5 5.4.4.0, RPM 6.0.2 and Podman 4.9.3. Each resulting
image accounts for 1,005 RPMs: 542 unchanged packages from the exact pinned base
and 463 matching verified transaction archives. All 133 compared declared files
match; the source manifest is separately checked against its original bytes.
Distinct execution receipts and actual command records prove both native package
steps ran. Their material comparison SHA-256 is
`791296edc0fcd653d39448d5b648e7ba579563b50e6bf6451f827d74bcf39c7a`.

The result is `material-inputs-equal/equivalence-unproven`. Native image IDs differ:
`3f2e9c0c21641242f6a0e9bcac0e3c8c04ff0291126aa590153a57d19fcea911` and
`f0051a75eb5d5b41cff3293502d038e9f292027d5872f8ea7d63b3a6852fb2ab`.
Podman's reported runtime `Config` is equal, while creation/history/layers,
digests and the public derivative's base-name/base-digest annotations differ.
Reported image sizes differ by 13 bytes; those inspections do not identify the
responsible filesystem paths. Cached metalink and libsolv files differ while
retained repomd/data identities match. No timestamp or cache normalization was
applied, and no release/ISO pruning or freshness operation was authorized.

Successful artifact `10083040414` is 1,608,870,789 bytes. The complete downloaded
ZIP independently matches SHA-256
`95d4e5dc930bc9797cdebdeb1b054ad33b535c6db40bf15e2194bc01387e7249`.
Actual evidence is retained locally under `output/target-refresh-34294737585/`.
That completed runner did not export a complete filesystem inventory. The next
extension must collect a newly built pair while those exact images still exist;
it cannot backfill missing filesystem observations into this successful run.

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

## Prepared complete image observation

The new `target_image.py` coordinator records exact raw manifests and raw OCI
configuration through Skopeo's `--raw` and `--config --raw` interfaces. The config
bytes must match both the manifest descriptor and the exact built image ID.
Runtime configuration, platform fields, remaining config fields (including
creation/history/rootfs identity) and manifest fields are compared separately.
All fields remain in the evidence; a changed annotation is not silently ignored.

For each exact built image ID, the coordinator checks native Podman mount state,
mounts that image and validates its canonical returned path beneath the actual
native Podman GraphRoot. Custom `target-filesystem.py` traversal runs in a
separate no-network container with a read-only root and read-only `/image` bind;
`--read-only-tmpfs=false` keeps its temporary directories read-only too. The bind
is nonrecursive and traversal refuses another filesystem. The reader opens
directories/files without following symlinks and visits the static image mount,
not the reader container's generated `/proc`, `/sys`, host files or runtime root.
The recorder itself is bind-mounted read-only and receives no extra capabilities.

The complete path inventory records raw path bytes (base64) with readable names,
types, modes, UID/GID, sizes, link counts, observed inode/device/block facts,
observed access/modification/change timestamps, regular-file SHA-256 hashes,
symlink targets and complete hardlink groups. Directory entries and stat facts
are rechecked; inconsistent reads and unaccounted hardlink counts fail the
observation. No file content is exported. Device/FIFO/socket entries are recorded
as metadata and never opened as data streams.

The host's standard `getfattr` program reads only that validated image mount with
all namespaces selected, physical/no-dereference recursion, absolute names and
base64 value encoding. This covers namespaces that a capability-limited container
could hide. Only the disposable Actions job adds the `attr` package; no custom
checkout program runs as host root, and the recorder container receives no
`SYS_ADMIN` capability. Native xattr snapshots before and after traversal must
agree. Native stderr/nonzero exits, unsupported escaped path/name mapping,
duplicate records or a path outside the validated mount fail rather than assigning
metadata incorrectly. Raw native dumps are retained for diagnosing any such case.

The exact image is unmounted in `finally`, without `--all` or `--force`, and native
readback must show its mount gone. The reader has a unique, initially absent
container name. Bounded native stop/remove/absence checks reap that exact reader
before ordinary image unmount, including when a timed-out Podman client leaves
the reader running. Missing auto-removed readers are accepted through native
`--ignore` and a separate absence result, not by ignoring a running container.
If evidence I/O prevents ordinary cleanup
logging, bounded native Podman state/unmount calls still attempt cleanup and the
observation remains failed. This does not promise recovery after runner destruction
or an uncatchable process kill.

The mounted phase has a 15-minute deadline within an earlier 60-minute overall
observation deadline. GNU `timeout` runs with the native command's privilege and
has a bounded termination grace period. The reader also checks a 14-minute
internal deadline and has a 2 GiB memory/swap ceiling. One million entries,
64 MiB of path bytes, 64 GiB of content reads, 128 MiB of emitted metadata and
directory depth 256 are explicit fail-closed ceilings. Native metadata dumps are
size-checked before host parsing. Hitting a deadline/budget produces an incomplete
observation; nothing is silently truncated. Actions preparation and experiment
steps are bounded separately to leave time for exact cleanup before the job limit.

`filesystem-observation.json.gz` retains the complete per-image observation;
`final-image-differences.json.gz` retains every changed path/field, hardlink
relationship and OCI field. Small adjacent JSON files contain counts and hashes.
Compression only packages evidence; observed image facts are unchanged. There is
no ignore list. Timestamp, inode/device and other storage-visible differences
remain visible alongside content, permission and xattr differences, allowing a
later decision based on concrete evidence.

New result status `recorded` means complete observation, not file equality.
The comparison reports `differences-observed` or `observed-identical`, always with
normalization and equivalence authorization false. Incomplete observations exit
nonzero. The existing RPM/material-intent comparison remains a separate result.
No policy for skipping images, renewing checkpoints or normalizing these fields
is implemented by this recorder. Its complete native run remains not-run.

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
- [jq 1.8 invocation](https://jqlang.org/manual/v1.8/#invoking-jq): `--` ends option
  processing even when `--args` is used to collect remaining string arguments.
- [Podman 4.9.3 image mount](https://docs.podman.io/en/v4.9.3/markdown/podman-image-mount.1.html),
  [unmount](https://docs.podman.io/en/v4.9.3/markdown/podman-image-unmount.1.html),
  [info](https://docs.podman.io/en/v4.9.3/markdown/podman-info.1.html) and
  [run](https://docs.podman.io/en/v4.9.3/markdown/podman-run.1.html): exact image
  mounts/counters, actual GraphRoot, nonrecursive binds and read-only runtime.
- [Skopeo 1.13.3 inspect](https://github.com/containers/skopeo/blob/v1.13.3/docs/skopeo-inspect.1.md):
  `--config --raw` preserves config bytes, distinct from the raw manifest.
- [getfattr upstream manual](https://man7.org/linux/man-pages/man1/getfattr.1.html):
  all-namespace physical traversal, no symlink dereference and encoded output.
- [Podman 4.9.3 stop](https://docs.podman.io/en/v4.9.3/markdown/podman-stop.1.html),
  [remove](https://docs.podman.io/en/v4.9.3/markdown/podman-rm.1.html) and
  [container exists](https://docs.podman.io/en/v4.9.3/markdown/podman-container-exists.1.html):
  bounded cleanup and distinct native absence/storage-error results.

These are mechanism references, not a new native result. Record actual run,
versions, archive hash and observed failures in the research report after Actions.

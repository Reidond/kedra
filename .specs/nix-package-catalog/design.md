# D3 design and prepared integration

Implements [AC-01–AC-11](requirements.md) over existing engine/store/composition
contracts. The earlier 17-path compiler-validated candidate and follow-ups are
retained separately from the active checkout. The current preparation adds three
native-format template files (20 implementation paths); this change has format/lint
checks only. [Verification](verification.md) preserves the earlier evidence.

## Catalog and CLI

`sysroot-catalog` is a flat-source workspace library using existing engine,
serde and serde_json dependencies. Its public Catalog, Package, Source,
SourceOrigin, Recipe and Policy types are ordinary Rust data. JSON is transport,
not an evaluator. Built-ins and independent callers share the same resolution API.

`Catalog::resolve` checks exact namespace/package allowlists, source declarations
and every graph node's exact builder/runtime identities, then invokes engine
planning. Object inputs must be declared `src-` identities; derived dependencies
use `Input::Node`. Namespace does not salt or relocate engine output identities.
The existing private store remains the filesystem/lifecycle boundary.

The CLI provides `catalog list`, `pins`, `plan`, `resolve`, `build` and
`contribute`. Plan/resolve are pure. Build resolves policy before Store::open.
Contribute requires already realized expected objects, matching derivations and
runtime foundation, exact canonical preflight bytes, source revision and a new
private destination. It emits system.json, contribution.json, catalog-pins.json
and catalog-results.json. Partial output after an I/O failure is unselected
diagnostic material; retry uses a new destination, not an overwrite.

## Applications, compiler and retention

- jq 1.8.2 uses its release sources and bundled Oniguruma, disables shared internal
  libraries, docs and maintainer regeneration, then checks the produced executable's
  ELF magic. Its licenses remain in the output. Recursive make builds dependencies.
- SQLite 3.53.4 uses the released amalgamation. A library node creates
  libsqlite3.so/headers; the shell node links with an exact store RPATH and explicit
  runtime dependency. File-prefix mapping removes source-directory compiler paths.
- Source archives are verified before bounded extraction; the enclosing directory
  is removed, hardlinks become independent identical files and executable bits are
  preserved. Archive checksum and canonical source identity are distinct evidence.
- Final pipeline compiler acquisition uses D2's reviewed immutable Fedora bootc
  base, not a local opaque historical builder ID. Native/version guards and the
  existing Fedora/updates GPG policy apply to compiler RPM resolution and build.
  Build-time comparison plus independent stopped-container readback require the
  exact preflight RPM contents. Retained image/tool/receipt hashes are runtime evidence.

The engine supplies execution isolation, output verification, rebuild comparison,
closure export/import and profile lifecycle. No parallel implementation is added.
Images/sources stay explicitly retained; a runtime bundle alone is not a source
rebuild kit. Running engine controllers inside another container requires object
paths visible at identical absolute paths to that controller and the outer daemon.
Set a shared TMPDIR for generated CLI E2E stores; controller-only `/work` is unsuitable.

## D2 contribution and installed behavior

The existing typed SystemDefinition uses `aarch64-linux`, the exact fresh D2
foundation and output aliases `jq`/`sqlite`. Segment::Input creates store references.
It contributes only 0644 configuration/metadata at:

- `/etc/profile.d/kedra-catalog.sh` and
  `/usr/lib/environment.d/60-kedra-catalog.conf`: selected store bin directories
  precede existing PATH, with `/usr/local/bin:/usr/bin:/bin` fallback.
- `/usr/lib/systemd/system/kedra-catalog-history.service`: manual-start oneshot,
  exact SQLite ExecStart, DynamicUser, StateDirectory, restricted filesystem/home
  access and no new privileges. `/var/lib/kedra-catalog/history.sqlite` survives restarts.
- `/usr/share/kedra/catalog.json`: selected versions, source revision, foundation
  and output inventory, outside the source adapter's reserved `/usr/share/sysroot`.

No `/usr/bin`/`/usr/lib` ABI replacement, ld.so configuration or writable-home
change is permitted. D2's fixed enabled-unit/native-generation policy stays intact.
The closed contribution receipt binds source revision, foundation, preflight,
definition, catalog pins, exact sysroot author hash and output aliases. Actual
object receipts are separate. D2 independently validates the definition/closures;
neither receipt is a new signing authority.

Preflight receives `catalog={pins,builder_rpms}` through the shared material helper.
It records source/API/adapter recipes, compiler policy and canonical RPM material;
the compiled sysroot author must match the existing sysroot artifact. Only the
changed workflow builds the compiler, realizes applications and invokes the author
between D2 foundation and compose. The isolated production signer is unchanged.

Reverting the D3 image/code removes the selected commands/configuration but does
not rewind `/var`. The service's new table is created if absent and receives
inserts only; incompatible existing schema causes SQL failure rather than a
destructive migration. Its database is deliberately retained on image rollback.
No caller must assume package/profile rollback rolls back persistent user data.

## Files and ownership

The prepared 17 paths comprise root Cargo.toml/Cargo.lock; three sysroot-catalog
files; sysroot Cargo.toml/main.rs/catalog.rs/tests/e2e_catalog.rs; image/catalog
Containerfile/builder.sh; release material.py/refresh.py; release-target.yml; and
container catalog_tests.rs/main.rs/native.rs. New source is prepared; shared
registrations/hooks await delivery-lead adoption. This spec adds only its own
directory. Parent owns existing documentation, worklog, Git and draft publication.

Adopt the narrow patch against final D2, preserving its resolve-base operation
and local controller/fixture changes. Earlier unregistered CLI/E2E copies contain
the corrected-in-private SHA2 formatting error; use the validated final copies.

## Risks and decisions

Repository movement between compiler resolution/build must fail RPM equality,
not relax GPG or comparison. Actual Fedora ABI, jq output linking, SQLite library
selection and independent byte equality require execution. PATH intentionally
selects catalog tools while absolute RPM paths remain available. DynamicUser,
StateDirectory and SELinux behavior require the actual installed harness; metadata
or file existence cannot pass that gate. Installed test selection must target the
D3 derived candidate, not unrelated native-artifact fixtures. Changes in these
findings belong in this spec and verification record before publication.

Review priorities are input/authority or byte-binding failures (high consequence;
mandatory refusal gates) and application/PATH/service failures (functional
acceptance gates). Unsupported platforms and D4/D5 capabilities are separately
owned and explicitly unqualified; no likelihood or performance estimate is asserted.

## Single-source native templates

Owner steering moves the static text from catalog.rs into
`usr/src/kedra/image/catalog/templates/`, with these mirrored relative paths:

- `etc/profile.d/kedra-catalog.sh`
- `usr/lib/environment.d/60-kedra-catalog.conf`
- `usr/lib/systemd/system/kedra-catalog-history.service`

One Rust registration per relative path supplies both include_str and the installed
`/relative/path`. Template bodies occur only in those files. They stay under the
excluded development tree so the shared/desktop foundation cannot receive raw
placeholders. Generated catalog.json remains generated data, with namespace and
versions taken from the actual selected catalog results.

The closed `${KEDRA_CATALOG_JQ_BIN}`, `${KEDRA_CATALOG_SQLITE_BIN}` and
`${KEDRA_CATALOG_SQLITE}` tokens become existing Segment::Input values;
`${KEDRA_CATALOG_SOURCE_REVISION}` becomes the already validated exact Git revision.
No environment lookup occurs. `${PATH:-/usr/local/bin:/usr/bin:/bin}` is untouched
literal text for the native shell/environment.d consumers. There are no template
expressions/imports, discovery scanners or new dependencies. Unknown, unclosed or
unbraced reserved catalog tokens refuse.

The same template registration emits source/destination metadata and hashes into
catalog pins; a serialized copy of the closed binding table is also covered by
the pins digest. Preflight checks each declared template source against its
compiled hash and includes it in recipe material; D2's existing committed-source
verification applies. The compiled author contains the same bytes. SystemFile
mode/provenance/priority/replaces=None and engine path/alias/collision validation
are unchanged. The installed case reads selected versions from the generated
inventory while retaining actual jq, SQLite and persistence execution.

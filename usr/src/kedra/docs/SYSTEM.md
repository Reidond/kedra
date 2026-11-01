# Typed system composition

`sysroot-engine` supplies typed Rust `SystemDefinition`, `SystemFile` and
`SystemContent` contributions. Files have explicit provenance, priority and
replacement; templates use typed `Argument` segments to refer to verified package
outputs. Serialized JSON transports the same types and is not an expression
language. Native configuration remains ordinary systemd, niri and bootc files.

The `.kedra` package language ([PACKAGES](PACKAGES.md#language-v1)) does not
replace this model. A checked package `config` template lowers after the build
into `SystemFile` values, with named literal, package-path and source-revision
bindings. The contribution adapter also generates the command launchers, the
PATH configuration and `/usr/share/kedra/catalog.json`. It refuses duplicate
configuration paths, references to unselected outputs, incompatible foundations,
unrealized graph objects and input material that differs from the independently
hashed release preflight, and compares the actual foundation and compiler RPM
material with that preflight before it accepts realized outputs.

## Plan and compose

The Kedra adapter resolves one committed source revision, including the files and
home baselines its `system/` declarations select, then observes an exact retained Fedora 44 image of the target's
architecture, which must be the machine's own (`aarch64` for `qemu-arm64`,
`x86_64` for `desktop`).
All desired RPM names must already be present and removed names absent. Each
engine output names the runtime root it was built for; composition accepts it only
if that root's RPM inventory (name, epoch, version, release, architecture, header
and payload SHA-256) equals the foundation's exactly, nothing missing or extra, and
records the root's recipe and inventory hash in the plan and in the installed
`/usr/share/sysroot/composition.json`. The runtime root never has to be the
foundation image itself: a release builds it from the RPM lock over the Fedora
base, so package identities do not change with each commit's foundation image.
Composition itself performs no DNF transaction and activates nothing: starting
services, creating accounts, migrating data and other effectful steps are not part
of it.

```sh
sysroot system plan --repo . --target qemu-arm64 --store /private/engine-store \
  --foundation sha256:<exact-image-id> --definition /private/authored-system.json
sysroot system compose --repo . --target qemu-arm64 --store /private/engine-store \
  --foundation sha256:<exact-image-id> --definition /private/authored-system.json \
  --output-dir /private/new-context
```

`--definition` is optional. Initialize the private store and retain the exact
image with `sysroot store init` and `store add-image` first. Commands emit JSON;
failures go to stderr. A new output directory is required. Planning verifies
image/object bytes and runs read-only foundation observations in the journaled
bubblewrap sandbox, inside the foundation image's unpacked root.

File rules, checked before anything is written:

- Paths are absolute and lie under `/etc/` or `/usr/`. `/usr/lib/sysroot` (the
  store), `/usr/share/sysroot/composition.json`, `/etc/ld.so.preload`,
  `/etc/ld.so.conf` and `/etc/ld.so.conf.d` are reserved. An authored definition
  cannot write `/usr/share/sysroot` (source manifests and home baselines) or use
  the committed source's `kedra-source:` provenance or `kedra.*` provenance keys.
- Two files on one path need an explicit replacement: the one with the higher
  priority names the other's provenance in `replaces`. Any other
  duplicate, a `replaces` with nothing to replace, and a file below another file's
  path are refused.
- Configuration overlay paths (`/etc/`, `/usr/share/`, `/usr/lib/systemd/`,
  `/usr/lib/environment.d/`, `/usr/lib/tmpfiles.d/`, `/usr/lib/sysusers.d/`,
  `/usr/lib/udev/rules.d/`, `/usr/lib/sysctl.d/`, `/usr/lib/bootc/kargs.d/`) enter
  the payload, but only where the foundation has no symlinked ancestor and no
  executable, hardlinked or non-regular file at that path. A file elsewhere, such
  as an executable or a library, only asserts the foundation's existing bytes and
  mode and never replaces them.
- A store path in any file must belong to a declared output, and a typed template
  path must exist in that output.

The plan and composition identity is input-addressed over the canonical resolved
plan: actual file bytes and modes, verified object receipts and the observed
foundation material. Artifact hashes are recorded separately in the exported
manifest, so the identity has no self-reference.

Language contributions add another independently bound layer before these calls.
`sysroot catalog contribute` requires an explicit language root/entry/target/lock,
target policy, exact foundation/builder-root resolution, private store, foundation image,
catalog policy, exact lowercase source commit, canonical release input material
and its expected SHA-256. It writes a new mode-0700 directory containing:

- `system.json`: the typed `SystemDefinition`;
- `contribution.json`: source, foundation, preflight, author, pins, definition and
  output identities;
- `catalog-pins.json`: the complete frontend pin/material envelope;
- `catalog-results.json`: verified realized object receipts.

The adapter verifies every selected graph node against the expected derivation and
runtime root, checks that the system foundation and the runtime root hold exactly
the preflight's foundation RPMs, verifies each runtime closure, includes
runtime-library outputs, and
binds the exact frontend binary to the same `sysroot` artifact used by preflight.
This is public build authority only; it does not sign or publish an image.

## Verify and consume

Home baselines travel as ordinary payload files like any other committed
source; composition treats no application's baseline specially. See
[managed home files](HOME.md).

The reusable `VerifiedComposition::open` consumer requires an independently
retained identity and owns a private snapshot. It verifies the complete foundation,
canonical plan, static Containerfile, configuration/object trees, references and
actual artifact bytes. It never extracts an untrusted root filesystem on the host.
The manifest is limited to 64 MiB; payload and foundation limits remain explicit.

```sh
sysroot system verify --context /private/context --expected-identity <identity> \
  --workdir /private/verification-scratch
kedra-lab replay --image composition:/private/context \
  --composition-identity <identity> --target qemu-arm64 \
  --output <typed-alias> --program bin/program -- argument
```

The static image itself is realized by `sysroot-engine` (`realize_composition`),
the same code `sysroot derive build` uses for its parent image. It loads the
retained archive, checks native ID, observations, paths and inherited
ONBUILD/volumes, then builds only the verified static context with pulls and
network disabled, and checks that the result's layers extend the foundation's.
Cache bindings in the owner-private derive state tie image ID, identity, payload,
foundation and daemon together; owned interrupted work resumes under a
per-context lock. Foreign tags and unknown state refuse. `kedra-lab replay` is
part of the sanctioned harness: it calls that realization (state under its
artifact root) and runs the selected typed output directly in the image.

Composition requests default to no source overlay and require the separate
identity. Working-tree/binary overrides and VM composition are refused. Ordinary
lab-tools adaptation is separate and networked; the system-profile composition
case uses the static image directly. None of these consumers gains installed or
signing authority.

The context contains deterministic `payload.tar` (root-owned entries, canonical
modes, zero timestamps), verified `foundation.tar`, a static `Containerfile` and
`composition.json`. The manifest binds source/target, exact foundation/archive,
observed seven-column RPM content, typed file provenance, runtime object receipts
and artifact hashes.

The Containerfile is exactly `FROM <tag>` plus `ADD payload.tar /`, with a
digest-derived local foundation tag, because BuildKit would read a bare
`sha256:` image ID in `FROM` as a registry name. A consumer refuses any other
Containerfile bytes, verifies artifact hashes, loads the retained archive,
verifies its exact image ID and assigns that tag before building with
pulls/network disabled. Exported files carry no signing or installed-system
authority.

The separate [native stage](NATIVE.md) (`sysroot derive`) applies the steps the
target's `.kedra` block declares and the composition's source manifest carries:
it compiles GLib schemas, changes declared service links, seeds initial account
defaults and regenerates the target's generic initramfs. Boot, update,
installation, SELinux and signing are tested separately ([STATUS](STATUS.md)).

## One image pipeline

Both release targets build the same way, natively on their own runner
(`.github/workflows/release-target.yml`, `image/release/compose.py`):

1. Lock: `sysroot catalog lock resolve` resolves every Fedora RPM once
   ([package delivery](PACKAGES.md#fedora-rpm-lock)).
2. Foundation: `image/Containerfile` and `assemble.sh` install the locked RPMs
   offline over the Fedora bootc base and add the binaries and the committed
   payload. `assemble.sh` validates the niri and Noctalia baselines, names the OS Kedra in
   `os-release`, records the RPM material and runs `sysroot --version`. It applies
   no image step.
3. Packages: the catalog builders and `sysroot catalog build|contribute` realize
   the target's `.kedra` packages (Codex and Bitwarden Desktop, from pinned
   official archives) and refuse an export that would shadow a foundation command.
4. Compose: `sysroot system compose` with that contribution.
5. Derive: `sysroot derive build` applies the target's declared steps.
6. Trust layer: `image/release/Containerfile` adds the signed-update identity and
   public trust, and runs `bootc container lint`.

The lab's full local build (`KEDRA_LAB_IMAGE=build`) replays steps 1 to 5,
without the trust layer.

See [package declarations](PACKAGES.md), [engine operations](ENGINE.md),
[architecture](ARCHITECTURE.md) and [current status](STATUS.md).

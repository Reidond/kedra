# Typed system composition

`sysroot-engine` supplies typed Rust `SystemDefinition`, `SystemFile` and
`SystemContent` contributions. Files have explicit provenance, priority and
replacement; templates use typed `Argument` segments to refer to verified package
outputs. Serialized JSON transports the same types. Native configuration remains
ordinary systemd, niri and bootc files.

The `.kedra` package frontend is the narrow language exception. It does not replace
the system composition model. A checked package `config` template lowers after
build into `SystemFile` values, with named literal, package-path and frozen-source
bindings. The contribution adapter also generates the selected command PATH and
`/usr/share/kedra/catalog.json`. It refuses duplicate configuration paths,
references to unselected outputs, incompatible foundations, unrealized graph
objects and input material that differs from independently hashed release
preflight.

Legacy Rust/JSON catalog contributions retain an explicit reader.
`package-inputs.json` now selects language definitions for current source/release
preflight, with no fallback to retired lists. Contribution compares actual full
foundation/compiler RPM material with preflight before accepting realized outputs.
Local installed catalog checks pass; hosted release qualification and production
publication are separate gates.

## Plan and compose

The Kedra adapter resolves one committed source revision, including target overlay
and home baselines, then observes an exact retained Fedora 44 image of the target's
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
Composition itself performs no DNF transaction.

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
bubblewrap sandbox, inside the foundation image's unpacked root. It does not activate configuration.

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

The context contains deterministic `payload.tar`, verified `foundation.tar`, a
static `Containerfile` and `composition.json`. The manifest binds source/target,
exact foundation/archive, observed seven-column RPM content, typed file provenance,
runtime object receipts and artifact hashes. Export checks foundation symlink
ancestors. Engine paths and generated provenance are reserved. An executable or
library file outside configuration overlays can only assert independently verified
existing foundation bytes and mode; passthrough does not replace it.

The Containerfile uses a digest-derived local foundation tag. A consumer verifies
artifact hashes, loads the retained archive, verifies its exact image ID and
assigns that tag before building with pulls/network disabled. Exported files carry
no signing or installed-system authority.

The separate [native stage](NATIVE.md) (`sysroot derive`) applies the steps the
target's `.kedra` block declares and the composition's source manifest carries:
it compiles GLib schemas, changes declared service links, seeds initial account
defaults and regenerates the target's generic initramfs. Final installed
behavior, boot/update/install/SELinux and protected signing remain independent
gates.

## One image pipeline

Both release targets build the same way, natively on their own runner
(`.github/workflows/release-target.yml`, `image/release/compose.py`):

1. Lock: `sysroot catalog lock resolve` resolves every Fedora RPM once
   ([package delivery](PACKAGES.md#fedora-rpm-lock)).
2. Foundation: `image/Containerfile` and `assemble.sh` install the locked RPMs
   offline over the Fedora bootc base and add the binaries, the committed payload,
   the private agents and Bitwarden. `assemble.sh` checks the Bitwarden runtime
   libraries, validates the niri and Noctalia baselines, names the OS Kedra in
   `os-release`, records the RPM material and runs `sysroot --version`. It applies
   no image step.
3. Packages: the catalog builders and `sysroot catalog build|contribute` realize
   the target's `.kedra` source packages.
4. Compose: `sysroot system compose` with that contribution.
5. Derive: `sysroot derive build` applies the target's declared steps.
6. Trust layer: `image/release/Containerfile` adds the signed-update identity and
   public trust, and runs `bootc container lint`.

The lab's full local build (`KEDRA_LAB_IMAGE=build`) replays steps 1, 2, 4 and 5
without catalog packages or the trust layer.

See [package declarations](PACKAGES.md), [engine operations](ENGINE.md),
[architecture](ARCHITECTURE.md), [actual status](STATUS.md), and the
[composition evidence](../../../../.specs/nix-system-composition/verification.md).

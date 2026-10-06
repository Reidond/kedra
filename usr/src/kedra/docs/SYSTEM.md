# Typed system composition

`sysroot-engine` supplies typed Rust `SystemDefinition`, `SystemFile` and
`SystemContent` contributions. Files have explicit provenance/priority/replacement;
templates use `Argument` input segments to refer to verified package outputs.
Serialized JSON transports those types. Native configuration remains ordinary
systemd/niri/bootc files; there is no new configuration language.

The Kedra adapter resolves one committed source revision, including target overlay
and home baselines, then observes an exact retained native ARM Fedora 44 image.
All desired RPM names must already be present and removed names absent. Engine
outputs must use that same foundation. This first slice does no RPM acquisition
or mutation after selecting the foundation.

```sh
sysroot system plan --repo . --target qemu-arm64 --store /private/engine-store \
  --foundation sha256:<exact-image-id> --definition /private/authored-system.json
sysroot system compose --repo . --target qemu-arm64 --store /private/engine-store \
  --foundation sha256:<exact-image-id> --definition /private/authored-system.json \
  --output-dir /private/new-context
```

`--definition` is optional. Initialize the private store and retain the exact
image with existing `sysroot store init` / `store add-image` commands first.
Commands emit JSON; failures go to stderr. A new output directory is required.
Planning verifies image/object bytes and uses the existing journaled executor for
read-only foundation observations. It does not activate configuration.

For a declared niri main-file baseline, the existing qemu-arm64 adapter additionally
admits one non-executable `config.kdl` source object and reads its verified bytes
back before export. The build store pins this public input; unchanged content
reuses its canonical tree identity across source revisions. Planning predicts
the same receipt without importing or pinning. Source references, executable or
unsupported text content refuse. A source with no niri baseline emits no record.

`/usr/share/sysroot/home-artifacts.json` carries that receipt as an ordinary
mode0644 resolved payload file under the reserved source namespace. The normal
baseline and source manifest remain present. This is a build input, never a
runtime output or duplicate installed store tree; catalog outputs and composition
schemas are unchanged. Context hashing and the existing signed-image chain cover
the record. A receipt establishes content identity, not installed authority.
Explicit ordinary-store unpin/GC after export cannot affect installed home loading
or recovery. [Niri review](TEXT-REVIEW.md) describes strict present-record checks
and legacy absence behavior. Other targets and legacy assembly retain their
existing baseline transport.

The reusable `VerifiedComposition::open` consumer requires an independently
retained identity and owns a private snapshot. It verifies the complete foundation,
canonical plan, exact static Containerfile, config/object trees, references and
actual artifact bytes. It never extracts an untrusted root filesystem on the host.
The manifest is limited to64MiB; payload/foundation limits remain explicit. Use:

```sh
sysroot system verify --context /private/context --expected-identity <identity> \
  --workdir /private/verification-scratch
kedra-lab replay --image composition:/private/context \
  --composition-identity <identity> --target qemu-arm64 \
  --output <typed-alias> --program bin/program -- argument
```

`kedra-lab replay` is part of the existing sanctioned harness. It loads the retained
archive, checks native ID/observations/paths and inherited ONBUILD/volumes, then
builds only the verified static context with pulls/network disabled. It runs the
selected typed output directly in that image. Cache bindings tie image ID,
identity, payload, foundation and daemon together; owned interrupted work resumes
under a per-context lock. Foreign tags/unknown state refuse.

Composition requests default to no source overlay and require the separate identity.
Working-tree/binary overrides and VM composition are refused. Ordinary lab-tools
adaptation is separate and networked; the system-profile composition unit case
uses the static image directly. This does not confer installed signing authority.

The context contains a deterministic config/runtime `payload.tar`, verified
`foundation.tar`, static `Containerfile` and `composition.json`. The manifest
binds source/target, exact foundation/archive, observed RPM content material,
typed file provenance, runtime object receipts and artifact hashes. Export checks
foundation symlink ancestors. Engine paths and generated provenance are reserved.
An executable/library file outside config overlays can only assert independently
verified existing foundation bytes/mode; explicit passthrough does not replace it.

The Containerfile uses a digest-derived local foundation tag. A consumer must
verify artifact hashes, load the retained archive, verify the resulting exact
image ID, and assign that tag before building with pulls/network disabled. Local
Kedra image construction and installed behavior use the existing container
harness; exported files carry no signing or installed-system authority.

This is static composition over an already assembled foundation. The separate
[native stage](NATIVE.md) compiles GLib schemas, changes declared service links,
seeds initial account defaults and generates generic QEMU initramfs images.
Changing the foundation RPM lists requires a matching rebuilt foundation;
[PACKAGES](PACKAGES.md) describes declarations and delivery. Native generation
and installed container workflows do not qualify boot/update/install/SELinux
or production signing integration.
See [actual evidence](../../../../.specs/nix-system-composition/verification.md)
and [engine operations](ENGINE.md).

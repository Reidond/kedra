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

This is static composition over an already assembled foundation. Changed GLib
overrides, initramfs inputs, unit files and home baselines can require schema
compilation, initramfs regeneration, service enablement or home seeding. Those
transformations and boot/update/install/SELinux qualification are separate gates.
See [actual evidence](../../../../.specs/nix-system-composition/verification.md)
and [engine operations](ENGINE.md).

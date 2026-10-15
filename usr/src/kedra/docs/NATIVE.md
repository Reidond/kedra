# Native artifacts

Static composition installs declared files and exact package closures. A separate
closed native stage generates the caches and links required by existing Linux
tools. It consumes a verified composition and an independently selected native
derivation identity; no arbitrary command/script field is accepted.

The Rust `NativeStep` API supports:

- GLib: compile the full merged schema directory with strict validation.
- Systemd: concrete unit enable/disable/mask operations and a declared default target.
- Initial skeleton: copy only committed home baselines to `/etc/skel`; refuse
  inherited managed-file deletions that cannot be reconciled safely.
- QEMU initramfs: enumerate kernels in the image, generate generic images, and
  verify storage/crypto plus required graphics/input modules.

The qemu-arm64 release declares its steps once, in
`image/release/native-steps.json` (`{"schema_version": 1, "steps": [...]}`).
`sysroot` derives the image; the test harness is not involved:

```sh
sysroot derive plan --context /private/context --expected-identity <composition-id> \
  --steps image/release/native-steps.json --state /private/derive-state
sysroot derive build --context /private/context --expected-identity <composition-id> \
  --steps image/release/native-steps.json --state /private/derive-state \
  --expected-derivation <native-id>
```

`plan` prints the normalized `NativePlan` and builds nothing. `build` prints
`{image, engine, parent_image, material}`. Both require native aarch64 Linux and a
local Unix Docker socket with BuildKit (the Docker CLI and its buildx plugin).
`--state` is an owner-private directory (created mode 0700 if absent) holding
locks, journals, bindings and verification snapshots; the release composer and
its installed catalog case share one.

## Execution and identity

The steps are implemented in Rust (`sysroot-engine` `native_apply`), not in a
script. The executor is the running `sysroot` binary itself: `build` streams its
own bytes, the plan and a fixed recipe to BuildKit, and the recipe's single `RUN`
bind-mounts them read-only at `/run/sysroot-native/` and runs the hidden
`sysroot derive apply` as root in the image. The mounts leave nothing in a layer.
`apply` refuses to run unless it is root, at that exact executable and plan path,
in an image whose `composition.json` names the plan's parent. Never run it on a
machine.

The native identity hashes the normalized steps, the parent composition identity,
the implementation version, the fixed recipe and `driver_sha256`, which is now the
SHA-256 of the executor binary (formerly the embedded Python driver). A different
`sysroot` build therefore gives a different identity. The release composer
requires `driver_sha256` to equal the pinned preflight `sysroot` artifact.

The build runs with networking disabled (`--network none` and `RUN
--network=none`), no pulls, no host paths, sockets or devices and no privileged
entitlements; its context is streamed from memory and the verified snapshot. The
ordinary package executor stays nonroot. Native dracut configuration is
executable native syntax; the caller must intentionally select those verified
input bytes.

## Receipt and verification

Inside the build, `apply` checks the parent RPM rows and every planned input,
refuses inherited managed baseline files that the committed source no longer
declares (formerly a separate probe of the foundation; the composition image is
the foundation plus the payload, so the result is the same), hashes the whole
root filesystem before and after the steps and refuses any change other than the
declared outputs and selected unit links. It then re-checks the RPM rows and
writes `/usr/share/sysroot/native-receipt.json` with output bytes/hash/mode,
approved unit links, skeleton paths, kernel/module/listing evidence and the
hashes of the image tools it ran (`rpm`, `glib-compile-schemas`, `systemctl`,
`dracut`, `lsinitrd`, `modinfo`). The executor runs on the image's own C library,
which the RPM material binds.

`build` then independently checks the result: owner/identity/parent labels, that
the image's filesystem layers strictly extend the exact parent's, and a readback.
The readback runs the same executor in an offline throwaway BuildKit stage on the
final image (`sysroot derive observe`), which re-reads every recorded artifact,
kernel, initramfs listing and tool hash and exports only a report; the host then
validates the receipt against the plan and compares the full RPM rows with the
composition's foundation observation. Generated files/receipts are single-link;
inherited immutable foundation files may have valid hardlinks. The parent static
composition image is realized the same way (foundation load with archive name
checks, os-release/RPM/path/passthrough probes, offline build), also with a
layer-lineage check against the foundation.

Results are published through a journaled transaction: a nonce-owned pending tag,
a durable binding in `--state`, then `kedra-native:<identity>`. A repeated `build`
re-reads the bound image instead of trusting the binding; a final tag without a
binding, foreign labels or a changed daemon refuse.

This stage currently consumes inherited verified dracut configuration; changed
dracut input support requires a separate versioned native input boundary rather
than broadening `/usr/lib` writes. Initial skeleton generation never applies
changes to an existing live home.

Force-killing a consumer can leave its private verification snapshot because
destructor cleanup cannot run. Managed snapshots carry durable locked leases;
explicit `sysroot system recover --workdir <state>` validates and collects
recognized abandoned copies while preserving active, stopped or unsafe entries.
This snapshot cleanup does not retire native image journals/tags: those remain for
the public native retry. The four native publication-window retry cases remain
unqualified; see [D5 evidence](../../../../.specs/nix-cache-recovery/verification.md).

Container generation/content and systemd/default/user workflow checks are separate
from kernel boot, Secure Boot/SELinux, installer/update and production signing.
The container harness obtains derived images through `sysroot derive build`
(`KEDRA_LAB_SYSROOT`). Actual coverage and failures are in
[native qualification](../../../../.specs/nix-native-artifacts/verification.md).
Package sources and delivery are explained in [PACKAGES](PACKAGES.md).

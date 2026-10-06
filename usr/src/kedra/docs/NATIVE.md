# Native artifacts

Static composition installs declared files and exact package closures. A separate
closed native stage generates the caches and links required by existing Linux
tools. It consumes a verified composition and an independently selected native
derivation identity; no arbitrary command/script field is accepted.

The Rust `NativeDefinition`/`NativeStep` API supports:

- GLib: compile the full merged schema directory with strict validation.
- Systemd: concrete unit enable/disable/mask operations and a declared default target.
- Initial skeleton: copy only committed home baselines to `/etc/skel`; refuse
  inherited managed-file deletions that cannot be reconciled safely.
- QEMU initramfs: enumerate kernels in the image, generate generic images, and
  verify storage/crypto plus required graphics/input modules.

Rust declarations serialize to JSON transport for the existing harness:

```sh
kedra-lab derive-plan --image composition:/private/context \
  --composition-identity <parent-id> --native-plan /private/native-plan.json
kedra-lab derive --image composition:/private/context \
  --composition-identity <parent-id> --native-plan /private/native-plan.json \
  --derivation-identity <native-id>
```

The fixed recipe/driver and normalized declarations are hashed into the native
identity. Tools execute as root only inside an isolated harness image build, with
no network, host paths/sockets/devices or privileged entitlements. The ordinary
package executor stays nonroot. Native dracut configuration is executable native
syntax; the caller must intentionally select those verified input bytes.

The receipt records output bytes/hash/mode, approved unit links, skeleton paths,
kernel/module/listing evidence and tool hashes. The harness independently checks
actual output bytes, unchanged RPM material, exact parent filesystem layers,
ownership/cache binding and unrelated filesystem changes. Generated files/receipts
are single-link; inherited immutable foundation files may have valid hardlinks.

Existing static schema1 replay remains unchanged. This stage currently consumes
inherited verified dracut configuration; changed dracut input support requires a
separate versioned native input boundary rather than broadening `/usr/lib` writes.
Initial skeleton generation never applies changes to an existing live home.

Force-killing a consumer can leave its private verification snapshot because
destructor cleanup cannot run. Managed snapshots now carry durable locked leases;
explicit `sysroot system recover` validates and collects recognized abandoned
copies while preserving active, stopped or unsafe entries. This snapshot cleanup
does not retire native image journals/tags: those remain for the public native
retry. The four native publication-window retry cases remain unqualified; see
[D5 evidence](../../../../.specs/nix-cache-recovery/verification.md).

Container generation/content and systemd/default/user workflow checks are separate
from kernel boot, Secure Boot/SELinux, installer/update and production signing.
Actual coverage and failures are in [native qualification](../../../../.specs/nix-native-artifacts/verification.md).
Package sources and delivery are explained in [PACKAGES](PACKAGES.md).

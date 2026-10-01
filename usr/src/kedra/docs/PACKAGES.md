# Package declarations and delivery

The current system has two package sources. Fedora is the foundation; it does not
require every additional application to be distributed as an RPM.

| Kind | Declaration | Built/installed into | Delivered to the machine |
|---|---|---|---|
| Fedora system/desktop packages | `image/packages.list`, plus `image/targets/<target>/packages.list`; removals in `image/remove.list` | DNF installs RPMs while assembling the foundation image | Complete signed Kedra image through bootc |
| Engine-built packages | Typed Rust `BuildGraph`/`BuildNode` recipes; serialized graph passed to `sysroot build --plan` | Verified ordinary executable/library trees in `/usr/lib/sysroot/store/<object-id>` | Selected runtime closure and native config included in a composed image |

Fedora package names include niri, Firefox, PipeWire and NetworkManager. Add a
shared package to the shared list or a machine-only package to its target list.
The existing image build resolves/install/upgrade steps and records actual RPM
content material. Changing a list requires a newly built matching foundation;
the engine composer refuses missing required packages or present removed packages.
It does not perform a DNF transaction after selecting that foundation.

Engine recipes describe explicit source, builder/runtime images, dependencies and
commands. Their output is not automatically an RPM. Rust `SystemDefinition.outputs`
selects output IDs for composition; configuration templates refer to their exact
store addresses. Fedora RPMs retain their usual `/usr/bin`, `/usr/lib` and other
filesystem paths. Engine packages keep separate hashed store paths; profiles,
`sysroot run` or service/config references select those paths. They do not
automatically overwrite the foundation's global executable/library paths.
The library API is implemented and tested; a production package
collection replacing the existing Fedora lists is not claimed.

Existing sysroot/helper, bundled agents and Bitwarden inputs also enter the image
through their reviewed binary/archive build paths. They are not all Fedora RPMs.
Those sources remain explicit rather than being captured from the workstation.

At deployment time the machine receives a whole signed OCI image, not a sequence
of live `dnf install` calls. The installed helper verifies the selected image and
bootc stages it for the owner's chosen reboot. Live home/data remain independent.
Connecting the new composer/native stages to production release signing is a
separate qualification gate; current independent build/replay tests do not imply
that this integration is already shipped.

See [engine recipes](../crates/sysroot-engine/README.md),
[composition](SYSTEM.md), [architecture](ARCHITECTURE.md) and [actual status](STATUS.md).

# Nix-style engine delivery charter

The owner authorized five concrete follow-on deliverables, each as its own PR,
using Astra subagents: derived-image boot, signed-pipeline integration, real Rust
catalog, independent consumer reuse and trusted cache/recovery. This continues
the retained [research proposal](../../../../.specs/nix-recreation/proposal.md)
and the four implemented draft layers through PR32/3ba7d1b.

Goal: one personal Kedra OS can consume engine-built packages and native artifacts
through the existing signed bootc workflow; a second generated project can use
the same library/catalog with independent definitions and policy. Actual boot,
install/update/rollback, application behavior and safe recovery establish success.
Kedranix remains read-only reference. No Nix language/nixpkgs compatibility or
bootc replacement is part of these five PRs.

The causal claim is that connecting verified store artifacts to the established
OS pipeline can provide these workflows without replacing deployment authority
or inventing a language, daemon or fleet service. The concrete consumers are the
owner's one current Kedra target and one generated independent consumer. No
unmeasured speed, incident frequency or percentage improvement is asserted.

Keep current FHS source layout, Fedora RPM foundation, ordinary nonroot package
execution, writable home, narrowly scoped installed helper and isolated main-only
production signer. Initial native release integration is qemu-arm64; desktop
retains its existing assembly until native x86 behavior is independently qualified.

Alternative retained: keep existing assembly and use the engine only for private
package environments. It already works but does not satisfy the requested new OS
and catalog delivery. Full NixOS/store-based OS replacement is deferred because
it would add separate boot-loader, installation and persistent-state authority.

Falsifiers/gates: if generated boot artifacts do not boot with required firmware
trust/SELinux, repair the bounded native path before release wiring; never weaken
security. If a catalog needs undeclared host paths/network or arbitrary FHS
replacement, narrow its supported package contract. If authenticated cache data
can publish before policy/root verification, reject that design.

Main-only production signing/publication requires reviewed branches on main.
The five draft PRs do not authorize merging themselves. Disposable fixture trust
can qualify normal install/update/rollback behavior; protected production-secret
resolution and live stable publication remain distinct post-merge observations.

Implementation and test changes are reversible through the stack. Signed releases
and persistent-data transitions carry independent operational consequences and
must preserve recovery/rollback ordering. No current workstation installation,
disk enrollment, home apply, production-key capture or automatic reboot.

See [topology](../../../../.specs/nix-delivery/topology.md),
[execution](../../../../.specs/nix-delivery/execution-plan.md) and
[gates](../../../../.specs/nix-delivery/roadmap.md).

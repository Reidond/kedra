# Derived boot qualification

Source starts from PR32/3ba7d1b; D1 implementation is frozen before runtime.
Only public CLI/manual and the existing native QEMU harness qualify behavior.

Current source gates: pass — scoped Rust formatting, full workspace formatting/
Clippy, assigned Python Ruff and build-disk Bash syntax. Read-only Astra source
review found one qualification gap: the fixed guest observer cannot identify a
circular final fixture digest itself. Disposition: for every actual cold/warm
boot the host compares its bootc image reference/digest and native provenance
with the retained image.json's exact fixture ID/imported manifest/native tuple.
The observer's passed flag alone never qualifies lineage.

| Case | State | Evidence |
|---|---|---|
| DB-T1 public refusal | not-run | Complete tuple/target/overlay/binary override cases pending. |
| DB-T2 conversion/material | not-run | Actual immutable fixture and Podman/disk conversion pending. |
| DB-T3 generated initramfs boot | not-run | Host matching reference/digest/native tuple plus guest BLS/kernel/initrd evidence pending. |
| DB-T4 security/desktop | not-run | Actual firmware/lockdown/SELinux/AVCs/Metal/screenshot pending. |
| DB-T5 restart/preservation | not-run | Named instance warm restart and owned cleanup pending. |
| DB-T6 standard gates/publication | partial | Format/Clippy/Ruff/syntax pass; source commit/release binary/E2E/PR pending. |

No new image, VM boot, installation, signature admission, production release or
update qualification is claimed by source implementation. Runtime records stay
under ignored retained artifacts, with pinned controller/script hashes.

## First actual current-desktop attempt

Pinned current sourcefcfae2b composes20 files in76.23s, native planning18.09s and
eight public refusal cases pass. Existing retained runtime checks pass.
Composition0f584dba00438433b4d600268b1e0876ded7ed84b4f8b2e0783255263343dc46;
planned native identity7598286845b52a928bc273a1fb2707b19d002876f4955cb5dbafe4d1a305bea8.

Actual generation completes but admission fails305.03s: enabled greetd is solely
an Alias=display-manager.service link, while receipt collection/validation require
a link named greetd.service. Actual image shows all three selected services
enabled; alias omission is a native model/collector defect, not a failed systemd
operation. Keep greetd declared and repair precise alias semantics before retry.
Owned failed journal/tag/image remains diagnostic; diagnostic container removed.
No disk/VM was created and no boot/security/desktop pass is claimed.

Safe evidence: `target/nix-delivery/derived-boot/attempt1-summary.json` and
`native-enable-diagnostic.json`. Frozen source/binary hashes stayed unchanged.
Default state bytes are unchanged, but its recorded QEMU/TPM PIDs are absent and
initial liveness was not captured. Therefore preserved running-state is not
asserted; this qualification invoked no VM commands. Capture actual process
identity/liveness before the next named-VM attempt.

Correction source gates pass: selected-target aliases are collected/validated
only as valid top-level same-type links to declared enabled units; nested
dependency links retain matching names, removals require selected disable targets,
and default.target cannot stand in for enable evidence. Read-only Astra review
found no further concrete defect. Full workspace formatting/Clippy/release build
and extracted-driver Ruff pass; runtime requalification is pending. The fixed
observer additionally retains bounded root AVC observations for actual review.

Actual alias-only regression after correction on ada24e6: enable174.52s and
disable148.47s both pass. Independent readonly systemctl reports enabled/exit0
with display-manager→greetd versus disabled/exit1 and alias absent. Disable
receipt retains the exact selected removed_symlink target. Pinned invocation
hashes match before/after; `attempt2/alias-results.json` retains evidence.
These systemd-only checks have no requested kernels and confer no VM boot gate.
Full current-source graphical composition/generation is now running serially.

## Second actual graphical attempt

Current ada24e6 composition62.25s, full native plan14.09s and corrected-source8/8
refusals pass. Full graphical derivation212.72s passes with native image
9085959da3cd38e0739e3f1de391beba71d6ef9c108b17b55840f7c6f897f126,
identity672001cf6c000970cdfce80a78f6d2c8047c884a91a845979eebba77f06db330,
12 artifacts and actual kernel7.2.7-200.fc44.aarch64/required drivers.

VM fixture adaptation, immutable layer/material readback and imported Podman
metadata/material pass. Disk installation then fails487.30s because unsigned
fixture admission requires a signature. Inherited `enforce-container-sigpolicy`
and production reject/sigstoreSigned policy bytes remain identical across9d6,
native output and fixtureb764140e…; no bypass/weakening was attempted.
No disk, boot or screenshot was produced. Exact owned builder/output volume
cleanup passes; default state/observed absent-PID baseline is unchanged. Evidence:
`attempt2/{summary.json,inherited-trust.json,disk-build-failure.log}`.

This refutes the scout's assumption that inherited unsigned fixture disk install
could proceed. D1 now needs a scoped disposable signed-fixture producer and strict
normal consumer admission. Existing BIB uses an ID-only image in its private
`[overlay@/run/osbuild/containers/storage2]` graphroot, so a named repository scope
alone cannot match. The fixture-only addition to that exact scratch-store scope
requires its generated key and signedIdentity exactReference to a preselected
unique name:tag. Existing production docker entries/storage fallback/key/identity
and install enforcement remain unchanged. A separate named ordinary builder-store
scope supports strict pre-BIB verification. No circular final digest in policy,
insecure consumer rule or consumer override. Producer signing creates signatures
but is not verification evidence; actual installed-version admission/BIB is pending.

## Disposable signed-fixture correction

The implemented route keeps all target production trust bytes unchanged and
places the fixture public key/policy only in a separate disposable buildroot.
Its exact ordinary/BIB image-ID rules require the unique signed reference.
Strict default-policy copy probes with Podman5.8.7/Skopeo1.22.3 refuse unsigned
and wrong-key-only images and admit allowed-key-only images, including BIB's
ID-only additional-image-store transport. Keys exist only in builder tmpfs and
are deleted before BIB. The final target manifest and buildroot manifest have
separate digest-pinned references; receipts retain both plus policy/key/recipe
hashes. Probe evidence: `signature-probe/f53d011c3032bdf1b7c4d05a/summary.json`.

Source is frozen for full signed BIB and cold/warm boot qualification, which
remain not-run. Targeted workspace/compiler, Ruff and Bash checks pass; these
do not establish installation or boot.

Third attempt on6c38ab05 passes current composition78.25s and native274.76s,
with twelve artifacts and kernel7.2.7. Disk preparation fails120.36s before
signing/BIB: the closed QEMU Docker context omitted `sign-fixture.sh`.
The correction adds exactly that allowed input and hashes the context ignore
file into the builder cache/receipt. No policy, native bytes or verifier changed.
The retry retains the exact6c38ab05 image source and pinned controller, with the
corrected disk-recipe revision separately recorded; no newer image-source claim.

Two earlier failed-attempt foundation archive copies are retired after fresh
full SHA-256, UID/single-link/type/size and retained-manifest checks. The active
context, canonical store, manifests/payloads and owner media remain. APFS/active
build allocation means observed free space did not increase; no reclaimed-byte
claim. Exact evidence: `prior-context-retirement.json`.

Retry1 reaches the signed helper after corrected builder/import/material checks,
then fails322.81s before signing matrix/BIB because the builder lacks `cmp`.
The correction uses its existing SHA-256 tool for the four exact-byte
comparisons; no package or trust-policy addition. Manual read-only inspection
of actual builderf07f2791 confirms every remaining external helper command is
installed. The temporary inspection container is removed automatically.
All private keys were confined to builder tmpfs; failure cleanup removes the
owned builder. No disk/VM result is claimed. Retry evidence remains
`attempt3/vm-image-retry1.*` and the captured public builder logs.

## Qualified signed disk and cold/warm boot

Retry2 passes the full strict default-policy signature matrix in both actual
source layouts, then BIB/install/converted-disk/host-receipt validation in806.088s.
Image source remains6c38ab05, disk recipee19113d and pinned controllerf235c2c6;
this is not a newer image-source claim. Producer normalization changes the
manifest while preserving config/native material; both digests are recorded.
Private keys are independently absent before BIB. Target production trust bytes
stay exact; final signed manifestf56c2d27… and disk SHA26a24900… are retained in
`attempt3/qualification-summary.json`. Minimum free34.219GiB exceeds the30GiB
floor; the32GiB guard never fires. Owned builder/output volume cleanup passes.

Cold30.119s/guest19.322s and warm22.113s/guest18.745s pass with distinct boot IDs.
Host comparisons bind booted reference/digest, native tuple/receipt and production
trust to image.json. Running kernel7.2.7, selected vmlinuz/generated initramfs,
three loaded drivers, Secure Boot/lockdown and enforcing targeted SELinux pass.
Actual niri VirGL/ANGLE Metal, all session doctor checks, settings screenshots
and Meta+Return opening foot pass. Graceful cold stop passes4.039s. Final backing
disk hash and default state/container/engine remain exact; default native PIDs
retain their observed absent baseline. The new named VM remains running.

AVC capture is complete but includes bootupd_t permissive-domain label notices
and chcon MAC_ADMIN denials; no AVC-free claim. Public trust presence is not
enrollment/production identity qualification. `sysroot update status` stops at
the fixture sudo boundary. Optional vm logs fails on unprivileged bootc status;
the follow-up uses the fixed root-owned current-boot report without root access,
with its own script revision and public workflow verification pending.
Production signing/update/install and other hardware remain separate gates.

Corrected public vm logs passes2.032s on diagnostic script1f32337, preserving
the separately pinned image/disk/controller revisions. Captured bootc JSON
exactly matches the current warm observer; user journal, serial and2560×1600
display inventory are readable. No root privilege is added. Earlier permission
failure stays preserved; `attempt3/logs-host-verification.json` records the
public workflow, full before/after hashes and unchanged default resources.

DB-T2 passes actual declared-artifact corruption: a privately labelled offline
child flips one initramfs byte while retaining size/mode/receipt/provenance.
Controller-owned observer refuses the immutable child for native byte mismatch;
the original fixture passes before/after against its independent receipt.
Exact owned child/tag/alias/containers are removed; controller/default/VM state
hashes stay exact. Evidence:
`attempt3/artifact-tamper-059885477dda5668e44a9e88/summary.json`.
All six local case groups pass; new-source draft/CI readback is next.

Publication: gh-stack submits draft [PR33](https://github.com/Reidond/kedra/pull/33)
at8a8796c954c8e8dbce09e393ae69a198ea06aa9e above exact PR32 branch. Local/remote
head and immediate-parent ancestry match; all lower layers remain unchanged.
New-source Actions are running, not claimed passed. This documentation follow-up
records observed publication without changing the qualified implementation.

# Verified status

The owner’s current policy is **automatically signed GHCR images only**. At 00:00 UTC, changed inputs must pass public validation, isolated OCI signing and strict verification before stable publication. No human approval or manual signing action is required. Unchanged inputs publish nothing. Local ISO construction remains on demand and never uploads.

The automatic production changed-image and no-change paths are now verified. Run [34746729066](https://github.com/Reidond/kedra/actions/runs/34746729066) at main `5dea673f6ebd3a86c44797517889e5a80ba5e78a` completed build, isolated sign-image and publish-stable successfully without a reviewer gate. Both `ghcr.io/reidond/kedra-desktop:stable` and immutable run tag `run-34746729066-1` resolve to `sha256:acafed578d8431d204c1ab0d20c52202c6bbb938ccc4af31efc2ef2737fd7c14`.

Publication artifact `10313929124` independently matches ZIP SHA-256 `122832f43653688aa139bcb9cabf15c656f4c8ad1497c158ad64c54fdebfea6c`. Its receipt is `verified`, with `previous_digest: null`, and binds the exact main source and published digest. The verified image identity is protocol 2, desktop/Fedora44/x86_64, rank `(1,10,1)`. Remote Docker readback independently confirmed both tags. `KEDRA_RELEASES_ENABLED=true`; environment 21492153153 retains only its main branch restriction, with administrator bypass disabled and no required reviewers.

Repeat run [34747330145](https://github.com/Reidond/kedra/actions/runs/34747330145) at the same exact main source succeeded after verified stable-image comparison and package preflight. Changed-image construction, signing and stable publication were all skipped; the run produced zero artifacts. Independent Docker readback confirms stable remains at the same signed digest. This demonstrates an actual production no-change run, not only a fixture result.

## Adwaita desktop source configuration

The `codex/adwaita-desktop` changes from `ca3ce333ac33fa17e81f6c92610e4cdca802018e`
implement [GNOME HIG-informed desktop defaults](DESKTOP.md): a dark Adwaita-like
Noctalia shell around native light/default GTK applications, a full-width top bar,
Adwaita fonts/icons/cursor, rounded niri windows and familiar overview, launcher,
lock and screenshot shortcuts. Existing user preferences remain authoritative;
the Noctalia home projection still contains only its three supported safe fields.

On 2026-09-13, native validation against the cached signed production image
`sha256:acafed578d8431d204c1ab0d20c52202c6bbb938ccc4af31efc2ef2737fd7c14`
passed in isolation: niri 26.04 and Noctalia 5.0.1 accepted the new configuration
without warnings, and Noctalia's full effective export was inspected. The lock
shortcut uses the verified v5.0.1 command `noctalia msg session lock`. GLib 2.88.3
strict schema compilation passed with schemas 50.1; inherited deprecated-path
warnings remained. All nine GSettings defaults read back correctly, a temporary
user `prefer-dark` override/reset worked, and fontconfig resolved Adwaita Sans/Mono.
The five explicitly listed Adwaita/schema packages were already present in that
cached image. Manual palette calculations checked light and dark primary roles
at contrast ratios above 4.5:1; this is not a full accessibility audit.

Shell syntax, Git whitespace, Rust 1.98.1 formatting, Clippy with warnings denied
and release build passed. The Windows E2E command succeeded but executed zero
cases, so it provides no new Linux behavior coverage. A new image build,
graphical GTK/Noctalia rendering and VM/physical desktop qualification are
**not-run** for these changes. The earlier published image and ISO results below
remain evidence for their original source only. See WL-20260913-07 in the
[worklog](../worklog.md) for this source task's checks and next step.

## Removed legacy downloads

At the owner’s explicit direction, GitHub Release IDs 385117864 (r1), 385328126 (r2) and 385128562 (legacy channel), including all 31 uploaded assets, were deleted. The remote Releases list is empty. Their source Git tags remain; no source tag was removed. Historical signed-image/installation results remain in Git history and worklog, not as current download availability.

The owner confirmed on 2026-09-13 that nobody installed r1/r2. No deployed-system migration is required; earlier migration blockers assumed installations that do not exist. Fresh installations enroll directly in the signed GHCR workflow. Historical disposable-VM results remain valid only within their recorded scope.

## Verified implementation and remaining qualification

At exact f52c9e77e17eb029a6b25c5a23807ef655484795, workspace runs 34714337528 and 34714339961, direct GHCR 34714337561, desktop 34714337517, legacy signed update 34714337526 and home transition 34714337529 pass. Direct-GHCR artifact 10304408023 independently matched ZIP SHA-256 46ba4dda3f3a3ab5b099274486d3121f3a5386c8bb49fb31659bf0ca6f6f8a2d.

The direct-GHCR VM test covers actual v2 enrollment/check/stage and signed A/B/A boots, signature/scope refusals, replay/equivocation, offline failure, pending preservation, idempotence, identity-health retained rollback, persistent data, hold and resume. It uses a disposable local TLS registry and generated keys; it does not publish to production GHCR.

Production 34715490862 failed because the public build could not read an environment-scoped fingerprint variable. The subsequent scope fix retains source-key validation, independent signer-environment comparison and signer-output binding in the publisher. Production run 34717057402 at 7e923d4907e3b5caa28260983a0e7ef884a3dd87 passed its public build and was cancelled before signing for the final policy change. Queued schedule 34728220456 was also cancelled. Neither cancellation is successful signing or stable publication.

The shared bootc compatibility contract, exact image identity/material checks and local CLI/OpenSSL interoperability pass their recorded checks. Automatic production signing/publication, the no-change repeat and full local ISO construction with diskless smoke all pass. Deterministic registry races/interruption and actual OCI-platform mismatch remain unqualified. Physical/Secure Boot, an installed-system forward update using production images and key rotation are separate boundaries. Publication does not mean a workstation was staged, rebooted or installed.

The local builder completed successfully in Ubuntu WSL using Podman 4.9.3 and Skopeo 1.13.3. It produced `/var/tmp/kedra-local-iso.iM7J7gze/installer/kedra-desktop-44-acafed578d8431d2.iso`: 2,858,758,144 bytes, SHA-256 `b923a289347a878678dd49222c8d433f609eb0830a7e35f309e0ecd9d776660c`. `sha256sum -c SHA256SUMS` passed. `installer.json` binds the signed production digest and source above and records `diskless_smoke_passed: true`; the smoke check passed offline signed-payload verification and Anaconda startup without disks attached.

The fresh encrypted offline installation passed in a private Xvfb session with only the verified ISO and two generated 64 GiB QCOW2 disks attached. Serial-mapped `vda` (`KEDRA-INSTALL-ONLY`) was selected; `vdb` (`KEDRA-KEEP-DATA`) remained unselected. A generated administrative account was configured; root remained locked. The Anaconda Complete screenshot was independently reviewed. Installer ACPI shutdown exited QEMU successfully, and `qemu-img compare` confirmed the unselected sentinel disk was unchanged.

ISO-free second boot reached LUKS unlock and the niri/Noctalia desktop. The generated account and sudo worked. Exported native checks confirmed the expected signed digest/source/trust, LUKS2 on `vda3`, enforcing SELinux, read-only `/sysroot`, writable home with successful write/read/remove, required `sysroot doctor` checks passing, and no partitions or mounts on `vdb`. A separate awake-session Noctalia GUI Shut Down check passed from an ISO-free boot: the visible Shut Down tile was selected, QEMU promptly exited with code 0 without an injected ACPI event or terminal poweroff, and the sentinel comparison passed. An authenticated `sudo -n systemctl poweroff --no-block` recovery check also exited cleanly with the sentinel unchanged.

An earlier QMP `system_powerdown` sent a short ACPI power key: the guest journal records `Power key pressed short` followed by S3 suspend/resume, not a shutdown attempt. The display stayed inactive after virtual-GPU resume; that VM was deliberately stopped with QMP `quit`, a forced host stop, and subsequently recovered to the installed desktop. VM suspend/resume remains unqualified. This does not establish a Noctalia or polkit shutdown defect.

The complete ISO is also available at `C:\Users\reido\Downloads\kedra-desktop-44-acafed578d8431d2.iso`; its SHA-256 was independently verified against the value above. Only the ISO was copied to Downloads. Generated password/passphrase files were removed, and no QEMU system process remains running.

The Ubuntu WSL build initially reported a Podman cleanup warning because `netavark` was missing. Installing that helper restored cleanup; the leftover inspection container was removed. The local ISO and smoke passes above are unchanged. Global sudoers hashes matched before and after the isolated build.

The earlier signed candidate 34697167136 at b4e9f78 passed its exact fresh encrypted offline installation and ten qualification checks. That historical installation result does not qualify installation of the newly published GHCR-only image or restore deleted Release assets.

[INSTALL](INSTALL.md) describes local media construction; [UPDATES](UPDATES.md) describes direct updates and recovery. The [worklog](../worklog.md) retains exact historical evidence and the next operational step.

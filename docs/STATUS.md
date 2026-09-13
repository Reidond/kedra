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

## Native GTK and Qt integration follow-up

The follow-up from `4d1d0d2888a43c3ea2cdf281ab9be44ef263a582` adds KDE platform
integration and native Breeze styles for Qt 5/6, KDE Qt Quick Controls styles,
Breeze icons and system `kdeglobals` defaults. The systemd user environment
selects the KDE platform theme while retaining explicit user choices.
GTK/libadwaita Adwaita defaults and niri's desktop/portal identity remain intact.
See [desktop integration](DESKTOP.md#gtk-and-qt-applications).

Disposable native Xvfb application probes pass for Qt 5.15.18 and Qt 6.11.2:
both resolve Breeze, Adwaita Sans 11, Breeze icons and the document-open icon.
The measured KDE packages are Plasma integration/Breeze 6.7.5, KF6 styles/icons
6.30.0 and the KF5 desktop style 5.116.1. Fedora systemd 259.8 environment
generation defaults to `kde` and preserves an explicit `qt6ct` value. GTK 3
also resolved Adwaita/Adwaita Sans 11 in a real Xvfb client without the GNOME
daemon. Both Qt versions honored a personal `kdeglobals` override to Fusion and
Adwaita Mono 12. GTK 3 Xwayland uses the static settings fallback and requires application restart after
changes. An isolated GNOME XSettings 50.1 experiment published initial settings
but failed dynamic font propagation outside GNOME; that daemon is not shipped
or enabled by this follow-up.

The desktop VM workflow now prepares real GTK 3 Wayland/Xwayland, libadwaita,
Qt 5, Qt 6 and Qt 6 personal-preference application cases. QMP interaction opens
native file choosers, selects generated text files and checks their returned
content, with screenshots at each stage. The package inventory is captured before
adding test-only bindings. Syntax and whitespace checks pass according to the
implementation worker; new GUI and image qualification remain pending. No
exact-base Actions run existed when this follow-up began.

Workspace run [34758096731](https://github.com/Reidond/kedra/actions/runs/34758096731)
passed at `610da61`. Home-transition run
[34758096745](https://github.com/Reidond/kedra/actions/runs/34758096745) failed
because its fixture still expected niri `gaps 12` after the baseline changed to
8. The correction in `prepare.py` and `home.py` is locally ready and passes
native niri validation. Its signed A/B/A rerun
[34758449303](https://github.com/Reidond/kedra/actions/runs/34758449303) at
`076814d0a68933d0a384b7db02ba4598ffa4fe88` passed the full workflow, including STAGE_B, ACCEPT_B_ROLLBACK_STAGED and ROLLBACK_A_HOME graphical VM markers. Desktop run
[34758096727](https://github.com/Reidond/kedra/actions/runs/34758096727) was
cancelled when the test-only Qt chooser fix at `24c61b7` was pushed. At
`24c61b7a5cfb2310b97d3c3e8439e335dd8b843e`, workspace
[34758255401](https://github.com/Reidond/kedra/actions/runs/34758255401) passed;
desktop [34758255393](https://github.com/Reidond/kedra/actions/runs/34758255393)
failed after candidate build/validation, disposable-disk creation and login,
before the toolkit cases. Artifact `10317828820` shows that the old fixture set
the already-selected light mode, so home staging correctly refused an unchanged
value. A dynamic mode choice is being implemented in `check.sh` and `recovery.py`.
The failure screenshot shows the rendered shell/bar, a cartoon wallpaper and a
large black focused window of unknown identity. The current source adds
the native-validated Noctalia 5.0.1 wallpaper default `color:#222226`, preserving
personal overrides, and adds wallpaper readback plus niri window inventory to
the VM evidence. The [desktop design](DESKTOP.md) links the tagged wallpaper
implementation and example. Later startup inventory identifies the black client
as `xwaylandvideobridge`; niri-specific autostart exclusion is now implemented,
with GUI qualification pending.
See WL-20260913-08 for the
continuing evidence.

At current source `711bdf223120efab675c7fd2dff73618c7573cf1`, workspace
[34759390627](https://github.com/Reidond/kedra/actions/runs/34759390627) passes
with matching head/status/conclusion independently checked. Desktop
[34759390621](https://github.com/Reidond/kedra/actions/runs/34759390621) failed
after passing candidate/disk creation, wallpaper readback and Noctalia
projection/recovery markers: `niri-review.py:118` still expected the historical
`gaps 12` baseline. Artifact `10318453176` records that failure and identifies
the focused 628×716 black tile as `xwaylandvideobridge` (Wayland to X Recording
bridge). The fixture is being corrected. `build/assemble.sh` now adds
`NotShowIn=niri;` to the packaged bridge autostart entry, retaining its package
and manual launcher. New VM checks require no bridge process/window at startup
and after the session workflow; execution of this correction is pending.
Native Wayland portal sharing remains configured; legacy X11 bridge capture is
opt-in and separately unqualified. See [desktop sharing](DESKTOP.md#screen-sharing-and-the-x11-bridge).
the toolkit cases have not run. Home-transition
[34759390618](https://github.com/Reidond/kedra/actions/runs/34759390618) passed
at `711bdf223120efab675c7fd2dff73618c7573cf1`.
The earlier signed home-transition pass does not qualify this exact source or
the new graphical workflows.

Current source `e0e1031a92b189ae30e419ac1211196930445f8c` includes the bridge
autostart exclusion and desktop fixture corrections. Workspace
[34760607440](https://github.com/Reidond/kedra/actions/runs/34760607440) passes.
Desktop [34760607452](https://github.com/Reidond/kedra/actions/runs/34760607452)
failed in the new gtk3-wayland fixture because `toolkit-app.py` imported Gdk 4
before Gtk 3, causing a GI namespace conflict; a fixture correction is underway.
Artifact `10319262013` confirms wallpaper-get, empty startup window inventory,
KEDRA_NIRI_NO_VIDEOBRIDGE_PASS, Noctalia projection/recovery, niri line
review/recovery and portal/keyring/doctor passes. The independently inspected
failure screenshot shows an uncluttered charcoal desktop/top bar without the
black bridge tile. Toolkit GUI behavior remains unqualified. Home-transition
[34760607435](https://github.com/Reidond/kedra/actions/runs/34760607435) passed
at `e0e1031a92b189ae30e419ac1211196930445f8c`.

Latest source `a165312` changes only the GTK GUI fixture and documentation/worklog;
runtime image/home source is unchanged from that passing home-transition run.
Workspace [34761609811](https://github.com/Reidond/kedra/actions/runs/34761609811)
passes. Desktop [34761609814](https://github.com/Reidond/kedra/actions/runs/34761609814)
failed waiting 45 seconds for the first GTK 3 Wayland chooser's selected marker.
Artifact `10319930862` shows the ready app and native Adwaita chooser; GTK 3
theme/icon/backend checks passed. The fixture lexically compares `/var/home`
from `Path.home()` against the `/home` symlink path typed through QMP, a possible
callback failure. The source now uses `samefile(sample)` and exact content
verification, explicitly reports callback failures and captures a post-submit
screenshot plus toolkit journal/result/window inventory on timeout. Disposable
Fedora alias, syntax and whitespace checks pass; the corrected VM rerun is
pending and the exact historical timeout trigger is not yet proven. Its final failure
screenshot follows trap poweroff and does not establish a compositor failure.
The six-case toolkit suite remains incomplete. Passing home-transition at
runtime-equivalent `e0e1031` and workspace CI retain their recorded scopes.

The subsequent desktop run
[34762761532](https://github.com/Reidond/kedra/actions/runs/34762761532) at
`e8f3272` corrects that diagnosis: artifact `10319314507` shows the path correctly
entered, the Open button enabled after one second, stage still dialog and no
callback error. The path-alias comparison was a latent fixture bug, not this
observed timeout. GTK 3's [chooser source](https://raw.githubusercontent.com/GNOME/gtk/gtk-3-24/gtk/gtkfilechooserwidget.c)
debounces location changes for 150ms; QMP had sent Return after 80ms. The fixture
now types the path, waits one second, captures it, then sends one Return and
captures again. That corrected GUI rerun remains pending, with no full toolkit
pass claimed.

Signed home-transition
[34762761530](https://github.com/Reidond/kedra/actions/runs/34762761530) passed
at `e8f32722eff2419c802660c67d77798000978db0`. Latest source
`6cb1b87d794707c6130338ad5bfe4041d50bcc1f` changes only the GUI keyboard timing
and documentation/worklog. Its workspace
[34763954122](https://github.com/Reidond/kedra/actions/runs/34763954122) and desktop
[34763954151](https://github.com/Reidond/kedra/actions/runs/34763954151) are in
progress; home-transition
[34763954121](https://github.com/Reidond/kedra/actions/runs/34763954121) is pending.
The prior signed workflow success does not establish latest-source GUI success.

Desktop [34763954151](https://github.com/Reidond/kedra/actions/runs/34763954151)
at `6cb1b87` passes both GTK 3 Wayland and Xwayland ready/dialog/selected workflows
with exact selected-file content; artifact `10319744781` records the results.
Libadwaita reaches its ready/dialog stages through GTK 4 FileChooserNative and
the Nautilus portal, then times out. The confirmed screenshot shows the first
Return navigated to the directory and selected the 28-byte sample with Open
enabled, but had not confirmed opening it. The source now adds a libadwaita-only
second Return after checking for completion/failure, with an additional
open-confirmed screenshot and unchanged strict file/content assertions. This
follows the [Nautilus 50 chooser](https://raw.githubusercontent.com/GNOME/nautilus/50.0/src/resources/ui/nautilus-file-chooser.blp).
The corrected rerun and remaining cases are pending; no full six-case pass is
claimed. Home-transition 34763954121 remains in progress.

Local Rust 1.98.1 formatting, all-target Clippy with warnings denied, release
build, WSL Bash syntax and Git whitespace pass. The Windows workspace E2E
command passes with zero cases, providing no new Linux behavioral coverage.

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

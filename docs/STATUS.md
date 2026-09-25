# Verified status

The owner's current policy is **automatically signed GHCR images only**. At 00:00 UTC, changed inputs must pass public validation, isolated OCI signing and strict verification before stable publication. No human approval or manual signing action is required. Unchanged inputs publish nothing. Local ISO construction remains on demand and never uploads.

## Current summary (2026-09-25)

- **Production.** `ghcr.io/reidond/kedra-desktop:stable` resolves to `sha256:fe61a11d37b97ca04b58d87cd98fc5e95bb33af69e1c84d0d7fdda26041d7675`, published by release run [36077781275](https://github.com/Reidond/kedra/actions/runs/36077781275) (schedule, run #25) from main `17105c5d57a355852999f0be39c8c3548b9c0c8c`. Its receipt is `verified`. Thirteen signed stable publications exist in total; see the table below. Anonymous registry readback on 2026-09-25 confirms the digest.
- **Desktop source.** PR #14 (Adwaita desktop, adw-gtk3, Noctalia 5.1 compatibility, right-side clock) merged on 2026-09-14 as `17105c5`. On that exact commit, workspace [34820938028](https://github.com/Reidond/kedra/actions/runs/34820938028), desktop [34820938007](https://github.com/Reidond/kedra/actions/runs/34820938007), signed home [34820938085](https://github.com/Reidond/kedra/actions/runs/34820938085), direct GHCR [34820938156](https://github.com/Reidond/kedra/actions/runs/34820938156) and signed updates [34820938031](https://github.com/Reidond/kedra/actions/runs/34820938031) pass. Push release [34820938097](https://github.com/Reidond/kedra/actions/runs/34820938097) published it.
- **Daily images are not VM-tested at their exact digests.** Publication does not wait for the VM workflows; each nightly image differs from the tested source build only in refreshed Fedora inputs.
- **bootc compatibility.** Fedora 44 moved `bootc-1.16.13-1.fc44` to stable on 2026-09-25. The shared contract still qualifies 1.16.10, so builds that resolve 1.16.13 fail closed before signing until the contract is deliberately updated and requalified. PR #15 (`89edf80`) bumps the contract to 1.16.13 and passes workspace, agents, signed updates, direct GHCR, desktop and signed home workflows (runs 36130170372–36130170636); a fresh Anaconda installation with 1.16.13 is not-run.
- **Not qualified:** Secure Boot (all VMs so far booted non-Secure-Boot OVMF), physical hardware, production-image forward update on an installed system, deterministic registry race/interruption, native OCI-platform mismatch, key rotation, VM suspend/resume, scaling/high contrast/accessibility, authenticated agent and Bitwarden use. The local ISO was built and installed only from `acafed57…` (2026-09-13).

## Production publications

| Run | # | Date (UTC) | Source | Published stable digest |
|---|---|---|---|---|
| [34746729066](https://github.com/Reidond/kedra/actions/runs/34746729066) | 10 | 09-13 dispatch | `5dea673` | `sha256:acafed578d8431d204c1ab0d20c52202c6bbb938ccc4af31efc2ef2737fd7c14` |
| [34747330145](https://github.com/Reidond/kedra/actions/runs/34747330145) | 11 | 09-13 dispatch | `5dea673` | none (no-change) |
| [34752129104](https://github.com/Reidond/kedra/actions/runs/34752129104) | 12 | 09-13 push | `9b5005a` | none (no-change) |
| [34792981175](https://github.com/Reidond/kedra/actions/runs/34792981175) | 13 | 09-14 | `ca3ce33` | `sha256:5e2dd1bb589574dac2175e52f643be0b879f2e2ae0a1c4de3a61b409a35f12a9` |
| [34820938097](https://github.com/Reidond/kedra/actions/runs/34820938097) | 14 | 09-14 push | `17105c5` | `sha256:528b0faa074dce510b9a8662693347fbe3547e3a44aa52cb87849867aee31867` |
| [34913413931](https://github.com/Reidond/kedra/actions/runs/34913413931) | 15 | 09-15 | `17105c5` | none: failed closed on a transient HTTP 500 downloading cosign; fetches now retry |
| [35040133877](https://github.com/Reidond/kedra/actions/runs/35040133877) | 16 | 09-16 | `17105c5` | `sha256:c6d1535259d2495b135eb10b892a279550ef52015ee201f4cc0fd6c356e63113` |
| [35166692228](https://github.com/Reidond/kedra/actions/runs/35166692228) | 17 | 09-17 | `17105c5` | `sha256:1dc46b761e246e1c1b91568fdc93b20615f59b14c30bf598e62b86a13cd03927` |
| [35291343086](https://github.com/Reidond/kedra/actions/runs/35291343086) | 18 | 09-18 | `17105c5` | `sha256:f5f99c74b62d0f95a8e584d5ed4597ed936d16def47657c3e11ff9da2fac7201` |
| [35409458319](https://github.com/Reidond/kedra/actions/runs/35409458319) | 19 | 09-19 | `17105c5` | `sha256:9c1ebf0140cf630a9c3c294e8618f0017643f7c8e62daf946877183b99844868` |
| [35479024601](https://github.com/Reidond/kedra/actions/runs/35479024601) | 20 | 09-20 | `17105c5` | `sha256:b3e6c74686ad0deba7b211c66c8057575ce99cfe3984cd3daee7c30a12375f27` |
| [35547977428](https://github.com/Reidond/kedra/actions/runs/35547977428) | 21 | 09-21 | `17105c5` | `sha256:36885fd9c2d1eba6a1bf52ff93e797ae451d06669aaa78559590526a62f718eb` |
| [35672122067](https://github.com/Reidond/kedra/actions/runs/35672122067) | 22 | 09-22 | `17105c5` | `sha256:77374239346a677ec5fc6df30a98570a98ef174369d4fe0942e53ede1501d6b0` |
| [35802122457](https://github.com/Reidond/kedra/actions/runs/35802122457) | 23 | 09-23 | `17105c5` | `sha256:7c64834abd21dc92d1a3a5ac6b8326c5d2a0970446f6f066bc33091b761563b8` |
| [35938559348](https://github.com/Reidond/kedra/actions/runs/35938559348) | 24 | 09-24 | `17105c5` | `sha256:64a118939ba59d43930cc50936c205eada3d0d35f48eb273be17f721c1fe44c8` |
| [36077781275](https://github.com/Reidond/kedra/actions/runs/36077781275) | 25 | 09-25 | `17105c5` | `sha256:fe61a11d37b97ca04b58d87cd98fc5e95bb33af69e1c84d0d7fdda26041d7675` |

Each nightly rebuild is driven by a new `quay.io/fedora/fedora-bootc:44` base digest and, on most days, changed Fedora packages. Every published receipt is `verified` and chains `previous_digest` to the row above it. Immutable `run-<id>-1` tags and signatures are retained; there is no garbage collection.

## Agents, credentials and repository skills

- **Codex** 0.153.4 (x86_64) is bundled under `/usr/libexec/sysroot/agents/codex` from the pinned official archive with Sigstore verification. [test-agents 34695674783](https://github.com/Reidond/kedra/actions/runs/34695674783) and the desktop VM (`KEDRA_R05_IMAGE_RUNTIME_PASS`) pass for runtime selection, profiles and `--version`/`--help`. Authenticated model use, MCP, skills and hooks are not qualified.
- **Claude** is not bundled: `public_preinstallation_approved` remains false pending the owner's Commercial Terms decision. `sysroot claude --runtime user` is the only path.
- **Repository skill discovery (R11):** on 2026-09-07 Codex CLI 0.153.4 marketplace and fresh-profile probes found no Kedra skills at the root or crate cwd. Read the canonical `plugins/kedra/skills/*/SKILL.md` files explicitly; no global installation is authorized.
- **Bitwarden Desktop** 2026.8.0 is bundled at `/usr/lib/bitwarden`. The desktop VM covers native sandbox startup while logged out (`KEDRA_R06_LOGGED_OUT_PASS`) and `SSH_AUTH_SOCK` propagation. Vault login/unlock, key serving, signing approval and Git over SSH are not-run.

## History

The sections below are the chronological qualification record, newest first. Each keeps its original source scope.

### First automatic production publication (2026-09-13)

The automatic production changed-image and no-change paths are now verified. Run [34746729066](https://github.com/Reidond/kedra/actions/runs/34746729066) at main `5dea673f6ebd3a86c44797517889e5a80ba5e78a` completed build, isolated sign-image and publish-stable successfully without a reviewer gate. Both `ghcr.io/reidond/kedra-desktop:stable` and immutable run tag `run-34746729066-1` resolve to `sha256:acafed578d8431d204c1ab0d20c52202c6bbb938ccc4af31efc2ef2737fd7c14`.

Publication artifact `10313929124` independently matches ZIP SHA-256 `122832f43653688aa139bcb9cabf15c656f4c8ad1497c158ad64c54fdebfea6c`. Its receipt is `verified`, with `previous_digest: null`, and binds the exact main source and published digest. The verified image identity is protocol 2, desktop/Fedora44/x86_64, rank `(1,10,1)`. Remote Docker readback independently confirmed both tags. `KEDRA_RELEASES_ENABLED=true`; environment 21492153153 retains only its main branch restriction, with administrator bypass disabled and no required reviewers.

Repeat run [34747330145](https://github.com/Reidond/kedra/actions/runs/34747330145) at the same exact main source succeeded after verified stable-image comparison and package preflight. Changed-image construction, signing and stable publication were all skipped; the run produced zero artifacts. Independent Docker readback confirms stable remains at the same signed digest. This demonstrates an actual production no-change run, not only a fixture result.

### Right-side bar clock

The layout follow-up from `308b03f467a50df91aca20930857122ed471785b` places the
clock in the right-side status group immediately before Control Center and
leaves the center empty. Cached native Noctalia 5.0.1 validation passes without
warnings; full effective export confirms the exact order and no default clock
reinserted. Local 5.1 validation is not-run because that binary is unavailable.
Exact source `b4c942d88f6be1fdfcf21c83267f7afb2fd66e1c` now passes desktop
[34786061737](https://github.com/Reidond/kedra/actions/runs/34786061737), signed
home [34786061738](https://github.com/Reidond/kedra/actions/runs/34786061738) and
workspace [34786061733](https://github.com/Reidond/kedra/actions/runs/34786061733) /
[34786063461](https://github.com/Reidond/kedra/actions/runs/34786063461).
Independent review of artifact `10327410805`'s `vm/desktop.png` and
`vm/settings.png` confirms the clock after battery and immediately before
Control Center, an empty center and no collision at 1280×768. The requested
layout is qualified in the disposable VM. Physical displays/other scaling remain
separate; no user-machine apply or production publication occurred. Earlier
`4d030519` GUI evidence retains its centered-clock scope. See WL-20260914-02.
Home artifact `10327097877` records passing stage B, accepted B/rollback staging
and retained-A home rollback markers at 22:48–22:49 UTC.

### adw-gtk3 follow-up

**Latest exact-source result:** `4d0305194341b702f3e39fb6abba6a1f6b3f29a0`
passes desktop [34784038994](https://github.com/Reidond/kedra/actions/runs/34784038994),
workspace [34784039095](https://github.com/Reidond/kedra/actions/runs/34784039095) /
[34784041029](https://github.com/Reidond/kedra/actions/runs/34784041029), direct
GHCR [34784039027](https://github.com/Reidond/kedra/actions/runs/34784039027) and
signed-update regression [34784039089](https://github.com/Reidond/kedra/actions/runs/34784039089).
Desktop artifact `10325794069` contains 35 PNGs: GTK 3 Wayland/X11 report
adw-gtk3, Adwaita Sans 11/icons and successful exact-content file selection;
libadwaita 1.9.3 retains Adwaita-empty/native light StyleManager behavior; Qt 5/6
KDE/Breeze and personal-font workflows also pass through KEDRA_R07_SESSION_PASS.
Candidate adw-gtk3-theme 6.4-3.fc44/CSS path are verified. Independent screenshot
review confirms rounded GTK 3 controls and preserved native libadwaita.
Signed home [34784039080](https://github.com/Reidond/kedra/actions/runs/34784039080)
also passes, with STAGE_B, ACCEPT_B_ROLLBACK_STAGED and ROLLBACK_A_HOME at
21:57–21:58 UTC. Source and disposable-VM qualification are complete; PR #14
remains open, with no production publication or installation. Physical displays,
high contrast and dynamic GTK 3 Xwayland dark-mode propagation remain separate.
Persisted State.app_version=5.0.1 is preserved at source level; migration of a
record created by the old 5.0.1 CLI into the 5.1 runtime is not-run as an E2E.
The signed A/B/A pass qualifies the current 5.1 managed workflow.
The earlier failures below retain their original source scope.

At `f9d46a79462bb433d3120071ad44e6e2dbe511dc`, desktop
[34782406383](https://github.com/Reidond/kedra/actions/runs/34782406383) builds
the candidate with adw-gtk3-theme 6.4-3.fc44, but refreshed Fedora packages supply
Noctalia 5.1.0-1.fc44. The VM correctly refuses unqualified Noctalia home review
shortly after login, before toolkit cases. This is a compatibility safety guard,
not an adw-gtk3 failure. Native 5.1 export/IPC/override comparison is in progress;
exact-version compatibility support is now implemented from native evidence,
without blanket acceptance or a package pin. Home-transition
[34782406358](https://github.com/Reidond/kedra/actions/runs/34782406358) also fails
on stage-B boot: artifact `10325981426`, stage-b/serial.log line 67, records the
same unqualified-version refusal in `sysroot home init`. Workspace push
[34782406364](https://github.com/Reidond/kedra/actions/runs/34782406364) and PR
[34782409405](https://github.com/Reidond/kedra/actions/runs/34782409405) pass.
Prior full Noctalia 5.0.1 GUI evidence does not qualify the new runtime.
See WL-20260914-01 for the continuation across local midnight.

The source now accepts runtime versions exactly 5.0.1/5.1.0, mapping their
measured CLI version strings to the existing safe projection. Persisted
APP_VERSION/state identity remains 5.0.1, preserving old records; unknown
versions still refuse. The native 5.1 probe passes config/export, IPC dark/light,
settings-path and stopped-writer/restart checks with clean RPM verification.
Local Rust 1.98.1 formatting, all-target Clippy, release build and whitespace
pass. The orchestrator committed/pushed exact source
`4d0305194341b702f3e39fb6abba6a1f6b3f29a0`; workspace
[34784041029](https://github.com/Reidond/kedra/actions/runs/34784041029) /
[34784039095](https://github.com/Reidond/kedra/actions/runs/34784039095), desktop
[34784038994](https://github.com/Reidond/kedra/actions/runs/34784038994), signed home
[34784039080](https://github.com/Reidond/kedra/actions/runs/34784039080), direct GHCR
[34784039027](https://github.com/Reidond/kedra/actions/runs/34784039027) and legacy
signed updates [34784039089](https://github.com/Reidond/kedra/actions/runs/34784039089)
are in progress. No outcome, new GUI qualification or publication is inferred.

The follow-up from `3dd56e9254a5b531d71b4ff7e177cb3c3160f42f` implements official
Fedora 44 `adw-gtk3-theme` 6.4-3.fc44 with GNOME and GTK 3 fallback defaults.
Disposable native GTK 3.24.52 applications on X11 and nested-niri Wayland render
with adw-gtk3/Adwaita Sans 11. New Wayland clients honor a personal adw-gtk3-dark
GSettings selection and reset. Strict schema compilation passes with inherited
deprecated-path warnings. The runtime probe records ADW_GTK3_RUNTIME_PASS and
its disposable container was removed.

Libadwaita 1.9.3/PyGObject 3.56.3 initializes Adwaita-empty with dark=false,
high_contrast=false and color_scheme=0 in a real Xvfb application. Plain GTK
4.22.5 can use the package's GTK 4 CSS through the shared theme setting; all
RPM-owned assets remain intact. Qt/Breeze is unchanged. The VM fixture now
checks candidate/fixture RPM/CSS consistency, exact GTK 3 theme on both backends
and native Adw StyleManager behavior; the final Actions VM run passes above.
See [desktop guidance](DESKTOP.md#gtk-and-qt-applications) for package/upstream
sources and explicit dark-variant selection. WL-20260913-10 and WL-20260914-01
are completed; earlier full Adwaita evidence remains scoped to its original theme.

### Noctalia Greeter evaluation

Evaluation at `7f76857f0e903b61891f7bf1584bece98c5c73cc` recommends Noctalia
Greeter 1.5.0 as a separately qualified follow-up. [PR #14](https://github.com/Reidond/kedra/pull/14)
remains open with its existing greetd/tuigreet implementation and green checks;
no Greeter implementation, merge or production publication occurred. The
[desktop evaluation](DESKTOP.md#noctalia-greeter-evaluation) records sources and
the proposed pinned Actions build using official Fedora dependencies, static
administrator-owned Adwaita styling and preserved PAM/keyring/niri integration.
Greeter login, session selection, TTY recovery, scaling and SELinux checks are
not-run. Runtime-directory handling and portable compiler flags require review
before adoption. This is a completed evaluation, not a blocked implementation.

### Adwaita desktop source configuration

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
cases, so it provides no new Linux behavior coverage. At that initial milestone,
new-image and graphical checks were not-run; the later exact-source native/VM
results follow below. Physical qualification remains separate. The earlier
published image and ISO results remain evidence for their original source only.
See WL-20260913-07 in the
[worklog](../worklog.md) for this source task's checks and next step.

### Native GTK and Qt integration follow-up

**Current exact-source result:** `71cb8c9158871f82cb38f1fa59df2e7260dd51cf`
passes workspace [34769161713](https://github.com/Reidond/kedra/actions/runs/34769161713)
and desktop [34769161693](https://github.com/Reidond/kedra/actions/runs/34769161693).
Artifact `10321492608` contains 35 PNGs and passing wallpaper-source,
no-video-bridge, managed Noctalia fallback-warning absence at startup/after
restart, all six native GTK 3 Wayland/X11, libadwaita and Qt 5/6 KDE-dialog/user
font workflows with exact selected-file content, KEDRA_TOOLKITS_PASS and
KEDRA_R07_SESSION_PASS markers. Independent desktop.png/settings.png review
confirms blue Adwaita accents and charcoal panels without yellow/navy fallback
or the black bridge tile.

Exact-head signed home-transition
[34769161731](https://github.com/Reidond/kedra/actions/runs/34769161731) also
passes, including STAGE_B, ACCEPT_B_ROLLBACK_STAGED and ROLLBACK_A_HOME markers
at 17:02–17:03 UTC. Independent GPT-6 Astra read-only review found no actionable
defects. Source and disposable-VM visual/workflow qualification are complete;
physical hardware, scaling/high contrast and full accessibility remain separate.
No production publication or installation is claimed. Next: review/merge a PR
if the owner authorizes, then separately qualify the published image and physical
target. The milestones below preserve earlier failures and their corrections.

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

Desktop [34765138011](https://github.com/Reidond/kedra/actions/runs/34765138011)
at `12c17e1`, artifact `10320836651`, passes GTK 3 Wayland/Xwayland and
libadwaita ready/dialog/selected workflows with exact file content. Qt 5 passes
native Wayland, Breeze, Adwaita Sans and Breeze-icon readbacks and renders its
KDEPlatformFileDialog/KFileWidget. Its chooser then times out: Ctrl+L/full
path/Return navigates to `/home/kedra-test/` but does not select the sample.
The fixture now uses KDE's Name editor with Alt+N, Ctrl+A, full
path and delayed Return. Local syntax/whitespace pass; the VM rerun is pending.
Qt 5 selection and Qt 6 cases remain incomplete;
the desktop workflow has no overall pass. Home-transition
[34765138099](https://github.com/Reidond/kedra/actions/runs/34765138099) is in progress.

Desktop [34766395233](https://github.com/Reidond/kedra/actions/runs/34766395233)
at `a99553f`, artifact `10320693521`, passes all six native GUI
ready/dialog/selected cases through QMP keyboard interaction and exact file
content: GTK 3 Wayland/Xwayland Adwaita, libadwaita through Nautilus, Qt 5/6
native Wayland Breeze with visible KDEPlatformFileDialog/KFileWidget, and
the Qt 6 Adwaita Mono 12 user override. The overall workflow nevertheless
**fails** afterward: fixture cleanup uses `rmdir` on a generated XDG_CONFIG_HOME
that contains normal Qt-written files. The local cleanup fix removes only that
rmdir; it still deletes the explicit kdeglobals override and leaves normal Qt
files on the disposable snapshot. All six GUI/override assertions remain,
Bash syntax/whitespace pass and the complete desktop rerun is pending. Home-transition
[34766395330](https://github.com/Reidond/kedra/actions/runs/34766395330) passed
at `a99553fcd4d5b27cc60352cbe9adc497e86aa587`.

Latest `2653cde` changes only desktop cleanup and docs/worklog, preserving runtime
image source; no new home workflow was triggered. Workspace
[34767484936](https://github.com/Reidond/kedra/actions/runs/34767484936) passes.
Desktop [34767484937](https://github.com/Reidond/kedra/actions/runs/34767484937)
**passes** at `2653cde3b7c9b43d4e2c10a6fabcca2ee6719378`. Artifact `10320989840`
contains 35 PNGs and passing wallpaper-source, no-video-bridge, all six
ready/dialog/selected toolkit cases with exact file content, KEDRA_TOOLKITS_PASS
and KEDRA_R07_SESSION_PASS markers. Measured versions are GTK 3.24.52,
GTK 4.22.5/libadwaita 1.9.3, Qt 5.15.18, Qt 6.11.2 and KDE platform/Breeze 6.7.5.

Independent visual review of desktop.png/settings.png nevertheless finds yellow
Noctalia selected accents and dark navy panels despite the configured Adwaita
blue/charcoal palette. Its cause is now identified in Noctalia 5.0.1's
[theme service](https://raw.githubusercontent.com/noctalia-dev/noctalia/v5.0.1/src/theme/theme_service.cpp):
both light/dark palette variants require terminal-color objects. The prior
Adwaita file omitted them, causing runtime fallback while the UI still displayed
Custom/Adwaita. Complete terminal colors are now implemented. A disposable native
Noctalia 5.0.1 session under nested niri renders charcoal panels/blue accents
without a fallback warning; native config validation and whitespace pass.
The VM workflow now has a read-only managed-journal fallback guard. The later
71cb8c9 Actions visual rerun passes as recorded above; the preceding 2653cde
functional GUI pass retains its historical visual limitation.

Local Rust 1.98.1 formatting, all-target Clippy with warnings denied, release
build, WSL Bash syntax and Git whitespace pass. The Windows workspace E2E
command passes with zero cases, providing no new Linux behavioral coverage.

### Removed legacy downloads

At the owner’s explicit direction, GitHub Release IDs 385117864 (r1), 385328126 (r2) and 385128562 (legacy channel), including all 31 uploaded assets, were deleted. The remote Releases list is empty. Their source Git tags remain; no source tag was removed. Historical signed-image/installation results remain in Git history and worklog, not as current download availability.

The owner confirmed on 2026-09-13 that nobody installed r1/r2. No deployed-system migration is required; earlier migration blockers assumed installations that do not exist. Fresh installations enroll directly in the signed GHCR workflow. Historical disposable-VM results remain valid only within their recorded scope.

### Verified implementation and remaining qualification

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

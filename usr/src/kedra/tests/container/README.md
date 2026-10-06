# Container scenarios and the Kedra lab

This crate tests the installed Kedra OS in containers and lets you run the desktop from any stage on your machine. It follows the owner's Testcontainers integration-harness specification. The adoption record and acceptance status are below.

A container boots the image under test with systemd as PID 1. The harness then starts the real Kedra session (`kedra-session` → `niri-session` → niri + Noctalia) nested in a headless parent compositor. Declarative YAML scenarios and native Rust tests run against it through `cargo test`.

Nothing here enters the OS image, installer or release. A container does not boot a kernel. These stay in the VM workflows (`../vm`, `../README.md`):

- UEFI Secure Boot
- SELinux enforcement
- greetd/tuigreet password login on a VT, and PAM keyring unlock
- bootc switch, update and rollback
- the installer
- clients that receive no virtual-keyboard input in the nested session (Xwayland, Qt)

## Prerequisites

- A Docker Engine API endpoint that can run privileged containers with a private cgroup namespace. On macOS that is OrbStack or Docker Desktop; in CI it is the GitHub runner's Docker.
  - Testcontainers uses `DOCKER_HOST` or `/var/run/docker.sock`, not Docker contexts.
- The pinned Rust toolchain.
- Network access for pulling images, Fedora packages in the lab layer, and the pinned Codex/Bitwarden inputs of full builds.

The engine's architecture selects the default target: `qemu-arm64` (aarch64) on Apple Silicon, `desktop` (x86_64) on x86_64.

## Commands

| Operation | Command |
|---|---|
| Validate and list; provisions nothing | `cargo test -p kedra-container-tests --test container -- --list` |
| Run all | `cargo test -p kedra-container-tests --test container` |
| Run all with two isolated workers | `cargo test -p kedra-container-tests --test container -- --test-threads 2` |
| Run one or a group; filtered before provisioning | `cargo test -p kedra-container-tests --test container -- desktop_session` |
| Retain failed containers (local only; refused when `CI` is set) | `KEDRA_LAB_KEEP=failed cargo test …` |
| Remove one execution's resources | `kedra-lab clean --execution <id>` |
| Run the desktop and keep it | `cargo run -p kedra-container-tests --bin kedra-lab -- up` |

Settings are environment variables for `cargo test`, or flags for `kedra-lab`:

| Variable / flag | Values | Default |
|---|---|---|
| `KEDRA_LAB_TARGET` / `--target` | `desktop`, `qemu-arm64` | engine architecture |
| `KEDRA_LAB_IMAGE` / `--image` | see the table below | `stable` |
| `KEDRA_LAB_OVERLAY` / `--overlay` | `worktree` (layer the working tree), `none` | `worktree`; `none` for `build` |
| `KEDRA_LAB_BINARIES` / `--binaries` | directory with Linux `sysroot` and `sysroot-helper` | built in the lab builder |
| `KEDRA_LAB_ARTIFACTS` | output directory | `target/kedra-lab` |
| `KEDRA_QEMU_HOME` | native QEMU runtime and retained VM root | durable per-user paths (see `qemu/README.md`) |
| `KEDRA_LAB_KEEP` | `never`, `failed`, `always` | `never` |
| `KEDRA_LAB_ORDER` | `forward`, `reverse` | `forward` |

One worker remains the default; one or two workers are supported. Every case still
gets a fresh container. A selection with no cases applicable to the target does
not prepare an image and explicitly reports no coverage. Reports are atomically
updated after every completed case. The first SIGINT/SIGTERM interrupts pending
Docker command/build/pull waits and cleans owned containers; a second signal uses
the normal forced-exit behavior. Reports identify interrupted runs and the planned
case count. Engine removal has a 30-second bound; cleanup failures remain visible.

### Image stages

Every stage resolves to an immutable base and is recorded in the report.

| `--image` | Image under test |
|---|---|
| `stable` | the signed production image `ghcr.io/reidond/kedra-<target>:stable`, pinned to its digest |
| `run-<id>-<attempt>` | an immutable signed publication |
| `sha256:<hex>` | a digest in the production repository |
| `builds:<tag>` | an Actions candidate in `ghcr.io/reidond/kedra-<target>-builds` |
| `ref:<reference>` | any local or pullable image, for example a CI candidate |
| `build` / `build:<revision>` | a full local build of the working tree or one commit (below) |
| `composition:/absolute/context` | verified static context, with a separately retained expected identity |

For composition, provide `--composition-identity <identity>` or
`KEDRA_LAB_COMPOSITION_IDENTITY`. Overlay defaults to `none`; worktree overlays,
binary overrides and VM replay refuse. `kedra-lab replay --image composition:…`
builds the static image without pulls/network; add `--output <typed-alias>
--program <relative-path> -- <args>` to execute the copied closure with networking
disabled. Verification consumes a private snapshot and checks foundation/load/tag,
payload semantics and cache image provenance. Interrupted owned cache publication
resumes on retry; unknown/foreign state refuses.

For closed native generation, use `kedra-lab derive-plan --image composition:…
--composition-identity <parent> --native-plan /private/plan.json`, then `derive`
with the independently retained `--derivation-identity <identity>`. The fixed
offline recipe generates GLib schemas, declared systemd links, initial account
skeletons and QEMU initramfs contents; it accepts no arbitrary script/command.
It verifies unchanged RPM material, actual artifacts and exact parent image
layers. See [native declarations and limits](../../docs/NATIVE.md).

The installed native case uses `KEDRA_LAB_IMAGE=composition:…`,
`KEDRA_LAB_COMPOSITION_IDENTITY`, `KEDRA_LAB_NATIVE_PLAN` and
`KEDRA_LAB_DERIVATION_IDENTITY`, with filter `native_artifacts`. Both native
variables are required together; ordinary and static-composition cases remain
separate selections. It runs directly on the derived System-profile image without
a lab-tools adaptation. TLS Docker endpoints use the already locked ring provider;
retained archive import allows600s while ordinary requests keep120s.

The composition-only `native::composition_unit` System-profile case consumes the
static image and validates/starts a generated unit. Ordinary scenario discovery
omits it. Lab desktop adaptation still runs its normal networked tools layer, and
is separate from the offline static replay. See [system composition](../../docs/SYSTEM.md)
and [actual replay evidence](../../../../../.specs/nix-context-replay/verification.md).

For an already cached `ref:` image, BuildKit receives a harness-owned
`kedra-lab-base:<image-id>` tag because a bare local `sha256:...` in FROM is parsed
as a Docker Hub image name. The report retains the actual immutable local image ID.

**Overlay** (default for published stages) layers the working tree over the stage in two steps:

1. It snapshots the tree into a throwaway Git repository and archives the payload with the real `sysroot source archive`.
2. It replays the payload half of the release build: binaries, `payload.tar`, the account baseline in `/etc/skel`, schema compilation, and niri/Noctalia validation. Files the base shipped and the tree removed are deleted.

The overlay refuses package-list changes. It warns about image-build inputs (`Containerfile`, `assemble.sh`, agents, Bitwarden, release) that differ from the base's recorded source revision, because the overlay cannot apply them.

**Full local builds** replay the Actions candidate build in disposable containers:

1. Linux binaries and `sysroot source archive` in the Rust builder.
2. The pinned Codex and Bitwarden inputs through `image/agents/prepare.sh` and `image/bitwarden/prepare.py`, in a Fedora 44 builder with the pinned uv. The scripts accept `KEDRA_LOCAL_BUILDER=1` inside a container.
3. The stage's own `Containerfile` and `assemble.sh` on `quay.io/fedora/fedora-bootc:44`, pinned to its platform digest.

The result is an unsigned `kedra-local-<target>:<key>`. It is never pushed, signed or installed, and it has no release identity layers. `build:<revision>` needs the root-filesystem layout (PR #20 or later).

## The lab: `kedra-lab`

```bash
cargo run -p kedra-container-tests --bin kedra-lab -- up
```

`up` starts a retained environment named `default` with the working tree over `stable`, at 2560×1600 and scale 2 (1280×800 logical). Other options:

- `--image …`, `--target …`, `--display 2560x1600@2`, `--name …`.

| Command | Effect |
|---|---|
| `shot [label]` | Saves a PNG and JSON source/display/timing receipt to `target/kedra-lab/shots/`; scale comes from actual niri IPC. |
| `prepare-sync` | Builds/caches the host-native source archiver once per relevant Rust/Cargo/toolchain input. `up` also prepares it. |
| `sync [--shot LABEL]` | Validates and applies changed niri/Noctalia home files and managed deletions, then optionally captures the result. No Rust build, image pull or reboot in the hot path. |
| `exec [--root] -- CMD…` | Runs a command in the session, as the test account unless `--root`. |
| `logs` | Saves the journal, unit, session and process state. |
| `ls`, `down`, `clean --all` | List and remove harness-owned containers only. Selection is by the `dev.kedra.lab.owner` label. |
| `clean --images [--keep TAG]…` | Removes harness-built images (lab tools, overlays, builders, local builds) that no container uses. They are caches and are rebuilt on demand. |
| `image` | Resolves and builds the lab image, and prints its provenance. |

A typical desktop change:

1. Edit `etc/skel/.config/niri/config.kdl` or a Noctalia file.
2. Run `kedra-lab sync --shot changed` and inspect the PNG.
3. Open a panel with `kedra-lab exec -- noctalia msg settings-toggle`.
4. Run `kedra-lab shot settings` and look at the PNG.

Timings on an M2 Pro with OrbStack, cached layers: environment and session ready in about 3–7 s; a full `up` in about 16 s.

Sync manages only image-declared `.config/niri/` and `.config/noctalia/` files in
the disposable `kedra-test` account. It refuses package/rootfs/target changes and
guest-edited managed files. GUI overrides remain intact and conflicting setting
names are reported. Staged configuration is checked by the installed applications;
Noctalia 5.1 accepts some unknown enum-like strings, so validation is not a claim
that every setting has its intended visible effect. Inspect the screenshot.

The lab keeps backups and a transaction under
`~/.local/state/kedra-lab/sync` inside the guest. A subsequent sync recovers an
interrupted write when current files still match its own writes; it refuses to
overwrite newer guest edits. This is separate from production `sysroot home`.
If the source archiver becomes stale, run `prepare-sync` explicitly. Other home
groups, OS packages and system files require an overlay/full image rebuild.

Measured local working-tree results on 2026-09-28 (M2 Pro, OrbStack/Docker 29.4.0,
cached ARM image plus overlay; software rendering): five edit+capture samples had
median **5.000 s**, maximum **9.177 s**; five capture commands had median **0.331 s**,
maximum **0.467 s** (millisecond reporting precision). All 13 scenarios passed in forward and reverse order, and a
two-worker run passed all 13 in **111.60 s**. These suite timings are single-run
observations, not the five-run suite performance gate. Active-case interruption
exited 130 in **1.707 s**, retained its report and left no test container from that
execution; a separate retained lab stayed usable. See worklog WL-20260928-04.

### Native QEMU status

`kedra-lab vm tools check` is a read-only host/runtime preflight that does not
require Docker. The native runtime and GPU VM lifecycle are not yet implemented
or qualified; see [QEMU prerequisites](qemu/README.md). UTM tooling, ARM identity
and existing boot-level CI remain unchanged until the replacement is qualified.

### Manual GPU desktop

Use [`kedra-lab vm`](qemu/README.md) for a retained native QEMU/HVF desktop on
Apple Silicon. It shares the home-sync transaction with the container lab and
captures actual guest pixels through grim. Container screenshots remain software
rendered and cannot qualify GPU behavior. The old `--live`, `live-tools`,
cocoa-way/waypipe builder and TCP bridge have been removed.

## How the environment works

| Spec component | Here |
|---|---|
| Environment provider | `environment.rs`. Testcontainers 0.28 starts a privileged container with a private cgroup namespace, tmpfs `/run`, `/run/lock` and `/tmp`, and ownership labels. It waits for `systemctl is-system-running` within a bounded time and removes the container on drop. `docker.rs` runs execs, bounded by `timeout(1)`, and file transfer through the Engine API, so a retained lab container works from later processes. |
| Application adapter | `image.rs` (stages, overlay), `builder.rs` (binaries, snapshots, payloads, Git bundles), `localbuild.rs` (full builds), `session.rs` (account, session, screenshots) |
| Fixture manager | `test_user` (a disposable wheel account from `/etc/skel` with a generated password) and `desktop_session`. Fixtures are scoped to one container. |
| Scenario runner / assertions | `scenario.rs`, `actions.rs` |
| Native tests | `native.rs`, using the same `Context` as scenarios |
| Diagnostics | `Environment::collect` (journal, failed units, sessions, processes, final screenshot) and `report.rs` (`report.json`, `junit.xml`) |

Shared Linux Cargo output is locked through copying the built executables into
the builder container's private storage. Host binary/payload caches are addressed
by their actual content hashes, and source bundles are published atomically.
Concurrent preparations therefore do not exchange mutable output files after
compilation. Test container names include the full case-name hash, so valid long
names with the same shortened prefix remain isolated with two workers.

### The session

`/usr/libexec/kedra-session` runs as `kedra-lab-session.service` with `PAMName=greetd`. The Fedora greetd PAM session stack, including pam_systemd and pam_gnome_keyring, therefore registers a real logind session. The packaged `niri.service`, `kedra-noctalia.service`, portals, PipeWire and XDG autostart start as they do after a login.

Before that, the login keyring is created and unlocked with the account password (`gnome-keyring-daemon --unlock`), which is what pam_gnome_keyring does with a typed password.

### Lab layer adaptations

The lab tools layer (`lab/tools.Containerfile`) installs sway, grim, wlrctl, wtype and the GUI probe bindings. It refuses to add, upgrade or remove any package of the image under test. It also makes these container-only adaptations, each explained in its file:

| Adaptation | Why |
|---|---|
| `niri.service` drop-in: `WAYLAND_DISPLAY=wayland-host WSL_DISTRO_NAME=kedra-lab` | There is no DRM device. niri 26.04 keeps `WAYLAND_DISPLAY`, and so selects its winit backend as a window of the parent, when `WSL_DISTRO_NAME` is set. |
| `kedra-lab-host.service`: sway headless (pixman) at the requested size | Parent compositor. niri's output is `winit`; the harness sets its scale over IPC. |
| greetd and bootloader-update: `ConditionVirtualization=!container` | No VT and no bootloader. Their enablement state stays testable. |
| rtkit `--no-limit-resources` | rtkit's per-UID `RLIMIT_NPROC` counts every container's rtkit on a shared kernel. A second lab container's rtkit then fails, and `xdg-desktop-portal` times out on it. |
| journald `ReadKMsg=no` | The kernel log belongs to the host. |
| (CI host) unload Ubuntu's `unix-chkpwd` AppArmor profile | Ubuntu 24.04 attaches that profile by path, so it also confines the container's Fedora `unix_chkpwd`. That copy then cannot read the container's accounts, and PAM refuses the user manager (`test-container.yml`, 2026-09-27; the diagnosis is recorded in the worklog). |
| (CI host) `kernel.apparmor_restrict_unprivileged_userns=0` | Ubuntu 24.04 puts every unprivileged user namespace under its `unprivileged_userns` profile, including those in the container. That denies Bitwarden's Chromium sandbox and `bwrap`, as the kernel audit log recorded. |
| Single-link copies in `/etc`, `/usr/lib/systemd`, `/usr/share/containers` | bootc images hard-link files to their embedded ostree objects. A booted composefs system shows one link, and `sysroot`'s trusted reads require exactly one. |

The remaining differences are recorded, not hidden:

- No SELinux labels. The probes compare labels only where SELinux exists.
- The host's kernel and uptime.
- Software rendering (llvmpipe).
- Keyboard input through niri's virtual keyboard. GTK 3 accepts `wlrctl`, GTK 4 needs `wtype`, and Xwayland and Qt 6.11 clients received none (2026-09-27).

## Scenario format (version 1)

Files live in `scenarios/*.yaml` and reusable steps in `setups/*.yaml`. Parsing is strict: unknown fields, duplicate keys, merge keys and unsupported tags are rejected. Everything is validated before any container exists:

- names and targets
- durations and bounded JSON selectors (`$`, `.key`, `[n]`)
- that references resolve to earlier captures, declared fixtures or inputs
- action inputs
- include cycles

```yaml
version: 1
name: noctalia_appearance          # [a-z0-9_], unique
profile: desktop                   # system | desktop (desktop implies test_user + desktop_session)
targets: [qemu-arm64]                     # optional
timeout: 5m                        # optional scenario deadline (default 10m)
fixtures: [test_user]              # optional for profile: system
steps:
  - exec:                          # one command, argv only; shells and interpreters are refused
      name: label for reports
      as: session                  # root (default) | user | session
      argv: [noctalia, msg, color-scheme-get]
      env: {NAME: value}
      stdin: text
      timeout: 30s                 # default 60s, bounded by the scenario deadline
      expect:                      # exit defaults to 0
        stdout: custom Adwaita     # exact, one trailing newline ignored
        stdout_nonempty: true
        stdout_contains: [..]
        stdout_lacks: [..]
        stderr_contains: [..]
        stderr_lacks: [..]
        json:
          - {select: $.a[0].b, equals: 1}      # exact
          - {select: $.a, subset: {x: 1}}      # objects: keys subset; arrays: same length
          - {select: $.gone, absent: true}     # missing is distinct from null
      capture:
        mode: {stdout: trim}       # or raw
        width: {json: $.width}     # keeps the JSON type
  - eventually:                    # repeats a read-only observation; never a mutation
      timeout: 60s
      interval: 1s
      observe: {as: user, argv: [..], expect: {..}}
  - assert:
      - {value: "${mode}", equals: dark}      # or differs, one_of, contains
  - action: {name: screenshot, with: {name: desktop}, capture: {path: shot}}
  - include: {setup: some_setup, with: {input: value}, capture: {output: local}}
```

A string that is exactly one reference, such as `"${x}"`, keeps the value's type. Otherwise references interpolate as text, and missing references fail. The available references are:

- `fixtures.test_user.{name,uid,home}`
- `fixtures.desktop_session.{wayland_display,niri_socket,output}`
- captures, and setup `inputs.*`

Actions:

| Action | Effect |
|---|---|
| `screenshot {name}` | Saves `<test artifacts>/screenshots/<name>.png`. |
| `type_text {text}` | Types through niri's virtual keyboard with wlrctl. |
| `probe {name, args?, as?, timeout?}` | Runs a vetted guest script from `lab/probes`. Outputs `stdout`, and `json` when stdout is JSON. |
| `settle {for ≤ 30s}` | Waits, for redraws before a screenshot. |

Control flow belongs in native tests or probes.

## Isolation, cleanup and reports

- **Isolation.** Every test gets its own container, so every mutable service starts fresh. An execution shares only read-only images and the Cargo caches (named volumes; Cargo's fingerprints decide freshness). Container names and labels carry the execution ID, `<unix seconds>-<pid>`. Tests run one at a time.
- **Cleanup.** Explicit termination comes first; Testcontainers' drop and `watchdog` (SIGINT/SIGTERM) are fallbacks. testcontainers-rs has no Ryuk reaper, so after a hard kill run `kedra-lab clean --execution <id>` or `--all`. A cleanup failure fails an otherwise passing test, and it never replaces a test failure.
- **Reports.** They go to `target/kedra-lab/runs/<execution>/`:
  - `report.json` and `junit.xml`, including the image provenance.
  - Per test: `steps.json` with durations and expected/actual values, and screenshots.
  - On failure: `journal.log`, `failed-units.txt`, `sessions.txt`, `processes.txt` and a final screenshot.

  Error text keeps the tail, where tracebacks are. Artifacts stay out of fixture directories. The journal is capped at 8 MiB.

## Adoption record (spec §11)

| Decision | Choice |
|---|---|
| Binding | testcontainers 0.28.0 (blocking, watchdog); bollard 0.21 through its re-export; libtest-mimic 0.8.2 as the native runner (`harness = false`); serde-saphyr 1.3.0 for strict YAML |
| Runtime | Docker Engine API. Measured: OrbStack 2.2.3 / Docker 29.4.0, kernel 7.0.14, cgroup v2. CI uses GitHub runners' Docker. |
| Profiles | `system` (booted systemd) and `desktop` (plus account and session) |
| Application mode | The OS image itself in a container, systemd as PID 1 |
| Schema / migrations | Not applicable. Image contents come from the stage's own build, or from the overlay replay of its payload steps. |
| Reset strategy | A dedicated container per test |
| Deadlines | Boot 90 s, session 90 s + 60 s for IPC, step 60 s by default, scenario 10 m by default, full image build 30 m per step |
| Artifacts | `target/kedra-lab/runs/<execution>/` (ignored by Git); uploaded by `test-container.yml` |
| HTTP/database operations | Not applicable. `exec`/`observe` against the OS take their place. |
| Unit tests for the parser and assertions | None, by owner decision (AGENTS.md). Strict parsing and assertion sensitivity are checked by running the harness. |
| Parallel scenarios, container reuse | One or two isolated workers; default one. A two-worker ARM run passed on 2026-09-28. No mutable container reuse between tests. |

## Acceptance status

Local runs on 2026-09-27: M2 Pro, OrbStack 2.2.3, qemu-arm64 target. See `worklog.md` WL-20260927-02.

| Case | Status |
|---|---|
| A1 fresh execution | Local pass. 13 tests in one run (101 s), working tree over `stable`. The same suite on a full local build of the working tree also passes: 12 in one run, and the doctor scenario after its release-trust expectation was corrected. CI (`test-container.yml`) not-run. |
| A2 filtering | Pass. A filter provisions only the selected tests; `--list` provisions nothing. |
| A3 concurrent executions | Pass. Two executions of the same scenario ran at once, with the same fixture names and a retained lab container alongside. Both passed and nothing was left behind. Earlier concurrency exposed and fixed the rtkit per-UID limit. |
| A4 order independence | Partly observed. Scenarios passed alone, in the suite and repeated. A reversed-order run is not-run. |
| A5 partial startup failure | Observed. Failed overlay and session starts removed their containers. Not a scripted fault injection. |
| A6, A7, A10 | Not-run as deliberate fault injections (cancellation, async timeout, teardown failure) |
| A8 strict parsing | Pass, before any container. Refused: duplicate keys, unknown fields, two operations in one step, shell or interpreter argv, unresolved references, bad selectors, unknown actions, profile-incompatible identities, recursive includes. |
| A9 assertion sensitivity | Pass. A wrong expected wallpaper failed with `expected stdout "color:#000000", got "color:#222226"` although the command succeeded. |
| A11 CI parity | Not-run. CI builds with the same `KEDRA_LAB_IMAGE=build` path as local runs. |
| A12 portability | Not applicable: one project |

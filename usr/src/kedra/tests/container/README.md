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

- A Docker Engine API endpoint that can run privileged containers with a private cgroup namespace. On macOS that is OrbStack or Docker Desktop; in the release workflow's candidate validation it is the GitHub runner's Docker.
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
| Remove one execution's containers | `kedra-lab clean --execution <id>` |
| Remove everything this checkout's harness created | `kedra-lab clean` |
| Delete shared cache entries and stores no checkout uses | `kedra-lab gc` (`--dry-run` reports only) |
| Run the desktop and keep it | `cargo run -p kedra-container-tests --bin kedra-lab -- up` |

Settings are environment variables for `cargo test`, or flags for `kedra-lab`:

| Variable / flag | Values | Default |
|---|---|---|
| `KEDRA_LAB_TARGET` / `--target` | `desktop`, `qemu-arm64` | engine architecture |
| `KEDRA_LAB_IMAGE` / `--image` | see the table below | `stable` |
| `KEDRA_LAB_OVERLAY` / `--overlay` | `worktree` (layer the working tree), `none` | `worktree`; `none` for `build` |
| `KEDRA_LAB_BINARIES` / `--binaries` | directory with Linux `sysroot` and `sysroot-helper` (overlays and full local builds; they must be the stage's own) | built in this checkout's Cargo volume |
| `KEDRA_LAB_ARTIFACTS` | output directory | `target/kedra-lab` |
| `KEDRA_QEMU_HOME` | native VM instances, shots and logs | `target/kedra-lab/qemu` (see `qemu/README.md`) |
| `KEDRA_SHARED_CACHE` | absolute directory of the [shared cache](#shared-cache) | `<primary checkout>/.cache/kedra` |
| `KEDRA_LAB_KEEP` | `never`, `failed`, `always` | `never` |

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

Closed native generation is product code: `sysroot derive plan|build` (see
[native declarations and limits](../../docs/NATIVE.md)). The harness only calls
it. Static composition replay uses the same engine realization, with state in
`target/kedra-lab/derive` (the artifact root's `derive` directory).

The installed native cases use `KEDRA_LAB_IMAGE=composition:…`,
`KEDRA_LAB_COMPOSITION_IDENTITY`, `KEDRA_LAB_DERIVATION_IDENTITY` and
`KEDRA_LAB_SYSROOT`, a native Linux `sysroot` binary of the engine's architecture
that runs `derive build` (it is also the executor bound into the derivation
identity), with filter `native_artifacts` (the `e2e_native` fixture) or
`catalog` (a release composition). The steps are the ones the composition's
committed source declares (its target's `.kedra` block); there is no separate
steps file. Ordinary and static-composition cases remain separate selections. It runs directly on the derived System-profile image without
a lab-tools adaptation. TLS Docker endpoints use the already locked ring provider;
retained archive import allows600s while ordinary requests keep120s.

The ordinary `native::engine_sandbox` System-profile case (qemu-arm64) runs the
engine's Nix-style commands as the test user on the installed image, which carries
no Docker: it saves a small pinned Ubuntu image on the harness engine, copies the
archive in, and checks `store add-image --archive`, `add-root`, `build`, `run`,
`profile switch|run`, `develop` isolation (no network, read-only root and store, no
host home or Docker socket, cleared environment) and `store recover` after the
controller of a running build is killed. The harness container is privileged, so
bubblewrap's user namespaces and `/proc` mount are available.

The `native::catalog` System-profile case checks the installed catalog
packages: every exported command is a launcher of its exact store output, first on
the login and user-manager PATH; private `libexec` exports (the bundled Codex) are
on neither; no command shadows a foundation command it does not declare it
replaces, so Fedora's `/usr/bin/jq` stays `jq`. It runs on derived release
compositions and on full local builds (`KEDRA_LAB_IMAGE=build`), which build the
catalog packages too. The ordinary `native::catalog_shadowing` case runs `sysroot
catalog build` as the test user on the installed image against a saved pinned
Fedora 44 image: an export named `cat` is refused until its package declares
`replaces = ["cat"]`, and a declared replacement of a command Fedora lacks is
refused.

The composition-only `native::composition_unit` System-profile case consumes the
static image and validates/starts a generated unit. Ordinary scenario discovery
omits it. Lab desktop adaptation still runs its normal networked tools layer, and
is separate from the offline static replay. See [system composition](../../docs/SYSTEM.md#verify-and-consume).

For an already cached `ref:` image, BuildKit receives a harness-owned
`kedra-lab-base:<image-id>` tag because a bare local `sha256:...` in FROM is parsed
as a Docker Hub image name. The report retains the actual immutable local image ID.

**Overlay** (default for published stages) layers the working tree over the stage in two steps:

1. It snapshots the tree into a throwaway Git repository and archives the payload with the real `sysroot source archive`.
2. It replays the payload half of the release build: binaries, `payload.tar`, the derive steps that read the payload (schema compilation and the account baseline in `/etc/skel` with its applied home baseline), and `assemble.sh`'s niri/Noctalia validation. Files the base shipped and the tree removed are deleted.

The overlay refuses package-list changes and changes to the target's declared systemd or initramfs steps (when the base records its steps). It warns about image-build inputs (`Containerfile`, `assemble.sh`, `.kedra` packages, release; agents and Bitwarden inputs before they became packages) that differ from the base's recorded source revision, because the overlay cannot apply them.

**Full local builds** replay the Actions candidate build in disposable containers:

1. Linux binaries and `sysroot source archive` in the Rust builder.
2. The stage's own foundation (`Containerfile` and `assemble.sh`) on `quay.io/fedora/fedora-bootc:44`, pinned to its platform digest. A stage with `image/catalog/install-rpms.sh` installs offline from an RPM lock: the host `sysroot catalog lock resolve` (the prepared host archiver, store `cache/rpm-store`) resolves it once per stage and base digest, and `lock repo` adds the foundation repository as `rpms/` ([package delivery](../../docs/PACKAGES.md#fedora-rpm-lock)). An interrupted resolution leaves staging that `sysroot store recover --store <artifacts>/cache/rpm-store` removes.
3. For a stage whose source declares image steps, the stage's own Linux `sysroot` first runs the release's catalog steps: the public preflight material `image/release/material.py` computes (with the run's single pin envelope), then the stage's `image/catalog/build-packages.sh`, the script the release build runs: a runtime root and a builder root per role from the RPM lock, the reviewed archives through `image/catalog/prepare-sources.py`, `sysroot catalog build` of every selected package (Codex and Bitwarden) and `sysroot catalog contribute`. It then composes that foundation with the snapshot's committed payload and the contribution (`sysroot system compose --definition`) and derives the declared steps (`sysroot derive build`), as the release does ([one image pipeline](../../docs/SYSTEM.md#one-image-pipeline)). It runs in a disposable builder container (the Rust builder also carries bubblewrap, jq, the pinned uv and the pinned Docker CLI with buildx) that reaches the lab's Docker socket at `/var/run/docker.sock` as an ordinary user in the socket's group, with seccomp, AppArmor and system paths unconfined so the engine's bubblewrap sandbox can observe the foundation; the engine store and contexts die with the container. The release trust layer is not built locally. An older stage builds its complete image in step 2.

The result is an unsigned `kedra-local-<target>:<checkout>-<key>`. It is never pushed, signed or installed, and it has no release identity layers. `build:<revision>` needs the root-filesystem layout (PR #20 or later).

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
| `sync [--shot LABEL]` | Merges the working tree's managed home files into the running account with `sysroot home update`, then optionally captures the result. No Rust build, image pull or reboot in the hot path. |
| `exec [--root] -- CMD…` | Runs a command in the session, as the test account unless `--root`. |
| `logs` | Saves the journal, unit, session and process state. |
| `ls`, `down` | List this checkout's harness containers, and remove a retained environment. |
| `clean` | Removes everything this checkout's harness created: every container, image and volume labeled `kedra.checkout=<id>`, its scoped image tags, the engine images its `sysroot derive` state binds (the engine refuses a `kedra-<kind>:<identity>` tag whose binding is gone, so they go together), and the large trees under `target/kedra-lab/` (`cache`, `derive`, `composition`, `native`; sealed store objects are made writable first). Run reports and screenshots stay. Pulled images stay unlabeled, because a label would change the image under test; each pull the harness makes is appended to `target/kedra-lab/pulled-images.txt` with the image ID and repository digests it resolved to, to remove by exact ID once no other checkout uses it. `clean` does not remove them: other checkouts and tools share pulled images by content and record their use nowhere this checkout can read, so deleting one could pull it from under a concurrent run. It also removes the native VM's stopped instances (`target/kedra-lab/qemu`) and then drops this checkout's [shared cache](#shared-cache) roots on VM disks, RPM locks and engine stores (`--vm-runtime` also the root on the QEMU runtime); the entries stay for other checkouts until `gc`. `clean --execution <id>` removes only one execution's containers. |
| `gc [--dry-run]` | Deletes [shared cache](#shared-cache) entries, abandoned build areas and shared engine stores that no checkout roots, after dropping the roots of checkouts that no longer exist, and reports the space freed. Corrupt or unknown entries are reported and left in place; `gc --remove KIND/KEY` deletes one named entry and every root on it. |
| `image` | Resolves and builds the lab image, and prints its provenance. |

A typical desktop change:

1. Edit `system/home/niri/config.kdl` or another managed home file.
2. Run `kedra-lab sync --shot changed` and inspect the PNG.
3. Open a panel with `kedra-lab exec -- noctalia msg settings-toggle`.
4. Run `kedra-lab shot settings` and look at the PNG.

Timings on an M2 Pro with OrbStack, cached layers: environment and session ready in about 3–7 s; a full `up` in about 16 s.

Sync stages the working tree's home baselines as a private image root inside the
disposable `kedra-test` account and runs the production
`sysroot home update --root` there, so only the files declared `managed` in
`system/*.kedra` change. Guest edits are merged, not
overwritten; a conflict leaves the live file and writes `<file>.kedra-new`
([managed home files](../../docs/HOME.md)). Each file's declared check runs
before it is written and its reload afterwards (niri reloads itself; Noctalia
restarts). Sync refuses package and rootfs changes, including the data file, and
a removed baseline leaves its live file alone. Validation is not a claim that
every setting has its intended visible effect: Noctalia accepts some unknown
enum-like strings with a warning. Inspect the screenshot. If the source archiver
becomes stale, run `prepare-sync` explicitly. OS packages and system files
require an overlay or full image rebuild.

Measured local working-tree results on 2026-09-28 (M2 Pro, OrbStack/Docker 29.4.0,
cached ARM image plus overlay; software rendering): five edit+capture samples had
median **5.000 s**, maximum **9.177 s**; five capture commands had median **0.331 s**,
maximum **0.467 s** (millisecond reporting precision). All 13 scenarios passed in forward and (with a since-removed order option) reverse order, and a
two-worker run passed all 13 in **111.60 s**. These suite timings are single-run
observations, not the five-run suite performance gate. Active-case interruption
exited 130 in **1.707 s**, retained its report and left no test container from that
execution; a separate retained lab stayed usable. See worklog WL-20260928-04.

### Native QEMU status

`kedra-lab vm` runs the native QEMU/HVF desktop on Apple Silicon: `vm tools check`
is a read-only host/runtime preflight without Docker, and the prepared runtime,
retained instances, installer media and `vm check` (the shared guest Secure Boot
check for the running boot) are described in [qemu/README.md](qemu/README.md),
with their recorded qualification there and in STATUS.

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
| Diagnostics | `Environment::collect` (journal, failed units, sessions, processes, final screenshot) and `report.rs` (`report.json`) |

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
| (CI host) unload Ubuntu's `unix-chkpwd` AppArmor profile | Ubuntu 24.04 attaches that profile by path, so it also confines the container's Fedora `unix_chkpwd`. That copy then cannot read the container's accounts, and PAM refuses the user manager (the removed `test-container.yml` workflow, 2026-09-27; the diagnosis is recorded in the worklog). |
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

## Shared cache

Every checkout of this repository (the primary checkout and its worktrees)
shares one cache of build artifacts, so a new worktree reuses what another
already built instead of building its own: `<primary checkout>/.cache/kedra/`,
where the primary checkout is the parent of `git rev-parse --git-common-dir`
(`KEDRA_SHARED_CACHE`, an absolute directory, moves it). It is gitignored and the
one location outside a checkout's `target/` that AGENTS.md "Artifacts stay in the
checkout" allows. `shared_cache.rs` and `shared_store.rs` implement it.

| Artifact | Location | Keyed by |
|---|---|---|
| Native QEMU runtime (`vm tools prepare`) | `entries/qemu-runtime/<key>/content` | the recipe and pins `prepare-runtime.py` reads (`inputs.json`, patches, the ANGLE and build-tools locks, the script), the host architecture and the toolchain the runtime bundles (Xcode, Metal toolchain, linked Homebrew library versions) |
| VM base disks (`vm image`) | `entries/vm-disk/<key>/content` | the fixture's layers and config without its `kedra.checkout` label, the bootc builder pin and the disk recipe |
| Host RPM store | `stores/rpm` | an engine store: RPM objects by content |
| Engine store of full local builds | Docker volume `kedra-cache-<cache id>-engine`, labeled `kedra.cache=<cache id>` | an engine store: RPM objects, base images, runtime and builder roots, sources and package outputs, all input-addressed |
| sccache compilation cache (mise.toml `SCCACHE_DIR`) and its server's socket (`SCCACHE_SERVER_UDS`) | `sccache/`, `sccache.sock` | sccache's own keys (compiler, arguments, inputs); bounded by `SCCACHE_CACHE_SIZE` (10G), which sccache enforces by evicting the least recently used entries; `kedra-lab gc` reports its size |

**Entries.** The key is the SHA-256 of the entry kind and the canonical JSON of
its declared inputs. A checkout that needs an entry takes the key's file lock in
`locks/`; if the entry exists it verifies it against its `receipt.json` (the
inputs, and the type, mode, size and SHA-256 of every file) and reuses it.
Otherwise it builds into `work/<kind>-<key>/` (the runtime's build tree lives
there, so a failed build resumes), seals the result read-only, writes the receipt
and publishes the entry with one rename, then deletes the build area. Another
checkout asking for the same key meanwhile waits on the lock and then reuses the
entry. An entry that differs from its receipt (tampered, corrupt or unknown) is
refused and left in place, never repaired; `kedra-lab gc --remove KIND/KEY`
deletes it. Instances keep only their writable overlay disk, firmware variables
and TPM in the checkout; the overlay's backing file is the shared base disk.

**Stores.** Engine stores keep their own guarantees: content-addressed objects
and an exclusive lock held by every `sysroot` command, so concurrent builds from
several checkouts interleave command by command and reuse what another command
admitted; a command killed mid-transaction leaves the store refusing until
`store recover`, which kedra-lab runs at the start of every build (no command
holds the engine lock between commands, so only killed commands' transactions
can be left). kedra-lab adds what the engine leaves to its caller: a store is
created only under the cache's exclusive store lock (`Store::create` is not safe
against a concurrent creator), and builds hold that lock shared. Every admission
pins, so each checkout records the images, roots and objects its latest build
used; a build that finishes while no other build holds the store unpins what no
checkout's record names and runs `store gc --delete`. Each full local build's
foundation image is unpinned when the build ends.

**Roots and GC.** `roots/<checkout id>/` (the id is the `kedra.checkout` label's)
holds `checkout.json`, naming the checkout, and one file per entry or store the
checkout uses. `kedra-lab clean` drops the checkout's roots on disks and stores,
and with `--vm-runtime` on the runtime. `kedra-lab gc` drops the roots of
checkouts whose directory no longer exists, then deletes unrooted entries,
abandoned build areas and unrooted stores (the engine volume too), each only
while it holds that entry's or store's lock, and reports the space freed.

## Isolation, cleanup and reports

- **Isolation.** Every test gets its own container, so every mutable service starts fresh. An execution shares only read-only images and its checkout's Cargo volumes (`kedra-lab-<checkout>-cargo-registry`, `kedra-lab-<checkout>-target-<arch>`; Cargo's fingerprints decide freshness within one checkout). Container names and labels carry the execution ID, `<unix seconds>-<pid>`. Tests run one at a time.
- **Ownership.** Every container, volume and image the harness creates carries `kedra.checkout=<id>`, the first 12 hex digits of the SHA-256 of the canonical checkout path (AGENTS.md "Artifacts stay in the checkout"), and the images it names are tagged `<checkout>-<key>`, so one checkout never reuses, lists or removes another's objects. Images the engine derives (full local builds) cannot carry a label without changing the image under test; `kedra-lab clean` removes them by their scoped tag. Volumes from before this scheme (`kedra-lab-target-aarch64`, `kedra-lab-cargo-registry`, `kedra-qemu-podman-cache`) are unowned; remove them by hand once no checkout still builds in them.
- **Cleanup.** Explicit termination comes first; Testcontainers' drop and `watchdog` (SIGINT/SIGTERM) are fallbacks. testcontainers-rs has no Ryuk reaper, so after a hard kill run `kedra-lab clean --execution <id>`, or `kedra-lab clean` once this checkout's checks are recorded. A cleanup failure fails an otherwise passing test, and it never replaces a test failure.
- **Reports.** They go to `target/kedra-lab/runs/<execution>/`:
  - `report.json`, including the image provenance.
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
| Artifacts | `target/kedra-lab/runs/<execution>/` (ignored by Git); the release workflow uploads its candidate validation's reports |
| HTTP/database operations | Not applicable. `exec`/`observe` against the OS take their place. |
| Unit tests for the parser and assertions | None, by owner decision (AGENTS.md). Strict parsing and assertion sensitivity are checked by running the harness. |
| Parallel scenarios, container reuse | One or two isolated workers; default one. A two-worker ARM run passed on 2026-09-28. No mutable container reuse between tests. |

## Acceptance status

Local runs on 2026-09-27: M2 Pro, OrbStack 2.2.3, qemu-arm64 target. See `worklog.md` WL-20260927-02.

| Case | Status |
|---|---|
| A1 fresh execution | Local pass. 13 tests in one run (101 s), working tree over `stable`. The same suite on a full local build of the working tree also passes: 12 in one run, and the doctor scenario after its release-trust expectation was corrected. The container workflow then passed both targets (main run 36617035048, 2026-09-29) before its removal on 2026-10-08; the release's `validate-candidate` job runs the same harness. |
| A2 filtering | Pass. A filter provisions only the selected tests; `--list` provisions nothing. |
| A3 concurrent executions | Pass. Two executions of the same scenario ran at once, with the same fixture names and a retained lab container alongside. Both passed and nothing was left behind. Earlier concurrency exposed and fixed the rtkit per-UID limit. |
| A4 order independence | Observed. Scenarios passed alone, in the suite, repeated and, on 2026-09-28, in reverse order through a since-removed order option. |
| A5 partial startup failure | Observed. Failed overlay and session starts removed their containers. Not a scripted fault injection. |
| A6, A7, A10 | Not-run as deliberate fault injections (cancellation, async timeout, teardown failure) |
| A8 strict parsing | Pass, before any container. Refused: duplicate keys, unknown fields, two operations in one step, shell or interpreter argv, unresolved references, bad selectors, unknown actions, profile-incompatible identities, recursive includes. |
| A9 assertion sensitivity | Pass. A wrong expected wallpaper failed with `expected stdout "color:#000000", got "color:#222226"` although the command succeeded. |
| A11 CI parity | Not-run. CI builds with the same `KEDRA_LAB_IMAGE=build` path as local runs. |
| A12 portability | Not applicable: one project |

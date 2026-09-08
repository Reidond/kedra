# Continue Kedra implementation

## Objective and current scope

Build a personal Fedora 44 bootc desktop managed by Rust `sysroot`. The owner
requested full implementation through a usable, understandable installer and
explicitly authorized disposable VM testing. Public releases may contain reviewed
project files only. Work is on `codex/usable-system`; main is unchanged.

Read AGENTS.md, worklog.md, PLAN.md, RESEARCH.md and the canonical Kedra context
skill before resuming. Inspect Git state and exact-source Actions evidence.
The worklog current-status table and packet reports distinguish tested subsets
from full acceptance. Never infer implementation from a plan or old work entry.

## Implemented and tested

The three-package flat Rust workspace provides source planning/archives from
committed Git blobs with target overlays/provenance, release signature/checkpoint
verification, synthetic line review and Noctalia disposition models, and Linux
private SQLite storage. The shared core performs no filesystem/process I/O.
Source/Git operations belong to the ordinary-user CLI. The helper has a storage
library but its binary still refuses privileged operations. The ordinary-user CLI
now connects private storage to the three-field Noctalia review model; native
GUI tests pass. Selected-field source patches and exact commit receipts pass Linux tests. Generic
files, discard and activation remain open.

Source `a2c0e6314351c325021e74c10c0edb5b8da2355b` passed Linux
[check 34192732940](https://github.com/Reidond/kedra/actions/runs/34192732940):
formatting, Clippy, 81 tests plus one doctest, release build and independent
OpenSSL interoperability. Eight storage tests cover corruption, links/types/modes,
concurrency and process interruption. Linux agent wrappers also pass native Codex
0.153.4/0.153.3 runtime/profile checks in [34186921224](https://github.com/Reidond/kedra/actions/runs/34186921224).
Model turns, authentication and complete discovery remain unqualified.

R01 disposable strict Sigstore policy, negative cases, signed A-to-B boot and
rollback pass, including inherited enforcement on initial and rolled-back A.
R02 minimal QCOW2 boots with enforcing SELinux in Actions and local WSL2/KVM.
R07 [run 34192732970](https://github.com/Reidond/kedra/actions/runs/34192732970)
passes graphical login, niri/Noctalia IPC, services, unlocked synthetic keyring
and native Noctalia 5.0.1 settings projection. The inspected 1280x768 capture now
fills the VM window with readable controls; physical display/audio/suspend and
owner credentials are unqualified.

The plain Containerfile consumes a reviewed source archive, installs Fedora
packages, and seeds ordinary writable defaults through /etc/skel for new users.
This is not enrollment or management of an existing home. Automatic bootc
fetch/apply is masked. Bundled coding agents and Bitwarden are not packaged yet.

## Next concrete work

The generic Anaconda ISO retains explicit graphical/rescue entries, native bootc
interactive defaults and no preset disk choice. Early generic media failed
SELinux startup; corrected labeling reaches enforcing userspace in
[34180796587](https://github.com/Reidond/kedra/actions/runs/34180796587), but its UI
still failed. ADR 0010 adopts upstream Lorax's permissive installer environment
while requiring the separate installed desktop to remain enforcing. ADR 0011
adapts two version/hash-guarded Anaconda 44.30-2 properties: embedded local payloads
do not require networking, and a locked root does not satisfy the admin requirement.
Updated build [34185915639](https://github.com/Reidond/kedra/actions/runs/34185915639)
at `cfc956d` passes media build/startup and the local offline/disk/admin UI flow.
Actual installation failed GetBlob import in the 1.6 GiB installer /var/tmp, while
the encrypted target had 59 GiB free. The VM is stopped and sentinel comparison
passes. ADR 0013 selected-disk scratch then allowed bootc import/GRUB/finalization
to complete in media 34190162895, but Anaconda cleanup failed on the now-read-only
target. A native-equivalent remount correction is building in 34195114452 at
923a282. Full owner installation/boot remains pending.
This is unsigned localhost-origin research media, not an owner release.

A diagnostic boot of the older media with a temporary kernel override reached
the UI and a deliberate encryption plan on one of two generated disks. Installation
was never started. The VM is stopped; comparison confirms the unselected sentinel
disk remains identical. Actual acceptance must use rebuilt media without that
diagnostic override. See the R02 report and latest worklog entries.

1. Verify/download the updated media and test two fresh disposable VM disks:
   deliberate target selection, encryption, owner account, first boot and an
   unchanged sentinel disk. Never attach host disks. Check the upstream-mentioned
   remount-service concern, offline flow and installed SELinux enforcement.
   Record actual results under R02.
2. Join interactive installation to the strict signed image origin/policy proven
   in R01. Configure release authority, target-bound promotion and recovery only
   after the relevant evidence. No production signing keys exist yet.
3. Implement the narrow helper protocol and fixed installed trust/state paths.
   Verification output from the unprivileged CLI does not authorize deployment.
4. Connect the tested home models to private persistence and coordinated live-file
   activation. Preserve selected snapshots, visible edits, exact local-only policy
   and publication receipts. Test crashes/concurrent writers before real adoption.
5. Package private agent executables behind sysroot, prove personal runtime/profile
   independence, and integrate Bitwarden SSH without exporting keys or broad vault
   sessions. Owner GitHub API/registry/model authentication is separate from SSH.
   A question about the Commercial Terms required for public Claude preinstallation
   is pending with the owner; do not infer acceptance or enable that package yet.
6. Complete RPM refresh/no-change/failure cases, two-target independence, offline
   recovery, owner installation guidance and remaining desktop qualification.

The current workstation must not be enrolled, formatted or switched. QEMU 8.2.2,
OVMF 2024.02 and graphical test dependencies are installed in the existing Ubuntu
WSL2 environment under the user's VM authorization. Research fixtures contain
no owner passwords, vault data or production keys.

## Continuation rules

Use standard Cargo checks; no custom runner or first-party src directories.
Maintain worklog.md at milestones and handoff. Research status.json and reports
are authoritative for case results; a green container build is not an install or
hardware pass. Existing R11 native plugin discovery limitations do not block
reading canonical checkout skills. Never install these skills globally or in the
OS. Read topic skills on demand and update durable findings with source/version,
failure behavior and gate impact.

The acceptance story remains: install A, establish recovery and separate
credentials, use bundled and personal agents independently, preserve three
home-edit dispositions and GUI settings, publish approved content, stage and boot
signed B, verify health, reject bad releases, survive interruption, and roll back
without losing newer personal data. Repeat target updates independently. No final
owner installer or complete management workflow is claimed yet.

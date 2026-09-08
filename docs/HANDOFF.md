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
library but its binary still refuses privileged operations.

Source `d74c4c0c8899ca67960ceff90a42c3750713a9f8` passed Linux
[check 34179189211](https://github.com/Reidond/kedra/actions/runs/34179189211):
formatting, Clippy, 56 tests plus one doctest, release build and independent
OpenSSL interoperability. Eight storage tests cover corruption, links/types/modes,
concurrency and process interruption. This does not prove live-file transactions.

R01 disposable strict Sigstore policy, negative cases, signed A-to-B boot and
rollback pass, including inherited enforcement on initial and rolled-back A.
R02 minimal QCOW2 boots with enforcing SELinux in Actions and local WSL2/KVM.
R07 [run 34179189185](https://github.com/Reidond/kedra/actions/runs/34179189185)
passes graphical login, niri/Noctalia IPC, services, unlocked synthetic keyring
and native Noctalia 5.0.1 settings projection. Virtual display sizing remains
cramped; physical display/audio/suspend and owner credentials are unqualified.

The plain Containerfile consumes a reviewed source archive, installs Fedora
packages, and seeds ordinary writable defaults through /etc/skel for new users.
This is not enrollment or management of an existing home. Automatic bootc
fetch/apply is masked. Bundled coding agents and Bitwarden are not packaged yet.

## Next concrete work

The separate generic Anaconda ISO build
[34176407860](https://github.com/Reidond/kedra/actions/runs/34176407860) at
`76dc82a` passes with explicit graphical/rescue entries, native bootc interactive
defaults and no preset partition commands. The 2,540,959,744-byte ISO SHA-256 is
`8734723fb87db17a129d4293858a0638164ee4db898990453fadd03eed788205`.
It is unsigned localhost-origin research media. The earlier legacy ISO was
rejected for unsuitable text/partition/origin defaults and was never booted.

1. Finish the generic media transfer/checksum and test two disposable VM disks:
   deliberate target selection, encryption, owner account, first boot and an
   unchanged sentinel disk. Never attach host disks. Check the upstream-mentioned
   remount-service concern. Record actual results under R02.
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

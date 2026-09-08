# ADR 0013: Use the selected destination for installer layer scratch

Date: 2026-09-08. Status: implemented; full-install validation pending.

The generic ISO from Actions 34185915639 passes Anaconda startup and the
offline/admin UI flow. Its first actual installation failed during bootc's GetBlob
import: writing `/var/tmp/container_images_...` exhausted the installer filesystem.
In the 8 GiB VM that filesystem has 1.6 GiB capacity, while the encrypted destination
still had 59 GiB available. The selected disk was partially written. After stopping
the VM, qemu-img comparison confirms the unselected sentinel remains identical.
This is a failed install, not a first-boot pass.

The containers/image local-store transport stages an uncompressed layer as a large
temporary file. Its default big-file directory is `/var/tmp`; changing TMPDIR does
not reliably change that path. Increasing VM RAM would conceal the installer's
disk-scratch requirement and would not support larger payloads.

Extend the exact-source Anaconda 44.30-2 adapter with a guarded DeployBootcTask
change. After native storage approval and root cleanup, create a fresh private
scratch directory inside the selected mounted physical root. Bind it to itself so
bootc 1.16.10's empty-root check recognizes a mount boundary, then bind it over the
installer's `/var/tmp` for the bootc subprocess. Keep the original bootc arguments
and signature behavior. Do not introduce a private mount namespace: Anaconda
depends on native installation mounts remaining visible afterward.

After the subprocess finishes or fails, unmount `/var/tmp`, unmount scratch, and
remove the now-empty directory. No forced/lazy unmount or recursive cleanup is
used. Busy mounts or unexpected residual contents stop installation and remain
available for explicit recovery. Reject a missing mount or the running root before
scratch creation. This does not select, format or inspect another disk.

The adapter retains the upstream GPL header and refuses unknown original source
hashes. Inert lifecycle tests cover success, both bind failures, deployment failure,
unmount failure, and invalid roots without performing host mounts. Running those
exact-source tests against local copies passes. Actual disk-backed import, cleanup,
owner creation and first boot require the next rebuilt-media experiment.

Sources reviewed 2026-09-08:

- [containers/image big-file temporary paths](https://github.com/containers/image/blob/main/internal/tmpdir/tmpdir.go)
  and [local-store GetBlob](https://github.com/containers/image/blob/main/storage/storage_src.go).
- [bootc 1.16.10 installation source](https://github.com/bootc-dev/bootc/blob/v1.16.10/crates/lib/src/install.rs):
  prepare_install runs before the empty-root check; require_dir_contains_only_mounts
  accepts mount boundaries. Pointing TMPDIR at an ordinary target subdirectory
  would conflict with that check.
- [Independent Dakota installer experiment](https://github.com/projectbluefin/dakota-iso/blob/main/docs/skills/e2e-ci.md#enospc-root-cause-and-fix)
  records the same ENOSPC symptom and successful disk-backed `/var/tmp` binding.
  Its unrelated debug/VM settings are not adopted here.

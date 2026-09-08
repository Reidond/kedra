# R10: protected persistent-state subset

Date: 2026-09-08 (Europe/Kiev). Linux storage subset: pass.

Installed-helper native subset now passes
[34208979891](https://github.com/Reidond/kedra/actions/runs/34208979891) at `bb53ce7`.
The real helper enrolls signed A, refuses unprivileged calls, extra protocol fields,
policy drift, invalid metadata and unsigned/wrong-key/missing-signature images,
preserves an existing pending digest, stages/boots B, records booted source identity,
rolls back to A while retaining newer data and trust high-water state, enforces
the rollback hold, and resumes only when explicitly requested. Valid signed
metadata advances its anti-replay high-water even when native image verification
fails; the running/staged OS remains unchanged in those image-refusal cases.
No production trust/key or public promotion is configured. Native process-kill,
full-disk journal faults and live-home reconciliation remain separate open cases.
See [ADR 0016](../../adr/0016-installed-deployment-helper.md).

[Run 34179189211](https://github.com/Reidond/kedra/actions/runs/34179189211)
at `d74c4c0c8899ca67960ceff90a42c3750713a9f8` passed formatting, Clippy,
56 workspace tests plus one doctest, release build and OpenSSL interoperability
on Ubuntu 24.04/Rust 1.98.1. Eight Linux storage tests include concurrency,
corruption, link/type/mode/inode checks and process exit before/after commit.
The preceding run 34179085588 failed at Clippy because rusqlite's limit setters
return Results; the correction propagates those errors.

Added a Linux-only storage library target to the existing helper package. The
binary still refuses operations. ADR 0008 specifies the private-directory,
owner/link/type/inode guards, version/checksum checks and transactional history.

Windows workspace compile/format/Clippy checks pass, but they do not compile or
execute this Linux-only storage implementation. The recorded Actions run executes
the generated-directory tests. No workstation home, root state, vault or
enrollment was touched.

Still required: privileged protocol authorization, fixed
installed paths/environment, source/manifest eligibility integration, live-file
activation, full-disk/permission faults, boot journals and offline recovery. A
successful storage test alone cannot open those other R10/R04 gates.

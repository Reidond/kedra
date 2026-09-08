# ADR 0012: Persist the qualified Noctalia projection as ordinary-user review state

Date: 2026-09-08. Status: implemented; native review subset verified.

ADR 0007's pure three-field disposition model and ADR 0008's private SQLite
storage have separate passing evidence. Connect them in the ordinary-user sysroot
package. Linking the helper package's storage library does not launch its binary,
elevate the caller or grant deployment authority. Three Cargo packages remain.

Initialization reads the installed source manifest and verifies the Noctalia
baseline's destination, hash and source provenance. System input reads pin a
regular inode with openat2 before reading through its held descriptor; symlinks,
special files, unsafe ownership/modes and unexpected sizes are refused. The
three reviewed fields are explicit in the baseline; border defaults remain true
as observed in native R07 run 34184975810. No effective default is guessed.

The ordinary user chooses a private store path. Parent aliases, including bootc's
/home to /var/home mapping, resolve to a user-owned parent without other-user write
permissions. The store itself retains the existing strict owner/mode/link checks.
This is protection from accidental unsafe locations and other users, not a sandbox
against a malicious process with the same UID. Machine/user-bound canonical state
is updated with compare-and-swap; missing/corrupt/incomplete records never reset.

Native Noctalia full export is bounded and timed out. Raw exports are projected
before persistence, and errors do not quote private parser input. Review captures
the current value while keeping S pinned across later application writes. Exact
local-only values and persistent app-owned fields remain separate dispositions.
Failed decisions do not commit the newly observed capture. Unchanged captures do
not create needless history revisions.

The initial commands cover initialization, status, selection and local dispositions.
They do not implement generic text/line review, source export, discard or activation.
Those remain R03/R04 work. The R07 generated-account experiment in run 34188179252
at 3d108e9 passes the CLI against native GUI writes, including durable selection
and local dispositions. Workspace run 34188179270 passes 71 Linux tests plus one
doctest. This does not qualify the unfinished source-export or activation operations.

Evidence: docs/research/R03-home-review/REPORT.md, docs/research/R07-desktop/REPORT.md,
crates/sysroot/home/linux/tests.rs and build/research/r07/check.sh.

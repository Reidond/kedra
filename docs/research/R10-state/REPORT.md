# R10: protected persistent-state subset

Date: 2026-09-08 (Europe/Kiev). Linux execution: not-run, pending Actions.

Added a Linux-only storage library target to the existing helper package. The
binary still refuses operations. ADR 0008 specifies the private-directory,
owner/link/type/inode guards, version/checksum checks and transactional history.

Windows workspace compile/format/Clippy checks pass, but they do not compile or
execute this Linux-only storage implementation. The next Actions check will run
the generated-directory tests, including process termination before and after
SQLite commit. No workstation home, root state, vault or enrollment was touched.

Still required: actual Linux results, privileged protocol authorization, fixed
installed paths/environment, source/manifest eligibility integration, live-file
activation, full-disk/permission faults, boot journals and offline recovery. A
successful storage test alone cannot open those other R10/R04 gates.

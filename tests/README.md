# End-to-end and manual verification

The owner chose end-to-end and manual testing only on 2026-09-08; see AGENTS.md.
Unit/model/mock tests, doctests and repository self-checks are removed.

`cargo test --workspace --test 'e2e_*' --locked` runs the actual CLI agent-launch
and home-export workflows on Linux using generated data and real subprocesses,
Git and SQLite. `release-interop.py` checks the public CLI against independent
OpenSSL signatures. Installed application, installer, signing/update/rollback
and recovery behavior belongs in the disposable VM workflows or recorded manual
tests. Standard Rust formatting, Clippy and builds remain required.

Historical research reports retain the results of tests that existed at their
recorded commits. They are not instructions to recreate unit tests. No real home,
vault data, transcripts or production signing keys enter test fixtures.

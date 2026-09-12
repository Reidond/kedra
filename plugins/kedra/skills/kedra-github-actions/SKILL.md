---
name: kedra-github-actions
description: Maintain Kedra build, midnight package refresh, signing, installer and VM workflows.
---

# Actions

Read build/release/README.md, docs/UPDATES.md and docs/ARCHITECTURE.md. OS/ISO builds belong in Actions; local Rust/CLI checks are allowed. check.yml uses standard Cargo tools and real CLI/OpenSSL E2E. test-* workflows retain actual desktop, signed-update, home transition and installer coverage.

The refresh trigger is 00:00 UTC. Actions can queue or skip schedules; never claim exact completion time. Package checks must execute with fresh metadata, stable reviewed repositories and complete installed closure. DNF layer cache cannot establish freshness. Failed resolution is not no-change. See references/fedora-refresh.md.

Pin Actions and external inputs. Minimize permissions and disable persistent checkout credentials. Build jobs have public trust; protected signing jobs execute no checkout, candidate code or repository scripts with production keys. Never bypass environment review, current-main checks, exact digest or predecessor ordering to automate publication.

Candidate image signing, exact-media qualification, metadata signing and promotion are distinct. Keep immutable version assets and a single verified mutable channel bundle. Preserve uncertainty after partial publication; no blind overwrite.

Production image-builder is pinned in installer/inputs.json. Its generic ISO uses --bootc-installer-payload-ref; older bootc-image-builder interfaces differ. Preserve native OCI digest across registry/storage/media copies. Installer labeling and media-only permissive SELinux do not weaken installed enforcing SELinux/signature policy.

Run real changed workflow coverage and record exact source/run in STATUS/worklog. Do not add tracked research outputs or repository self-check code.

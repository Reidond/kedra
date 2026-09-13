---
name: kedra-github-actions
description: Maintain Kedra signed-container CI, midnight package checks and native VM workflows.
---

# Actions

Read docs/UPDATES.md, docs/RELEASES.md and docs/ARCHITECTURE.md. OS image builds run in Actions. ISO construction is explicit/local through installer/build-local.py and never uploads. Do not create GitHub Releases, machine bundles or ISO/checksum assets.

The 00:00 UTC trigger reconciles the reviewed official Fedora 44 base and complete installed RPM closure. Do not let cached DNF layers claim freshness. Required repository, signature or solver failure is an error. Changed inputs produce a candidate; identical inputs do nothing, with no checkpoint renewal.

Build jobs have public trust. Automatic isolated signing executes no checkout/candidate/repository code while production keys exist. The main-only environment has no human approval gate. Sign and verify exact OCI digest/repository, then advance GHCR stable only after current-source and ordering checks. Pin Actions/tools and minimize credentials.

check.yml uses standard Cargo tools and actual CLI/OpenSSL workflows. Native tests retain signed-update, desktop, home-transition, agent and RPM coverage with disposable inputs. No unit/model/mock/doctests or repository scanners.

Local installer changes retain pinned image-builder, labeling, offline payload verification and deliberate disk choice. Media permissive SELinux never weakens installed enforcing SELinux/signature policy. Record actual local smoke/fresh-install results separately from image builds.

Update STATUS/worklog with exact observed outcomes. Never call staged booted, signed installed or syntax qualified.

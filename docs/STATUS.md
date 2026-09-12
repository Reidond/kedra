# Verified status

Implementation and cross-review on `codex/iso-only-releases` are complete for the owner's final model: **signed GHCR images only**, direct stable-image updates, no-change does nothing, and local on-demand ISO construction. At exact source `f52c9e77e17eb029a6b25c5a23807ef655484795`, direct-GHCR A/B/A native qualification and the workspace/desktop/legacy-signed/home regressions pass. Remaining native/production cases are listed below. The intermediate ISO-only GitHub Release layout was superseded.

Local checks pass: Windows Cargo 1.98.1 formatting, Clippy with warnings denied and release build; the Cargo E2E command passes with **zero Windows cases**. New image-material CLI sealed-image, no-change/content, rank/replay, tamper and scope cases pass, as does legacy OpenSSL interoperability. Python AST parsing of 26 files, WSL Bash syntax, delegated validation of nine workflow YAML/Bash definitions, whitespace/reference checks and GPT-6 Astra cross-reviews pass. Reviews corrected backwards resolution-time ordering, retained rollback during identity-health failure, shared bootc compatibility/native path filters and migration prerequisites. The exact-source Linux/native outcomes below supplement these local results; production stable publication and full local-installer execution remain separate.

## Current boundaries

`installer/build-local.py` creates one local ISO from an explicitly reviewed signed image with the pinned builder, strict native policy, offline payload verification and no upload. Its help, refusal and syntax checks are available; full local build/fresh-install qualification remains not-run. The GitHub ISO workflow and obsolete publication qualification document were removed. Current operational commands are described in [INSTALL](INSTALL.md) and [UPDATES](UPDATES.md).

The signed candidate from [34697167136](https://github.com/Reidond/kedra/actions/runs/34697167136) at `b4e9f781154bced4f4006a3bb0d0d9e2623fcdb0` completed manual protected image signing and installer construction. Its exact 2,862,204,928-byte ISO (`e01393b2f64256bb489a3983eaf7fa394f0e97e958e93044aded3532896e13c2`) passed all ten fresh encrypted offline VM qualification cases. Both sentinel disks compared identical after clean shutdown; temporary credentials were deleted and guests stopped. This is retained historical evidence for that exact source/image.

Promotion [34707063280](https://github.com/Reidond/kedra/actions/runs/34707063280) passed preparation and was cancelled before protected metadata signing for the distribution correction. No r3 release or new metadata signature was produced. The final GHCR-only source requires a new current-main build and manual OCI signing after merge.

## Existing remote artifacts pending cleanup

Historical GitHub Releases remain until the direct-GHCR updater and explicit legacy migration are validated. Deleting them earlier breaks old clients; current development does not authorize premature deletion.

| Legacy record | Observed state |
|---|---|
| r1 | Release ID 385117864; 13 uploaded assets; source c660c58d9bbbbe34119f6ea35a03528485455848 |
| r2 | Published/latest release ID 385328126; 17 exact assets; source 0eb1cf09c0eab5f4488a780552f51582d3e92bdf |
| Legacy channel | Release ID 385128562; sole asset 559221144; sequence/generation 2; expires 2026-09-16T07:50:54Z |

R2 key-free recovery preserved original approved bytes; failed promotion 34325343190 remains failed. The qualified newer candidate and r2 are distinct. R2's ISO SHA-256 is 15ddfafb8567297220e02831b6639f0fceb7efce0cd47bcf799fae97c5a1eb4b. Full historic identities remain in worklog. No remote cleanup has occurred in this source change.

## Prior accepted checks

At a221c7e, workspace 34695674811, agents 34695674783, RPM 34695674806, signed-update 34695674826, desktop 34695674736, home-transition 34695674733 and installer 34695674820 passed. Workspace 34696414523 passed at 0c5c670. Retired observer 34695674816 remains failed; it never made an equivalence/freshness decision.

At `f52c9e7`, workspace [34714337528](https://github.com/Reidond/kedra/actions/runs/34714337528) and [34714339961](https://github.com/Reidond/kedra/actions/runs/34714339961), direct GHCR [34714337561](https://github.com/Reidond/kedra/actions/runs/34714337561), desktop [34714337517](https://github.com/Reidond/kedra/actions/runs/34714337517), legacy signed update [34714337526](https://github.com/Reidond/kedra/actions/runs/34714337526) and home transition [34714337529](https://github.com/Reidond/kedra/actions/runs/34714337529) all pass. Direct-GHCR artifact `10304408023` is 1,577,576 bytes, ZIP SHA-256 `46ba4dda3f3a3ab5b099274486d3121f3a5386c8bb49fb31659bf0ca6f6f8a2d`, using official Fedora base `170e6e9b15d0678865d32d4052eee3a62606eaeedd52b0b51d1a32e4fc541436`.

The direct-GHCR workflow passes actual v2 enrollment/check/stage and signed A/B/A boots, signature/scope refusals, replay/equivocation, offline failure, pending preservation, idempotence, identity-health retained rollback, persistent data, hold and resume. It uses a disposable local TLS registry and generated authority; it is not production GHCR publication. Legacy-state migration, deterministic tag race, helper/bootc interruption, actual OCI-platform mismatch, production stable publication and full local ISO construction remain not-run. Remote r1/r2/channel deletion stays deferred until the bridge and migration are verified. Physical/Secure Boot, owner forward update and key rotation remain separate gates.

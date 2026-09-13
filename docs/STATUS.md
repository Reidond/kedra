# Verified status

The owner’s current policy is **automatically signed GHCR images only**. At 00:00 UTC, changed inputs must pass public validation, isolated OCI signing and strict verification before stable publication. No human approval or manual signing action is required. Unchanged inputs publish nothing. Local ISO construction remains on demand and never uploads.

The automatic-signing source change is prepared on codex/automatic-ghcr-signing and is not yet committed or deployed. The reviewed environment change is complete: reviewer rule 64952111 was removed; environment 21492153153, administrator bypass disabled and sole main deployment rule 59436566 remain. Both signing secrets and both public variables retain their prior update timestamps; the public fingerprint is unchanged. Publication opt-in remains temporarily false. No production GHCR stable publication is claimed.

## Removed legacy downloads

At the owner’s explicit direction, GitHub Release IDs 385117864 (r1), 385328126 (r2) and 385128562 (legacy channel), including all 31 uploaded assets, were deleted. The remote Releases list is empty. Their source Git tags remain; no source tag was removed. Historical signed-image/installation results remain in Git history and worklog, not as current download availability.

Legacy r1/r2 online update discovery depended on the deleted channel and no longer works. Retain local signed records, journals, high-water and rollback hold. Follow the explicit [legacy migration](UPDATES.md#one-time-legacy-migration) after a compatible signed GHCR image is available. Existing local boots and saved recovery media do not depend on the deleted Release assets.

## Verified implementation and outstanding production execution

At exact f52c9e77e17eb029a6b25c5a23807ef655484795, workspace runs 34714337528 and 34714339961, direct GHCR 34714337561, desktop 34714337517, legacy signed update 34714337526 and home transition 34714337529 pass. Direct-GHCR artifact 10304408023 independently matched ZIP SHA-256 46ba4dda3f3a3ab5b099274486d3121f3a5386c8bb49fb31659bf0ca6f6f8a2d.

The direct-GHCR VM test covers actual v2 enrollment/check/stage and signed A/B/A boots, signature/scope refusals, replay/equivocation, offline failure, pending preservation, idempotence, identity-health retained rollback, persistent data, hold and resume. It uses a disposable local TLS registry and generated keys; it does not publish to production GHCR.

Production 34715490862 failed because the public build could not read an environment-scoped fingerprint variable. The subsequent scope fix retains source-key validation, independent signer-environment comparison and signer-output binding in the publisher. Production run 34717057402 at 7e923d4907e3b5caa28260983a0e7ef884a3dd87 passed its public build and was cancelled before signing for the final policy change. Queued schedule 34728220456 was also cancelled. Neither cancellation is successful signing or stable publication.

The shared bootc compatibility contract, exact image identity/material checks and local CLI/OpenSSL interoperability pass their recorded checks. Full automatic production signing/stable publication, legacy-state migration, deterministic registry races/interruption, actual OCI-platform mismatch and full local ISO construction remain unqualified. Physical/Secure Boot, owner forward update and key rotation are separate boundaries.

The earlier signed candidate 34697167136 at b4e9f78 passed its exact fresh encrypted offline installation and ten qualification checks. That historical result does not qualify later GHCR-only source, restore deleted Release assets, or establish a current stable tag.

[INSTALL](INSTALL.md) describes local media construction; [UPDATES](UPDATES.md) describes direct updates and migration. The [worklog](../worklog.md) retains exact historical evidence and the next operational step.

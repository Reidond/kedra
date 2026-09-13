# Direct GHCR updates

Read docs/UPDATES.md. Current source enrolls with sysroot update enroll, discovers stable with update check and stages its verified exact digest with update stage. It does not reboot or apply home changes.

The fixed installed helper validates native OCI signature/repository/target, image-owned identity and retained ordering. User checkouts are irrelevant to authority. Rollback holds forward updates; explicit stage --resume clears the hold after normal checks. Persistent home and /var are not rewound.

The owner confirmed on 2026-09-13 that nobody installed r1/r2 (docs/STATUS.md). Fresh installations enroll directly; no legacy bridge is required for delivery. Retained compatibility code refuses unreconciled legacy operations, and older readers refuse newer state. Never delete journals or clear a hold to bypass a refusal.

Do not claim signed checkpoint freshness for the new path; there are no channel/release assets or renewal. No-change CI does nothing. Image/signing age, publication and actual successful package resolution remain distinct observations.

Local recovery must work through the retained image/boot menu or locally constructed verified installer without AI or GitHub Releases.

# Direct GHCR updates

Read docs/UPDATES.md. Current source enrolls with sysroot update enroll, discovers stable with update check and stages its verified exact digest with update stage. It does not reboot or apply home changes.

The fixed installed helper validates native OCI signature/repository/target, image-owned identity and retained ordering. User checkouts are irrelevant to authority. Rollback holds forward updates; explicit stage --resume clears the hold after normal checks. Persistent home and /var are not rewound.

Old r2 software requires deliberate migration. First run legacy sysroot update status on the old OS and require no pending intent, awaiting-reboot operation, staged deployment/replacement or queued rollback. Complete/reconcile prior work before switching; preserve rollback hold/high-water and signed records. Then use the owner-reviewed exact signed bootc switch with --enforce-container-sigpolicy, chosen reboot and enroll --migrate-legacy --expected-digest. Import refuses unreconciled legacy operations; the v2 status path cannot repair a v1 journal. Older readers refuse newer state. Never delete journals or clear a hold to make migration succeed.

Do not claim signed checkpoint freshness for the new path; there are no channel/release assets or renewal. No-change CI does nothing. Image/signing age, publication and actual successful package resolution remain distinct observations.

Local recovery must work through the retained image/boot menu or locally constructed verified installer without AI or GitHub Releases.

# Direct GHCR updates

Read usr/src/kedra/docs/UPDATES.md. `sysroot update check` reads the identity label of the fixed repository's stable image and reports whether it is newer than the booted image; `sysroot update stage` stages its exact digest with `bootc switch --enforce-container-sigpolicy`. Neither reboots or applies home changes. There is no enrollment: trust comes from the booted image.

bootc verifies the signature with the installed strict policy before it fetches the configuration or any layer, and downloads each new layer once. The fixed installed helper (crates/sysroot-helper/management/ghcr.rs) adds what the policy cannot: the signed identity's target, architecture and channel, and refusal of an image older than the booted one (bootc deploys any validly signed digest). `check` does not verify signatures. User checkouts are irrelevant to authority. Rollback holds forward updates; an explicit `stage --resume` clears the hold after a successful stage. Persistent home and /var are not rewound.

The only root state is the rollback hold (`/var/lib/sysroot/deployment`, state schema 3; helper protocol schema 3). Older stores are refused, never migrated; never delete state or clear a hold to bypass a refusal.

Do not claim signed checkpoint freshness: there are no channel/release assets or renewal, and a registry can withhold newer images (`sysroot update status` warns when the signed resolution is older than 36 hours). An unchanged release run does nothing. Image/signing age, publication and actual successful package resolution remain distinct observations.

Local recovery must work through the retained image/boot menu or locally constructed verified installer without AI or GitHub Releases.

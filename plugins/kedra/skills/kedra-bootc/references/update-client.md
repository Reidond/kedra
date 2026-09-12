# Installed update client

Read docs/UPDATES.md for actual commands. Current-source check reads the fixed signed channel; enroll --channel and stage --channel use installed trust and independent helper verification. These commands do not reboot or apply home changes. Published r1 retains the explicit signed-file workflow.

A source checkout need not exist or be current to update the OS. Never reset or pull user source as part of a check. Digest-pinned bootc advances using verified exact-digest switching; normal upgrade does not move that digest.

Keep pending-deployment safety and rollback holds. Repeated state is distinct from replacing another pending image. --resume clears a hold only after normal verification. Replay floors survive rollback; same-generation changed metadata and expired incoming checkpoints refuse.

Source, image build age and successful repository-resolution freshness are distinct. Failure is not current/no-change. Historical-only predecessor authentication cannot grant incoming eligibility.

User home preflight/recovery remains user-owned and separate from root deployment. Staging is not live home apply. Retain deterministic TTY/boot-menu recovery and unknown-schema refusal.

No background notification or automatic staging service is claimed by these commands. Do not enable competing upstream fetch/apply/reboot timers incidentally; inspect actual policy before adding one.

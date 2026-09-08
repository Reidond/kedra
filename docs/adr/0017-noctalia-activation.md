# ADR 0017: Coordinate narrow native Noctalia activation

Date: 2026-09-08. Status: implemented for qualification; Linux/native results pending.
Gates: R03, R04, R10. Qualified application target: Noctalia 5.0.1 only.

Noctalia retains its override table in memory and writes it atomically. Editing
settings.toml while its writer remains active can lose either side's changes.
Native R07 run 34212238491 at 3ce1c1a proves immediate stop after an IPC setting
write preserves the setting and permits restart. Upstream source at
f95e95cafde8c23f1d3a62b969e2b5717c96d741 implements synchronous override writes and
graceful SIGTERM handling. This establishes a writer boundary, not general home
activation or crash-recovery qualification.

The user reviews `sysroot home --state PATH plan`, then passes its exact plan ID
to `apply`. `plan --discard KEY` and `discard KEY --plan ID` restore just that
field to its selected, published or accepted value. Plans contain only the three
supported safe fields and source provenance. Unknown/private native values stay
outside the review database and source export.

The Linux bridge supports the default profile and the installed
kedra-noctalia.service without user overrides. Fedora systemd 259.8 supplies
/usr/lib/systemd/user/service.d/10-timeout-abort.conf globally; the manually
inspected file is root-owned, regular and contains only the Service section and
TimeoutStopFailureMode=abort beyond comments. That exact path/directive is allowed
after trusted-file checks; other directives and paths still refuse. It serializes cooperating operations
at both the review store and application directory, stops the managed writer,
recaptures effective values, and modifies only changed fields with toml_edit.
Unexpected file types, links, ownership, writable-by-others modes, unsupported
ACLs/xattrs, labels/groups that cannot be preserved and unqualified app versions
refuse. No root privilege or checkout script is involved.

Before changing the live name, one SQLite transaction reserves review mutations
and writes the recovery journal. Private atomic-I/O temporary files preserve the
original full native file until successful read-back; these are not review
snapshots and are never exported. File receipts contain identities and hashes,
not native file bodies. Existing files use rename exchange; absent files use
no-replace. Detected competing changes retain data for explicit recovery.

After restart and native validation/read-back, persist Validated, clean the known
checkpoint, then commit the accepted state and Completed journal together.
Recovery recognizes a rename completed before its journal write and a cleanup
completed after durable validation. A pending marker makes older strict readers
refuse and blocks other review mutations; removing it preserves the previous
state format. The SQLite storage schema remains unchanged.

`recover` inspects the safe journal; `recover resume` continues only with matching
state and installed baseline; `recover abort` restores only exact recorded file
versions; `recover keep-current` explicitly keeps live data, clears the pending
reservation without advancing the baseline, and leaves private checkpoints for
inspection. Changed or missing recovery data fails rather than inventing success.
After validated cleanup, use resume or keep-current; abort cannot manufacture a
deleted old checkpoint. Pre-reservation interruption can leave an unreferenced
private candidate; it never changed the live file and is not automatically swept.

The lock coordinates this CLI, not arbitrary same-UID or root programs. Native
read-back cannot establish protection against a malicious peer or a writer that
changes a file after every check. Multi-file home groups and generic line files
remain separate unimplemented work. No workstation home may be enrolled as a
substitute for R04 tests in generated fixtures and disposable desktops.

Sources: Noctalia v5.0.1 source (config-overrides.cpp, application-services.cpp),
[native lifecycle run](https://github.com/Reidond/kedra/actions/runs/34212238491),
ADRs 0007, 0008 and 0014; tests in the activation and replace_file modules.

The isolated tests cited here are historical evidence at their recorded commits.
They were removed on 2026-09-08 under the owner's end-to-end/manual-only policy.
Native desktop tests now provide the ongoing activation qualification.

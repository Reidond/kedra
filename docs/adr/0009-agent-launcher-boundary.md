# ADR 0009: explicit agent runtime and version-specific state

Date: 2026-09-08. Status: Linux wrapper tests pass (run 34181692426 at 082f8d4);
native matrix and distribution qualification pending.

Keep agent execution entirely in the unprivileged CLI. Use absolute private
bundled paths and explicit personal-runtime selection. Validate the checkout's
local origin and committed editing target without fetching or rewriting work.
Pass arguments directly and replace the wrapper process using Unix exec so
upstream interaction, exit status and signals remain intact.

Use an inherited advisory file lock to coordinate wrappers on a checkout. This
does not constrain ordinary editors, malicious same-user code or agents that
close inherited descriptors. A long-lived child can retain the lock; separate
worktrees are the escape for intentional concurrent work, not lock-file deletion.
No daemon, custom Git engine or privileged execution path is introduced.

Place management state under the user's private XDG state root, separated by
agent, runtime class and exact reported version. This intentionally avoids
cross-version schema migration on an OS rollback, at the cost of a possible new
official login after a runtime update. Never copy OAuth sessions between profiles.
Keep existing state for explicit later recovery. HOME stays unchanged. Config
directory controls are not a sandbox and propagate to upstream child processes.

Seed only new Codex management configuration with keyring credential storage.
Retain existing preferences. Apply Claude's update-disable control only to the
bundled invocation. Save/restore caller controls across nested wrappers selecting
personal scope. Always exclude broad BW_SESSION values while retaining SSH agent
and native authentication interfaces. No model backend, token synchronization,
global skill installation or automatic personal-runtime update is added.

The launcher guide and R05 report record source/version details, failure behavior,
test results and remaining preinstallation/discovery/authentication gates.

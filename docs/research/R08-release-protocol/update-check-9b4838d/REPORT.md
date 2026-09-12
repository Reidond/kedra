# Installed current-channel check with the development CLI

Recorded 2026-09-09 (Europe/Kiev), Codex. **Pass for the current r1 channel,
preserved installed status, URL/root refusal and download-environment isolation.**

This is an actual manual/E2E continuation of the retained, enrolled owner r1 VM.
The installed OS remains source `c660c58d9bbbbe34119f6ea35a03528485455848`, image
`sha256:bb4f2b68996a4fc260972d9654440f62ee078bcf92a0996a8f7e34ea0133118b`.
A separately verified development CLI ran from the ordinary owner's new private
working directory. It did not replace `/usr/bin/sysroot` or the installed helper.
The VM had no installer ISO or host port forwarding; QEMU user NAT supplied the
connection to the fixed public channel. No installed trust or profile was changed.

## Exact development artifact and procedure

| Item | Identity |
|---|---|
| Development source | `9b4838d7de56233aca023e24c41b6d5d62c6c342` |
| Native workspace run | [34296984680](https://github.com/Reidond/kedra/actions/runs/34296984680) |
| Linux CLI artifact | `10083542936` |
| Artifact ZIP SHA-256 | `82c0d17817b10ff916ca63e8234f2eebd097b32f1f1653a0b87642c79cc90965` |
| Development CLI SHA-256 | `c994926557c3e34c172b1890589817db1bd80bbc8436382ff1dcffb258162817` |
| Private manual-runner SHA-256 | `dbd55b933c3f5a7fef26daa64b1eaa90efa33bfc34929c9e7a03334d47e8c586` |

The exact binary and runner hashes were confirmed in the guest before execution.
The temporary transfer server served only the authorized files over WSL loopback
and was stopped afterward. It was separate from the command's real anonymous
HTTPS request to the existing GitHub channel. This development artifact is not an
owner-signed OS release or installer.

The ordinary owner authenticated sudo, then ran the private Bash runner. Each real
CLI invocation retained separate stdout, stderr and exit-code files in a new guest
evidence directory. It observed installed `/usr/bin/sysroot update status`, ran
`./sysroot update check --json`, and observed installed status again. It then
exercised an unsupported `--url` argument, an actual root invocation, and an
ordinary check with deliberately invalid `HTTPS_PROXY` and `CURL_CA_BUNDLE`
environment values, followed by a final installed-status observation.

The capture copied only uniquely labelled public JSON blocks from the guest's
existing serial log. The original complete log and private guest files remain
unchanged and are not published. JSON formatting in these files is canonicalized
for readability; the runner's exact-byte status comparison is retained in its
result record.

## Actual observations

The [current check](current-check.json) returned exit 0, `state: current`, valid
signatures, fresh channel metadata and `replay_checked: true`. It verified release
sequence 1 and checkpoint generation 1 against the existing installed floor and
matched the booted r1 image and its installed source provenance. The public-key
SPKI fingerprint remained
`a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e`.

The [before](current-status-before.json), [after](current-status-after.json) and
[final](current-status-final.json) installed responses were exactly equal in the
guest. Enrollment, replay high-water, booted digest, empty staged/rollback slots,
empty operation and absence of a rollback hold were preserved. Both successful
checks reported no staging, reboot, home activation, deployment authorization or
high-water advancement, and supplied no `next_trust_state`.

| Case | Result | Evidence |
|---|---|---|
| Ordinary fixed-public-channel check of enrolled r1 | pass, exit 0 | [Current check](current-check.json) |
| Exact installed status preserved across all cases | pass | [Before](current-status-before.json), [after](current-status-after.json), [final](current-status-final.json), [runner result](current-result.json) |
| Arbitrary endpoint override refused | pass, exit 2 and empty stdout | [Runner result](current-result.json) |
| Root invocation refused by the actual CLI | pass, exit 78 and empty stdout; expected ordinary-user diagnostic checked | [Runner result](current-result.json) |
| Invalid proxy/CA environment does not alter the download | pass, exit 0 and verified current r1 | [Isolated-environment check](current-isolated-environment.json) |

The existing helper Status operation can reconcile an earlier deployment operation.
These stable-r1 observations demonstrate unchanged status for this run; they do
not establish a general promise of byte-for-byte read-only journal storage.

## Remaining qualification

Available, staged, booted-new-release and held states are **not-run** for this
development CLI. NIC-down failure/restoration and a doctor run for this exact
continuation are also **not-run** at this milestone. Malformed/oversized HTTP
responses, signature/expiry/replay failures and deterministic concurrent-state
refusal have not been exercised through this fixed public endpoint.

The separate v2 fresh-installation and promotion work is not evidence for these
remaining command cases. The next steps are a controlled guest-only network
failure/restoration check, then actual available/staged/booted observations around
the independently qualified and promoted r2 image. No enrollment, staging,
rollback, hold release or checkpoint renewal occurred in this current-state run.

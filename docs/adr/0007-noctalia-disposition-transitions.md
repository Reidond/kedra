# ADR 0007: narrow Noctalia projection and independent review dispositions

Date: 2026-09-08. Status: accepted for the pure state model; activation remains R04.

The Noctalia adapter consumes `noctalia config export full` from the qualified
5.0.1 binary. It retains only theme.mode, shell.button_borders and
shell.input_borders. Required fields must be present; unknown fields are discarded
before serialization, and malformed input errors never include raw excerpts.
It does not reimplement Noctalia's include/override/default loader or capture a
state directory. Other settings remain outside this initial projection.

The model stores accepted baseline, latest safe live projection, pinned selected
values, exact local-only decisions, persistent app-owned keys and per-key source
publication chains separately. Selecting a value never follows a later GUI write.
Local-only/app-owned keys cannot enter the selected export. Exact local-only policy
expires when its value changes; app-owned policy persists until explicitly cleared.

Source commit receipts consume selection only after the caller observes the
selected values in that separate source commit. They do not claim a network push
or deployment. A chain is needed when source advances twice before the first image
boots: deploying the first selected value must retain the second pending value
and a still-newer live edit. A three-way merge of B/L/N alone cannot express this.

Preparing a baseline transition changes nothing. Known published/selected values
already present in N establish the correct merge anchor; only the fulfilled
publication prefix is retired. Unrelated N changes preserve selection/policy.
Foreign overlapping changes stop the whole group with all prior state retained.
Acceptance requires the same review-state fingerprint and observed desired values.
The separate activator must coordinate writers, validate, durably apply and read
back files before calling acceptance; this model performs no file mutation.

Internal state uses versioned canonical JSON with an installation-instance binding.
Loading checks invariants and exact reserialization, preventing duplicate map keys
or omitted fields from silently dropping policy. Unknown/corrupt/cross-instance
state fails; there is no empty-state fallback. This proves serialized-state
reconstruction, not crash-safe filesystem persistence.

Ten synthetic tests and a path-free Cargo example cover the model. The R07 VM
experiment will additionally feed the actual native full export through the
projection, change a setting through Noctalia IPC and verify the projected result.
Real adoption, plain-text integration, source writes and writer/crash recovery are
still unavailable. See the R03 Noctalia report for observed results.

Sources: PLAN.md section 6; home state-model skill reference;
[Noctalia configuration/export](https://docs.noctalia.dev/noctalia/configuration/),
reviewed 2026-09-08; local Rust 1.98.1 synthetic tests.

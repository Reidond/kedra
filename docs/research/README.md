# Research evidence

Use RESEARCH.md for the full R01-R11 work specifications. `status.json` is an
aggregate index; individual cases in reports are authoritative. `blocked` does
not prohibit unrelated safe work. No case passes merely because a feature is
documented or its scaffold exists.

For each packet, create `docs/research/Rxx-topic/REPORT.md`, `environment.json`,
`results.json`, and small synthetic fixtures or redacted logs. Record question,
approaches, exact source/toolchain/package/image/runner identities, workflow run,
expected and actual outcomes, exit codes, privacy review, limitations, and ADR.
Use only `not-run`, `pass`, `fail`, or `blocked` per case. Separate static checks,
real agent discovery, VM boot, real accounts and physical hardware evidence.

Never upload real tokens, keys, vault caches, agent transcripts, private home
snapshots, serials, or unredacted logs. Put large non-sensitive test artifacts in
CI storage with retained checksums and references, not unchecked repo archives.

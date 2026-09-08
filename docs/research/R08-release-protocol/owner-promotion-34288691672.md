# Owner release r1: exact-byte publication recovery

Recorded 2026-09-09 (Europe/Kiev), Codex. **Pass for the bounded manual recovery
and public release/channel verification.** The historical
[promotion run 34288691672](https://github.com/Reidond/kedra/actions/runs/34288691672)
still has a failed publisher; recovery does not turn that run green.

## Published identity

- [Version desktop-44-x86_64-r1](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-r1),
  release ID `385117864`, published **2026-09-08 23:40:35 UTC**.
- [Current desktop-44-x86_64-channel](https://github.com/Reidond/kedra/releases/tag/desktop-44-x86_64-channel),
  release ID `385128562`, published **2026-09-08 23:43:09 UTC**.
- Both Git tags resolve to accepted source
  `c660c58d9bbbbe34119f6ea35a03528485455848`. The image is
  `ghcr.io/reidond/kedra-desktop@sha256:bb4f2b68996a4fc260972d9654440f62ee078bcf92a0996a8f7e34ea0133118b`,
  candidate run `34255228394`, attempt 1.
- Dedicated public-key SPKI DER SHA-256:
  `a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e`.
- Complete `kedra-desktop-44-34255228394-1.iso`: **2,856,306,688 bytes**,
  SHA-256 `9c1401489d1c47119249ab213c9a187b47db6c5112ebccce567cf0cc4a76d988`.
  Its ten installation cases pass in the
  [exact-media report](../R02-installer/owner-34255228394/REPORT.md).

## Failure and recovery boundary

The owner approved the exact metadata directly in GitHub. Public preparation and
protected metadata signing succeeded; signed artifact `10080732048` was retained.
Release creation returned HTTP 500, but GitHub had created one empty draft with
the expected tag, accepted source, title and body. Authenticated release-list and
numeric-ID reads found it; `/releases/tags/desktop-44-x86_64-r1` returned 404 because
that endpoint serves published releases. The v1 publisher also used that endpoint
for later draft inventory, so an unchanged rerun was not a repair.

The existing release opt-in was temporarily disabled and both production workflows
were confirmed quiescent. Recovery reused that draft and the original signed
bytes, uploaded the original 13 assets without clobbering, and verified every
download before publication. No signature was regenerated and no draft/tag was
deleted. The version was published first; the single signed-pair channel bundle
was then uploaded to its numeric release ID and verified before/after publication.
The previous opt-in value `KEDRA_RELEASES_ENABLED=true` was restored and verified.
Main, release authority and environment protections stayed unchanged.

| Check | Result |
|---|---|
| Original signatures, scope, freshness and full ISO | pass — native channel/unpack/artifact verification |
| Original draft identity and empty inventory | pass — authenticated listing and ID readback |
| Every uploaded and downloaded asset | pass — all 13 exact size/SHA-256 comparisons |
| Downloaded parts plus signed metadata | pass — native Windows assembly reproduced the qualified whole ISO |
| Version publication and actual Git tag | pass — numeric-ID publication; tag resolves to c660c58 |
| Channel ID upload/download and anonymous public bytes | pass — 1,744-byte bundle matches the original approved bytes |
| Anonymous channel native verification | pass — fresh release sequence 1, checkpoint generation 1 |
| Public-channel first enrollment | pass — separate retained-r1-VM follow-up; see the [enrollment report](owner-r1-enrollment/REPORT.md) |
| Enrollment refusal/health/shutdown follow-up | pass — repeated enrollment preserved status; doctor, clean QEMU exit and final sentinel comparison pass |

The checkpoint was issued **2026-09-08 23:05:17 UTC**, expires
**2026-09-15 23:05:17 UTC**, and records successful resolution at
**2026-09-08 17:13:07 UTC**. The initial channel asset ID is `551542020`.
Its SHA-256 is `d46571c912d881f96b5c6da266ae923ad18aa0ccaeb932d70aa6e2a50edf797a`.
Future clients must fetch and verify the current channel, including their retained
replay state; this historical checkpoint is not a perpetual freshness claim.

## Exact r1 asset bytes

These are asset-byte checksums, not a new signed checksum format. The public-key
file checksum differs from the SPKI fingerprint above.

| Asset | Bytes | SHA-256 |
|---|---:|---|
| candidate.json | 1008 | `322a525bfbfbdbd7aaa7c07fcd7346582e83294315e569866d8bf52243317320` |
| channel.json | 1744 | `d46571c912d881f96b5c6da266ae923ad18aa0ccaeb932d70aa6e2a50edf797a` |
| checkpoint.json | 403 | `e97d4c85569f7555e1579c982c104de9022e17846c6878ddde80df7d74893365` |
| checkpoint.sig | 97 | `11d2f0de7be956705f8eaa30287a6946130c17cd3382743a4c19fc177791e0d7` |
| qualification.json | 704 | `9c7323ae707f341f1bc634f020db2b5ea77b2a7325c462b89a74d6b57e0515cb` |
| release-key.sha256 | 65 | `1b4b1f3c8530d4cbb8c219ad79bc5840aa46e88aba1d54aa08dc4c1360855e0d` |
| release.json | 854 | `03678d846d8ed88c0ea08dfab4ce302240a9a0ab54d58aba990bbf3cc7f017fb` |
| release.pub | 178 | `0eed364e4f141c11f28b1ee22f8b307535eaa16705972b860d3a795ec6ae8ffb` |
| release.sig | 97 | `786705bf2c0a37e5e8e91cb229b2f6e83c13170fa9bdb0fa363600c402375757` |
| signing-request.json | 911 | `6147ae61f1aa4b0d13c1058ae79f20b86d8a744b8c5a9c6a43ed5f8761379341` |
| source.json | 4849 | `4451de9c4363a4d3a7a3f789816b6ec9d169b14283aeb3a5a322c045fe745208` |
| kedra-desktop-44-34255228394-1.iso.part00 | 1992294400 | `983caa52b1d46a744faae1f7bac3feac65157eeda9ff06bf90393f8dad72922d` |
| kedra-desktop-44-34255228394-1.iso.part01 | 864012288 | `ea085f0608fdb1a18394821aa8ae8e8ad7681234d06e26782ef529d0ac76078e` |

Local continuation evidence is under `output/owner-recovery-34288691672/`:
the authenticated draft record, complete `readback/` files,
`assembled-readback/` ISO and native `verified-public-channel/` output. Worklog
entries WL-20260909-06/07 record the actual approval and recovery operations.
Published assets carry the original source, qualification and signed records;
private VM paths, passwords, keys and raw private logs are excluded from this report.

## Remaining work

R1 uses candidate schema 1 and authenticates its complete ISO through signed
release metadata. It does not publish separate SHA256SUMS/SHA256SUMS.sig,
packages.txt or provenance.json. Those v2 additions and the repaired publisher
(numeric IDs, bounded readback, exact Git tag checks) remain prepared and
source-reviewed, with full native execution not-run. This recovery is one observed
interruption case, not general race/crash qualification. The later ordinary-user
enrollment returned `enrolled: true`, exact bb4f2b68 booted identity, generation
and highest sequence 1, with no staged deployment, rollback hold or operation.
Repeated enrollment was refused with identical status, doctor passed, and clean
guest shutdown plus the stopped-sentinel comparison passed; exact native records
are in the separate enrollment report. Post-enrollment reboot persistence and a
new owner image were not exercised. Whole-target
equivalence, no-change renewal, expired recovery, key rotation and physical
hardware remain separate gates. See [installation](../../INSTALL.md) and
[release verification](../../RELEASES.md) for usable r1 commands.

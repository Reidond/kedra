# Executable verification

Only E2E/manual workflows and sanctioned container harness; no source scanners,
unit/model/mock tests or new runner.

| Case | Requirement | Expected evidence |
|---|---|---|
| CR-T1 | CR01 | strict-policy exact signed source copy, every selected config/layer blob verified, complete Docker export admitted |
| CR-T2 | CR02/03 | public consumer accepts correct independent identity; wrong identity, modified config/payload/manifest/foundation, recomputed outer hashes and unsafe members refuse before build |
| CR-T3 | CR04/05 | copy export, remove producer store/source, load archived foundation, static build/run real executable and dependency with no pulls/network |
| CR-T4 | CR04 | existing foreign foundation/cache tag and forbidden overlay/ONBUILD/volumes refuse; verified cache reuse preserves provenance |
| CR-T5 | CR05 | composition-only systemd native case validates generated unit, starts it, verifies successful status/output/config/identity |

Lab adaptation is separate and networked. Container qualification does not qualify
boot, signed deployment or SELinux enforcement. Record failures and not-run gaps.

# Verification plan

Only public CLI/manual and existing container/native QEMU harness workflows.

| Case | Requirement | Expected workflow |
|---|---|---|
| DB-T1 | DB01 | Missing/wrong identity, wrong target, overlay/binary override refuse before VM provisioning. |
| DB-T2 | DB02 | Changed native artifact or fixture regeneration refuses; correct conversion records distinct native/fixture/imported/disk IDs. |
| DB-T3 | DB03–04 | Current committed source graphical image boots exact generated kernel/initrd; host matches bootc reference/digest/native tuple to image.json; virtio DMA/GPU/input and crypto/storage support. |
| DB-T4 | DB04 | Actual SecureBoot/SetupMode/shim/lockdown, Enforcing/targeted policy/AVCs, unit health, Metal renderer and independently viewed screenshot. |
| DB-T5 | DB05 | Named VM warm restart passes identity/security/health; owned stop preserves unrelated default/media. Record forced shutdown separately. |
| DB-T6 | DB06 | Standard compiler/linter/E2E gates, exact-source review, publication/ancestry/CI readback; no inferred signature/install proof. |

Every case starts not-run. Runtime evidence belongs under ignored retained lab
artifacts, with safe outcome links in verification/status rather than raw logs.

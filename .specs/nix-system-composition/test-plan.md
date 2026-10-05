# Verification plan

Only public workflow E2E/manual evidence; no unit/model/mock/source-scanner tests.

| Case | Requirement | Workflow / expected result |
|---|---|---|
| SC-T1 | SC01/02/06 | Compile an external Rust authoring consumer; plan generated committed Kedra inputs; typed replacements and references resolve deterministically |
| SC-T2 | SC03/04/05 | Native retained Fedora observation, real engine output, compose twice into fresh contexts; hashes match, tar contents and process behavior are correct |
| SC-T3 | SC05/06 | Change native config, compose again; identity changes and previous context remains usable; existing output refuses without changing it |
| SC-T4 | SC01/04 | Ambiguous replacement, unsafe/reserved path, unknown/missing reference, mixed foundation and corrupt object refuse before successful publication |
| SC-T5 | SC02/03 | Unsupported target, missing requested RPM, removed RPM still present and wrong foundation release refuse |

Compiler/linter checks enforce code contracts; they do not establish OS behavior.
Installed unit/desktop behavior, boot, SELinux, update/install and signed releases
are not covered by context export and must remain explicit not-run gates unless
qualified through the existing container/VM harness during this slice.

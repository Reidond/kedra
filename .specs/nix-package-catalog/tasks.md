# D3 work and acceptance ownership

Status: implementation prepared and privately compiler-checked; owning-layer
adoption, application/runtime qualification and publication are pending.
The owner already authorized the five-PR delivery. Continue within that scope.

| Task | Mechanism and affected scope | Current state | Acceptance cases |
|---|---|---|---|
| T1 | Verify upstream archives and normalize/admit complete source trees; record exact package/source correspondence | pass for retained local inputs using the prior sysroot binary | TC-01 |
| T2 | Reusable catalog/recipe/policy library plus CLI resolution before store access; prevents named-package substitution and enables D4 | prepared; type/lint checks pass, CLI execution not-run | TC-02, TC-03 |
| T3 | jq and SQLite library→shell recipes use the existing executor; provide actual application behavior, dependency retention and byte comparison | prepared; application execution not-run | TC-04, TC-05, TC-06 |
| T4 | Fedora compiler recipe and D2 material/workflow hooks make builder inputs acquirable, checked and part of no-change material | prepared; resolver/build/material execution not-run | TC-07, TC-08 |
| T5 | Typed contribution author binds reviewed output/foundation and preflight; preserve existing FHS/trust while delivering selected packages | prepared; author/D2 transport execution not-run | TC-09 |
| T6 | Installed PATH/inventory and manual persistent SQLite service; sanctioned native case demonstrates user-visible delivery | prepared and compiler-checked; installed execution not-run | TC-10, TC-11 |
| T7 | Delivery lead adopts 17-path narrow patch above exact final D2, preserving later D2 changes; runs standard gates on actual owning source | private checks pass; adoption/owning-source checks pending | TC-12 |
| T8 | Serialize real source/engine/candidate tests, repair failures without weaker boundaries, record artifacts and publish one dependent draft | pending; no runtime/CI/publication claim | TC-01–TC-12 |

T2's external-consumer interface is enablement; D4 owns the independent compiled
consumer, isolation and lifecycle half. T4/T5 reuse D2's candidate runner and
release authority; D2 owns installer/update/rollback and protected production
observations. D5 owns authenticated substitution and crash/ENOSPC extensions.

Execution order: T7 adoption and actual tool build, T2 CLI refusals, T4 compiler
acquisition, T3 nonroot builds/rebuild/transfer, T5 author/compose, T6 installed
behavior, then final T7/T8 verification/publication. Heavy execution is exclusive;
pure source preparation may proceed in disjoint files. The retained private Cargo
directory was handed to later-layer workers after D3 source/evidence were frozen.
Do not treat those workers' later source state as the D3 validation snapshot.

Owner format follow-up: T5 also owns AC-12/AC-13 template embedding, closed token
binding, selected-version inventory and provenance updates; T4 owns the adjusted
preflight recipe inventory; T6 consumes declared versions in the installed case.
These additions are prepared with format/lint checks only and map to TC-13 below.
T7 adoption now includes 20 implementation paths and current private spec copies.

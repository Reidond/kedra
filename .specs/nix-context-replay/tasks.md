# Tasks

- [x] Acquire and verify complete signed foundation and local archive load.
- [x] Freeze/review verifier and harness API boundary.
- [x] Implement strict snapshot verifier and optional public CLI verification.
- [x] Implement sanctioned static replay source/load/build/cache path.
- [x] Compiled program/runtime dependency and generated-unit consumer workflows.
- [x] Actual tamper/identity/link/cache refusal and producer-removal evidence.
- [x] Standard workspace and legacy release gates; installed unit container case.
- [x] Update actual docs/status/worklog; submit/attach/read back next draft stack PR.

Published draft [PR31](https://github.com/Reidond/kedra/pull/31) above PR29 through
gh-stack30. Implementation source09473e1; new-source CI pending.

Cold independent daemon and boot/SELinux/install qualification are separate
gates unless actually executed; archive loading into a cached daemon is not cold.
Actual pre-build-journal interruption/retry passes; later publication windows
remain not-run. Current-repository20-file/1435-RPM static context builds successfully.

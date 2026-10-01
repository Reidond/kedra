# Verification

Only CLI/process/manual and sanctioned container-harness workflows.

| Case | Requirements | Actual outcome evidence |
|---|---|---|
| NA-T1 | NA02–04 | fixed native recipe accepts reviewed plan; unknown command/path/unit/identity/RPM changes refuse |
| NA-T2 | NA05 | changed GLib default visible to fresh user; explicit user override still wins |
| NA-T3 | NA05 | unit is-enabled/start/status/journal success; masked unit refuses startup; native default target correct |
| NA-T4 | NA05 | fresh account gets selected skeleton; preexisting modified home preserved; unsupported deletion refuses |
| NA-T5 | NA05 | image-local kernel initramfs generation, generic storage/crypto and required graphics/input modules verified |
| NA-T6 | NA06 | TLS cold daemon empty inventory/new ID, exported ELF/library/config replay with producer absent, safe owned cleanup |
| NA-T7 | NA07 | real observed post-temp/post-ID/binding/tag journal checkpoints recover exact image or safely rebuild; missed windows not passed |

VM boot/security/install/update are separate gates. All unobserved cases are
explicit not-run; metadata/source review is not executable qualification.

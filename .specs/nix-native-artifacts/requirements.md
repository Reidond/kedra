# Native artifacts and isolated consumer qualification

Owner continuation2026-10-01 above PR31/1c85ca4 in gh-stack30; annotation1 asks
for derived native artifacts and remaining cold-daemon/fault qualification.

- NA01: retain current source layout and package declarations. Fedora RPM lists
  remain foundation requirements; no RPM transaction after foundation selection.
  Engine-built Rust recipes produce ordinary store artifacts, not forced RPMs.
- NA02: a closed typed Rust native-derivation plan selects GLib schema compilation,
  concrete system-unit enable/disable/mask/default operations, initial account
  skeleton materialization and native QEMU ARM initramfs generation. No arbitrary
  command/script field or new configuration language.
- NA03: consume verified composition snapshots and separate expected derivation
  identity. Keep static schema1 byte-exact replay unchanged. Root native tools run
  only in the sanctioned isolated image-build container, with no host mounts,
  sockets/devices/network/privileged entitlements. Package executor stays nonroot.
- NA04: bind input recipe/tool/parent/foundation/RPM identities and actual generated
  artifact bytes/modes/links/kernel/module evidence plus final image/daemon. Reject
  unrelated output paths/aliases, changed RPM inventory and unknown receipts.
- NA05: actual generated defaults/unit enablement/fresh-account seeding/initramfs
  content must be tested through existing CLI/harness processes. Existing modified
  home remains untouched; unsupported managed baseline deletion refuses.
- NA06: existing cached native Docker DinD input supplies a fresh independent TLS
  daemon/storage with empty images and different ID. Restore only retained context,
  build/run offline and check producer absence. Owner-labelled bounded cleanup only.
- NA07: capture later real cache publication interruptions without product test
  hooks; qualify exact observed stages. Missed windows are not passes; reconstructed
  state is explicitly separate evidence. Preserve foreign resources/data.
- NA08: record actual coverage and simple package declaration/deployment guide;
  publish next dependent draft through gh-stack. No signed release or installation.

Container generation/content does not qualify kernel boot, Secure Boot/SELinux,
update/install, automatic home reconciliation or production signed integration.

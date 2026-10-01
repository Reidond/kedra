# Derived-image boot requirements

- DB01: public VM CLI accepts a complete independently selected composition/native
  tuple, only qemu-arm64/no overlay/no binary override. Bad tuples refuse safely.
- DB02: native preparation returns verified immutable image/material; QEMU fixture
  adaptation must retain receipt/boot artifacts or fail. Record fixture identity
  separately from native identity, imported manifest and disk checksum.
- DB03: compose current committed source over complete retained signed9d6 base;
  use graphical target and current service/skel/defaults, not the old synthetic
  multi-user native E2E as proof of the actual desktop.
- DB04: named disposable VM boots selected generated initramfs/kernel with required
  drivers; real firmware Secure Boot/lockdown, enforcing SELinux and healthy
  niri/Noctalia/Metal are observed. Actual screenshot is reviewed/shown.
- DB05: repeat warm boot retains provenance/security/health; owned cleanup preserves
  default VM/container/media. Expected unsigned fixture trust failure is explicit;
  production signer/install/update are not inferred from boot.
- DB06: public refusals/tamper paths and standard source gates pass, worklog/status
  actual outcomes retained, draft PR through gh-stack above exact PR32.

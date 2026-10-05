# Typed system composition

Status: implementation authorized by the owner's 2026-10-01 request to continue
the next layer through `gh stack`. Parent: engine PR #28, `7329ef1`.

Builds and declarative OS configuration are the requested scope. This slice
turns typed Rust contributions and Kedra's existing committed source plan into a
portable, deterministic context over an exact retained native ARM Fedora 44
foundation. Kedranix remains reference only.

- SC01: reusable Rust types describe files, explicit replacements, provenance and
  typed engine output references. No new configuration language or ambient eval.
- SC02: Kedra reads one committed revision through the existing target/source
  resolver. Home baselines remain writable-home inputs; private exclusions and
  target replacement rules remain in force.
- SC03: observe Fedora version and canonical seven-column RPM material from the
  exact retained foundation through the existing private executor. Require all
  requested package names and absence of removed names. Bind observations and
  verified foundation archive evidence into composition identity.
- SC04: include only verified engine runtime closures using the same foundation.
  Refuse missing, corrupt or mixed foundation objects. Typed references must be
  declared; protect reserved store/provenance paths and foundation ABI paths.
- SC05: export a new context with deterministic payload, composition manifest,
  retained foundation archive and a static Containerfile. It performs no package
  transactions. Existing output paths are refused; partial work is not published
  as a successful context. No installation, signing or production authority.
- SC06: config changes produce a new input-addressed identity and preserve previous artifacts.
  Verify public CLI workflows, a compiled external Rust authoring consumer,
  native executor/package observation, context repeatability and refusal paths.

Full RPM acquisition, source-reproducible Fedora, x86 execution, signed release
integration, boot/update/install and SELinux enforcement are later qualification
gates. Retained foundation material is an observation, not a hermetic source build.

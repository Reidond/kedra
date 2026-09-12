# Installer

User instructions: [install Kedra](../docs/INSTALL.md). Build/publish instructions: [release operations](../build/release/README.md).

The pinned image-builder produces a separate Anaconda environment with an embedded signed desktop OCI payload. The installer asks for disk selection, encryption and an administrative owner account; no disk or password is preset. Offline payload verification runs before installation and strict container signature policy is inherited by the installed OS.

`build-iso.sh` adds the required native SELinux-labeling stage to the pinned generic-ISO manifest. `anaconda-adapter.py` applies hash-guarded compatibility fixes for target-backed import scratch storage and persistent mount preparation. `finalize-fstab.py` retains the physical `/sysroot` mount and read-only policy. Unsupported upstream/source changes fail rather than silently bypassing these checks.

The media uses Fedora's permissive installer SELinux environment. The installed desktop remains enforcing. `smoke.py` boots the ISO without disks and requires offline verification and Anaconda startup; full encrypted install, data-disk preservation and ISO-free boot are separate manual VM checks. `prepare-test-trust.py` and `signed-payload.Containerfile` support disposable-authority E2E only.

# Local installer

Run `python3 installer/build-local.py --help`. [INSTALL.md](../docs/INSTALL.md) documents prerequisites and deliberate disk selection.

The local entrypoint takes an explicit reviewed signed GHCR digest and a new output directory. It checks the fixed key fingerprint, enforces native signature/repository policy, extracts the signed payload's helper/source/trust without executing its programs, and builds a separate Anaconda environment with the pinned image-builder. It writes one complete local ISO plus identity/hash records and never uploads.

`build-iso.sh` adds the required native SELinux-labeling stage. `anaconda-adapter.py` retains hash-guarded target scratch and persistent-mount preparation. `finalize-fstab.py` preserves physical /sysroot and read-only policy. Unsupported upstream changes fail.

The media uses Fedora's permissive installer SELinux environment; the installed desktop remains enforcing. Offline payload verification precedes Anaconda installation. Optional `--smoke` boots without disks; fresh encrypted installation and untouched-disk checks remain separate manual qualification.

Sources: [container storage configuration](https://github.com/containers/storage/blob/main/docs/containers-storage.conf.5.md), [Skopeo](https://github.com/containers/skopeo), and the pinned builder source identified by installer/inputs.json. Local end-to-end qualification must accompany changes to this entrypoint; syntax alone is insufficient.

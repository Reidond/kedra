# Build boundary

All OS and installer builds run in GitHub Actions. The current check workflow builds Rust bootstrap binaries only. R01/R02/R07/R08 must establish the image pipeline before a production Containerfile/release workflow is introduced.

Future assembly: common etc/usr/home inputs, then selected host inputs; report same-path replacement and record source provenance. Exclude `.gitkeep` and documentation from assembled rootfs. Never `COPY .` into an image. Home content becomes `/usr/share/sysroot/home/<user>/`, not a live-home overwrite. Build Rust binaries in a Fedora-compatible builder to avoid a newer host glibc ABI dependency.

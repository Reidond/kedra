# Interactive installer research

`installer/Containerfile` builds a separate Fedora Anaconda environment. The
desktop is the separately built OCI payload; installer tooling is not added to
the everyday desktop. The research workflow uses the pinned bootc-image-builder
`bootc-installer` type, not its unattended first-disk `anaconda-iso` type.

Current output is research-only. The initial desktop payload uses a localhost
research origin and is not a promoted signed release. Do not install it on a
physical machine. Its boot configuration must be inspected, then exercised in a
disposable multi-disk VM with deliberate disk choice, encryption and account
creation. Full signature/origin handoff must be joined to R01 before promotion.

All media builds run in Actions. Normal installation instructions and stable
download links will be supplied after those tests; no default account, password,
disk identifier or unattended erase configuration is shipped here.

See [R02 evidence](../docs/research/R02-installer/REPORT.md).

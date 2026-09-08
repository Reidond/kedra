# Interactive installer research

`installer/Containerfile` builds a separate Fedora Anaconda environment. The
desktop is the separately built OCI payload; installer tooling is not added to
the everyday desktop. The workflow now uses pinned image-builder v82.0.0's
`bootc-generic-iso` path with explicit graphical/rescue boot entries and native
Anaconda `bootc` interactive defaults. There are no preset partition commands.

The installation environment has Anaconda's normal privileged console/rescue
session. Its media-only account setup is separate from the installed payload,
whose root account remains locked and whose owner account is created interactively.

Current output is research-only. The initial desktop payload uses a localhost
research origin and is not a promoted signed release. Do not install it on a
physical machine. Its boot configuration must be inspected, then exercised in a
disposable multi-disk VM with deliberate disk choice, encryption and account
creation. Full signature/origin handoff must be joined to R01 before promotion.

All media builds run in Actions. Normal installation instructions and stable
download links will be supplied after those tests; no owner account/password,
disk identifier or unattended erase configuration is supplied for the installed OS.

See [R02 evidence](../docs/research/R02-installer/REPORT.md).

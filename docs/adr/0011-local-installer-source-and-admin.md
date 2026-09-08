# ADR 0011: local installer payload and accessible administrator

Date: 2026-09-08. Status: narrowly adapted source; build and VM validation pending.

The diagnostic Anaconda UI exposed two behaviors in Fedora's
`anaconda-core-44.30-2.fc44`: BootcSourceModule.network_required always returns
true, even for the embedded containers-storage source; and a seen rootpw command
counts as an administrator even when that command deliberately locks root.
The former blocks a networkless installation from the bundled image. The latter
allows an inaccessible desktop to be planned without a normal administrative user.

Apply two small adaptations only in Kedra's separate installer environment. Local
containers-storage sources do not require network availability; other/unknown
transports retain the network requirement. A configured locked root does not
satisfy the administrator requirement. An unlocked wheel user does. The rootpw
lock stays in interactive defaults, so the owner must create an administrative
account before beginning installation.

The adapter verifies exact original SHA-256 values and unique replacement anchors
before any writes. It retains upstream source and copyright headers, adds marked
Kedra comments, tests the native property bodies against local/remote/unknown and
locked/unlocked/admin cases, and compiles the resulting Python. A package/source
change fails the build for review instead of applying a fuzzy patch. The adapter
is not installed in the desktop and must be removed when upstream implements the
required behavior or a supported configuration mechanism replaces it.

No bootc command, signature policy, TLS setting, storage selection or partitioning
logic is changed. Inspection of the actual DeployBootcTask and bootc 1.16.10 shows
that future-origin fetch validation is opt-in via run_fetch_check. No skip-fetch
or no-signature flag is added. The embedded image still goes through bootc's
installation path; strict source policy and real signed origin require the R01/R08
handoff tests before release promotion.

Source identities are recorded in installer/anaconda-adapter.py. Sources inspected
from the exact ISO built by run 34180796587 and the same Fedora RPM bytes in the
prior media, 2026-09-08:
`pyanaconda/modules/payloads/source/bootc/bootc.py`,
`pyanaconda/modules/users/users.py`, and
`pyanaconda/modules/payloads/payload/rpm_ostree/installation.py`.
Upstream [Anaconda](https://github.com/rhinstaller/anaconda) is GPL-2.0-or-later;
retain these adaptation sources and the Fedora source-package provenance when
distributing the media. [bootc 1.16.10 install source](https://github.com/bootc-dev/bootc/blob/v1.16.10/crates/lib/src/install.rs)
documents the separate fetch and signature controls. R02 owns UI/installation
acceptance; the current prepared patch is not itself a passed installation test.

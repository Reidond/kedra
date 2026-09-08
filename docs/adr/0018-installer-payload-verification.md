# ADR 0018: Verify the embedded payload before interactive installation

Date: 2026-09-08. Status: prepared; native signed-media qualification pending.
Gates: R01, R02, R08, R10.

Fresh unsigned research media 34207121856 at 85ed4ab completes encrypted
installation and boots a healthy desktop. R01 34218886062 at 41e0d67 passes
strict native signed updates and rollback on the refreshed Fedora base. These
are distinct results; the installer must preserve and verify the image signature
and the installed target must inherit its strict policy before owner promotion.

bootc 1.16.10's installation source path treats the embedded image as already
pulled, while its install configuration sets the target's future signature
enforcement. Therefore install-policy alone is insufficient proof of payload
verification. The generic v82 osbuild manifest copies the embedded payload with
the native skopeo stage, whose default preserves signatures. This transport must
be checked on the generated ISO, not inferred from a successful image build.

The research workflow generates ephemeral keys outside build/artifact contexts.
A desktop derivative contains only public trust and uses the isolated
registry.kedra.test:5000/kedra/r02 identity. Its source file/package provenance is
retained, with an explicit generated-fixture scope and separate registry target.
It is signed in local container storage without publication to a production
registry. A strict local-copy preflight must succeed and a wrong-key copy must
fail. The exact digest becomes both the embedded source and installed target.

The installer contains a fixed root-owned payload descriptor and the same public
trust. Its systemd service submits a path-free VerifyInstaller request to the
installed Rust helper. The helper checks root authorization, public-key fingerprint,
scope, source-manifest binding and exact native policy, then runs fixed-argument
Skopeo verification with digest preservation. Reusing native storage avoids a
second full image copy into RAM scratch. This operation does not enroll a machine
or write an installation disk. Anaconda requires successful completion of this
service. The diskless smoke test requires both signature and Anaconda markers.

The media and desktop contain the tested bootc install-policy drop-in with
enforce-container-sigpolicy=true. The installed OS remains enforcing SELinux;
the separate Anaconda runtime retains its researched permissive SELinux mode.
No default disk, owner password, unattended formatting or reboot is introduced.

The private signing files are removed in an always-run cleanup step; public
evidence and research media cannot become owner releases. Production signing,
promotion, outer ISO signature verification and final release enrollment are
separate work. Native tests and manual installation are the evidence boundary;
unit and repository self-tests are excluded by the owner's AGENTS.md decision.

Sources: bootc v1.16.10 crates/lib/src/install.rs (prepare_install and embedded
source import), osbuild's org.osbuild.skopeo/container-deploy stages and
osbuild/util/containers.py; exact v82 manifest from run 34207121856.

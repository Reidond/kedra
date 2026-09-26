# Private official agent inputs

`fetch.py --target TARGET` downloads pinned public artifacts into a new explicit
directory. It does not install an agent or edit a user profile. The target comes
from the closed release table in `build/release/material.py` (`desktop` is
x86_64, `utm` is aarch64) and selects the matching official
`codex-package-<arch>-unknown-linux-musl` archive and Sigstore bundles from
`inputs.json`; an unknown target or triple is refused. `--verification-tools`
includes the pinned Cosign build tool for the runner's own architecture
(`cosign-linux-amd64` or `cosign-linux-arm64`, chosen by `platform.machine()`),
because the verifier executes on the runner rather than in the image.
`--research-tools` also includes the separate older Codex package used only by
the coexistence experiment. `pins.py` holds this selection for both scripts.
The top-level `cosign_test_tool` record remains the linux-amd64 build that the
x86_64 VM fixtures also read; `cosign_test_tool_aarch64` is the arm64 build.

`package.py --target TARGET` rechecks artifact identities (including the runner's
Cosign build before executing it), verifies the Codex/code-mode-host/
bubblewrap Sigstore bundles against the exact official release workflow identity,
and checks extracted bytes against every member of the pinned official archive.
It creates a new deterministic tar containing the unchanged six-file Codex package
under `/usr/libexec/sysroot/agents/codex/`, component notices, the pinned source
archive, and public package metadata. No build verifier, older personal test
runtime, authentication state, profile or Kedra skill collection enters the tar.

The x86_64 and aarch64 packages come from the same Codex 0.153.4 release and
source revision. The aarch64 archive and bundle digests are the official GitHub
release asset digests observed on 2026-09-25 and were rechecked by download;
the aarch64 package has the same six-file layout and manifest shape.

The known redistributed package components are Codex 0.153.4, its code-mode host,
bubblewrap, ripgrep 15.2.0/PCRE2 10.45 and the upstream patched zsh. Codex's root
LICENSE/NOTICE and corresponding source (including vendored bubblewrap/build files)
are retained, along with the zsh patch and pinned component license notices.
Ratatui 0.30.2's source commit was obtained from the crate matching Codex's locked
checksum; its MIT license is retained with Codex's existing attribution.

`prepare.sh TARGET CONTEXT INPUTS EVIDENCE` is the Actions-only composition of fetch,
verification and package preparation. The image consumes only the explicit tar.
Runtime/profile independence remains tested separately: archive verification is
not proof of authenticated model calls or native desktop compatibility.

Claude is not included while `public_preinstallation_approved` is false. Its
published preinstallation policy requires the owner's Commercial Terms agreement,
unmodified binaries, native authentication choices and direct end-user billing.
An unanswered question is not agreement. Personal runtime selection stays available.

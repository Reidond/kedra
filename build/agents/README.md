# Private official agent inputs

`fetch.py` downloads pinned public artifacts into a new explicit directory. It
does not install an agent or edit a user profile. `--verification-tools` includes
the pinned Cosign build tool; `--research-tools` also includes the separate older
Codex package used only by the coexistence experiment.

`package.py` rechecks artifact identities, verifies the Codex/code-mode-host/
bubblewrap Sigstore bundles against the exact official release workflow identity,
and checks extracted bytes against every member of the pinned official archive.
It creates a new deterministic tar containing the unchanged six-file Codex package
under `/usr/libexec/sysroot/agents/codex/`, component notices, the pinned source
archive, and public package metadata. No build verifier, older personal test
runtime, authentication state, profile or Kedra skill collection enters the tar.

The known redistributed package components are Codex 0.153.4, its code-mode host,
bubblewrap, ripgrep 15.2.0/PCRE2 10.45 and the upstream patched zsh. Codex's root
LICENSE/NOTICE and corresponding source (including vendored bubblewrap/build files)
are retained, along with the zsh patch and pinned component license notices.
Ratatui 0.30.2's source commit was obtained from the crate matching Codex's locked
checksum; its MIT license is retained with Codex's existing attribution.

`prepare.sh CONTEXT INPUTS EVIDENCE` is the Actions-only composition of fetch,
verification and package preparation. The image consumes only the explicit tar.
Runtime/profile independence remains tested separately: archive verification is
not proof of authenticated model calls or native desktop compatibility.

Claude is not included while `public_preinstallation_approved` is false. Its
published preinstallation policy requires the owner's Commercial Terms agreement,
unmodified binaries, native authentication choices and direct end-user billing.
An unanswered question is not agreement. Personal runtime selection stays available.

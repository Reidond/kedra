# Signed container images

The only published product artifacts are the signed OCI images of each enabled target: `ghcr.io/reidond/kedra-desktop` (x86_64) and `ghcr.io/reidond/kedra-qemu-arm64` (aarch64, the Apple Silicon QEMU virtual machine). Each repository has its own key, signing environment, `stable` tag and rank history. `stable` is discovery; verification and deployment use the immutable digest. A GitHub source commit, a tag name or a successful build alone is not image-signing authority.

The fixed public keys are `usr/src/kedra/image/release/authority/<target>.pub`. Independently confirm their SPKI DER SHA-256:

```text
desktop  a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
qemu-arm64  80551368732ccb49be7916db9ccfd1e3caa11a767e86749378565ec0b8a1a515
```

The installed root policy requires native Sigstore signatures from the installed target's key and its exact Kedra repository. A desktop system never accepts a `qemu-arm64` image and vice versa: target, architecture, OCI platform and repository are all bound by the signed image identity. Attachment discovery and initial-install policy must stay enabled. Never add permissive defaults or disable signature checks to make a pull work.

On the installed OS, `sysroot update check` reads the identity of the fixed GHCR repository's `stable` image through the installed helper, and `sysroot update stage` has bootc verify its signature with that policy while staging the exact digest, after the helper has refused another target or an image older than the booted one ([UPDATES](UPDATES.md#how-an-update-is-verified)). Image-owned identity/resolved-input records are bound by the signed OCI content. Local writable files and caller-supplied claims cannot establish root trust.

Builds, isolated signing and verified stable publication run automatically in Actions after validation, natively per target (`ubuntu-24.04` for desktop, `ubuntu-24.04-arm` for qemu-arm64). Each signing environment restricts deployment to main and has no human-review gate; no checkout, repository script or candidate code executes while production private keys are available. Build-time scope/rank/material checks and independent key/signature verification remain mandatory. No-change builds do not publish, sign metadata or renew checkpoints. See [release operations](../image/release/README.md) for the owner steps a new target needs before its first publication.

Package builds use a binary cache per target, `ghcr.io/reidond/kedra-cache-desktop` and `ghcr.io/reidond/kedra-cache-qemu-arm64` ([ENGINE](ENGINE.md#binary-cache)). Preflight copies `/usr/share/sysroot/composition.json` out of the strictly verified previous stable image; `sysroot catalog build` then substitutes each package closure that image records, pulled anonymously, and admits it only if every object equals its record. Anything else builds, and no cache failure fails a release. After the candidate's identity and material pass inspection, the build job pushes every realized package closure with its existing `packages: write` token; that step may fail without failing the release. The signer job, its keys and checkout rules are unchanged: the cache adds no signing authority and no secret.

The cache is bounded, unlike signed images (which are never garbage-collected): after `publish-stable` succeeds, the `prune-cache` job deletes every version of the target's cache package whose tag is not one this run pushed or found present, which is exactly the set of closures the newly published stable image records. Consumers admit nothing else, so a pruned closure could never be used again. The job prunes only when every contributed output has a recorded tag (a failed or partial push prunes nothing), uses the same job token and may fail without failing the release. A cache package therefore holds one stable image's package closures plus whatever later unpublished runs pushed, until the next publication. A machine on an older stable than the current one finds its closures pruned and builds them.

Owner action, once after the first run that pushes for each target: in the `kedra-cache-<target>` package settings set visibility to public (anonymous pulls by later runs and by machines need it; until then every pull misses and the run builds) and confirm under "Manage Actions access" that Reidond/kedra has the Admin role, which the GitHub API requires for `prune-cache` to delete versions (a package the workflow token created normally grants it). If the API still refuses, `prune-cache` reports it and old versions can be deleted on the package's versions page.

The ARM release source now routes its unsigned candidate through
`image/release/compose.py`: a fresh Fedora foundation defers native transforms,
the public composition APIs freeze committed configuration, and the existing
harness derives schemas, systemd links, initial defaults and QEMU initramfs.
Source and full RPM material must match before and after generation. Pinned
recipes, binaries and source inputs determine no-change equality; realized
native identities and artifact hashes are recorded separately. The existing
public-trust derivative and isolated signer consume the verified result.
Desktop retains complete assembly. This integration's current execution status
is in [D2 verification](../../../../.specs/nix-release-composition/verification.md);
source implementation does not establish protected-main publication success.

The disposable ARM installer/update workflow runs on a disposable aarch64 Linux
host (`--disposable-host --work DIR`, admitted by the shared VM launcher's guard)
or in an explicit `--local-fixture` mode restricted to its dedicated
ordinary-user controller container. It uses
generated fixture keys and actual committed source, then the normal encrypted
Anaconda installer and public updater. Its A/B fixture ranks share one native
derivation, so that workflow does not qualify a changed-kernel upgrade.
Production preparation, candidate inspection and publication retain their
current-main guards; read-only base resolution grants no signing authority.

For fixture-only corrections, local runs may select `--retained-candidate` with
its independent `--retained-candidate-sha256` and an explicit `--fixture-revision`.
The candidate's original `--source-revision` remains separate. Admission requires
an ancestor source, a closed set of fixture/document changes, unchanged production
recipes and executables, and fresh image/material readback before generating keys.
Product changes require a fresh candidate. Actions always build their selected
source; retained admission is local-only and creates no production authority.

Installer media is published only when the owner dispatches `.github/workflows/iso.yml` for one target: a full new installer release. On the target's native runner it builds an ISO from the current signed `stable` digest with `usr/src/kedra/installer/build-local.py`, which checks the fixed public key, strict signature and repository policy and the image's target scope. It splits the ISO into parts below 2 GiB and attaches them with their `PARTS.SHA256SUMS`, the builder's unchanged `SHA256SUMS` and `installer.json` (marked `"uploaded": true`) to a new GitHub Release tagged `<target>-<first 16 digest hex>`, at the image's source revision. An existing release for that image is never replaced. The checksums detect damaged downloads and are not signatures; the installer verifies its signed embedded OS payload offline. The workflow holds no signing secret and neither signs nor moves an image. [INSTALL.md](INSTALL.md) covers downloading and building media locally. No machine bundles or release metadata signatures are published.

The r1, r2 and legacy-channel GitHub Releases and all 31 uploaded assets have been removed; their source Git tags remain. The owner confirmed that nobody installed those releases, so no deployed-system migration is required. The protocol-1 release-metadata commands (`sysroot release verify|assemble|channel|history|unpack`), the four-file `sysroot update` enrollment and `--migrate-legacy` were removed on 2026-10-14, and `sysroot update enroll` on 2026-10-15; `sysroot release key --public-key <target>.pub` still prints a key's SPKI DER SHA-256 for the comparison above. [STATUS](STATUS.md) records verified signed GHCR publication per target and remaining installation qualification.

Local retained-registry fixtures additionally require `--supervised-admission`.
The existing owning supervisor receives one JSON `fixture-admission` event before
registry start and each public installer U/W/A call. It verifies the exact live
wrapper PID/start/boot, fixture-context path/hash, expected next phase and current
monitor health, then measures host free space against83,751,862,272 bytes before
returning that event's32-hex nonce on the wrapper's private stdin. It must keep the
existing overall operation deadline running during the at-most30-second handshake;
EOF, timeout or a mismatched nonce fails through the original cleanup trap.
There is no command hook or stored approval. The wrapper then rechecks Linux
capacity at that same cold threshold (zero cache credit) and TLS remaining life
of36000 seconds before the call.
Source/producer admission and fresh controller/storage custody are independent;
a nonce never waives them. Host mode does not use this local protocol.

For supervised local fixtures, failed U/W/A installer logs are captured with a
16MiB tail limit under the fresh runner's0700 `kedra-ghcr-diagnostics` directory,
outside both private-input cleanup and public evidence. Ordinary owner-private,
single-link source files and exclusive0600 outputs are required. Public evidence
contains only size/hash/completeness metadata and actual installer/capture exits.
These raw logs can contain generated credentials: keep this local custody private
and do not upload it. Capture failure never replaces A's original failure exit.

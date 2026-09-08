# Exact owner-media installation qualification

Recorded 2026-09-09 (Europe/Kiev) by Codex. **Pass for the ten v1 manual-VM
installation cases.** This does not promote the release, enroll a workstation,
or qualify physical hardware. The inspected development base is `8288cc2`;
the media and installed software remain the accepted `c660c58` source below.

![Installed niri/Noctalia desktop](desktop.png)

## Exact media and authority

| Identity | Observed value |
|---|---|
| Candidate run | [34255228394, attempt 1](https://github.com/Reidond/kedra/actions/runs/34255228394) |
| Accepted source | `c660c58d9bbbbe34119f6ea35a03528485455848` |
| Target | desktop / Fedora 44 / x86_64 / ghcr.io/reidond/kedra-desktop |
| Image digest | `sha256:bb4f2b68996a4fc260972d9654440f62ee078bcf92a0996a8f7e34ea0133118b` |
| Candidate JSON SHA-256 | `322a525bfbfbdbd7aaa7c07fcd7346582e83294315e569866d8bf52243317320` |
| Installed source.json SHA-256 | `4451de9c4363a4d3a7a3f789816b6ec9d169b14283aeb3a5a322c045fe745208` |
| Public-key SPKI SHA-256 | `a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e` |
| ISO filename | `kedra-desktop-44-34255228394-1.iso` |
| ISO bytes | `2856306688` |
| Whole ISO SHA-256 | `9c1401489d1c47119249ab213c9a187b47db6c5112ebccce567cf0cc4a76d988` |

Artifact `10079175405` is 2,801,602,518 bytes. Its downloaded ZIP independently
matches GitHub's SHA-256
`eb3c602d36b9872c9b0564a7a1a8b680389b6e210168ab38c092856307cfeadc`.
The ordered 1,992,294,400-byte and 864,012,288-byte parts reconstruct the exact ISO
above. The candidate/source/ISO were independently rehashed during review.
The immutable candidate still correctly says `fresh_install_qualified: false`;
the separate qualification proposal binds its exact hash to this later result.

## Procedure and cases

The local VM used QEMU 8.2.2 (`1:8.2.2+ds-0ubuntu1.18`), KVM/q35, four virtual
CPUs, 8 GiB RAM and OVMF `2024.02-2ubuntu0.9`. The firmware file SHA-256 was
`949bfa5389c4c48582737481e7d24f46b3a16b276ef44c4089a56858c6a0a446`.
The WSL2 kernel was `6.18.33.2-microsoft-standard-WSL2`. Secure Boot was not tested.
Only two newly generated 64 GiB QCOW2 disks and the verified read-only ISO were
attached. No network, host disks, personal home, vault or owner credentials were
attached. The second disk held generated sentinel data at its beginning and end.

The ordinary ISO boot menu was used without kernel overrides. The installer
showed both disk identities; only `vda` / `KEDRA-INSTALL-ONLY` was selected.
`vdb` / `KEDRA-KEEP-DATA` stayed unselected. Encryption was enabled, and the
generated `kedra-owner` account was configured with a password and wheel
membership. Passwords were generated privately and are absent from this evidence.
The actual guest's `kedra-installer-verify.service` reported `Result=success`
and `ExecMainStatus=0` before disk installation.

| Required v1 case | Actual observation | Result |
|---|---|---|
| encrypted_install | Anaconda completed; vda3 is `crypto_LUKS`, containing Btrfs root/home | pass |
| unselected_disk_preserved | Stopped-disk comparisons after installation and after the desktop session both reported identical virtual contents | pass |
| iso_free_boot | Installer VM stopped cleanly; the next launch removed its CD-ROM/ISO argument and booted only the retained generated disks | pass |
| owner_desktop_login | LUKS unlock and generated owner password reached niri/Noctalia; UID 1000 has wheel membership; ordinary sudo authentication succeeded | pass |
| selinux_enforcing | Installed doctor reports `Enforcing` | pass |
| exact_booted_image | Native bootc and installed helper both report the exact bb4f2b68 digest above | pass |
| container_policy | Both desired and booted native image references use registry transport and `containerPolicy`; installed helper accepts inherited trust/scope/policy | pass |
| home_writable | `/var/home` is read-write; ordinary owner exclusively created and removed a temporary empty file, producing `HOME_WRITE_PASS` | pass |
| root_read_only | Native findmnt reports `/sysroot` as Btrfs with `ro` | pass |
| doctor | Human and JSON interfaces pass every required session check, including native configuration, portal response, services and unlocked login keyring | pass |

Anaconda completion was followed by guest `poweroff`; QEMU returned exit 0.
The ISO-free session also shut down through Noctalia's Shut Down control, with
QEMU exit 0. Its immediate shutdown closed the QMP socket before an attempted
follow-up screenshot; this was not a guest failure. A final native comparison
again returned exit 0 and identical virtual disk contents. All local VMs were
stopped at completion. No installation/boot repair or signature-policy exception
was used. These launch/exit observations were recorded by the controlling session;
screenshots alone are not evidence that media was detached or disks preserved.

## Public evidence

- [Disk selection](disk-selection.png), [Anaconda completion](installation-complete.png),
  [encrypted layout](encrypted-layout.png), [doctor](desktop-doctor.png),
  [owner identity and mounts](installed-mounts.png),
  [home file write and sudo authentication](home-write-sudo.png).
- Actual guest stdout: [installed helper status](installed-status.json),
  [native bootc status](bootc-status.json), [doctor JSON](doctor.json),
  [native key/source identity](provenance.txt).
- [Final stopped-disk comparison](disk-comparison.json).

The JSON observations were captured from actual guest commands through the
generated VM's serial device, with sudo used only where required. Doctor ran as
the ordinary desktop owner. Its `release_setup` field establishes public-trust
presence only; exact image enforcement is established by the native bootc result
and successful installed-helper validation. Native bootc's `status.readOnly`
describes management state, not filesystem mount flags; the root mount result
comes from findmnt. No secrets or real user data are included in the selected
public evidence.

## Remaining boundary

The system is correctly `enrolled: false`, with no staged image, rollback or
deployment journal before promotion. Production metadata signing/publication,
initial channel enrollment, renewal, rotation, interrupted publication, wider
home transitions and physical hardware remain separate gates. This v1 candidate
uses its unchanged accepted workflow; the later v2 release-inventory code and
optional caller-home status feature are not part of this ISO. Full R02/R08 and
the owner's overall implementation goal remain open.

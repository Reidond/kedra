# Exact owner-media installation qualification — candidate 34293133114

Recorded 2026-09-09 (Europe/Kiev) by Codex. This report records the manual
qualification subset for candidate `34293133114`, built from accepted main
`0eb1cf09c0eab5f4488a780552f51582d3e92bdf`. It does not by itself publish the
candidate, enroll a workstation, or qualify physical hardware.

## Exact media and authority

| Identity | Observed value |
|---|---|
| Candidate run | [34293133114, attempt 1](https://github.com/Reidond/kedra/actions/runs/34293133114) |
| Candidate JSON SHA-256 | `1763899dae4b0a2cbef0cb384347c31ad1f819642281ea20adec7d3567683521` |
| Accepted source | `0eb1cf09c0eab5f4488a780552f51582d3e92bdf` |
| Image digest | `sha256:71b928fd53a593ece7d08ad84cf68b6dfd366a6606fd1bd0f38b09a7a95c381a` |
| Source manifest SHA-256 | `c55f7142c78d6240ed08f2a3b852d720142e844df6186879054e627b699e113f` |
| Packages SHA-256 | `bf504505b65c6aa771f7ef6faf3c1c12fcfd3a13ad0c9912ac88ece9279db13b` |
| Public-key SPKI SHA-256 | `a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e` |
| ISO | `kedra-desktop-44-34293133114-1.iso`, 2,856,314,880 bytes |
| Whole ISO SHA-256 | `15ddfafb8567297220e02831b6639f0fceb7efce0cd47bcf799fae97c5a1eb4b` |

The archive, source, authority, package inventory, provenance, ordered parts
and complete ISO were independently checked before the VM run. The ISO parts
match the candidate's declared hashes. The candidate's immutable metadata still
has `fresh_install_qualified: false`; the promotion workflow consumes the
separate qualification record produced from this observed run.

## Manual VM procedure

The VM used QEMU 8.2.2, UEFI/OVMF, four virtual CPUs and 8 GiB RAM. It attached
only two newly generated 64 GiB QCOW2 disks and the verified read-only ISO. The
disks were identified by serial: `KEDRA-INSTALL-ONLY` and
`KEDRA-KEEP-DATA`. No host disk, host forwarding, network interface, owner
profile, vault or real credential was attached. Generated test passwords are
not retained in this report.

Only the install disk was selected in Anaconda. Encryption was enabled. The
installer's verification service reported success before installation. Anaconda
completed and the VM was powered off; QEMU returned exit 0. The untouched data
disk compared identical at that point. A second launch removed the ISO and booted
only the retained disks. The generated owner reached niri/Noctalia, and the VM
was powered off again with QEMU exit 0; the untouched disk again compared
identical.

## Required cases

| Case | Actual observation | Result |
|---|---|---|
| `encrypted_install` | Selected disk contains `crypto_LUKS` and Btrfs root/home | pass |
| `unselected_disk_preserved` | `qemu-img compare` reports identical sentinel disk after installer and desktop shutdown | pass |
| `iso_free_boot` | Second launch has no ISO/CD-ROM argument and reaches the installed desktop | pass |
| `owner_desktop_login` | Generated `kedra-owner` authenticates, belongs to wheel and authenticates sudo | pass |
| `selinux_enforcing` | Doctor reports `Enforcing` | pass |
| `exact_booted_image` | Native bootc and helper report the candidate `71b928fd` digest | pass |
| `container_policy` | Native image reference is registry-qualified with `containerPolicy`; helper trust accepts it | pass |
| `home_writable` | Ordinary owner creates and removes a temporary home file | pass |
| `root_read_only` | `findmnt` reports `/sysroot` read-only; `/var/home` is writable | pass |
| `doctor` | All required desktop, portal, audio, service, keyring and configuration checks pass | pass |

The native source manifest hash and release-key fingerprint observed in the guest
match the reviewed candidate inputs. No installer repair or signature-policy
exception was used.

## Boundary

This is an exact candidate-media/manual VM result. It is not a Secure Boot,
physical-device, owner-authentication, production-publication, channel-update,
renewal, rotation or workstation-enrollment result. Those gates remain separate.
The signed release 1 and its enrolled retained VM remain unchanged while this
candidate awaits protected metadata promotion.

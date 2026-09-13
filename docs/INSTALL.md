# Build and install local media

Kedra publishes signed container images to `ghcr.io/reidond/kedra-desktop`. Installation media is constructed locally from an explicitly reviewed digest. No GitHub Release or ISO download is required.

## Build one ISO

Use a Linux x86_64 build host with Python 3.11+, sudo, rootful Podman using its default `/var/lib/containers/storage`, Podman's Netavark network helper, Skopeo and OpenSSL already installed. On Ubuntu, include the `netavark` package explicitly when installing Podman with `--no-install-recommends`; omitting it can prevent cleanup of inspection containers. Allow sufficient temporary disk space for the desktop, Anaconda image and ISO. The script does not install prerequisites or change host trust policy.

Use a trusted Kedra checkout and independently confirm the public-key fingerprint:

```text
a175f7086eebc2d7835e941b51b49a0e47bbc7c01ad4e090952e8ac74fe8c02e
```

Review the signed image's exact digest through the completed Actions signing result or [image verification](RELEASES.md). Substitute that full digest:

```sh
python3 installer/build-local.py --image ghcr.io/reidond/kedra-desktop@sha256:REVIEWED_DIGEST --output-dir /absolute/path/to/new-installer
```

The output directory must not already exist. The script checks the fixed public fingerprint and strict native container signature policy before extracting the image's helper, source and trust. It uses the image's recorded Fedora base and the digest-pinned builder in installer/inputs.json. Legacy images without recorded resolved inputs additionally require an explicitly reviewed `--base-image quay.io/fedora/fedora-bootc@sha256:BASE_DIGEST`.

The result is one complete `.iso`, `installer.json` and `SHA256SUMS` in the selected directory. No existing output is overwritten and nothing is uploaded. Local checksum files describe the built ISO; they are not release signatures. The embedded signed OS payload is independently verified offline before installation.

Rootful Podman retains its normal image/build cache. Temporary build data is removed after success and retained with its reported location after failure. Do not share the container store with another image-build operation while creating media.

For an additional diskless startup check, install xorriso and QEMU with usable KVM, then add `--smoke`. This checks embedded signature verification and Anaconda startup without attaching disks. It does not perform an installation.

## Install deliberately

1. Attach the local ISO to a disposable UEFI VM, or write it to a deliberately selected USB using a trusted image-writing tool.
2. Select only the intended system disk in Anaconda. Verify its identity and capacity; leave data disks unselected.
3. Enable encryption, retain its recovery passphrase and create an administrative owner account. No disk or password is preset.
4. Review pending changes, then install the offline-verified payload.
5. Shut down, remove the ISO/USB, boot the installed disk and sign in.

The installed desktop uses enforcing SELinux; installer-media policy is separate. Check `sysroot doctor` and `sysroot update status` after login. Home remains writable and system root read-only.

After a fresh installation, use `sysroot update enroll`, then the [update workflow](UPDATES.md).

Physical hardware and Secure Boot require separate qualification. See [status](STATUS.md) for actual tested media and remaining limits.

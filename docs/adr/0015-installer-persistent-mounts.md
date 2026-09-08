# ADR 0015: preserve separate home mounts and address the bootc physical root

Date: 2026-09-08. Status: implemented for fresh-media validation; not yet qualified.

The local UEFI installation from run 34195114452 at `923a282` completed Anaconda,
encryption, bootc deployment, GRUB and owner creation. The unselected 64 GiB disk
remained identical. Booting the installed disk without the ISO unlocked LUKS and
authenticated the owner with enforcing SELinux, but the session was unhealthy:
the owner's home was missing and systemd-remount-fs failed.

The generated owner directory existed under the deployment's persistent
`/var/home`, behind the separately mounted Btrfs home subvolume. Anaconda 44.30-2's
`PrepareBootcMountTargetsTask.run` binds `/var` and boot but omits the other mounts
that its OSTree sibling binds. Account creation consequently wrote into the
directory hidden by the real home mount at first boot. The existing exact-source
SHA-256 guard for installation.py remains
`614ac3f3061d959144e0a2e80919012c7254d44b1fab04daea35b2bef52f3f86`.

After binding persistent `/var` and preparing its directories, the adapted bootc
task now uses the native `_setup_internal_bindmount` for other selected mount
points in parent-before-child order. Root, `/var` and API mount points are excluded.
This matches the sibling's established lifecycle and cleanup tracking; it does
not choose disks or replace user files. Three inert native-method cases check
separate `/home`, separate `/var/home`, nested boot mounts and absence of extra
mounts. Earlier scratch/network/admin cases remain intact.

The installed fstab also addressed its Btrfs physical-root device as `/`, but the
booted logical root is an overlay. systemd-remount-fs failed with `overlay: No
changes allowed in reconfigure`. bootc documents that a legacy physical-root
fstab entry may address `/sysroot`; rootflags are preferred for early mount options.
The observed boot command line already includes bootc-generated `root=UUID=...`
and `rootflags=subvol=/root`.

An installer-only `%post --nochroot --erroronfail` step now normalizes that single
entry to `/sysroot` and explicitly retains read-only policy. It preserves other
mounts, comments, identifiers and filesystem options. It operates only on the
fixed mounted `/mnt/sysroot` physical filesystem, using native `ostree admin
--sysroot=/mnt/sysroot --print-current-dir` to locate the deployment. The result
must stay in its default stateroot deployment directory and contain a Kedra/bootc
payload with a bounded, root-owned
regular fstab. Missing/duplicate/unsupported root entries fail installation;
seven pure text and five path cases cover transformation, idempotence and refusal. No installed
service masks the failure or weakens root protection.

Local diagnostic copying of this generated owner's hidden home into its selected
home subvolume, plus the fstab normalization, tests the cause. This is not fresh
installer acceptance. Rebuilt media must independently pass complete installation,
owner home/defaults, enforcing first boot, zero failed units and sentinel retention.

Sources: exact native Anaconda 44.30-2 source and local screenshots in the R02
report; [bootc physical-root guidance](https://bootc.dev/bootc/bootc-install.html#finding-and-configuring-the-physical-root-filesystem).

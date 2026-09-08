# ADR 0010: Fedora's installer environment and installed desktop policy

Date: 2026-09-08. Status: researched media correction; new runtime tests pending.

Keep Anaconda as a separate installation environment and use Fedora Lorax's
documented `SELINUX=permissive`, `SELINUXTYPE=targeted` configuration on that media.
The installed desktop retains `SELINUX=enforcing` and must demonstrate enforcement
after actual installation. No permissive kernel argument is supplied for transfer
into the installed system. Image/metadata signature verification stays strict.

The first generic ISO omitted SELinux labels and froze in systemd. Adding the
standard labeling stage fixed that early boot: run 34180796587 at da140ff reached
enforcing userspace. Local UEFI testing then exposed a distinct incompatibility:
Anaconda's direct agetty-to-bash console remains in getty_t and is denied even
systemctl/getenforce access. The installer services did not reach the UI. An early
boot probe was therefore insufficient to establish a usable installer.

Upstream Lorax's runtime-postinstall template explicitly installs its permissive
SELinux config and directly launches the Anaconda rescue shell without a normal
PAM login transition. Its installation environment has full local disk/account
authority by design. Adopt that existing environment contract rather than invent
a new SELinux policy for the installer. Keep filesystem labels and SELinux audit
support. Also create Lorax's media-only install-user account for the WebUI; it is
not the owner account and does not enter the separate desktop payload.

The revised smoke waits for Anaconda's service and log initialization, not merely
basic.target, and records the intended media mode. UEFI/UI, deliberate multi-disk
selection, encryption, owner setup and installed enforcing boot remain separate
acceptance cases. No prior failed case is reclassified as passed.

Sources retrieved 2026-09-08, pinned Lorax revision
`ba5acecfb45cf860c39fc0c64fdf4630c78accd2`:
[runtime setup](https://github.com/weldr/lorax/blob/ba5acecfb45cf860c39fc0c64fdf4630c78accd2/share/templates.d/99-generic/runtime-postinstall.tmpl),
[media policy](https://github.com/weldr/lorax/blob/ba5acecfb45cf860c39fc0c64fdf4630c78accd2/share/templates.d/99-generic/config_files/common/selinux.config).
The exact tested ISO's anaconda-shell@.service and SELinux contexts were also
inspected. R02 owns media/install evidence; R07 retains enforcing desktop tests;
R01/R08 retain the independent cryptographic verification gates.

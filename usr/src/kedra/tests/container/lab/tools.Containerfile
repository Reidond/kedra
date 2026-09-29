# Test-only lab layer over a Kedra OS image. Never sign, promote or install it.
# Adds a headless parent compositor (sway), capture and input tools
# and GUI probe bindings,
# and adapts boot-only units to a container. The OS content under test is
# otherwise unchanged: installing these tools may not add, upgrade or remove
# any package the base image already has.
ARG BASE
FROM ${BASE}
LABEL dev.kedra.lab.owner=kedra-container-tests
RUN rpm -qa --qf '%{NAME}-%{EPOCHNUM}:%{VERSION}-%{RELEASE}.%{ARCH}\n' | LC_ALL=C sort > /tmp/kedra-lab-base-rpms && \
    dnf -y --best --setopt=install_weak_deps=False --repo=fedora --repo=updates install \
        sway grim wlrctl wtype python3-gobject python3-qt5-base python3-pyqt6-base && \
    rpm -qa --qf '%{NAME}-%{EPOCHNUM}:%{VERSION}-%{RELEASE}.%{ARCH}\n' | LC_ALL=C sort > /tmp/kedra-lab-lab-rpms && \
    changed=$(LC_ALL=C comm -23 /tmp/kedra-lab-base-rpms /tmp/kedra-lab-lab-rpms) && \
    if test -n "$changed"; then echo "Lab tools changed packages of the image under test:" >&2; echo "$changed" >&2; exit 1; fi && \
    mkdir -p /usr/share/kedra-lab && \
    LC_ALL=C comm -13 /tmp/kedra-lab-base-rpms /tmp/kedra-lab-lab-rpms > /usr/share/kedra-lab/added-rpms.txt && \
    rm /tmp/kedra-lab-base-rpms /tmp/kedra-lab-lab-rpms && dnf clean all && rm -rf /var/lib/dnf /var/cache/libdnf5
# bootc images embed their ostree repository, so each /usr and /etc file has a
# second link to its repository object. A booted composefs deployment presents
# one link, and sysroot's trusted reads require exactly that. Give the paths
# it reads (units, drop-ins, /etc) single-link copies; content is unchanged.
# New timestamps make the layer diff record the copies.
RUN find /etc /usr/lib/systemd /usr/share/containers -xdev -type f -links +1 -print0 | \
    while IFS= read -r -d '' file; do cp --preserve=mode,ownership -- "$file" "$file.kedra-lab" && mv -f -- "$file.kedra-lab" "$file"; done && \
    test -z "$(find /etc /usr/lib/systemd /usr/share/containers -xdev -type f -links +1 -print -quit)"
# A container has no VT for greetd and no bootloader to update. Conditions skip
# those units without changing their enablement, which scenarios still check;
# the session starts through the greetd PAM service instead (see session.rs).
COPY container-skip.conf /usr/lib/systemd/system/greetd.service.d/90-kedra-lab.conf
COPY container-skip.conf /usr/lib/systemd/system/bootloader-update.service.d/90-kedra-lab.conf
COPY rtkit-container.conf /usr/lib/systemd/system/rtkit-daemon.service.d/90-kedra-lab.conf
COPY journald-container.conf /usr/lib/systemd/journald.conf.d/90-kedra-lab.conf
COPY kedra-lab-host /usr/libexec/kedra-lab/host
COPY kedra-lab-host.service /usr/lib/systemd/user/kedra-lab-host.service
COPY niri-nested.conf /usr/lib/systemd/user/niri.service.d/90-kedra-lab-nested.conf
COPY software-rendering.conf /usr/lib/environment.d/90-kedra-lab-software-rendering.conf
COPY test-profile.toml /etc/skel/.config/noctalia/zz-research.toml
COPY probes/ /usr/libexec/kedra-lab/probes/
RUN chmod 0755 /usr/libexec/kedra-lab/host /usr/libexec/kedra-lab/probes/* && \
    ldconfig -X && \
    systemd-hwdb update && \
    /usr/lib/systemd/systemd-update-done
STOPSIGNAL SIGRTMIN+3
CMD ["/sbin/init"]

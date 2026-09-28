# Disposable native VM fixture. Never publish this derivative.
ARG BASE
FROM ${BASE}
LABEL dev.kedra.lab.owner=kedra-container-tests dev.kedra.lab.kind=qemu
RUN rpm -qa --qf '%{NAME}-%{EPOCHNUM}:%{VERSION}-%{RELEASE}.%{ARCH}\n' | LC_ALL=C sort > /tmp/base-rpms && \
    dnf -y --best --setopt=install_weak_deps=False --repo=fedora --repo=updates install \
        openssh-server grim wtype glx-utils egl-utils pipewire-utils weston-demo \
        python3-qt5-base python3-pyqt6-base && \
    rpm -qa --qf '%{NAME}-%{EPOCHNUM}:%{VERSION}-%{RELEASE}.%{ARCH}\n' | LC_ALL=C sort > /tmp/lab-rpms && \
    test -z "$(LC_ALL=C comm -23 /tmp/base-rpms /tmp/lab-rpms)" && \
    mkdir -p /usr/share/kedra-lab && \
    LC_ALL=C comm -13 /tmp/base-rpms /tmp/lab-rpms > /usr/share/kedra-lab/added-rpms.txt && \
    rm /tmp/base-rpms /tmp/lab-rpms && dnf clean all && rm -rf /var/lib/dnf /var/cache/libdnf5
COPY seed.py /usr/libexec/kedra-lab/seed
COPY seed.service /usr/lib/systemd/system/kedra-lab-seed.service
COPY session-exec /usr/libexec/kedra-lab/session-exec
COPY session-start /usr/libexec/kedra-lab/session-start
COPY test-profile.toml /etc/skel/.config/noctalia/zz-research.toml
COPY probes/ /usr/libexec/kedra-lab/probes/
RUN chmod 0755 /usr/libexec/kedra-lab/seed /usr/libexec/kedra-lab/session-* /usr/libexec/kedra-lab/probes/* && \
    useradd --uid 2000 --create-home --user-group kedra-test && \
    printf '[terminal]\nvt = 1\n[default_session]\ncommand = "/usr/bin/tuigreet --cmd /usr/libexec/kedra-session"\nuser = "greetd"\n[initial_session]\ncommand = "/usr/libexec/kedra-lab/session-start"\nuser = "kedra-test"\n' > /etc/kedra-lab-greetd.toml && \
    printf 'PasswordAuthentication no\nKbdInteractiveAuthentication no\nPermitRootLogin no\nAllowUsers kedra-test\n' > /etc/ssh/sshd_config.d/00-kedra-lab.conf && \
    printf 'kedra-test ALL=(root) NOPASSWD: /usr/bin/systemctl poweroff --no-block\n' > /etc/sudoers.d/kedra-lab-poweroff && \
    chmod 0440 /etc/sudoers.d/kedra-lab-poweroff && visudo -cf /etc/sudoers.d/kedra-lab-poweroff && \
    mkdir -p /usr/lib/systemd/system/sshd.service.d /usr/lib/systemd/system/greetd.service.d && \
    printf '[Unit]\nRequires=kedra-lab-seed.service\nAfter=kedra-lab-seed.service\n[Service]\nExecStart=\nExecStart=/usr/bin/greetd --config /etc/kedra-lab-greetd.toml\n' > /usr/lib/systemd/system/greetd.service.d/90-kedra-lab.conf && \
    printf '[Unit]\nRequires=kedra-lab-seed.service\nAfter=kedra-lab-seed.service\n' > /usr/lib/systemd/system/sshd.service.d/90-kedra-lab.conf && \
    mkdir -p /usr/lib/systemd/user/niri.service.d && \
    printf '[Service]\nEnvironment=RUST_LOG=niri=debug,smithay=debug\n' > /usr/lib/systemd/user/niri.service.d/90-kedra-lab-diagnostics.conf && \
    systemctl enable sshd.service kedra-lab-seed.service && bootc container lint

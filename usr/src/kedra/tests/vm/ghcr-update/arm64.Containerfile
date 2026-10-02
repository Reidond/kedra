# Complete engine-derived ARM fixture. Never publish as a production candidate.
FROM localhost/kedra-ghcr-arm:composed
LABEL dev.kedra.fixture=release-composition dev.kedra.fixture.target=qemu-arm64
COPY --chmod=0644 release.pub release-policy.json /usr/lib/sysroot/trust/
COPY --chmod=0644 policy.json /etc/containers/policy.json
COPY --chmod=0644 registries.yaml /etc/containers/registries.d/kedra.yaml
COPY --chmod=0644 tls.crt /etc/containers/certs.d/ghcr.io/ca.crt
COPY --chmod=0644 source.json resolved-inputs.json /usr/share/sysroot/
COPY --chmod=0644 install.toml /usr/lib/bootc/install/10-kedra.toml
COPY --chmod=0644 check.py /usr/libexec/kedra-ghcr-check.py
COPY --chmod=0644 identity-recovery.py /usr/libexec/kedra-ghcr-identity-recovery.py
COPY --chmod=0644 native-observer.py /usr/libexec/kedra-ghcr-native.py
COPY --chmod=0644 arm64-check.service /usr/lib/systemd/system/kedra-ghcr-test.service
RUN printf 'Kedra generated GHCR test VM\n' > /usr/share/sysroot/disposable-ghcr-test && \
    printf 'kedra-test ALL=(root) NOPASSWD: /usr/libexec/sysroot/helper ""\n' > /etc/sudoers.d/kedra-ghcr-test && \
    chmod 0440 /etc/sudoers.d/kedra-ghcr-test && visudo --check --file=/etc/sudoers.d/kedra-ghcr-test && \
    systemctl enable kedra-ghcr-test.service && \
    systemctl mask bootc-fetch-apply-updates.timer bootc-fetch-apply-updates.service && \
    bootc container lint

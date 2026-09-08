# Research derivative only: public trust, no private signer or test account.
ARG PAYLOAD_IMAGE
FROM ${PAYLOAD_IMAGE}
COPY trust/release.pub trust/release-policy.json /usr/lib/sysroot/trust/
COPY trust/source.json /usr/share/sysroot/source.json
COPY trust/policy.json /etc/containers/policy.json
COPY trust/registries.yaml /etc/containers/registries.d/kedra.yaml
COPY trust/install.toml /usr/lib/bootc/install/10-kedra.toml
COPY trust/research-only /usr/share/sysroot/research-only
RUN bootc container lint

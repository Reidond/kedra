FROM localhost/kedra-ghcr-arm:base
ARG KEDRA_IDENTITY
ARG VARIANT
LABEL org.kedra.image.identity=${KEDRA_IDENTITY}
COPY ${VARIANT}/image-identity.json /usr/share/sysroot/image-identity.json
RUN printf '%s\n' "$VARIANT" > /usr/share/sysroot/test-variant

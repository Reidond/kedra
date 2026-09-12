FROM localhost/kedra-ghcr:base
ARG KEDRA_IDENTITY
ARG VARIANT
LABEL org.kedra.image.identity=${KEDRA_IDENTITY}
COPY ${VARIANT}/image-identity.json /usr/share/sysroot/image-identity.json
COPY resolved-inputs.json /usr/share/sysroot/resolved-inputs.json
RUN printf '%s\n' "$VARIANT" > /usr/share/sysroot/test-variant

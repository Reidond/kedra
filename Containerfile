# Build only in Actions using a generated, reviewed context; no whole-repo COPY.
# Desktop payload; public trust and signed update identity are added by release builds.
ARG BASE_IMAGE
FROM ${BASE_IMAGE}
COPY sysroot /usr/bin/sysroot
COPY sysroot-helper /usr/libexec/sysroot/helper
ADD payload.tar /
ADD agents.tar /
ADD bitwarden.tar /
COPY assemble.sh /tmp/kedra-assemble.sh
RUN /bin/bash /tmp/kedra-assemble.sh && rm /tmp/kedra-assemble.sh

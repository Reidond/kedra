# Build only in Actions using a generated, reviewed context; no whole-repo COPY.
# Candidate desktop. Promotion remains gated by the recorded R01-R11 evidence.
ARG BASE_IMAGE
FROM ${BASE_IMAGE}
COPY sysroot /usr/bin/sysroot
COPY sysroot-helper /usr/libexec/sysroot/helper
ADD payload.tar /
ADD agents.tar /
ADD bitwarden.tar /
COPY assemble.sh /tmp/kedra-assemble.sh
RUN /bin/bash /tmp/kedra-assemble.sh && rm /tmp/kedra-assemble.sh

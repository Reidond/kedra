# Test-only working-tree overlay over a lab image. Never sign, promote or install it.
# Replays the payload half of the release Containerfile (binaries, payload.tar
# and assemble.sh's home-baseline, schema and validation steps). Package
# installation cannot be replayed: overlay-apply refuses package-list changes,
# which need a full image build.
ARG BASE
FROM ${BASE}
LABEL dev.kedra.lab.owner=kedra-container-tests dev.kedra.lab.overlay=worktree
RUN mkdir -p /usr/share/kedra-lab && cp /usr/share/sysroot/source.json /usr/share/kedra-lab/base-source.json
COPY sysroot /usr/bin/sysroot
COPY sysroot-helper /usr/libexec/sysroot/helper
ADD payload.tar /
COPY overlay-apply.sh /tmp/kedra-lab-overlay-apply.sh
RUN /usr/bin/bash /tmp/kedra-lab-overlay-apply.sh && rm /tmp/kedra-lab-overlay-apply.sh

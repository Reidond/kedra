#!/usr/bin/env bash
# Fixed initialization inside the disposable privileged controller container.
# Checkout commands run only as kedra-fixture; no daemon or host tooling starts.
set -euo pipefail
test "$(id -u)" -eq 0
test "$(uname -m)" = aarch64
test -f /.dockerenv || test -f /run/.containerenv
test "$(< /proc/self/cgroup)" = '0::/'
test -S /var/run/docker.sock
test -d /work
test -d /repo
sha256sum --check /usr/share/kedra-release-fixture/firmware/sha256.txt
mkdir /sys/fs/cgroup/kedra-fixture-init
members=$(cat /sys/fs/cgroup/cgroup.procs)
for pid in $members; do
    echo "$pid" > /sys/fs/cgroup/kedra-fixture-init/cgroup.procs 2>/dev/null || true
done
echo $$ > /sys/fs/cgroup/kedra-fixture-init/cgroup.procs
for controller in $(< /sys/fs/cgroup/cgroup.controllers); do
    echo "+$controller" > /sys/fs/cgroup/cgroup.subtree_control
done
socket_gid=$(stat -c %g /var/run/docker.sock)
if ! getent group "$socket_gid" >/dev/null; then
    groupadd --gid "$socket_gid" kedra-engine
fi
usermod --append --groups "$socket_gid" kedra-fixture
# /work must be a new explicitly owned fixture volume, never a host/home bind.
mkdir -p /work/runner /work/evidence
chown kedra-fixture /work /work/runner /work/evidence
chmod 0700 /work /work/runner /work/evidence
exec sudo --user=kedra-fixture -- /usr/bin/env -i HOME=/home/kedra-fixture \
    PATH=/usr/local/bin:/usr/bin:/bin UV_PYTHON_INSTALL_DIR=/opt/kedra-python \
    UV_PYTHON_DOWNLOADS=never DOCKER_HOST=unix:///var/run/docker.sock "$@"

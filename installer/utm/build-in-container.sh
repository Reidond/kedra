#!/bin/bash
# Runs as root inside the disposable, privileged Fedora 44 arm64 container that
# `installer/utm/kedra-utm.py iso` starts on the Mac's Docker engine. It gives the
# unchanged installer/build-local.py what it expects from an aarch64 Linux build
# host: rootful Podman with default storage, Skopeo, OpenSSL and an ordinary user
# with non-interactive sudo. The trusted checkout is mounted read-only at /kedra,
# /var/lib/containers and /var/tmp/kedra-out are Linux volumes, and /export is the
# new macOS output directory.
set -euo pipefail

image=${1:?reviewed payload digest required}
base=${2-}
output=/var/tmp/kedra-out
destination=/export

test "$(uname -m)" = aarch64
test -f /kedra/installer/build-local.py && test -d "$output" && test -d "$destination"

# Nested crun containers need the cgroup v2 controllers delegated below this
# container's private cgroup root; without it nested runs intermittently fail with
# "controller `pids` is not available". Never act on a shared (host) namespace.
if test -f /sys/fs/cgroup/cgroup.controllers; then
    if test "$(< /proc/self/cgroup)" != '0::/'; then
        echo 'kedra-utm: expected a private cgroup v2 namespace (docker run --cgroupns private)' >&2
        exit 1
    fi
    mkdir /sys/fs/cgroup/kedra-init
    while read -r pid; do
        echo "$pid" > /sys/fs/cgroup/kedra-init/cgroup.procs 2> /dev/null || true
    done < /sys/fs/cgroup/cgroup.procs
    for controller in $(< /sys/fs/cgroup/cgroup.controllers); do
        echo "+$controller" > /sys/fs/cgroup/cgroup.subtree_control
    done
fi

# Payload, Fedora base, Anaconda environment, builder and ISO scratch share the
# Docker VM's disk; fail before downloading gigabytes.
available=$(df --output=avail -B1 /var/lib/containers | tail -n 1 | tr -d ' ')
if test "$available" -lt $((40 * 1024 * 1024 * 1024)); then
    echo "kedra-utm: the Docker VM has $((available / 1024 / 1024 / 1024)) GiB free; the build needs at least 40 GiB" >&2
    exit 1
fi

echo 'kedra-utm: installing build prerequisites from signed Fedora 44 repositories' >&2
dnf -y -q --setopt=install_weak_deps=False '--setopt=*.gpgcheck=True' --repo=fedora --repo=updates \
    install podman skopeo openssl sudo python3 netavark
useradd --create-home --user-group kedra-build
printf 'kedra-build ALL=(root) NOPASSWD: ALL\n' > /etc/sudoers.d/kedra-build
chmod 0440 /etc/sudoers.d/kedra-build
chown kedra-build:kedra-build "$output"

arguments=(--image "$image" --output-dir "$output/iso")
if test -n "$base"; then
    arguments+=(--base-image "$base")
fi
sudo --user=kedra-build -- /usr/bin/env -i HOME=/home/kedra-build PATH=/usr/sbin:/usr/bin:/sbin:/bin \
    LANG=C.UTF-8 /usr/bin/python3 /kedra/installer/build-local.py "${arguments[@]}" < /dev/null

# Hand the ISO and its records to macOS; kedra-utm.py verifies them again there.
for file in "$output/iso"/*; do
    if ! test -f "$file" || test -L "$file"; then
        echo "kedra-utm: unexpected build output entry: $file" >&2
        exit 1
    fi
    cp -- "$file" "$destination/"
done

#!/usr/bin/env bash
# Select Docker's containerd image store on a disposable GitHub-hosted runner, as
# every release job that uses Docker does, so CI matches local engines such as
# OrbStack: image IDs are registry manifest digests and `docker save` writes the
# registry index as the archive root. Unrelated daemon settings are preserved,
# and the switch is refused while any container exists, because the store change
# would hide it.
set -euo pipefail
containers=$(docker ps --all --quiet)
test -z "$containers"
test ! -L /etc/docker/daemon.json
config=$(mktemp -d "${RUNNER_TEMP:?}/kedra-docker-config.XXXXXXXX")
trap 'rm -rf -- "$config"' EXIT
if sudo test -e /etc/docker/daemon.json; then
    sudo test -f /etc/docker/daemon.json
    # The copies stay the invoking user's; only reading the daemon configuration needs root.
    # shellcheck disable=SC2024
    sudo cat /etc/docker/daemon.json > "$config/original.json"
else
    printf '{}\n' > "$config/original.json"
fi
jq --exit-status 'type == "object" and ((.features // {}) | type == "object")' "$config/original.json" >/dev/null
jq '.features = ((.features // {}) + {"containerd-snapshotter": true})' \
    "$config/original.json" > "$config/daemon.json"
# shellcheck disable=SC2024
sudo dockerd --validate --config-file "$config/daemon.json" > "$config/validate.log" 2>&1
sudo install -m 0600 -o root -g root "$config/daemon.json" /etc/docker/daemon.json
sudo systemctl restart docker
docker info --format '{{json .DriverStatus}}' | \
    jq --exit-status 'any(.[]; . == ["driver-type", "io.containerd.snapshotter.v1"])' >/dev/null

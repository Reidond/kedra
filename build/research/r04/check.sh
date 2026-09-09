#!/usr/bin/bash
set -Eeuo pipefail
marker() { printf '%s\n' "$1" | tee /dev/ttyS1; }
trap 'code=$?; marker "KEDRA_R04_FAIL line=$LINENO code=$code"; systemctl poweroff --no-block; exit "$code"' ERR
test "$(getenforce)" = Enforcing
test "$(bootc status --json | jq -er .spec.image.signature)" = containerPolicy
state=/var/lib/kedra-r04
mkdir -p "$state" /var/lib/sysroot
chmod 0700 /var/lib/sysroot "$state"
if ! grep -q '^10.0.2.2 registry.kedra.test$' /etc/hosts; then
    printf '\n10.0.2.2 registry.kedra.test\n' >> /etc/hosts
fi
phase=$(cat "$state/phase" 2>/dev/null || printf initial)
variant=$(cat /usr/share/kedra-research/variant)
uid=$(id -u kedra-test)
as_user() { runuser -u kedra-test -- env XDG_RUNTIME_DIR="/run/user/$uid" DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" "$@"; }
for attempt in $(seq 1 60); do
    if pgrep -u greetd -x tuigreet >/dev/null; then break; fi
    sleep 1
done
pgrep -u greetd -x tuigreet >/dev/null
marker KEDRA_R04_LOGIN_READY
for attempt in $(seq 1 180); do
    if as_user systemctl --user is-active niri.service >/dev/null 2>&1 && pgrep -u "$uid" -x noctalia >/dev/null; then break; fi
    sleep 1
done
as_user systemctl --user is-active niri.service
environment=$(as_user systemctl --user show-environment)
niri_socket=$(printf '%s\n' "$environment" | sed -n 's/^NIRI_SOCKET=//p')
wayland=$(printf '%s\n' "$environment" | sed -n 's/^WAYLAND_DISPLAY=//p')
test -n "$niri_socket"
for attempt in $(seq 1 60); do
    if as_user env WAYLAND_DISPLAY="$wayland" noctalia msg log-level-status >/dev/null 2>&1; then break; fi
    sleep 1
done
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg log-level-status
# Generated guest only: exercise the real CLI/sudo/helper transport without a
# password prompt in this unattended fixture. This is not interactive auth proof.
if sysroot update status --home > "$state/root-home-status.json" 2>/dev/null; then
    echo 'root caller-home assessment unexpectedly succeeded' >&2
    false
fi
test ! -s "$state/root-home-status.json"
if test "$phase" = initial; then
    if as_user sysroot update status --home > "$state/unauthorized-home-status.json" 2>/dev/null; then
        echo 'unattended helper unexpectedly ran without sudo authorization' >&2
        false
    fi
    test ! -s "$state/unauthorized-home-status.json"
fi
printf 'kedra-test ALL=(root) NOPASSWD: /usr/libexec/sysroot/helper ""\n' > /etc/sudoers.d/kedra-r04-helper
chmod 0440 /etc/sudoers.d/kedra-r04-helper
visudo --check --file=/etc/sudoers.d/kedra-r04-helper
helper() { /usr/libexec/sysroot/helper < "$state/helper-$1.json" | tee "$state/response.json"; }
case "$variant:$phase" in
    A:initial)
        mkdir -p /run/kedra-r04-cases
        mount -o ro /dev/disk/by-label/KEDRA_R04_CASES /run/kedra-r04-cases
        cp /run/kedra-r04-cases/helper-*.json "$state/"
        umount /run/kedra-r04-cases
        helper enroll
        as_user env NIRI_SOCKET="$niri_socket" python3 /usr/libexec/kedra-research-r04-home.py prepare
        helper stage-b
        test "$(jq -r .journal.operation.phase "$state/response.json")" = awaiting_reboot
        printf boot-b > "$state/phase"
        marker KEDRA_R04_STAGE_B_PASS
        ;;
    B:boot-b)
        helper status
        test "$(jq -r .journal.operation.phase "$state/response.json")" = booted
        as_user env NIRI_SOCKET="$niri_socket" python3 /usr/libexec/kedra-research-r04-home.py accept-b
        helper rollback-a
        test "$(jq -r .journal.rollback_hold "$state/response.json")" = true
        printf rollback-a > "$state/phase"
        marker KEDRA_R04_ACCEPT_B_ROLLBACK_STAGED_PASS
        ;;
    A:rollback-a)
        helper status
        test "$(jq -r .journal.rollback_hold "$state/response.json")" = true
        test "$(jq -r .journal.high_water.highest_release_sequence "$state/response.json")" = 2
        as_user env NIRI_SOCKET="$niri_socket" python3 /usr/libexec/kedra-research-r04-home.py accept-a
        helper status
        test "$(jq -r .journal.high_water.highest_release_sequence "$state/response.json")" = 2
        printf complete > "$state/phase"
        marker KEDRA_R04_ROLLBACK_A_HOME_PASS
        ;;
    *) marker "KEDRA_R04_UNEXPECTED_PHASE"; false ;;
esac
systemctl poweroff --no-block

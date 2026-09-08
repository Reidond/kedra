#!/usr/bin/bash
set -euo pipefail
marker() { printf '%s\n' "$1" | tee /dev/ttyS1; }
trap 'code=$?; marker "KEDRA_R07_FAIL line=$LINENO code=$code"; journalctl -b -u greetd --no-pager -n 50; systemctl poweroff --no-block; exit "$code"' ERR
test "$(getenforce)" = Enforcing
uid=$(id -u kedra-test)
for attempt in $(seq 1 60); do
    if pgrep -u greetd -x tuigreet >/dev/null; then break; fi
    sleep 1
done
pgrep -u greetd -x tuigreet >/dev/null
marker KEDRA_R07_LOGIN_READY
as_user() {
    runuser -u kedra-test -- env XDG_RUNTIME_DIR="/run/user/$uid" \
        DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" "$@"
}
for attempt in $(seq 1 180); do
    if as_user systemctl --user is-active niri.service >/dev/null 2>&1 \
        && pgrep -u "$uid" -x noctalia >/dev/null; then break; fi
    sleep 1
done
as_user systemctl --user is-active niri.service
pgrep -u "$uid" -x noctalia >/dev/null
environment=$(as_user systemctl --user show-environment)
niri_socket=$(printf '%s\n' "$environment" | sed -n 's/^NIRI_SOCKET=//p')
wayland=$(printf '%s\n' "$environment" | sed -n 's/^WAYLAND_DISPLAY=//p')
test -n "$niri_socket"
test -n "$wayland"
for attempt in $(seq 1 120); do
    if as_user env WAYLAND_DISPLAY="$wayland" noctalia msg log-level-status >/dev/null 2>&1; then break; fi
    sleep 1
done
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg log-level-status
app_version=$(noctalia --version | awk '{print $2}' | sed 's/^v//')
projection=$(as_user noctalia config export full | /usr/libexec/kedra-research-project-noctalia --project "$app_version")
printf 'KEDRA_R03_PROJECT_BEFORE %s\n' "$projection"
original_theme=$(printf '%s' "$projection" | jq -er .theme_mode)
review_home=$(getent passwd kedra-test | cut -d: -f6)
review_state="$review_home/kedra-noctalia-review"
as_user sysroot home --state "$review_state" init
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set light
for attempt in $(seq 1 20); do
    projection=$(as_user noctalia config export full | /usr/libexec/kedra-research-project-noctalia --project "$app_version")
    if test "$(printf '%s' "$projection" | jq -er .theme_mode)" = light; then break; fi
    sleep 1
done
test "$(printf '%s' "$projection" | jq -er .theme_mode)" = light
printf 'KEDRA_R03_PROJECT_AFTER %s\n' "$projection"
as_user sysroot home --state "$review_state" stage theme.mode
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set auto
for attempt in $(seq 1 20); do
    review=$(as_user sysroot home --state "$review_state" status)
    if test "$(printf '%s' "$review" | jq -er '.fields[0].live.value')" = auto; then break; fi
    sleep 1
done
printf '%s' "$review" | jq -e '.fields[0].live.value == "auto" and .fields[0].selected.value == "light"'
as_user sysroot home --state "$review_state" selection | jq -e '.selection[0].after.value == "light" and .activation_performed == false and .checkout_changed == false'
as_user sysroot home --state "$review_state" unstage theme.mode
as_user sysroot home --state "$review_state" keep-local theme.mode | jq -e '.fields[0].local_only and (.fields[0].visible_change | not)'
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set light
for attempt in $(seq 1 20); do
    review=$(as_user sysroot home --state "$review_state" status)
    if test "$(printf '%s' "$review" | jq -er '.fields[0].live.value')" = light; then break; fi
    sleep 1
done
printf '%s' "$review" | jq -e '.fields[0].live.value == "light" and .fields[0].visible_change and (.fields[0].local_only | not)'
as_user sysroot home --state "$review_state" app-own theme.mode | jq -e '.fields[0].app_owned and (.fields[0].visible_change | not)'
as_user sysroot home --state "$review_state" clear-local theme.mode
marker KEDRA_R03_DURABLE_REVIEW_PASS
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$original_theme"
marker KEDRA_R03_NATIVE_PROJECTION_PASS
as_user env NIRI_SOCKET="$niri_socket" niri msg --json outputs
as_user env NIRI_SOCKET="$niri_socket" niri msg --json outputs | \
    jq -e '.["Virtual-1"].logical | .width == 1280 and .height == 768 and .scale == 1' >/dev/null
as_user systemctl --user start xdg-desktop-portal.service
as_user systemctl --user is-active pipewire.service wireplumber.service xdg-desktop-portal.service
as_user wpctl status
as_user busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop org.freedesktop.DBus.Peer Ping
as_user busctl --user call org.freedesktop.secrets /org/freedesktop/secrets org.freedesktop.DBus.Peer Ping
# Prove normal password login unlocked this synthetic account's keyring.
as_user busctl --user get-property org.freedesktop.secrets /org/freedesktop/secrets/aliases/default org.freedesktop.Secret.Collection Locked
test "$(as_user busctl --user get-property org.freedesktop.secrets /org/freedesktop/secrets/aliases/default org.freedesktop.Secret.Collection Locked)" = 'b false'
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg panel-toggle launcher
marker KEDRA_R07_SESSION_READY
sleep 15
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg panel-toggle launcher
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg settings-toggle
marker KEDRA_R07_SETTINGS_READY
sleep 15
marker KEDRA_R07_SESSION_PASS
systemctl poweroff --no-block

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
as_user systemctl --user is-active kedra-noctalia.service
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
review_home=$(getent passwd kedra-test | cut -d: -f6)
review_state="$review_home/kedra-noctalia-review"
as_user sysroot home --state "$review_state" init
review=$(as_user sysroot home --state "$review_state" status)
original_theme=$(printf '%s' "$review" | jq -er '.fields[0].live.value')
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set light
for attempt in $(seq 1 20); do
    review=$(as_user sysroot home --state "$review_state" status)
    if test "$(printf '%s' "$review" | jq -er '.fields[0].live.value')" = light; then break; fi
    sleep 1
done
test "$(printf '%s' "$review" | jq -er '.fields[0].live.value')" = light
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
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set light
as_user systemctl --user stop kedra-noctalia.service
if pgrep -u "$uid" -x noctalia >/dev/null; then
    echo 'Noctalia writer remained after the managed service stopped' >&2
    false
fi
as_user sysroot home --state "$review_state" status | jq -e '.fields[0].live.value == "light"' >/dev/null
as_user systemctl --user start kedra-noctalia.service
for attempt in $(seq 1 30); do
    if as_user env WAYLAND_DISPLAY="$wayland" noctalia msg log-level-status >/dev/null 2>&1; then break; fi
    sleep 1
done
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$original_theme"
marker KEDRA_R04_WRITER_LIFECYCLE_PASS
as_user systemctl --user show kedra-noctalia.service --property=FragmentPath --property=DropInPaths
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set light
as_user sysroot home --state "$review_state" stage theme.mode >/dev/null
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set auto
activation_plan=$(as_user sysroot home --state "$review_state" plan --discard theme.mode)
activation_id=$(printf '%s' "$activation_plan" | jq -er .plan_id)
printf '%s' "$activation_plan" | jq -e '.plan.observed.theme_mode == "auto" and .plan.desired.theme_mode == "light"' >/dev/null
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set dark
if as_user sysroot home --state "$review_state" discard theme.mode --plan "$activation_id" >/dev/null 2>&1; then
    echo 'Stale home plan unexpectedly succeeded' >&2
    false
fi
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set auto
native_settings="$review_home/.local/state/noctalia/settings.toml"
native_metadata=$(stat -c '%u:%g:%a:%C' "$native_settings")
as_user sysroot home --state "$review_state" discard theme.mode --plan "$activation_id" | jq -e '.operation_completed and .pending == null and .fields[0].live.value == "light" and .fields[0].selected.value == "light"' >/dev/null
test "$(stat -c '%u:%g:%a:%C' "$native_settings")" = "$native_metadata"
as_user systemctl --user is-active kedra-noctalia.service
as_user sysroot home --state "$review_state" recover | jq -e '.pending == null and .journal.phase == "completed" and (.native_file_contents_stored | not)' >/dev/null
test -z "$(find "$review_home/.local/state/noctalia" -maxdepth 1 -name '.sysroot-activation-*' -print -quit)"
as_user sysroot home --state "$review_state" unstage theme.mode >/dev/null
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$original_theme"
marker KEDRA_R04_NATIVE_DISCARD_PASS
as_user python3 /usr/libexec/kedra-research-agents.py
fixture_home=$(getent passwd kedra-test | cut -d: -f6)
test "$(printf '%s\n' "$environment" | sed -n 's/^SSH_AUTH_SOCK=//p')" = "$fixture_home/.bitwarden-ssh-agent.sock"
test "$(as_user bash --login -c 'printf %s "$SSH_AUTH_SOCK"')" = "$fixture_home/.bitwarden-ssh-agent.sock"
as_user systemd-run --user --unit=kedra-bitwarden-research --collect --service-type=exec /usr/bin/bitwarden
for attempt in $(seq 1 60); do
    if as_user env NIRI_SOCKET="$niri_socket" niri msg --json windows | jq -e 'any(.[]; (.app_id // "" | ascii_downcase) == "bitwarden")' >/dev/null; then break; fi
    sleep 1
done
as_user env NIRI_SOCKET="$niri_socket" niri msg --json windows | jq -e 'any(.[]; (.app_id // "" | ascii_downcase) == "bitwarden")' >/dev/null
as_user systemctl --user is-active kedra-bitwarden-research.service
marker KEDRA_R06_LOGGED_OUT_READY
sleep 15
as_user systemctl --user stop kedra-bitwarden-research.service
marker KEDRA_R06_LOGGED_OUT_PASS
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
as_user sysroot doctor --json | jq -e '.desktop_session_checks_passed and (.changes_performed | not)' >/dev/null
marker KEDRA_DOCTOR_SESSION_PASS
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg panel-toggle launcher
marker KEDRA_R07_SESSION_READY
sleep 15
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg panel-toggle launcher
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg settings-toggle
marker KEDRA_R07_SETTINGS_READY
sleep 15
marker KEDRA_R07_SESSION_PASS
systemctl poweroff --no-block

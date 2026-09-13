#!/usr/bin/bash
set -Eeuo pipefail
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
# Read the resolved native source for the generated VM's actual output as well
# as the default; this observes defaults without rewriting GUI overrides.
wallpaper_default=$(as_user env WAYLAND_DISPLAY="$wayland" timeout 10s noctalia msg wallpaper-get)
wallpaper_output=$(as_user env WAYLAND_DISPLAY="$wayland" timeout 10s noctalia msg wallpaper-get Virtual-1)
printf 'Native wallpaper default=%s Virtual-1=%s\n' "$wallpaper_default" "$wallpaper_output"
test "$wallpaper_default" = 'color:#222226'
test "$wallpaper_output" = 'color:#222226'
marker KEDRA_ADWAITA_WALLPAPER_SOURCE_PASS
# Synthetic VM window inventory helps correlate startup screenshots with apps.
as_user env NIRI_SOCKET="$niri_socket" timeout 10s niri msg --json windows
review_home=$(getent passwd kedra-test | cut -d: -f6)
review_state="$review_home/kedra-noctalia-review"
as_user sysroot home --state "$review_state" init
review=$(as_user sysroot home --state "$review_state" status)
original_theme=$(printf '%s' "$review" | jq -er '.fields[0].live.value')
# Keep the selected value and the later edit distinct from the adopted baseline.
# All three native modes participate regardless of the desktop's default.
case "$original_theme" in
    light) selected_theme=dark; later_theme=auto ;;
    dark) selected_theme=light; later_theme=auto ;;
    auto) selected_theme=light; later_theme=dark ;;
    *) echo "Unexpected native theme mode: $original_theme" >&2; false ;;
esac
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$selected_theme"
for attempt in $(seq 1 20); do
    review=$(as_user sysroot home --state "$review_state" status)
    if test "$(printf '%s' "$review" | jq -er '.fields[0].live.value')" = "$selected_theme"; then break; fi
    sleep 1
done
test "$(printf '%s' "$review" | jq -er '.fields[0].live.value')" = "$selected_theme"
as_user sysroot home --state "$review_state" stage theme.mode
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$later_theme"
for attempt in $(seq 1 20); do
    review=$(as_user sysroot home --state "$review_state" status)
    if test "$(printf '%s' "$review" | jq -er '.fields[0].live.value')" = "$later_theme"; then break; fi
    sleep 1
done
printf '%s' "$review" | jq -e --arg later "$later_theme" --arg selected "$selected_theme" '.fields[0].live.value == $later and .fields[0].selected.value == $selected'
as_user sysroot home --state "$review_state" selection | jq -e --arg selected "$selected_theme" '.selection[0].after.value == $selected and .activation_performed == false and .checkout_changed == false'
as_user sysroot home --state "$review_state" unstage theme.mode
as_user sysroot home --state "$review_state" keep-local theme.mode | jq -e '.fields[0].local_only and (.fields[0].visible_change | not)'
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$selected_theme"
for attempt in $(seq 1 20); do
    review=$(as_user sysroot home --state "$review_state" status)
    if test "$(printf '%s' "$review" | jq -er '.fields[0].live.value')" = "$selected_theme"; then break; fi
    sleep 1
done
printf '%s' "$review" | jq -e --arg selected "$selected_theme" '.fields[0].live.value == $selected and .fields[0].visible_change and (.fields[0].local_only | not)'
as_user sysroot home --state "$review_state" app-own theme.mode | jq -e '.fields[0].app_owned and (.fields[0].visible_change | not)'
as_user sysroot home --state "$review_state" clear-local theme.mode
marker KEDRA_R03_DURABLE_REVIEW_PASS
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$original_theme"
marker KEDRA_R03_NATIVE_PROJECTION_PASS
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$selected_theme"
as_user systemctl --user stop kedra-noctalia.service
if pgrep -u "$uid" -x noctalia >/dev/null; then
    echo 'Noctalia writer remained after the managed service stopped' >&2
    false
fi
as_user sysroot home --state "$review_state" status | jq -e --arg selected "$selected_theme" '.fields[0].live.value == $selected' >/dev/null
as_user systemctl --user start kedra-noctalia.service
for attempt in $(seq 1 30); do
    if as_user env WAYLAND_DISPLAY="$wayland" noctalia msg log-level-status >/dev/null 2>&1; then break; fi
    sleep 1
done
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$original_theme"
marker KEDRA_R04_WRITER_LIFECYCLE_PASS
as_user systemctl --user show kedra-noctalia.service --property=FragmentPath --property=DropInPaths
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$selected_theme"
as_user sysroot home --state "$review_state" stage theme.mode >/dev/null
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$later_theme"
activation_plan=$(as_user sysroot home --state "$review_state" plan --discard theme.mode)
activation_id=$(printf '%s' "$activation_plan" | jq -er .plan_id)
printf '%s' "$activation_plan" | jq -e --arg later "$later_theme" --arg selected "$selected_theme" '.plan.observed.theme_mode == $later and .plan.desired.theme_mode == $selected' >/dev/null
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$original_theme"
if as_user sysroot home --state "$review_state" discard theme.mode --plan "$activation_id" >/dev/null 2>&1; then
    echo 'Stale home plan unexpectedly succeeded' >&2
    false
fi
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$later_theme"
native_settings="$review_home/.local/state/noctalia/settings.toml"
native_metadata=$(stat -c '%u:%g:%a:%C' "$native_settings")
as_user sysroot home --state "$review_state" discard theme.mode --plan "$activation_id" | jq -e --arg selected "$selected_theme" '.operation_completed and .pending == null and .fields[0].live.value == $selected and .fields[0].selected.value == $selected' >/dev/null
test "$(stat -c '%u:%g:%a:%C' "$native_settings")" = "$native_metadata"
as_user systemctl --user is-active kedra-noctalia.service
as_user sysroot home --state "$review_state" recover | jq -e '.pending == null and .journal.phase == "completed" and (.native_file_contents_stored | not)' >/dev/null
test -z "$(find "$review_home/.local/state/noctalia" -maxdepth 1 -name '.sysroot-activation-*' -print -quit)"
as_user sysroot home --state "$review_state" unstage theme.mode >/dev/null
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$original_theme"
marker KEDRA_R04_NATIVE_DISCARD_PASS
as_user env WAYLAND_DISPLAY="$wayland" python3 /usr/libexec/kedra-research-recovery.py "$review_state" "$selected_theme" "$later_theme" "$original_theme"
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg theme-mode-set "$original_theme"
marker KEDRA_R04_NATIVE_RECOVERY_PASS
as_user env NIRI_SOCKET="$niri_socket" WAYLAND_DISPLAY="$wayland" python3 /usr/libexec/kedra-research-niri-review.py "$review_state"
as_user sysroot home status | jq -e '.pending_activation == null and (.activation_performed | not)' >/dev/null
test "$(stat -c '%a' "$review_home/.local/state/sysroot")" = 700
test "$(stat -c '%a' "$review_home/.local/state/sysroot/home")" = 700
marker KEDRA_HOME_DEFAULT_STATE_PASS
marker KEDRA_R04_NATIVE_NIRI_DISCARD_PASS
marker KEDRA_R04_NATIVE_NIRI_RECOVERY_PASS
marker KEDRA_R04_NIRI_INSTALLED_BASELINE_PASS
marker KEDRA_R03_NATIVE_NIRI_LINES_PASS
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
marker KEDRA_R07_PORTAL_PING_START
as_user timeout --kill-after=2s 20s busctl --user call org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop org.freedesktop.DBus.Peer Ping
marker KEDRA_R07_PORTAL_PING_PASS
as_user timeout --kill-after=2s 20s busctl --user call org.freedesktop.secrets /org/freedesktop/secrets org.freedesktop.DBus.Peer Ping
marker KEDRA_R07_SECRET_SERVICE_PING_PASS
# Prove normal password login unlocked this synthetic account's keyring.
as_user timeout --kill-after=2s 20s busctl --user get-property org.freedesktop.secrets /org/freedesktop/secrets/aliases/default org.freedesktop.Secret.Collection Locked
test "$(as_user timeout --kill-after=2s 20s busctl --user get-property org.freedesktop.secrets /org/freedesktop/secrets/aliases/default org.freedesktop.Secret.Collection Locked)" = 'b false'
as_user sysroot doctor --json | jq -e '.desktop_session_checks_passed and (.changes_performed | not)' >/dev/null
marker KEDRA_DOCTOR_SESSION_PASS
# Real graphical applications, keyboard input and native file selection.
# GUI drivers below use only the synthetic account and generated file.
toolkit_result="$review_home/toolkit-result.json"
for toolkit_case in gtk3-wayland gtk3-xwayland libadwaita qt5 qt6 qt6-override; do
    as_user rm -f "$toolkit_result"
    toolkit_unit="kedra-toolkit-$toolkit_case"
    backend_env=()
    case "$toolkit_case" in
        gtk3-wayland|libadwaita) backend_env=(GDK_BACKEND=wayland) ;;
        gtk3-xwayland) backend_env=(GDK_BACKEND=x11) ;;
        qt*) backend_env=(QT_QPA_PLATFORM=wayland) ;;
    esac
    if test "$toolkit_case" = qt6-override; then
        override_config=$(as_user mktemp -d "$review_home/kedra-qt-preference.XXXXXX")
        printf '[General]\nfont=Adwaita Mono,12,-1,5,50,0,0,0,0,0\n' | as_user tee "$override_config/kdeglobals" >/dev/null
        backend_env+=("XDG_CONFIG_HOME=$override_config")
    fi
    as_user systemd-run --user --unit="$toolkit_unit" --collect --service-type=exec \
        /usr/bin/env "${backend_env[@]}" /usr/bin/python3 /usr/libexec/kedra-research-toolkit-app.py "$toolkit_case" "$toolkit_result"
    for toolkit_stage in ready dialog selected; do
        for attempt in $(seq 1 45); do
            toolkit_service_state=$(as_user systemctl --user show "$toolkit_unit.service" --property=ActiveState --value)
            case "$toolkit_service_state" in
                active|activating) ;;
                *)
                    journalctl -b "_SYSTEMD_USER_UNIT=$toolkit_unit.service" --no-pager -n 40
                    echo "Toolkit $toolkit_case exited before $toolkit_stage (state=$toolkit_service_state)" >&2
                    false
                    ;;
            esac
            if test -f "$toolkit_result" && jq -e --arg stage "$toolkit_stage" '.stage == $stage' "$toolkit_result" >/dev/null; then break; fi
            if test -f "$toolkit_result" && jq -e '.stage == "failed"' "$toolkit_result" >/dev/null; then cat "$toolkit_result"; false; fi
            sleep 1
        done
        jq -e --arg stage "$toolkit_stage" '.stage == $stage' "$toolkit_result"
        marker "KEDRA_TOOLKIT_${toolkit_case}_${toolkit_stage}"
    done
    # Preserve runtime facts in serial evidence and allow the host to capture
    # the selected-file result before closing the real application.
    sleep 4
    as_user systemctl --user stop "$toolkit_unit.service"
    if test "$toolkit_case" = qt6-override; then
        as_user rm "$override_config/kdeglobals"
        as_user rmdir "$override_config"
    fi
done
marker KEDRA_TOOLKITS_PASS
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg panel-toggle launcher
marker KEDRA_R07_SESSION_READY
sleep 15
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg panel-toggle launcher
as_user env WAYLAND_DISPLAY="$wayland" noctalia msg settings-toggle
marker KEDRA_R07_SETTINGS_READY
sleep 15
marker KEDRA_R07_SESSION_PASS
systemctl poweroff --no-block

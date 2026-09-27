#!/usr/bin/bash
# VM-only desktop checks: what a container cannot host. The session's services,
# home review and recovery, Noctalia appearance, portals, agents, Bitwarden and
# the GTK 3/libadwaita file choosers are container scenarios in
# usr/src/kedra/tests/container (test-container.yml).
set -Eeuo pipefail
marker() { printf '%s\n' "$1" | tee /dev/ttyS1; }
trap 'code=$?; marker "KEDRA_R07_FAIL line=$LINENO code=$code"; journalctl -b -u greetd --no-pager -n 50; systemctl poweroff --no-block; exit "$code"' ERR
test "$(getenforce)" = Enforcing
# UEFI Secure Boot as seen by shim, the firmware variables (efivarfs prefixes
# each value with 4 attribute bytes), kernel lockdown and the kernel's report.
# The journal keeps the kernel line that the image's quiet karg hides.
check_secure_boot() {
    local efi=/sys/firmware/efi/efivars global=8be4df61-93ca-11d2-aa0d-00e098032b8c
    test "$(mokutil --sb-state)" = 'SecureBoot enabled'
    test "$(od -An -tx1 -v "$efi/SecureBoot-$global" | awk 'NR == 1 && NF == 5 { print $5 }')" = 01
    test "$(od -An -tx1 -v "$efi/SetupMode-$global" | awk 'NR == 1 && NF == 5 { print $5 }')" = 00
    grep -E '\[(integrity|confidentiality)\]' /sys/kernel/security/lockdown
    journalctl -k -b --no-pager -o cat | grep -Fx 'secureboot: Secure boot enabled'
}
check_secure_boot
marker KEDRA_SECUREBOOT_PASS
uid=$(id -u kedra-test)
for attempt in $(seq 1 60); do
    if pgrep -u greetd -x tuigreet >/dev/null; then break; fi
    sleep 1
done
pgrep -u greetd -x tuigreet >/dev/null
# The host types the generated password into tuigreet on VT 1.
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
# niri on the virtio GPU through its TTY/DRM backend, at the fixture mode.
as_user env NIRI_SOCKET="$niri_socket" niri msg --json outputs | \
    jq -e '.["Virtual-1"].logical | .width == 1280 and .height == 768 and .scale == 1' >/dev/null
# A real password login through greetd's PAM stack unlocked the login keyring.
as_user timeout --kill-after=2s 20s busctl --user call org.freedesktop.secrets /org/freedesktop/secrets org.freedesktop.DBus.Peer Ping
test "$(as_user timeout --kill-after=2s 20s busctl --user get-property org.freedesktop.secrets /org/freedesktop/secrets/aliases/default org.freedesktop.Secret.Collection Locked)" = 'b false'
marker KEDRA_R07_PAM_KEYRING_PASS
as_user systemctl --user start xdg-desktop-portal.service
# Keep the JSON of a failing doctor too; the assertion below still requires success.
doctor=$(as_user sysroot doctor --json || true)
# Record what the ordinary user observed for Secure Boot and kernel lockdown.
printf '%s\n' "$doctor" | jq -c '.checks[] | select(.name == "secure_boot" or .name == "selinux")' | tee /dev/ttyS1
printf '%s\n' "$doctor" | jq -e '.desktop_session_checks_passed and (.changes_performed | not)
    and any(.checks[]; .name == "secure_boot" and .passed and .required_for_session)
    and any(.checks[]; .name == "selinux" and .passed and .required_for_session)' >/dev/null
marker KEDRA_DOCTOR_SESSION_PASS
# Xwayland and Qt/KDE file choosers driven by real (QEMU) key events. In the
# container session these clients receive no virtual-keyboard input.
review_home=$(getent passwd kedra-test | cut -d: -f6)
toolkit_result="$review_home/toolkit-result.json"
for toolkit_case in gtk3-xwayland qt5 qt6 qt6-override; do
    as_user rm -f "$toolkit_result"
    toolkit_unit="kedra-toolkit-$toolkit_case"
    backend_env=()
    case "$toolkit_case" in
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
                    if test -f "$toolkit_result"; then cat "$toolkit_result"; fi
                    journalctl -b "_SYSTEMD_USER_UNIT=$toolkit_unit.service" --no-pager -n 40
                    echo "Toolkit $toolkit_case exited before $toolkit_stage (state=$toolkit_service_state)" >&2
                    false
                    ;;
            esac
            if test -f "$toolkit_result" && jq -e --arg stage "$toolkit_stage" '.stage == $stage' "$toolkit_result" >/dev/null; then break; fi
            if test -f "$toolkit_result" && jq -e '.stage == "failed"' "$toolkit_result" >/dev/null; then cat "$toolkit_result"; false; fi
            sleep 1
        done
        if ! jq -e --arg stage "$toolkit_stage" '.stage == $stage' "$toolkit_result"; then
            if test -f "$toolkit_result"; then cat "$toolkit_result"; fi
            journalctl -b "_SYSTEMD_USER_UNIT=$toolkit_unit.service" --no-pager -n 40
            as_user env NIRI_SOCKET="$niri_socket" timeout 10s niri msg --json windows
            false
        fi
        cat "$toolkit_result"
        marker "KEDRA_TOOLKIT_${toolkit_case}_${toolkit_stage}"
    done
    # Allow the host to capture the selected-file result before closing.
    sleep 4
    as_user systemctl --user stop "$toolkit_unit.service"
    if test "$toolkit_case" = qt6-override; then
        as_user rm "$override_config/kdeglobals"
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

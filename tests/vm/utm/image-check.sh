#!/usr/bin/bash
# In-container checks of the utm candidate; reads only, run through podman stdin.
set -euo pipefail
test "$#" -eq 1
contract=$1
test "$(uname -m)" = aarch64
jq -e '.target.id == "utm" and .target.architecture == "aarch64" and .target.image == "ghcr.io/reidond/kedra-utm"' \
    /usr/share/sysroot/source.json > /dev/null
niri --version
noctalia --version
noctalia msg --help
niri validate --config /usr/share/sysroot/home/default/.config/niri/config.kdl
validation_state=$(mktemp -d)
NOCTALIA_CONFIG_HOME=/usr/share/sysroot/home/default/.config \
    NOCTALIA_STATE_HOME="$validation_state" noctalia config validate
cat /etc/pam.d/greetd
test "$(systemctl is-enabled greetd)" = enabled
test "$(systemctl is-enabled bootc-fetch-apply-updates.timer || true)" = masked
test "$(systemctl get-default)" = graphical.target

# Virtual-machine integration and Venus/VirGL diagnostics from hosts/utm.
rpm -q qemu-guest-agent spice-vdagent vulkan-tools egl-utils mesa-dri-drivers mesa-vulkan-drivers
# /etc/sysconfig/qemu-ga stays as packaged.
test -z "$(rpm -V --configfiles qemu-guest-agent)"
# Fedora 44 blocks no guest-agent RPC; the hosts/utm drop-in must refuse the ones
# giving the Mac root in the guest, on top of Fedora's otherwise unchanged command.
blocked=guest-exec,guest-exec-status,guest-file-close,guest-file-flush,guest-file-open,guest-file-read,guest-file-seek,guest-file-write,guest-set-user-password,guest-ssh-add-authorized-keys,guest-ssh-get-authorized-keys,guest-ssh-remove-authorized-keys
execstart() {
    awk '/^ExecStart=/ { cmd = $0; more = /\\$/; next } more { cmd = cmd "\n" $0; more = /\\$/ } END { print cmd }' "$@"
}
systemctl cat qemu-guest-agent.service
effective=$(systemctl cat qemu-guest-agent.service | execstart)
grep -Fqx -- "  --block-rpcs=$blocked \\" <<< "$effective"
test "$(grep -vFx -- "  --block-rpcs=$blocked \\" <<< "$effective")" = \
    "$(execstart /usr/lib/systemd/system/qemu-guest-agent.service)"
# The installed agent itself, on a private socket, must disable exactly that set.
qga=$(mktemp -d)
qemu-ga --method=unix-listen --path="$qga/socket" --pidfile="$qga/pid" --statedir="$qga" \
    --block-rpcs="$blocked" < /dev/null &
qga_pid=$!
python3 - "$qga/socket" "$blocked" <<'PY'
import json
import socket
import sys
import time

path, blocked = sys.argv[1], sorted(sys.argv[2].split(','))
for _ in range(100):
    agent = socket.socket(socket.AF_UNIX)
    try:
        agent.connect(path)
        break
    except OSError:
        agent.close()
        time.sleep(0.1)
else:
    sys.exit('qemu-ga did not listen on ' + path)
agent.settimeout(30)
stream = agent.makefile('rw')


def call(command, **arguments):
    stream.write(json.dumps({'execute': command, 'arguments': arguments}) + '\n')
    stream.flush()
    return json.loads(stream.readline())


info = call('guest-info')['return']
disabled = sorted(c['name'] for c in info['supported_commands'] if not c['enabled'])
print('qemu-ga', info['version'], 'disabled:', ','.join(disabled))
if disabled != blocked:
    sys.exit('qemu-ga disabled RPC set differs from the utm block list')
refused = call('guest-exec', path='/usr/bin/true')
print('guest-exec:', json.dumps(refused))
if refused.get('error', {}).get('class') != 'CommandNotFound':
    sys.exit('qemu-ga did not refuse guest-exec')
PY
kill "$qga_pid"
wait "$qga_pid" || true
cat /usr/share/vulkan/icd.d/virtio_icd.aarch64.json
jq -e '.ICD.library_path | endswith("libvulkan_virtio.so")' /usr/share/vulkan/icd.d/virtio_icd.aarch64.json > /dev/null
test -f /usr/lib64/dri/virtio_gpu_dri.so
cat /usr/lib/environment.d/70-kedra-utm-graphics.conf
# Resolve through systemd's own user-environment generator, as a session would.
env -i PATH=/usr/bin /usr/lib/systemd/user-environment-generators/30-systemd-environment-d-generator \
    | grep -Fx 'GSK_RENDERER=gl'
cat /usr/lib/bootc/kargs.d/30-kedra-utm-console.toml
grep -Fq '"console=ttyAMA0,115200"' /usr/lib/bootc/kargs.d/30-kedra-utm-console.toml
grep -Fq '"console=tty0"' /usr/lib/bootc/kargs.d/30-kedra-utm-console.toml

# Bitwarden from the official arm64 tarball: relocated flat tree, no special bits.
jq -e '.target == "utm" and .architecture == "aarch64" and .runtime_root == "/usr/lib/bitwarden"
    and .package_scripts_executed == false' /usr/share/sysroot/bitwarden.json
test -x /usr/bin/bitwarden
test -x /usr/lib/bitwarden/bitwarden-app
test "$(stat -c '%a %U:%G' /usr/lib/bitwarden/chrome-sandbox)" = '755 root:root'
test -z "$(find /usr/lib/bitwarden -perm /7000 -print -quit)"
test -z "$(find /usr/lib/bitwarden ! -type f ! -type d -print -quit)"
grep -E '^Exec=/usr/bin/bitwarden( |$)' /usr/share/applications/com.bitwarden.desktop.desktop
compgen -G '/usr/share/icons/hicolor/*/apps/com.bitwarden.desktop.png'
napi=/usr/lib/bitwarden/resources/app.asar.unpacked/node_modules/@bitwarden/desktop-napi/desktop_napi.linux-arm64-gnu.node
dependencies=$(ldd /usr/lib/bitwarden/bitwarden-app /usr/lib/bitwarden/desktop_proxy "$napi")
printf '%s\n' "$dependencies"
if printf '%s\n' "$dependencies" | grep -F 'not found'; then
    echo 'Bitwarden runtime dependency missing' >&2
    exit 1
fi

# Private bundled Codex for the aarch64 musl triple.
jq -e '.target == "aarch64-unknown-linux-musl"' /usr/share/sysroot/agents/codex.json
codex_version=$(jq -er .version /usr/share/sysroot/agents/codex.json)
codex_home=$(mktemp -d)
test "$(HOME="$codex_home" /usr/libexec/sysroot/agents/codex/bin/codex --version)" = "codex-cli $codex_version"

# Boot chain and the bootc compatibility contract.
rpm -q shim-aa64 grub2-efi-aa64 bootupd mokutil efibootmgr kernel-core
test "$(rpm -q --qf '%{VERSION} %{ARCH}' bootc)" = "$contract aarch64"
compgen -G '/usr/lib/efi/shim/*/EFI/fedora/shimaa64.efi'
compgen -G '/usr/lib/efi/grub2/*/EFI/fedora/grubaa64.efi'
echo 'PASS: utm candidate image checks'

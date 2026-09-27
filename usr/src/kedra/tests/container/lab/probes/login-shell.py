#!/usr/bin/env python3
# Runs inside the lab container as the test account with the image's own python3; uv applies to host-side scripts only.
"""Print, as JSON, variables an interactive login shell of this account sees.

Login shells read /etc/profile.d (for example kedra-bitwarden.sh), which the
systemd user manager's environment does not.
"""
import json
import subprocess
import sys

names = sys.argv[1:] or ['SSH_AUTH_SOCK']
script = 'for name in "$@"; do printf "%s\\0" "${!name-}"; done'
output = subprocess.run(['bash', '--login', '-c', script, 'kedra-lab', *names],
                        check=True, capture_output=True, text=True, timeout=30).stdout
values = output.split('\0')[:-1]
print(json.dumps(dict(zip(names, values)), sort_keys=True))

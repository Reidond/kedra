# Login shells, including TTY sessions. Personal explicit agents take precedence.
if [ -n "${HOME:-}" ]; then
    : "${SSH_AUTH_SOCK:=$HOME/.bitwarden-ssh-agent.sock}"
    export SSH_AUTH_SOCK
fi

# Selected catalog tools precede Fedora commands in login shells.
export PATH="${KEDRA_CATALOG_JQ_BIN}:${KEDRA_CATALOG_SQLITE_BIN}:${PATH:-/usr/local/bin:/usr/bin:/bin}"

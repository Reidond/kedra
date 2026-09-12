#!/usr/bin/env bash
# Only the research assembler's PATH contains this observer.
exec /bin/bash /opt/kedra-refresh/target-native.sh dnf "$@"

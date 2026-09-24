#!/usr/bin/env bash
# Start (default) or stop the nested cosmic-comp: shell-host's scripts/dev/nested.sh, whose
# xdg-config seed already binds Ctrl+Alt+Space and the Launcher/AppLibrary system actions to
# `sill launcher ...` (KDE eats Super in the nested session, cosmic-comp#9).
#
# Usage: dev/nested.sh [start|stop]     SILL_NESTED_GPU=amd dev/nested.sh   (NVIDIA EGL fallback)
set -euo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
exec "$SHELL_HOST_DEV/nested.sh" "$@"

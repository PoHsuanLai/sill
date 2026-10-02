#!/usr/bin/env bash
# Start (default) or stop the nested cosmic-comp: shell-host's scripts/dev/nested.sh, whose
# xdg-config seed already binds Ctrl+Alt+Space and the Launcher/AppLibrary system actions to
# `sill launcher ...` (KDE eats Super in the nested session, cosmic-comp#9).
#
# The harness is shell-host's, never a copy: its `socket_of_pid` ends the `ss | awk` pipeline
# without SIGPIPE (the exit-141 bug that left the compositor running with no .nested-env was
# fixed there: its awk reads all of ss's output), so keep SHELL_HOST_DEV
# on a shell-host checkout at or after that fix.
#
# Usage: dev/nested.sh [start|stop]     SILL_NESTED_GPU=amd dev/nested.sh   (NVIDIA EGL fallback)
set -euo pipefail
# shellcheck source=dev/lib/compositor.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/compositor.sh"
# The compositor resolves the shortcut's bare `sill` against its own PATH, so
# the daemon built in this checkout must be on it before the compositor starts.
export PATH="$SILL_OUT:$SILL_TARGET/debug:$SILL_TARGET/release:$PATH"
exec "$SHELL_HOST_DEV/nested.sh" "$@"

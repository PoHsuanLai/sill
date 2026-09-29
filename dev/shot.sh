#!/usr/bin/env bash
# Screenshot. `dev/shot.sh OUT.png` shoots the nested compositor with grim (or the host with
# spectacle): shell-host's scripts/dev/shot.sh. `dev/shot.sh --surface NAME OUT.png` renders one
# surface offscreen through the daemon (`sill debug capture`), compositor-independent.
set -euo pipefail
# shellcheck source=dev/lib/compositor.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/compositor.sh"
if [ "${1:-}" = "--surface" ]; then
  [ "$#" -eq 3 ] || { echo "usage: dev/shot.sh --surface NAME OUT.png" >&2; exit 2; }
  exec "$SHELL_HOST_DEV/run.sh" "$SILL_BIN" debug capture "$2" "$(realpath -m "$3")"
fi
exec "$SHELL_HOST_DEV/shot.sh" "$@"

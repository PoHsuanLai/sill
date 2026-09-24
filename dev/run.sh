#!/usr/bin/env bash
# Run sill inside the nested compositor. With no arguments: `sill run` from this checkout's
# debug build (built first); with arguments: that command, e.g. `dev/run.sh foot`.
#
# Usage: dev/run.sh                 dev/run.sh target/release/sill run
set -euo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
if [ "$#" -eq 0 ]; then
  (cd "$SILL_REPO" && cargo build -p sill --features debug)
  exec "$SHELL_HOST_DEV/run.sh" "$SILL_BIN" run
fi
exec "$SHELL_HOST_DEV/run.sh" "$@"

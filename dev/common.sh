# Sourced by the dev/ wrappers: where the sibling shell-host checkout and this repo are.
# shell-host owns the nested-compositor harness (scripts/dev/); sill only wraps it.
SILL_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SHELL_HOST_DEV="${SHELL_HOST_DEV:-$SILL_REPO/../shell-host/scripts/dev}"
if [ ! -x "$SHELL_HOST_DEV/nested.sh" ]; then
  echo "dev: shell-host's harness not found at $SHELL_HOST_DEV (set SHELL_HOST_DEV)" >&2
  exit 1
fi
SILL_BIN="${SILL_BIN:-$SILL_REPO/target/debug/sill}"

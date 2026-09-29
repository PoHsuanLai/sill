# Where the sibling shell-host checkout and this repo are, and the two compositors a script runs
# on: shell-host's headless KWin (kwin-virtual.sh) and the nested cosmic-comp (nested.sh). The
# nested-compositor harness belongs to shell-host; sill only wraps it.
SILL_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SHELL_HOST_DEV="${SHELL_HOST_DEV:-$SILL_REPO/../shell-host/scripts/dev}"
if [ ! -x "$SHELL_HOST_DEV/nested.sh" ]; then
  echo "dev: shell-host's harness not found at $SHELL_HOST_DEV (set SHELL_HOST_DEV)" >&2
  exit 1
fi
SILL_BIN="${SILL_BIN:-$SILL_REPO/target/debug/sill}"
HOST_REPO="$(cd "$SHELL_HOST_DEV/../.." && pwd)"
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
SIZE="${SIZE:-1920x1080}"
WL=
BUS=

# Runs "$@" in the nested cosmic-comp's session (shell-host's run.sh).
nested() { "$SHELL_HOST_DEV/run.sh" "$@"; }

# The headless KWin, at $SIZE (or $1), on its private bus; sets WL (its socket, absolute) and BUS
# (its private bus) from its env file, read without sourcing it into this shell: `kwin-virtual.sh
# stop` kills every process whose environment names the bus, which must not include the script.
# Never borrows a KWin someone else started, refuses a socket that is not the headless KWin's,
# and refuses a bus that would be the real one. Needs OUT_DIR. KWIN_VIRTUAL_* variables exported
# beforehand reach kwin-virtual.sh.
start_kwin() {
  [ ! -e "$KWIN_ENV" ] || { echo "$KWIN_ENV exists: another headless KWin is running" >&2; return 1; }
  "$SHELL_HOST_DEV/kwin-virtual.sh" start "${1:-$SIZE}" >"$OUT_DIR/kwin.log" 2>&1 || return 1
  local runtime socket
  runtime=$(sed -n 's/^export XDG_RUNTIME_DIR=//p' "$KWIN_ENV")
  socket=$(sed -n 's/^export WAYLAND_DISPLAY=//p' "$KWIN_ENV")
  BUS=$(sed -n "s/^export DBUS_SESSION_BUS_ADDRESS='\\(.*\\)'\$/\\1/p" "$KWIN_ENV")
  KWIN_SCOPE=$(sed -n 's/^\(export \)\{0,1\}KWIN_SCOPE=//p' "$KWIN_ENV")
  case "$socket" in /*) WL="$socket" ;; *) WL="$runtime/$socket" ;; esac
  case "$WL" in
    *wayland-kwinvirt-*) ;;
    *) echo "refusing $WL: not the headless KWin's socket" >&2; stop_kwin; return 1 ;;
  esac
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $KWIN_ENV" >&2; stop_kwin; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "KWin's bus is the session's" >&2; stop_kwin; return 1; }
  require_private_system_bus "$BUS" || { stop_kwin; return 1; }
}

# Ends exactly the KWin start_kwin started; a no-op when none is running.
stop_kwin() {
  [ -f "$KWIN_ENV" ] || return 0
  "$SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1
}

# Sets scale $1 on the headless KWin's one output (the BenQ 4K at 2x of the user's session).
set_kwin_scale() {
  local output
  output=$(in_env kscreen-doctor -j 2>/dev/null | python3 -c '
import json, sys
print(json.load(sys.stdin)["outputs"][0]["name"])') || return 1
  in_env kscreen-doctor "output.$output.scale.$1" >>"$OUT_DIR/kwin.log" 2>&1
}

# The nested output's logical size, "W H".
nested_output_size() {
  nested wayland-info | sed -n 's/.*logical_width: \([0-9]*\), logical_height: \([0-9]*\).*/\1 \2/p' | head -1
}

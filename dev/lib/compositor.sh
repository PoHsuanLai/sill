# Where the sibling shell-host checkout and this repo are, and the two compositors a script runs
# on: shell-host's headless KWin (kwin-virtual.sh) and the nested cosmic-comp (nested.sh). The
# nested-compositor harness belongs to shell-host; sill only wraps it.
SILL_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SHELL_HOST_DEV="${SHELL_HOST_DEV:-$SILL_REPO/../shell-host/scripts/dev}"
if [ ! -x "$SHELL_HOST_DEV/nested.sh" ]; then
  echo "dev: shell-host's harness not found at $SHELL_HOST_DEV (set SHELL_HOST_DEV)" >&2
  exit 1
fi
# The build every script runs: SILL_PROFILE=debug (the default) or release, from cargo's target
# directory (CARGO_TARGET_DIR when set, else this checkout's target/). Both carry
# `--features debug` (the fake DDC, `sill debug ...`), which `build` passes. The latency budgets
# (a warm launcher open, the switcher's key-to-frame time, ...) are release budgets: a debug
# build is several times slower, so `budgets_apply` is false there and a script prints the
# measurement but does not fail it. Run the suite with SILL_PROFILE=release for the full verdict.
SILL_PROFILE="${SILL_PROFILE:-debug}"
# shellcheck disable=SC2034 # SILL_CARGO_PROFILE is read by daemon.sh, run.sh and lock.sh
case "$SILL_PROFILE" in
  debug) SILL_CARGO_PROFILE=() ;;
  release) SILL_CARGO_PROFILE=(--release) ;;
  *) echo "dev: SILL_PROFILE must be debug or release, not '$SILL_PROFILE'" >&2; exit 1 ;;
esac
SILL_TARGET="${CARGO_TARGET_DIR:-$SILL_REPO/target}"
SILL_OUT="$SILL_TARGET/$SILL_PROFILE"
SILL_BIN="${SILL_BIN:-$SILL_OUT/sill}"
budgets_apply() { [ "$SILL_PROFILE" = release ]; }
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

# The headless KWin's scale of its one output, from kscreen-doctor's JSON.
kwin_scale() {
  in_env kscreen-doctor -j 2>/dev/null | python3 -c '
import json, sys
print(json.load(sys.stdin)["outputs"][0].get("scale"))'
}

# Sets scale $1 (set_kwin_scale) and waits up to 3 s for kscreen-doctor to read it back; fails
# when the output does not end up at that scale.
require_kwin_scale() {
  set_kwin_scale "$1" || return 1
  for _ in $(seq 30); do
    python3 -c 'import sys; sys.exit(0 if abs(float(sys.argv[1]) - float(sys.argv[2])) < 0.01 else 1)' \
      "$(kwin_scale)" "$1" 2>/dev/null && return 0
    sleep 0.1
  done
  echo "the output's scale is $(kwin_scale), not $1" >&2
  return 1
}

# The nested output's logical size, "W H".
nested_output_size() {
  nested wayland-info | sed -n 's/.*logical_width: \([0-9]*\), logical_height: \([0-9]*\).*/\1 \2/p' | head -1
}

# Sets WL to the running nested cosmic-comp's socket (absolute), from the env file dev/nested.sh
# wrote, read in a subshell so nothing of it reaches this shell. Fails when no such socket exists.
# A script on the nested compositor then starts its own private bus (start_private_bus) and its
# scratch tree (scratch_new); it never stops the compositor.
use_nested_socket() {
  local env_file="$SHELL_HOST_DEV/.nested-env" display
  [ -f "$env_file" ] || { echo "no nested compositor (dev/nested.sh first)" >&2; return 1; }
  display=$(source "$env_file" && echo "$WAYLAND_DISPLAY")
  case "$display" in
    /*) WL="$display" ;;
    *) WL="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$display" ;;
  esac
  [ -S "$WL" ] || { echo "no nested compositor at $WL (dev/nested.sh first)" >&2; return 1; }
}

# True when a `sill run` of the person's draws on the compositor socket $WL (its WAYLAND_DISPLAY,
# read from /proc): only a daemon on the same output is in this script's screenshots. Daemons on
# other compositors (another agent's nested session, the real one) are none of its business.
daemon_on_wl() {
  local pid
  for pid in $(pgrep -u "$(id -u)" -f 'sill run'); do
    tr '\0' '\n' <"/proc/$pid/environ" 2>/dev/null | grep -qx "WAYLAND_DISPLAY=$WL" && return 0
  done
  return 1
}

# Waits up to 10 minutes for no other sill daemon on this script's compositor ($WL): it would be
# in the screenshots. Fails when one is still there.
await_no_process() {
  local waited=0
  while daemon_on_wl; do
    [ "$waited" = 0 ] && echo "another sill daemon is running; waiting for it to finish" >&2
    waited=$((waited + 1))
    [ "$waited" -gt 600 ] && { echo "it is still running after 10 min" >&2; return 1; }
    sleep 1
  done
}

# The height of a window that asks to be maximised on the nested output: shell-host's
# `probe_toplevel` (built on first use) stands in for foot, asks to be maximised and prints each
# configure as JSON into $OUT_DIR/probe.log. Prints the last maximised configure's height.
PROBE="$HOST_REPO/target/release/examples/probe_toplevel"
maximised_height() {
  [ -x "$PROBE" ] || (cd "$HOST_REPO" && cargo build --release -p shell-host --features debug-inject \
    --example probe_toplevel >&2) || return 1
  local log="$OUT_DIR/probe.log" probe
  # The probe reads commands on stdin, which must be pollable: a pipe, not /dev/null.
  sleep 20 | nested "$PROBE" >"$log" 2>&1 &
  probe=$!
  for _ in $(seq 100); do
    grep -q '"maximized":true' "$log" && break
    sleep 0.1
  done
  kill "$probe" 2>/dev/null; wait "$probe" 2>/dev/null
  grep '"maximized":true' "$log" | tail -1 | python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["height"])'
}

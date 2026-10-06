# Where the sibling shell-host checkout and this repo are, and the three compositors a script runs
# on: the nested cosmic-comp (shell-host's nested.sh), shell-host's headless KWin (kwin-virtual.sh)
# and casement (the sibling casement checkout's dev/up.sh, headless, INSIDE the jail: only
# dev/accept.sh --compositor casement, or a script run under ~/desktop/harness/jail.sh, can use
# it). The nested-compositor and KWin harnesses belong to shell-host; sill only wraps them.
#
# SILL_COMPOSITOR=casement|cosmic|kwin picks the compositor for `start_compositor`; unset, each
# script keeps its historical choice (cosmic where it ran nested, kwin where it ran headless
# KWin: its `start_compositor --default ...`) until the S4 switch makes casement the default.
# One set of helpers answers for all three: start_compositor / stop_compositor, set_scale,
# require_scale, output_scale, output_size, shot, maximised_height and nested (a command in the
# compositor's environment); input.sh has start_input / input / key / click.
SILL_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SHELL_HOST_DEV="${SHELL_HOST_DEV:-$SILL_REPO/../shell-host/scripts/dev}"
if [ ! -x "$SHELL_HOST_DEV/nested.sh" ]; then
  echo "dev: shell-host's harness not found at $SHELL_HOST_DEV (set SHELL_HOST_DEV)" >&2
  exit 1
fi
# The build every script runs: SILL_PROFILE=debug (the default) or release, from cargo's target
# directory (CARGO_TARGET_DIR when set, else this checkout's target/). Both carry
# `--features debug` (the fake DDC, `sill debug ...`), which `build` passes. The latency budgets
# (a warm launcher open, a banner's first frame, ...) are release budgets: a debug build is
# several times slower, so `budgets_apply` is false there (and on a host whose load is past its
# core count) and a script prints the measurement but does not fail it. Run the suite with
# SILL_PROFILE=release on a quiet host for the full verdict.
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
# A budget also needs a host that is not oversubscribed: the figures include `sill debug capture`
# and key round trips, which stretch several-fold when the one-minute load average is past the
# core count (another build or an agent suite running), and then say nothing about the shell.
budgets_apply() {
  [ "$SILL_PROFILE" = release ] || return 1
  awk -v cores="$(nproc)" '{ exit !($1 < cores) }' /proc/loadavg
}
budgets_why() {
  [ "$SILL_PROFILE" = release ] || { echo "a $SILL_PROFILE build; run with SILL_PROFILE=release"; return; }
  echo "host load $(cut -d' ' -f1 /proc/loadavg) is past its $(nproc) cores"
}
HOST_REPO="$(cd "$SHELL_HOST_DEV/../.." && pwd)"
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
SIZE="${SIZE:-1920x1080}"
WL=
BUS=
# Casement: the sibling checkout (a casement worktree beside this one, else ~/casement) and the
# tools dev/bootstrap.sh built there OUTSIDE the jail (casement, casementctl, casement-probe,
# fake-logind in dev/out/tools/bin; nothing inside the jail calls cargo). CASEMENT_ENV is the env
# file of the session start_casement made.
CASEMENT_REPO="${CASEMENT_REPO:-$(cd "$SILL_REPO/../casement" 2>/dev/null && pwd || echo "$HOME/casement")}"
CASEMENT_TOOLS="${CASEMENT_TOOLS:-$CASEMENT_REPO/dev/out/tools}"
CASEMENT_ENV=
# Casement's own bus (Debug1 lives on it): BUS is the script's, which a script that starts a bus per
# group (launcher.sh) replaces, so casementctl keeps its own address.
CASEMENT_BUS=
# The compositor start_compositor started (casement, cosmic or kwin).
COMPOSITOR=
# Where a script's logs and captures go: the target directory and dev/out in a checkout, but the
# jail's own scratch dir inside one (A8: state never lands in the checkout, so jails run in parallel).
if [ -n "${JAIL_SCRATCH:-}" ]; then
  SILL_OUT_ROOT="$JAIL_SCRATCH/sill"
  SILL_SHOTS="$SILL_OUT_ROOT/shots"
else
  SILL_OUT_ROOT="$SILL_TARGET"
  SILL_SHOTS="$SILL_REPO/dev/out"
fi

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

# ---------------------------------------------------------------- the compositor, whichever it is

# The compositor `start_compositor` uses: $SILL_COMPOSITOR, else the script's own default $1.
compositor_kind() {
  local kind="${SILL_COMPOSITOR:-$1}"
  case "$kind" in
    casement|cosmic|kwin) echo "$kind" ;;
    *) echo "dev: SILL_COMPOSITOR must be casement, cosmic or kwin, not '$kind'" >&2; return 1 ;;
  esac
}

# start_compositor [--default cosmic|kwin] [SIZE]: brings the compositor up and sets WL (its
# socket, absolute) and BUS (the private bus standing in for the session and the system bus).
#   cosmic    the running nested cosmic-comp's socket (reused, never stopped) and a new private bus;
#   kwin      shell-host's headless KWin at SIZE (default $SIZE) on its private bus;
#   casement  dev/up.sh of the casement checkout in this jail: a headless casement on the
#             wall clock at SIZE, its private bus (also fake-logind's), WL and BUS from its env file.
# Needs OUT_DIR. Fails (exit code 2 for "not here") when casement is asked for outside the jail.
start_compositor() {
  local default=cosmic
  if [ "${1:-}" = --default ]; then default="$2"; shift 2; fi
  COMPOSITOR=$(compositor_kind "$default") || return 1
  case "$COMPOSITOR" in
    casement) start_casement "$@" ;;
    kwin) start_kwin "$@" ;;
    cosmic) use_nested_socket && start_private_bus ;;
  esac
}

# Ends exactly what start_compositor started; a no-op when none is running.
stop_compositor() {
  case "$COMPOSITOR" in
    casement) stop_casement ;;
    kwin) stop_kwin ;;
    cosmic) stop_private_bus ;;
  esac
  COMPOSITOR=
}

# The casement tool NAME in the bootstrapped tools directory.
casement_bin() {
  local path="$CASEMENT_TOOLS/bin/$1"
  [ -x "$path" ] || { echo "casement: $path is missing (run $CASEMENT_REPO/dev/bootstrap.sh outside the jail)" >&2; return 3; }
  echo "$path"
}

# A casement tool in the session's environment (its bus and its socket; usable before scratch_new).
casement_run() {
  local tool="$1" bin
  shift
  bin=$(casement_bin "$tool") || return 3
  env WAYLAND_DISPLAY="$WL" XDG_RUNTIME_DIR="$(dirname "$WL")" DBUS_SESSION_BUS_ADDRESS="${CASEMENT_BUS:-$BUS}" "$bin" "$@"
}
casementctl() { casement_run casementctl "$@"; }

# Starts casement through its own dev/up.sh and sets WL, BUS, CASEMENT_ENV. $1 is a size "WxH"
# (default $SIZE); KWIN_VIRTUAL_OUTPUTS=2 (widgets.sh) means two outputs side by side. The clock
# is the wall clock: sill's animations and timers run on real time, and a stepped clock would
# stand still until a script advanced it. Refuses a bus that would be the session's.
start_casement() {
  local size="${1:-$SIZE}" spec up_out env_file
  mkdir -p "$OUT_DIR"
  spec="HEADLESS-1:$size@60000:120:0,0"
  if [ "${KWIN_VIRTUAL_OUTPUTS:-1}" = 2 ]; then
    spec="$spec;HEADLESS-2:$size@60000:120:${size%x*},0"
  fi
  [ -z "$CASEMENT_ENV" ] || { echo "start_casement: a casement session is already up ($CASEMENT_ENV)" >&2; return 1; }
  up_out=$("$CASEMENT_REPO/dev/up.sh" --clock wall --outputs "$spec" 2>"$OUT_DIR/casement-up.log")
  local rc=$?
  if [ "$rc" != 0 ]; then
    cat "$OUT_DIR/casement-up.log" >&2
    echo "start_casement: dev/up.sh exited $rc (2: not in the jail; 3: run dev/bootstrap.sh)" >&2
    return "$rc"
  fi
  env_file=$(tail -n1 <<<"$up_out")
  [ -f "$env_file" ] || { echo "start_casement: no env file '$env_file'" >&2; return 1; }
  CASEMENT_ENV="$env_file"
  WL=$(source "$env_file" && echo "$WAYLAND_DISPLAY")
  BUS=$(source "$env_file" && echo "$DBUS_SESSION_BUS_ADDRESS")
  CASEMENT_BUS=$BUS
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $env_file" >&2; stop_casement; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "casement's bus is the session's" >&2; stop_casement; return 1; }
  require_private_system_bus "$BUS" || { stop_casement; return 1; }
  # Casement's pointer starts on the output's corner (0, 0), where the hot corner and the bar's first
  # item would take it as a dwell and a hover. Park it on the left edge, halfway down: clear of the
  # corners, the bar, the dock, the right-hand notification column and the centred panels and
  # sheets (a launcher row under a resting pointer takes the hover and moves the selection).
  casementctl debug inject "move $(( ${size%x*} / 8 )) $(( ${size#*x} / 2 ))" >>"$OUT_DIR/casement-up.log" 2>&1 \
    || { echo "start_casement: could not park the pointer (casementctl debug inject)" >&2; stop_casement; return 1; }
}

# Ends exactly the casement start_casement started (dev/down.sh stops each process by the pid in
# the env file, never by name); a no-op when none is running.
stop_casement() {
  [ -n "$CASEMENT_ENV" ] || return 0
  "$CASEMENT_REPO/dev/down.sh" "$CASEMENT_ENV" >>"$OUT_DIR/casement-up.log" 2>&1
  CASEMENT_ENV=
  CASEMENT_BUS=
}

# A line for a step that cannot be proven on this compositor yet: `expected_skip WHY` prints it
# (dev/accept.sh lists these) and returns 2, the "could not run" verdict of a step.
expected_skip() {
  echo "expected skip: $*" >&2
  return 2
}

# Runs "$@" in the compositor's environment: the nested cosmic-comp's session (shell-host's
# run.sh), or, on casement or KWin, the script's own socket and bus.
nested() {
  case "$COMPOSITOR" in
    casement|kwin) env WAYLAND_DISPLAY="$WL" XDG_RUNTIME_DIR="$(dirname "$WL")" DBUS_SESSION_BUS_ADDRESS="$BUS" "$@" ;;
    *) "$SHELL_HOST_DEV/run.sh" "$@" ;;
  esac
}

# The output's logical size, "W H".
output_size() {
  case "$COMPOSITOR" in
    casement)
      casementctl debug output list --json | python3 -c '
import json, sys
o = json.load(sys.stdin)[0]
print(*o["logical"])' ;;
    *) nested_output_size ;;
  esac
}
# The nested output's logical size, "W H", from wayland-info.
nested_output_size() {
  nested wayland-info | sed -n 's/.*logical_width: \([0-9]*\), logical_height: \([0-9]*\).*/\1 \2/p' | head -1
}

# The first output's name and scale (a decimal), by compositor.
output_scale() {
  case "$COMPOSITOR" in
    casement)
      casementctl debug output list --json | python3 -c '
import json, sys
print(json.load(sys.stdin)[0]["scale"] / 120)' ;;
    *) kwin_scale ;;
  esac
}

# Sets scale $1 on the one output.
set_scale() {
  case "$COMPOSITOR" in
    casement)
      local name
      name=$(casementctl debug output list --json | python3 -c '
import json, sys
print(json.load(sys.stdin)[0]["name"])') || return 1
      casementctl debug output set "$name" --scale "$1" >>"$OUT_DIR/casement-up.log" 2>&1 ;;
    *) set_kwin_scale "$1" ;;
  esac
}

# Sets scale $1 (set_scale) and waits up to 3 s for the output to read it back; fails when it does not
# end up at that scale.
require_scale() {
  set_scale "$1" || return 1
  for _ in $(seq 30); do
    python3 -c 'import sys; sys.exit(0 if abs(float(sys.argv[1]) - float(sys.argv[2])) < 0.01 else 1)' \
      "$(output_scale)" "$1" 2>/dev/null && return 0
    sleep 0.1
  done
  echo "the output's scale is $(output_scale), not $1" >&2
  return 1
}

# Shoots the whole output into $1 (a PNG): grim on the nested socket (shell-host's shot.sh) for
# cosmic, `casementctl shot --output NAME` for casement (the last presented frame of that output, the
# size answered as JSON; `--window` and `--masked` are P1-C1 and not used here). Returns 2
# (expected skip) when this casement build has no capture at all; KWin's own scripts use spectacle
# themselves.
shot() {
  case "$COMPOSITOR" in
    casement)
      mkdir -p "$(dirname "$1")"
      local rc
      # The headless casement's first output (dev/up.sh) is HEADLESS-1.
      casementctl shot --output "${CASEMENT_OUTPUT:-HEADLESS-1}" "$1" >/dev/null
      rc=$?
      [ "$rc" != 2 ] || expected_skip "this casement build has no screenshot (casementctl shot exits 2)"
      return "$rc" ;;
    # shell-host's shot.sh falls back to the host's session when no nested compositor answers, so
    # it is never called without one.
    *) nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor" >&2; return 1; }
       "$SHELL_HOST_DEV/shot.sh" "$1" >/dev/null ;;
  esac
}

# The height of a window that asks to be maximised on the output. Cosmic: shell-host's
# `probe_toplevel` (built on first use) stands in for foot, asks to be maximised and prints each
# configure as JSON into $OUT_DIR/probe.log; prints the last maximised configure's height.
# Casement: `casement-probe toplevel --maximised`, which prints the same facts as JSON lines;
# returns 2 (an expected skip) while the probe has no such flag (it ignores unknown flags and
# prints no `maximised` field) or casement's P0 floating windows are never configured maximised.
PROBE="${SILL_PROBE:-$HOST_REPO/target/release/examples/probe_toplevel}"
maximised_height() {
  [ "$COMPOSITOR" != casement ] || { maximised_height_casement; return; }
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

maximised_height_casement() {
  local log="$OUT_DIR/probe.log" height
  in_env timeout 25 "$(casement_bin casement-probe)" toplevel --maximised >"$log" 2>&1 </dev/null || true
  height=$(grep -E '"maxim(ised|ized)":true' "$log" | tail -1 | python3 -c '
import json, sys
print(json.loads(sys.stdin.read())["height"])' 2>/dev/null)
  if [ -z "$height" ]; then
    expected_skip "no maximised configure: casement-probe toplevel has no --maximised (it ignores the flag) and P0 windows are floating; the window manager lands in P1/P2"
    return 2
  fi
  echo "$height"
}

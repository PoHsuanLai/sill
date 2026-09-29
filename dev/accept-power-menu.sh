#!/usr/bin/env bash
# Power menu acceptance (design/20 §1.8), on a private headless KWin (shell-host
# scripts/dev/kwin-virtual.sh). Starts `sill run` from this checkout's debug build (`--features
# debug`) with its own XDG_RUNTIME_DIR and scratch XDG dirs, on KWin's private bus (the daemon's
# session AND system bus; start_kwin refuses to go on if it would be the real one), and quits it
# at the end.
#
# SAFETY. A confirmed action would really log out, restart, shut down or suspend the machine if it
# reached logind. The daemon is started with `sill run --session-dry-run RECORD`: a debug build
# then takes every session action through a dry run that only prints and records the chosen one,
# never logind (crates/sill/src/run_args.rs, crates/sill-services/src/env/session_actions.rs).
# Before Enter is ever pressed, the script proves the dry run is in force from the daemon's own
# log (the "dry run, recording to" line, and `power_menu.backend=dry-run` with no
# `power_menu.backend=live`); without that proof the Enter step is skipped (exit 2), never run.
#
# KEYS. Every key is `sill debug key SURFACE KEY`: shell-host's `Inject::Key`, which enters the
# power menu where a real xkb key does (the same machine, focus, modality and IME), whether or
# not the compositor gave the menu the keyboard. The keys are only sent while the dry run is
# proven and the menu is mapped. dev/accept-real-input.sh drives the same menu with real
# Wayland keys through fake input.
#
#   1. Warm: `sill-power-menu-<out>` exists from the start, hidden (no capture).
#   2. `sill power-menu open` shows the sheet over the scrim: a capture with the sheet's pixels,
#      into dev/out/power-menu.png. `close` hides it; `toggle` twice round-trips.
#   3. The arrows move the focus: Right from Shut Down is the next offered button (Suspend, or
#      Log Out when logind refuses Suspend), Left twice is Restart (the panel's
#      `power_menu.focus=` lines).
#   4. Enter on the dry run records Restart (`session.dry_run would=Restart` in the record
#      file) and the menu closes; nothing else is recorded.
#   5. Escape closes it, recording nothing.
#   6. 5 s idle after it hides: 0 frames on the power menu surface.
#
# Exit 0 only when every step passed; 2 when a step could not run safely (named); 1 on a failure.
#
# Usage: dev/accept-power-menu.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-power-menu"
SHOTS="$SILL_REPO/dev/out"
RECORD="$OUT_DIR/dry-run.record"
mkdir -p "$OUT_DIR" "$SHOTS"
DRY_RUN=unproven
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
SIZE=1920x1080
WL=
BUS=

SCRATCH=$(mktemp -d)
RUNTIME="$SCRATCH/run"
STATE="$SCRATCH/state"
CACHE="$SCRATCH/cache"
mkdir -p "$RUNTIME" "$STATE" "$CACHE"
chmod 700 "$RUNTIME"

# Every `sill` call, `quit` included, must reach the daemon's own compositor and bus (the
# headless KWin's, not this shell's ambient session's): its own XDG_RUNTIME_DIR (so its IPC
# socket is never the real one's), XDG_STATE_HOME and XDG_CACHE_HOME, and KWin's private bus as
# BOTH DBUS_SESSION_BUS_ADDRESS and DBUS_SYSTEM_BUS_ADDRESS.
in_env() {
  env XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$WL" DBUS_SESSION_BUS_ADDRESS="$BUS" \
    DBUS_SYSTEM_BUS_ADDRESS="$BUS" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" "$@"
}
sill() { in_env "$SILL_BIN" "$@"; }

# The output's connector name (the surfaces' namespace suffix).
output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-"):
        print(s[len("sill-bar-"):]); break'
}

# A surface's cumulative frame counter from `sill debug stats`.
frames_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"])' "$1"
}

# How many pixels of the PNG $1 are not fully transparent.
opaque_pixels() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
print(sum(1 for a in img.getchannel("A").tobytes() if a > 0))
EOF
}

# Whether the menu captures (it is mapped) with the scrim and sheet drawn (most of the output
# covered), into $2 (default: the scratch capture).
shows() {
  local png="${2:-$OUT_DIR/menu.png}"
  sill debug capture "$MENU" "$png" >>"$1" 2>&1 || return 1
  [ "$(opaque_pixels "$png")" -gt 100000 ]
}

# Wait up to $2 tenths of a second for the menu to show (or, with $3 = hidden, to hide).
wait_for() {
  local log="$1" tenths="$2" want="${3:-shown}"
  for _ in $(seq "$tenths"); do
    if sill debug capture "$MENU" "$OUT_DIR/probe.png" >>"$log" 2>&1; then
      [ "$want" = shown ] && return 0
    else
      [ "$want" = hidden ] && return 0
    fi
    sleep 0.1
  done
  return 1
}

# Wait up to $2 tenths of a second for the daemon's log to hold $3 more lines matching $1 than
# it did when counted as $4.
wait_log() {
  local pattern="$1" tenths="$2" more="$3" before="$4"
  for _ in $(seq "$tenths"); do
    [ "$(grep -c -- "$pattern" "$OUT_DIR/daemon.log")" -ge $(( before + more )) ] && return 0
    sleep 0.1
  done
  return 1
}

count_log() { grep -c -- "$1" "$OUT_DIR/daemon.log"; }

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

# The headless KWin, and its socket (absolute) and bus from its env file, read without sourcing
# it into this shell: `kwin-virtual.sh stop` kills every process whose environment names the
# bus, which must not include this script.
start_kwin() {
  "$SHELL_HOST_DEV/kwin-virtual.sh" start "$SIZE" >"$OUT_DIR/kwin.log" 2>&1 || return 1
  local runtime socket
  runtime=$(sed -n 's/^export XDG_RUNTIME_DIR=//p' "$KWIN_ENV")
  socket=$(sed -n 's/^export WAYLAND_DISPLAY=//p' "$KWIN_ENV")
  BUS=$(sed -n "s/^export DBUS_SESSION_BUS_ADDRESS='\\(.*\\)'\$/\\1/p" "$KWIN_ENV")
  WL="$runtime/$socket"
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $KWIN_ENV" >&2; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "KWin's bus is the session's" >&2; return 1; }
  require_private_system_bus "$BUS" || return 1
}

stop_kwin() {
  "$SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1
}

start_daemon() {
  CONFIG=$(mktemp -d)
  backdrop_seed "$CONFIG" || return 1
  : >"$RECORD"
  use_fake_ddc "$CONFIG" >/dev/null
  require_fake_ddc || return 1
  in_env env XDG_CONFIG_HOME="$CONFIG" \
    "$SILL_BIN" run "${FAKE_DDC_FLAGS[@]}" --session-dry-run "$RECORD" \
    >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  MENU="sill-power-menu-$NAME"
  sleep 1
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  in_env timeout 10 "$SILL_BIN" quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  rm -rf "$CONFIG"
}

# The dry run is in force: the daemon said so, every power menu surface mounted on it, and none
# on the live session. Sets DRY_RUN=proven.
prove_dry_run() {
  local log="$OUT_DIR/dry-run.log"
  DRY_RUN=unproven
  {
    grep "sill: session: dry run, recording to $RECORD" "$OUT_DIR/daemon.log" || return 1
    grep "power_menu.backend=dry-run" "$OUT_DIR/daemon.log" || return 1
    if grep "power_menu.backend=live" "$OUT_DIR/daemon.log"; then return 1; fi
  } >"$log" 2>&1
  DRY_RUN=proven
  echo "dry run proven from the daemon's log (record file $RECORD)" | tee -a "$log" >&2
}

# Press the keys "$@" (`sill debug key` names: ArrowRight, Enter, Escape, ...) on the menu, one at
# a time, only while the menu is mapped and the dry run is proven.
type_keys() {
  local log="$1"; shift
  [ "$DRY_RUN" = proven ] || { echo "no keys without a proven dry run" | tee -a "$log" >&2; return 1; }
  for key in "$@"; do
    wait_for "$log" 5 shown || { echo "the menu is not up for '$key'" | tee -a "$log" >&2; return 1; }
    sill debug key "$MENU" "$key" >>"$log" 2>&1 || { echo "the daemon refused key '$key'" >&2; return 1; }
    echo "key $key" >>"$log"
    sleep 0.2
  done
}

# Open the menu and let the sheet settle; notes whether it holds the compositor's keyboard (the
# keys do not need it).
open_with_keyboard() {
  local log="$1" before
  before=$(count_log 'power_menu.keyboard=held')
  sill power-menu open >>"$log" 2>&1 || return 1
  wait_for "$log" 30 shown || return 1
  sleep 0.6 # the sheet's peek-in
  if [ "$(count_log 'power_menu.keyboard=held')" -gt "$before" ]; then
    echo "the menu holds the keyboard" >>"$log"
  else
    echo "no keyboard focus from the compositor (the injected keys do not need it)" >>"$log"
  fi
}

# 1.
step_warm() {
  local log="$OUT_DIR/warm.log"
  sill debug stats >"$log" 2>&1
  grep -q "\"$MENU\"" "$log" || { echo "$MENU is not in 'sill debug stats'" >&2; return 1; }
  if sill debug capture "$MENU" "$OUT_DIR/hidden.png" >>"$log" 2>&1; then
    echo "the power menu is mapped before any open" >&2; return 1
  fi
  echo "$MENU spawned at start, hidden" | tee -a "$log" >&2
}

# 2.
step_open_close_toggle() {
  local log="$OUT_DIR/open.log"
  : >"$log"
  sill power-menu open >>"$log" 2>&1 || return 1
  wait_for "$log" 30 shown || { echo "open: never mapped" >&2; return 1; }
  sleep 0.8
  shows "$log" "$SHOTS/power-menu.png" || { echo "open: the sheet did not draw" >&2; return 1; }
  sill power-menu close >>"$log" 2>&1 || return 1
  wait_for "$log" 20 hidden || { echo "close: still mapped" >&2; return 1; }
  sill power-menu toggle >>"$log" 2>&1 || return 1
  wait_for "$log" 20 shown || { echo "toggle: never mapped" >&2; return 1; }
  sill power-menu toggle >>"$log" 2>&1 || return 1
  wait_for "$log" 20 hidden || { echo "toggle: still mapped" >&2; return 1; }
  echo "open shows the sheet over the scrim; close hides it; toggle twice round-trips" | tee -a "$log" >&2
}

# The button Right moves to from Shut Down: the next one the last opening offered, round to the
# start (the buttons' order: Log Out, Restart, Shut Down, Suspend).
right_of_shut_down() {
  grep 'power_menu.offers=' "$OUT_DIR/daemon.log" | tail -n 1 | python3 -c '
import sys
offered = sys.stdin.read().strip().split("=", 1)[1].split(",")
order = ["Log Out", "Restart", "Shut Down", "Suspend"]
start = order.index("Shut Down")
for step in range(1, len(order)):
    label = order[(start + step) % len(order)]
    if label in offered:
        print(label); break'
}

# 3.
step_arrows() {
  local log="$OUT_DIR/arrows.log" before status right
  : >"$log"
  open_with_keyboard "$log"; status=$?
  [ "$status" = 0 ] || return "$status"
  grep -q "power_menu.opened focus=Shut Down" "$OUT_DIR/daemon.log" || {
    echo "the menu did not open on Shut Down (power_menu.default_action)" >&2; return 1; }
  grep 'power_menu.offers=' "$OUT_DIR/daemon.log" | tail -n 1 | tee -a "$log" >&2
  right=$(right_of_shut_down)
  [ -n "$right" ] || { echo "no offered button right of Shut Down" >&2; return 1; }
  before=$(count_log "power_menu.focus=$right")
  type_keys "$log" ArrowRight || return 2
  wait_log "power_menu.focus=$right" 20 1 "$before" || { echo "Right: no focus on $right" >&2; return 1; }
  before=$(count_log 'power_menu.focus=Restart')
  type_keys "$log" ArrowLeft ArrowLeft || return 2
  wait_log 'power_menu.focus=Restart' 20 1 "$before" || { echo "Left Left: no focus on Restart" >&2; return 1; }
  sleep 0.3
  sill debug capture "$MENU" "$SHOTS/power-menu-restart.png" >>"$log" 2>&1 || echo "capture failed" >&2
  echo "Right: Shut Down -> $right; Left Left: -> Restart" | tee -a "$log" >&2
}

# 4. Runs only on a proven dry run, on the menu step 3 left open at Restart.
step_enter_records() {
  local log="$OUT_DIR/enter.log"
  : >"$log"
  if [ "$DRY_RUN" != proven ]; then
    echo "the dry run is not proven: Enter is NOT pressed" | tee -a "$log" >&2
    sill power-menu close >/dev/null 2>&1
    return 2
  fi
  wait_for "$log" 5 shown || { echo "the menu is not open for Enter" >&2; return 1; }
  type_keys "$log" Enter || return 2
  for _ in $(seq 30); do grep -q 'would=Restart' "$RECORD" && break; sleep 0.1; done
  wait_for "$log" 20 hidden || { echo "Enter: the menu stayed up" >&2; return 1; }
  echo "record file after Enter:" >>"$log"; cat "$RECORD" >>"$log"
  [ "$(cat "$RECORD")" = "session.dry_run would=Restart" ] || {
    echo "the record is not exactly one Restart: $(cat "$RECORD")" >&2; return 1; }
  grep -q 'session.dry_run would=Restart' "$OUT_DIR/daemon.log" || return 1
  echo "Enter recorded 'would=Restart' on the dry run and closed the menu" | tee -a "$log" >&2
}

# 5.
step_escape() {
  local log="$OUT_DIR/escape.log" status lines
  : >"$log"
  lines=$(wc -l <"$RECORD")
  open_with_keyboard "$log"; status=$?
  [ "$status" = 0 ] || return "$status"
  type_keys "$log" Escape || return 2
  wait_for "$log" 20 hidden || { echo "Escape: the menu stayed up" >&2; return 1; }
  [ "$(wc -l <"$RECORD")" = "$lines" ] || { echo "Escape recorded an action" >&2; return 1; }
  echo "Escape closed the menu, recording nothing" | tee -a "$log" >&2
}

# 6.
step_idle() {
  local log="$OUT_DIR/idle.log" f0 f1
  sill power-menu close >/dev/null 2>&1
  sleep 1
  f0=$(frames_of "$MENU")
  sleep 5
  f1=$(frames_of "$MENU")
  echo "5 s idle while hidden: $MENU $(( f1 - f0 )) frames" | tee "$log" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

build || exit 1
start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; rm -rf "$SCRATCH"; exit 1; }
if ! start_daemon; then
  stop_daemon; stop_kwin; rm -rf "$SCRATCH"; exit 1
fi
prove_dry_run || echo "the dry run could not be proven from the daemon's log" >&2
status=0
for step in step_warm step_open_close_toggle step_arrows step_enter_records step_escape step_idle; do
  "$step"; result=$?
  case "$result" in
    0) echo "PASS $step" ;;
    2) echo "SKIP $step"; [ "$status" = 1 ] || status=2 ;;
    *) echo "FAIL $step"; status=1 ;;
  esac
done
stop_daemon
stop_kwin
rm -rf "$SCRATCH"
echo "logs in $OUT_DIR; captures $SHOTS/power-menu.png, $SHOTS/power-menu-restart.png"
exit "$status"

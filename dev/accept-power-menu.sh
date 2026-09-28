#!/usr/bin/env bash
# Power menu acceptance (design/20 §1.8), on the nested cosmic-comp (dev/nested.sh; a running
# wayland-1 is reused and never stopped). Starts `sill run` from this checkout's debug build
# (`--features debug`) with a scratch XDG_CONFIG_HOME and quits it at the end.
#
# SAFETY. The nested session shares logind with the machine, so a confirmed action would really
# log out, restart, shut down or suspend it. The daemon is started with SILL_POWER_MENU_DRY_RUN
# naming a record file: a debug build then gives every power menu a dry-run backend that offers
# every action and only prints and records the chosen one (crates/sill/src/power_menu.rs,
# crates/sill-surfaces/src/surfaces/power_menu/backend.rs). Before Enter is ever pressed, the
# script proves the dry run is in force from the daemon's own log (the "dry run, recording to"
# line, and `power_menu.backend=dry-run` with no `power_menu.backend=live`); without that proof
# the Enter step is skipped (exit 2), never run.
#
# OFFERS. The dry run is also started with SILL_POWER_MENU_DRY_RUN_OFFERS=logind: it offers what
# logind answers (the session service's read-only `Can*` calls), so an action this machine
# refuses (Suspend here) is drawn disabled and skipped by the arrows, while a chosen action is
# still only recorded. The panel logs the offers on each opening (`power_menu.offers=`), and the
# arrows step expects the next offered button to the right of Shut Down.
#
# KEYS. On the nested cosmic-comp the power menu gets no keyboard focus (no
# `power_menu.keyboard=held`) and `wtype` keys never reach it, and shell-host injects no keys
# (FINDINGS "M5 power menu"). So the keys go through the debug key path: the daemon is also
# started with SILL_POWER_MENU_KEYS naming a FIFO, and each word written to it (`right`,
# `enter`, ...) is fed to the power menu as that key, through the same machine a real key
# drives. The daemon refuses the key path without the dry run, so a scripted Enter can never
# reach logind; the script also writes a key only while the menu is mapped.
#
#   1. Warm: `sill-power-menu-<out>` exists from the start, hidden (no capture).
#   2. `sill power-menu open` shows the sheet over the scrim: a capture with the sheet's pixels,
#      into dev/out/power-menu.png, and a screenshot of the nested output,
#      dev/out/power-menu-cosmic.png. `close` hides it; `toggle` twice round-trips.
#   3. The arrows move the focus: Right from Shut Down is the next offered button (Suspend, or
#      Log Out when logind refuses Suspend), Left twice is Restart (the panel's
#      `power_menu.focus=` lines), screenshot dev/out/power-menu-cosmic-restart.png.
#   4. Enter on the dry run records Restart (`power_menu.dry_run would=Restart` in the record
#      file) and the menu closes; nothing else is recorded.
#   5. Escape closes it, recording nothing.
#   6. 5 s idle after it hides: 0 frames on the power menu surface.
#
# Exit 0 only when every step passed; 2 when a step could not run safely (named); 1 on a failure.
#
# Usage: dev/nested.sh && dev/accept-power-menu.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-power-menu"
SHOTS="$SILL_REPO/dev/out"
RECORD="$OUT_DIR/dry-run.record"
KEYS="$OUT_DIR/keys.fifo"
mkdir -p "$OUT_DIR" "$SHOTS"
DRY_RUN=unproven
BUS=
BUS_PID=

sill() { "$SILL_BIN" "$@"; }
nested() { "$SHELL_HOST_DEV/run.sh" "$@"; }

# The nested output's connector name (the surfaces' namespace suffix).
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

start_daemon() {
  CONFIG=$(mktemp -d)
  backdrop_seed "$CONFIG" || return 1
  : >"$RECORD"
  rm -f "$KEYS"; mkfifo "$KEYS" || return 1
  XDG_CONFIG_HOME="$CONFIG" SILL_POWER_MENU_DRY_RUN="$RECORD" SILL_POWER_MENU_KEYS="$KEYS" \
    SILL_POWER_MENU_DRY_RUN_OFFERS=logind \
    DBUS_SESSION_BUS_ADDRESS="$BUS" DBUS_SYSTEM_BUS_ADDRESS="$BUS" \
    nested "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  MENU="sill-power-menu-$NAME"
  sleep 1
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  stop_private_bus
  rm -rf "$CONFIG"
}

# The dry run is in force: the daemon said so, every power menu surface mounted on it, and none
# on the live session. Sets DRY_RUN=proven.
prove_dry_run() {
  local log="$OUT_DIR/dry-run.log"
  DRY_RUN=unproven
  {
    grep "sill: power menu: dry run, recording to $RECORD" "$OUT_DIR/daemon.log" || return 1
    grep "sill: power menu: scripted keys from $KEYS" "$OUT_DIR/daemon.log" || return 1
    grep "power_menu.backend=dry-run" "$OUT_DIR/daemon.log" || return 1
    if grep "power_menu.backend=live" "$OUT_DIR/daemon.log"; then return 1; fi
  } >"$log" 2>&1
  DRY_RUN=proven
  echo "dry run proven from the daemon's log (record file $RECORD)" | tee -a "$log" >&2
}

# Press the keys "$@" (script words) through the debug key path, one at a time, only while the
# menu is mapped and the dry run is proven.
type_keys() {
  local log="$1"; shift
  [ "$DRY_RUN" = proven ] || { echo "no keys without a proven dry run" | tee -a "$log" >&2; return 1; }
  for key in "$@"; do
    wait_for "$log" 5 shown || { echo "the menu is not up for '$key'" | tee -a "$log" >&2; return 1; }
    timeout 5 sh -c 'printf "%s\n" "$1" >"$2"' _ "$key" "$KEYS" || return 1
    echo "key $key" >>"$log"
    sleep 0.2
  done
}

# Open the menu and let the sheet settle; notes whether it holds the real keyboard.
open_with_keyboard() {
  local log="$1" before
  before=$(count_log 'power_menu.keyboard=held')
  sill power-menu open >>"$log" 2>&1 || return 1
  wait_for "$log" 30 shown || return 1
  sleep 0.6 # the sheet's peek-in
  if [ "$(count_log 'power_menu.keyboard=held')" -gt "$before" ]; then
    echo "the menu holds the keyboard" >>"$log"
  else
    echo "no keyboard focus from the nested compositor (keys go through the debug key path)" >>"$log"
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
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/power-menu-cosmic.png" >>"$log" 2>&1 || echo "screenshot failed" >&2
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
  type_keys "$log" right || return 2
  wait_log "power_menu.focus=$right" 20 1 "$before" || { echo "Right: no focus on $right" >&2; return 1; }
  before=$(count_log 'power_menu.focus=Restart')
  type_keys "$log" left left || return 2
  wait_log 'power_menu.focus=Restart' 20 1 "$before" || { echo "Left Left: no focus on Restart" >&2; return 1; }
  sleep 0.3
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/power-menu-cosmic-restart.png" >>"$log" 2>&1 || echo "screenshot failed" >&2
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
  type_keys "$log" enter || return 2
  for _ in $(seq 30); do grep -q 'would=Restart' "$RECORD" && break; sleep 0.1; done
  wait_for "$log" 20 hidden || { echo "Enter: the menu stayed up" >&2; return 1; }
  echo "record file after Enter:" >>"$log"; cat "$RECORD" >>"$log"
  [ "$(cat "$RECORD")" = "power_menu.dry_run would=Restart" ] || {
    echo "the record is not exactly one Restart: $(cat "$RECORD")" >&2; return 1; }
  grep -q 'power_menu.dry_run would=Restart' "$OUT_DIR/daemon.log" || return 1
  echo "Enter recorded 'would=Restart' on the dry run and closed the menu" | tee -a "$log" >&2
}

# 5.
step_escape() {
  local log="$OUT_DIR/escape.log" status lines
  : >"$log"
  lines=$(wc -l <"$RECORD")
  open_with_keyboard "$log"; status=$?
  [ "$status" = 0 ] || return "$status"
  type_keys "$log" escape || return 2
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

source "$SHELL_HOST_DEV/.nested-env"
nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
build || exit 1
# The nested compositor is shared: wait (up to 10 min) for another script's daemon to quit.
for _ in $(seq 120); do sill ping >/dev/null 2>&1 || break; echo "a sill daemon is running; waiting" >&2; sleep 5; done
if sill ping >/dev/null 2>&1; then
  echo "a sill daemon is still running; quit it first (this script starts its own)" >&2
  exit 1
fi
start_private_bus || exit 1
start_daemon || { stop_daemon; stop_private_bus; exit 1; }
BACKDROP_DIR=$(mktemp -d)
backdrop_up "$BACKDROP_DIR" || echo "backdrop: none (the shot shows the bare compositor)" >&2
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
backdrop_down
rm -rf "$BACKDROP_DIR"
stop_daemon
rm -f "$KEYS"
echo "logs in $OUT_DIR; screenshots $SHOTS/power-menu-cosmic.png, $SHOTS/power-menu-cosmic-restart.png"
exit "$status"

#!/usr/bin/env bash
# Power menu acceptance (design/20 §1.8), on a private headless KWin (shell-host
# scripts/dev/kwin-virtual.sh). Starts `sill run` from this checkout's debug build (`--features
# debug`) with its own XDG_RUNTIME_DIR and scratch HOME and XDG dirs, on KWin's private bus (the
# daemon's session AND system bus; start_kwin refuses to go on if it would be the real one), and
# quits it at the end.
#
# SAFETY. A confirmed action would really log out, restart, shut down or suspend the machine if it
# reached logind. The daemon is started with `sill run --session-dry-run RECORD`: a debug build
# then takes every session action through a dry run that only prints and records the chosen one,
# never logind (crates/sill/src/run_args.rs, crates/sill-services/src/env/session_actions.rs).
# Before Enter is ever pressed, the script proves the dry run is in force from the daemon's own
# log (the "dry run, recording to" line, and `power_menu.backend=dry-run` with no
# `power_menu.backend=live`); without that proof the Enter step is skipped (exit 2), never run.
#
# KEYS. By default every key is `sill debug key SURFACE KEY`: shell-host's `Inject::Key`, which
# enters the power menu where a real xkb key does (the same machine, focus, modality and IME),
# whether or not the compositor gave the menu the keyboard. The keys are only sent while the dry
# run is proven and the menu is mapped. With --real-input every key and press is a real Wayland
# event through one fake-input client, with a real serial and the compositor's keyboard focus.
#
# Default steps:
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
# --real-input steps (Enter is never pressed and no button is clicked):
#   1. The power menu opens and holds the real keyboard (`power_menu.keyboard=held`).
#   2. Real Right moves the focus Shut Down -> Suspend, Left Left -> Restart.
#   3. Real Escape closes it, recording nothing.
#   4. Opened again, a real click on the scrim closes it, recording nothing.
#
# Exit 0 only when every step passed; 2 when a step could not run safely (named); 1 on a failure.
#
# Usage: dev/accept/power-menu.sh [--real-input]
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
case "${1:-}" in
  --real-input) MODE=real ;;
  "") MODE=keys ;;
  *) echo "usage: dev/accept/power-menu.sh [--real-input]" >&2; exit 2 ;;
esac
OUT_DIR="$SILL_REPO/target/accept-power-menu"
SHOTS="$SILL_REPO/dev/out"
RECORD="$OUT_DIR/dry-run.record"
mkdir -p "$OUT_DIR" "$SHOTS"
scratch_new

# Whether the menu captures (it is mapped) with the scrim and sheet drawn (most of the output
# covered), into $2 (default: the scratch capture).
shows() {
  local png="${2:-$OUT_DIR/menu.png}"
  sill debug capture "$MENU" "$png" >>"$1" 2>&1 || return 1
  [ "$(opaque_pixels "$png")" -gt 100000 ]
}

start_menu_daemon() {
  [ "$MODE" = keys ] && { backdrop_seed "$CONFIG" || return 1; }
  : >"$RECORD"
  launch_daemon --session-dry-run "$RECORD" || return 1
  NAME=$(output_name) || return 1
  MENU="sill-power-menu-$NAME"
  sleep 1
}

# Press the keys "$@" (`sill debug key` names: ArrowRight, Enter, Escape, ...) on the menu, one at
# a time, only while the menu is mapped and the dry run is proven.
type_keys() {
  local log="$1"; shift
  [ "$DRY_RUN" = proven ] || { echo "no keys without a proven dry run" | tee -a "$log" >&2; return 1; }
  for key in "$@"; do
    wait_surface "$MENU" 5 up || { echo "the menu is not up for '$key'" | tee -a "$log" >&2; return 1; }
    sill debug key "$MENU" "$key" >>"$log" 2>&1 || { echo "the daemon refused key '$key'" >&2; return 1; }
    echo "key $key" >>"$log"
    sleep 0.2
  done
}

# Open the menu and let the sheet settle; notes whether it holds the compositor's keyboard (the
# injected keys do not need it).
open_with_keyboard() {
  local log="$1" before
  before=$(count_log 'power_menu.keyboard=held')
  sill power-menu open >>"$log" 2>&1 || return 1
  wait_surface "$MENU" 30 up || return 1
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
  wait_surface "$MENU" 30 up || { echo "open: never mapped" >&2; return 1; }
  sleep 0.8
  shows "$log" "$SHOTS/power-menu.png" || { echo "open: the sheet did not draw" >&2; return 1; }
  sill power-menu close >>"$log" 2>&1 || return 1
  wait_surface "$MENU" 20 down || { echo "close: still mapped" >&2; return 1; }
  sill power-menu toggle >>"$log" 2>&1 || return 1
  wait_surface "$MENU" 20 up || { echo "toggle: never mapped" >&2; return 1; }
  sill power-menu toggle >>"$log" 2>&1 || return 1
  wait_surface "$MENU" 20 down || { echo "toggle: still mapped" >&2; return 1; }
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
  wait_surface "$MENU" 5 up || { echo "the menu is not open for Enter" >&2; return 1; }
  type_keys "$log" Enter || return 2
  for _ in $(seq 30); do grep -q 'would=Restart' "$RECORD" && break; sleep 0.1; done
  wait_surface "$MENU" 20 down || { echo "Enter: the menu stayed up" >&2; return 1; }
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
  wait_surface "$MENU" 20 down || { echo "Escape: the menu stayed up" >&2; return 1; }
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

# Real 1.
step_real_keyboard() {
  local before
  [ "$DRY_RUN" = proven ] || { echo "no dry run proven: the menu is not opened" >&2; return 2; }
  before=$(count_log 'power_menu.keyboard=held')
  sill power-menu open >/dev/null 2>&1 || return 1
  wait_surface "$MENU" 30 up || { echo "open: never mapped" >&2; return 1; }
  wait_log 'power_menu.keyboard=held' 30 1 "$before" || {
    echo "the menu never held the real keyboard" >&2; return 1; }
  sleep 0.6
  echo "the power menu opened and holds the real keyboard" >&2
}

# Real 2.
step_real_arrows() {
  local before
  [ "$DRY_RUN" = proven ] || return 2
  mapped "$MENU" || { echo "the menu is not up for the arrows" >&2; return 1; }
  before=$(count_log 'power_menu.focus=Suspend')
  key Right
  wait_log 'power_menu.focus=Suspend' 20 1 "$before" || { echo "Right: no focus on Suspend" >&2; return 1; }
  before=$(count_log 'power_menu.focus=Restart')
  key Left; key Left
  wait_log 'power_menu.focus=Restart' 20 1 "$before" || { echo "Left Left: no focus on Restart" >&2; return 1; }
  echo "real Right: Shut Down -> Suspend; real Left Left: -> Restart" >&2
}

# Real 3.
step_real_escape() {
  [ "$DRY_RUN" = proven ] || return 2
  mapped "$MENU" || { echo "the menu is not up for Escape" >&2; return 1; }
  key Escape
  wait_surface "$MENU" 20 down || { echo "Escape: the menu stayed up" >&2; return 1; }
  [ ! -s "$RECORD" ] || { echo "Escape recorded: $(cat "$RECORD")" >&2; return 1; }
  echo "real Escape closed the menu, recording nothing" >&2
}

# Real 4.
step_real_scrim_click() {
  [ "$DRY_RUN" = proven ] || return 2
  sill power-menu open >/dev/null 2>&1 || return 1
  wait_surface "$MENU" 30 up || { echo "open: never mapped" >&2; return 1; }
  sleep 0.6
  click 40 $(( ${SIZE#*x} - 40 ))
  wait_surface "$MENU" 20 down || { echo "a scrim click left the menu up" >&2; return 1; }
  [ ! -s "$RECORD" ] || { echo "the scrim click recorded: $(cat "$RECORD")" >&2; return 1; }
  echo "a real click on the scrim closed the menu, recording nothing" >&2
}

cleanup() {
  local result=0
  stop_input
  stop_daemon || result=1
  stop_kwin
  scratch_drop
  return "$result"
}

build || exit 1
start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; scratch_drop; exit 1; }
start_menu_daemon || { cleanup; exit 1; }
prove_dry_run power_menu || echo "the dry run could not be proven from the daemon's log" >&2
if [ "$MODE" = real ]; then
  start_input || echo "no fake input client" >&2
  run_steps step_real_keyboard step_real_arrows step_real_escape step_real_scrim_click
else
  run_steps step_warm step_open_close_toggle step_arrows step_enter_records step_escape step_idle
fi
status=$STATUS
sill power-menu close >/dev/null 2>&1
cleanup || status=1
echo "logs in $OUT_DIR; captures in $SHOTS"
exit "$status"

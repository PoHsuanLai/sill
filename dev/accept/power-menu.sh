#!/usr/bin/env bash
# Power menu acceptance (design/20 §1.8), on a private headless KWin (shell-host
# scripts/dev/kwin-virtual.sh). Starts `sill run` from this checkout's debug build (`--features
# debug`) with its own XDG_RUNTIME_DIR and scratch HOME and XDG dirs, on KWin's private bus (the
# daemon's session AND system bus; start_compositor refuses to go on if it would be the real one), and
# quits it at the end.
#
# SAFETY. A confirmed action would really log out, restart, shut down or suspend the machine if it
# reached logind. The daemon is started with `sill run --session-dry-run RECORD`: a debug build
# then takes every session action through a dry run that only prints and records the chosen one,
# never logind (crates/sill/src/run_args.rs, crates/sill-services/src/env/session_actions.rs).
# Before a button is ever pressed, the script proves the dry run is in force from the daemon's own
# log (the "dry run, recording to" line, and `power_menu.backend=dry-run` with no
# `power_menu.backend=live`); without that proof the pressing steps are skipped (exit 2), never run.
#
# KEYS. By default every key is `sill debug key SURFACE KEY`: shell-host's `Inject::Key`, which
# enters the power menu where a real xkb key does (the same machine, focus, modality and IME),
# whether or not the compositor gave the menu the keyboard. The keys are only sent while the dry
# run is proven and the menu is mapped. With --real-input every key and press is a real Wayland
# event through one fake-input client, with a real serial and the compositor's keyboard focus.
#
# Default steps:
#   1. Warm: `sill-power-menu-<out>` exists from the start, hidden (no capture).
#   2. `sill power-menu open` shows the alert on its sheet: a capture with the sheet's pixels,
#      into dev/out/power-menu.png. `close` hides it; `toggle` twice round-trips.
#   3. Tab moves the keyboard to the next button: the frame differs from the opening's by the
#      focus ring's move (dev/out/power-menu-tab.png).
#   4. Space presses the button the keyboard is on (Cancel, after step 3's Tab: the alert has the
#      opening's action and Cancel, as the Mac's): the menu closes and nothing is recorded. Opened
#      again, Return presses the default button (Shut Down): the dry run records exactly
#      `session.dry_run would=PowerOff`.
#   5. Escape closes it, recording nothing.
#   6. 5 s idle after it hides: 0 frames on the power menu surface.
#
# --real-input steps (Return and Space are never pressed and no button is clicked):
#   1. The power menu opens and holds the real keyboard (`power_menu.keyboard=held`).
#   2. A real Tab moves the keyboard to the next button: the frame differs from the opening's.
#   3. Real Escape closes it, recording nothing.
#   4. Opened again, a real click outside the alert closes it, recording nothing.
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
OUT_DIR="$SILL_OUT_ROOT/accept-power-menu"
SHOTS="$SILL_SHOTS"
RECORD="$OUT_DIR/dry-run.record"
mkdir -p "$OUT_DIR" "$SHOTS"
scratch_new

# Whether the menu captures (it is mapped) with the alert's sheet drawn (about 340 x 320 px of
# the output opaque, so at least half of that), into $2 (default: the scratch capture).
shows() {
  local png="${2:-$OUT_DIR/menu.png}"
  sill debug capture "$MENU" "$png" >>"$1" 2>&1 || return 1
  [ "$(opaque_pixels "$png")" -gt 50000 ]
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
  echo "open shows the alert on its sheet; close hides it; toggle twice round-trips" | tee -a "$log" >&2
}

# 3.
step_tab() {
  local log="$OUT_DIR/tab.log" status before
  : >"$log"
  open_with_keyboard "$log"; status=$?
  [ "$status" = 0 ] || return "$status"
  grep -q "power_menu.opened default=Shut Down" "$OUT_DIR/daemon.log" || {
    echo "the menu did not open on Shut Down (the default)" >&2; return 1; }
  grep 'power_menu.offers=' "$OUT_DIR/daemon.log" | tail -n 1 | tee -a "$log" >&2
  capture "$MENU" "$OUT_DIR/before-tab.png" || { echo "no capture before Tab" >&2; return 1; }
  type_keys "$log" Tab || return 2
  sleep 0.4
  sill debug capture "$MENU" "$SHOTS/power-menu-tab.png" >>"$log" 2>&1 || { echo "no capture after Tab" >&2; return 1; }
  before=$(changed_fraction "$OUT_DIR/before-tab.png" "$SHOTS/power-menu-tab.png")
  echo "Tab: $before of the frame changed" | tee -a "$log" >&2
  [ "$before" != 0.0000 ] || { echo "Tab: the keyboard did not move" >&2; return 1; }
}

# 4. Runs only on a proven dry run, on the menu step 3 left open with the keyboard on the
# second button (Cancel, the one after the default).
step_presses_record() {
  local log="$OUT_DIR/press.log"
  : >"$log"
  if [ "$DRY_RUN" != proven ]; then
    echo "the dry run is not proven: no button is pressed" | tee -a "$log" >&2
    sill power-menu close >/dev/null 2>&1
    return 2
  fi
  wait_surface "$MENU" 5 up || { echo "the menu is not open for Space" >&2; return 1; }
  : >"$RECORD"
  type_keys "$log" Space || return 2
  wait_surface "$MENU" 20 down || { echo "Space: the menu stayed up" >&2; return 1; }
  sleep 0.3
  [ ! -s "$RECORD" ] || { echo "Space on Cancel recorded: $(cat "$RECORD")" >&2; return 1; }
  echo "Space pressed Cancel: the menu closed, recording nothing" | tee -a "$log" >&2
  : >"$RECORD"
  open_with_keyboard "$log" || return 2
  type_keys "$log" Enter || return 2
  for _ in $(seq 30); do grep -q 'would=PowerOff' "$RECORD" && break; sleep 0.1; done
  wait_surface "$MENU" 20 down || { echo "Enter: the menu stayed up" >&2; return 1; }
  [ "$(cat "$RECORD")" = "session.dry_run would=PowerOff" ] || {
    echo "the record is not exactly one PowerOff: $(cat "$RECORD")" >&2; return 1; }
  echo "Return recorded 'would=PowerOff' (Shut Down, the default) and closed the menu" | tee -a "$log" >&2
  : >"$RECORD"
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
step_real_tab() {
  local before
  [ "$DRY_RUN" = proven ] || return 2
  mapped "$MENU" || { echo "the menu is not up for Tab" >&2; return 1; }
  capture "$MENU" "$OUT_DIR/before-tab.png" || return 1
  key Tab
  sleep 0.4
  capture "$MENU" "$OUT_DIR/after-tab.png" || return 1
  before=$(changed_fraction "$OUT_DIR/before-tab.png" "$OUT_DIR/after-tab.png")
  [ "$before" != 0.0000 ] || { echo "real Tab: the keyboard did not move" >&2; return 1; }
  echo "real Tab moved the keyboard to the next button ($before of the frame changed)" >&2
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
  wait_surface "$MENU" 20 down || { echo "a click outside the alert left the menu up" >&2; return 1; }
  [ ! -s "$RECORD" ] || { echo "the outside click recorded: $(cat "$RECORD")" >&2; return 1; }
  echo "a real click outside the alert closed the menu, recording nothing" >&2
}

cleanup() {
  local result=0
  stop_input
  stop_daemon || result=1
  stop_compositor
  scratch_drop
  return "$result"
}

build || exit 1
start_compositor --default kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; scratch_drop; exit 1; }
start_menu_daemon || { cleanup; exit 1; }
prove_dry_run power_menu || echo "the dry run could not be proven from the daemon's log" >&2
if [ "$MODE" = real ]; then
  start_input || echo "no fake input client" >&2
  run_steps step_real_keyboard step_real_tab step_real_escape step_real_scrim_click
else
  run_steps step_warm step_open_close_toggle step_tab step_presses_record step_escape step_idle
fi
status=$STATUS
sill power-menu close >/dev/null 2>&1
cleanup || status=1
echo "logs in $OUT_DIR; captures in $SHOTS"
exit "$status"

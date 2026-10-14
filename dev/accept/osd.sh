#!/usr/bin/env bash
# OSD acceptance (design/20 §1.7), on casement (brought up and ended by start_compositor). Starts `sill run` from this checkout's debug build
# (`--features debug`) with a scratch HOME and XDG tree, on a private bus that is both its session
# and its system bus, and quits it at the end.
#
# The changes are made up with `sill debug osd` (a fixed level, shown on the output a real change
# would use): the nested session shares the real PipeWire default sink, which may be somebody's
# headphones, and a brightness key there could reach a real monitor over DDC/CI, so nothing here
# moves a real level. `sill display volume|brightness` take the same path to the OSD after
# their service call (crates/sill/src/osd.rs, `after_display`).
#
#   1. Warm: the OSD surface exists from the start, hidden (no capture).
#   2. `sill debug osd volume 60` shows it: a capture with the card's pixels within 200 ms.
#      Screenshot dev/out/osd-casement.png while it holds.
#   3. It hides by itself about 1.5 s after the change: the capture fails between the hold
#      (osd.hold_ms, 1500) and the hold plus the fade (--t-move) plus 400 ms of slack.
#   4. A change during the fade takes it back: shown again at 1.6 s, it is still up at 2.4 s.
#   5. 5 s idle after it hides: 0 frames on the OSD surface.
#
# Then, not one of the four: `sill debug osd volume 60 --mute muted` for a second screenshot,
# dev/out/osd-casement-muted.png, of quire's `Osd` card muted (non-fatal: it does not flip the
# script's exit status).
#
# Usage: dev/accept/osd.sh
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_OUT_ROOT/accept-osd"
SHOTS="$SILL_SHOTS"
mkdir -p "$OUT_DIR" "$SHOTS"
HOLD_MS=1500

# Whether the OSD captures (it is mapped) with more than 1000 opaque pixels (the card drew).
shows() {
  sill debug capture "$OSD" "$OUT_DIR/osd.png" >>"$1" 2>&1 || return 1
  [ "$(opaque_pixels "$OUT_DIR/osd.png")" -gt 1000 ]
}

start_osd_daemon() {
  backdrop_seed "$CONFIG" || return 1
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  OSD="sill-osd-$NAME"
  sleep 1
}

# 1.
step_warm() {
  local log="$OUT_DIR/warm.log"
  sill debug stats >"$log" 2>&1
  grep -q "\"$OSD\"" "$log" || { echo "$OSD is not in 'sill debug stats'" >&2; return 1; }
  if sill debug capture "$OSD" "$OUT_DIR/hidden.png" >>"$log" 2>&1; then
    echo "the OSD is mapped before any change" >&2; return 1
  fi
  echo "$OSD spawned at start, hidden" | tee -a "$log" >&2
}

# 2. and 3.
step_shows_then_hides() {
  local log="$OUT_DIR/show.log" start shown=-1 gone=-1 t
  : >"$log"
  start=$(now_ms)
  sill debug osd volume 60 >>"$log" 2>&1 || return 1
  # Timed at the capture's return; the pixel count after it is not counted.
  for _ in $(seq 40); do
    if sill debug capture "$OSD" "$OUT_DIR/osd.png" >>"$log" 2>&1; then
      t=$(( $(now_ms) - start ))
      [ "$(opaque_pixels "$OUT_DIR/osd.png")" -gt 1000 ] && { shown=$t; break; }
    fi
    sleep 0.02
  done
  sleep 0.3
  shot "$SHOTS/osd-casement.png" >>"$log" 2>&1 || echo "screenshot failed" >&2
  cp "$OUT_DIR/osd.png" "$OUT_DIR/osd-shown.png" 2>/dev/null
  while [ $(( $(now_ms) - start )) -lt 4000 ]; do
    t=$(( $(now_ms) - start ))
    if ! sill debug capture "$OSD" "$OUT_DIR/osd-late.png" >>"$log" 2>&1; then gone=$t; break; fi
    sleep 0.05
  done
  local limit=$(( HOLD_MS + 250 + 400 ))
  echo "shown ${shown} ms after the change; hidden by ${gone} ms (hold ${HOLD_MS} ms, limit ${limit} ms)" \
    | tee -a "$log" >&2
  # Shown by 200 ms is a latency budget (release builds only; the time includes the capture's own
  # round trip, so it also tracks host load); the hold and the hide are functional.
  [ "$shown" -ge 0 ] && budget_check "OSD shown ${shown} ms after the change (less the capture's own $(capture_cost_ms) ms) against 200 ms" "$(( $(captured_ms "$shown") <= 200 ))" \
    && [ "$gone" -ge "$HOLD_MS" ] && [ "$gone" -le "$limit" ]
}

# 4.
step_retrigger() {
  local log="$OUT_DIR/retrigger.log" start up=no
  : >"$log"
  sleep 0.5
  start=$(now_ms)
  sill debug osd brightness 40 >>"$log" 2>&1 || return 1
  while [ $(( $(now_ms) - start )) -lt $(( HOLD_MS + 100 )) ]; do sleep 0.02; done
  sill debug osd brightness 45 >>"$log" 2>&1 || return 1
  while [ $(( $(now_ms) - start )) -lt $(( HOLD_MS + 900 )) ]; do sleep 0.02; done
  shows "$log" && up=yes
  echo "a change 100 ms into the fade: still up $(( $(now_ms) - start )) ms after the first: $up" \
    | tee -a "$log" >&2
  [ "$up" = yes ] || return 1
  for _ in $(seq 60); do sill debug capture "$OSD" "$OUT_DIR/osd-late.png" >>"$log" 2>&1 || return 0; sleep 0.05; done
  echo "it never hid after the second change" >&2; return 1
}

# 5.
step_idle() {
  local log="$OUT_DIR/idle.log" f0 f1
  sleep 1
  f0=$(frames_of "$OSD")
  sleep 5
  f1=$(frames_of "$OSD")
  echo "5 s idle after the hide: $OSD $(( f1 - f0 )) frames" | tee "$log" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

# Not one of the four: a second screenshot, muted, from the same fake-change debug path
# (`sill debug osd volume N --mute muted`), so both looks are on file. Non-fatal.
step_muted_shot() {
  local log="$OUT_DIR/muted.log"
  : >"$log"
  sill debug osd volume 60 --mute muted >>"$log" 2>&1 || { echo "muted: sill debug osd failed" >&2; return 1; }
  for _ in $(seq 40); do
    shows "$log" && break
    sleep 0.02
  done
  sleep 0.3
  shot "$SHOTS/osd-casement-muted.png" >>"$log" 2>&1 || { echo "muted screenshot failed" >&2; return 1; }
}

cleanup() {
  local result=0
  stop_daemon || result=1
  stop_private_bus
  stop_compositor
  scratch_drop
  return "$result"
}

start_compositor || exit 1
nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (start_compositor failed)" >&2; exit 1; }
build || exit 1
await_no_process || exit 1
scratch_new
start_private_bus || { scratch_drop; exit 1; }
start_osd_daemon || { cleanup; exit 1; }
run_steps step_warm step_shows_then_hides step_retrigger step_idle
status=$STATUS
if step_muted_shot; then echo "PASS step_muted_shot"; else echo "FAIL step_muted_shot (non-fatal)"; fi
cleanup || status=1
echo "logs in $OUT_DIR; screenshots $SHOTS/osd-casement.png, $SHOTS/osd-casement-muted.png"
exit "$status"

#!/usr/bin/env bash
# Screenshot thumbnail acceptance (design/20 §1.13), on the nested cosmic-comp (dev/nested.sh; a
# running wayland-1 is reused and never stopped), and with --real-input the drag of the thumbnail
# out (shell-host's `SurfaceHandle::start_drag`) on shell-host's private headless KWin
# (scripts/dev/kwin-virtual.sh) driven by ONE fake-input client (scripts/dev/fake-input.sh).
#
# Starts `sill run` from this checkout's debug build (`--features debug`) on a private bus
# (dev/private-bus.conf) as both its session and its system bus, with scratch HOME and XDG
# directories, a fake DDC fixture and its own XDG_RUNTIME_DIR (the compositor's socket as an
# absolute WAYLAND_DISPLAY). Never the user's session (wayland-0), never ~/Pictures or the real
# trash: the scratch config's `screenshot.folders` is a temp folder, and `screenshot.tool` a
# stand-in that copies the services' fixture PNG there (never cosmic-screenshot), or "off" for
# the drag; `xdg-open` and `gio` are stand-ins first on the daemon's PATH that log their
# arguments; the output shot is grim on the nested socket only (no fallback).
#
#   1. `sill-screenshot-thumb-<out>` exists from the start, hidden (no capture).
#   2. `sill debug screenshot <temp>/Screenshot_a.png` shows it within 500 ms; screenshot
#      dev/out/screenshot-thumb-cosmic.png while it holds.
#   3. It hides after the hold (screenshot.thumbnail_hold_ms, 5200) plus the exit, and the file
#      is still there.
#   4. `sill screenshot screen` runs the stand-in, whose file the watch shows.
#   5. `sill debug click` on the card runs the opener with that file.
#   6. A new shot hovered at 4 s (`sill debug pointer`) is still up at 7 s, and gone 1.5 s plus
#      the exit after `sill debug leave`.
#   7. 5 s while hidden: 0 frames on the thumbnail surface.
#
# --real-input steps (`dock.position = "left"`, so nothing reserves the bottom edge and the
# thumbnail sits at the output's true bottom-right corner):
#   1. `sill debug screenshot <temp>/Screenshot_drag.png` (a copy of the services' fixture): the
#      thumbnail shows.
#   2. A real press on the card's picture, a first step 12 px left (past quire's 8 px drag
#      threshold, and not rightwards, which is the swipe's), a glide up-left over the bare
#      desktop and a release there. With no foot terminal installed there is no drop target, so
#      shell-host answers `Cancelled`: the daemon logs `screenshot drag: Cancelled`, the card is
#      still up 600 ms later and the file is still there.
#   3. The hold resumes (the banners' 1500 ms rest): the thumbnail unmaps within the rest plus
#      the exit and slack, and the file is kept.
#
# Exit 0 only when every step passed; 1 on a failure. On the nested compositor another sill
# daemon started from a `target/*/sill run` is waited for (up to 10 minutes).
#
# Usage: dev/accept/screenshot.sh [--real-input]
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
case "${1:-}" in
  --real-input) MODE=real; OUT_DIR="$SILL_REPO/target/accept-screenshot-drag" ;;
  "") MODE=nested; OUT_DIR="$SILL_REPO/target/accept-screenshot" ;;
  *) echo "usage: dev/accept/screenshot.sh [--real-input]" >&2; exit 2 ;;
esac
SHOTS="$SILL_REPO/dev/out"
FIXTURE="$SILL_REPO/crates/sill-services/tests/fixtures/shots/Screenshot_fixture.png"
SIZE=1920x1080
WIDTH=${SIZE%x*}
HEIGHT=${SIZE#*x}
HOLD_MS=5200
REST_MS=1500
LIMIT_MS=3200
mkdir -p "$OUT_DIR" "$SHOTS"
scratch_new
WATCHED="$SCRATCH/shots"
ELSEWHERE="$SCRATCH/elsewhere"
STUBS="$SCRATCH/stubs"
OPENED="$OUT_DIR/opened.log"
TRASHED="$OUT_DIR/trashed.log"
mkdir -p "$CONFIG/sill" "$WATCHED" "$ELSEWHERE" "$STUBS"
KWIN_UP=no
BOX=
RELEASED=0

# Whether the thumbnail is mapped with its card drawn.
shows() {
  mapped "$THUMB" || return 1
  [ "$(opaque_pixels "$OUT_DIR/probe.png")" -gt 5000 ]
}

# ms since $1 until the thumbnail shows, or -1 within $2 ms.
until_shown() {
  local start=$1 limit=$2 t
  while :; do
    t=$(( $(now_ms) - start ))
    [ "$t" -gt "$limit" ] && { echo -1; return; }
    shows && { echo "$t"; return; }
    sleep 0.02
  done
}

# ms since $1 until the thumbnail is unmapped, or -1 within $2 ms.
until_gone() {
  local start=$1 limit=$2 t
  while :; do
    t=$(( $(now_ms) - start ))
    [ "$t" -gt "$limit" ] && { echo -1; return; }
    mapped "$THUMB" || { echo "$t"; return; }
    sleep 0.05
  done
}

# The nested output only: grim on the nested socket, never a host screenshot.
nested_shot() {
  WAYLAND_DISPLAY="$WL" grim "$1"
}

stubs() {
  printf '#!/bin/sh\necho "$@" >>"%s"\n' "$OPENED" >"$STUBS/xdg-open"
  printf '#!/bin/sh\necho "$@" >>"%s"\n' "$TRASHED" >"$STUBS/gio"
  chmod +x "$STUBS/xdg-open" "$STUBS/gio"
  : >"$OPENED"
  : >"$TRASHED"
}

# Starts the daemon with the stand-in tool, the stubs first on its PATH and a backdrop wallpaper.
start_nested_daemon() {
  cat >"$CONFIG/sill/settings.toml" <<TOML
[screenshot]
folders = "$WATCHED"
tool = 'cp "$FIXTURE" "\$2/Screenshot_\$1_\$(date +%s%N).png"'
TOML
  backdrop_seed "$CONFIG" || return 1
  IN_ENV_EXTRA=(PATH="$STUBS:$PATH")
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  THUMB="sill-screenshot-thumb-$NAME"
  sleep 1.5
  echo "daemon on a private bus and runtime dir $RUNTIME; watching $WATCHED; $THUMB" >&2
}

# Starts the daemon on the headless KWin with the tool off and the dock on the left.
start_real_daemon() {
  cat >"$CONFIG/sill/settings.toml" <<TOML
[dock]
position = "left"
[screenshot]
folders = "$WATCHED"
tool = "off"
TOML
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  THUMB="sill-screenshot-thumb-$NAME"
  sleep 1.5
}

# The card's centre in the surface: the surface's own size less the room around the card.
card_centre() {
  sill debug capture "$THUMB" "$OUT_DIR/card.png" >/dev/null 2>&1 || return 1
  python3 - "$OUT_DIR/card.png" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
box = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
print((box[0] + box[2]) // 2, (box[1] + box[3]) // 2) if box else sys.exit(1)
PY
}

# The card's bounding box and the surface's size in a capture: "W H LEFT TOP RIGHT BOTTOM".
card_geometry() {
  sill debug capture "$THUMB" "$OUT_DIR/card.png" >/dev/null 2>&1 || return 1
  python3 - "$OUT_DIR/card.png" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
box = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
print(img.width, img.height, *box) if box else sys.exit(1)
PY
}

# 1.
step_warm() {
  local log="$OUT_DIR/warm.log"
  sill debug stats >"$log" 2>&1
  grep -q "\"$THUMB\"" "$log" || { echo "$THUMB is not in 'sill debug stats'" >&2; return 1; }
  if mapped "$THUMB"; then echo "the thumbnail is mapped before any screenshot" >&2; return 1; fi
  echo "$THUMB spawned at start, hidden" | tee -a "$log" >&2
}

# 2. and 3.
step_debug_shot_shows_then_hides() {
  local log="$OUT_DIR/debug-shot.log" start shown gone limit
  : >"$log"
  cp "$FIXTURE" "$ELSEWHERE/Screenshot_a.png"
  start=$(now_ms)
  sill debug screenshot "$ELSEWHERE/Screenshot_a.png" >>"$log" 2>&1 || { cat "$log" >&2; return 1; }
  shown=$(until_shown "$start" 1500)
  sleep 0.5
  nested_shot "$SHOTS/screenshot-thumb-cosmic.png" >>"$log" 2>&1 || echo "screenshot failed" >&2
  cp "$OUT_DIR/probe.png" "$OUT_DIR/thumb-shown.png" 2>/dev/null
  gone=$(until_gone "$start" 9000)
  limit=$(( HOLD_MS + 800 + 400 ))
  echo "shown ${shown} ms after 'sill debug screenshot'; hidden at ${gone} ms (hold ${HOLD_MS} ms, limit ${limit} ms); file kept: $(test -f "$ELSEWHERE/Screenshot_a.png" && echo yes || echo no)" \
    | tee -a "$log" >&2
  [ "$shown" -ge 0 ] && [ "$shown" -le 500 ] && [ "$gone" -ge "$HOLD_MS" ] && [ "$gone" -le "$limit" ] \
    && [ -f "$ELSEWHERE/Screenshot_a.png" ]
}

# 4. and 5.
step_tool_then_click() {
  local log="$OUT_DIR/tool.log" start shown at file
  : >"$log"
  start=$(now_ms)
  sill screenshot screen >>"$log" 2>&1 || { cat "$log" >&2; return 1; }
  shown=$(until_shown "$start" 3000)
  file=$(ls "$WATCHED"/Screenshot_screen_*.png 2>/dev/null | head -n 1)
  echo "the stand-in saved ${file:-nothing}; shown ${shown} ms after 'sill screenshot screen'" | tee -a "$log" >&2
  [ "$shown" -ge 0 ] && [ -n "$file" ] || return 1
  sleep 0.5
  at=$(card_centre) || { echo "no card to click" >&2; return 1; }
  # shellcheck disable=SC2086
  sill debug click "$THUMB" $at >>"$log" 2>&1
  for _ in $(seq 40); do grep -q . "$OPENED" && break; sleep 0.05; done
  gone=$(until_gone "$(now_ms)" 2000)
  echo "a click at $at ran the opener with: $(cat "$OPENED"); unmapped ${gone} ms later" | tee -a "$log" >&2
  grep -qx "$file" "$OPENED" && [ "$gone" -ge 0 ]
}

# 6.
step_hover_pauses() {
  local log="$OUT_DIR/hover.log" start at up7 left gone limit
  : >"$log"
  sill debug leave "$THUMB" >>"$log" 2>&1
  cp "$FIXTURE" "$ELSEWHERE/Screenshot_b.png"
  start=$(now_ms)
  sill debug screenshot "$ELSEWHERE/Screenshot_b.png" >>"$log" 2>&1 || return 1
  [ "$(until_shown "$start" 1500)" -ge 0 ] || { echo "hover: never shown" >&2; return 1; }
  while [ $(( $(now_ms) - start )) -lt 4000 ]; do sleep 0.05; done
  at=$(card_centre) || return 1
  # shellcheck disable=SC2086
  sill debug pointer "$THUMB" $at >>"$log" 2>&1
  while [ $(( $(now_ms) - start )) -lt 7000 ]; do sleep 0.05; done
  up7=no; shows && up7=yes
  left=$(now_ms)
  sill debug leave "$THUMB" >>"$log" 2>&1
  gone=$(until_gone "$left" 4000)
  limit=$(( REST_MS + 800 + 400 ))
  echo "hovered at 4 s: up at 7 s: $up7; hidden ${gone} ms after the leave (rest ${REST_MS} ms, limit ${limit} ms)" \
    | tee -a "$log" >&2
  [ "$up7" = yes ] && [ "$gone" -ge "$REST_MS" ] && [ "$gone" -le "$limit" ]
}

# 7.
step_idle() {
  local log="$OUT_DIR/idle.log" f0 f1
  sleep 1
  f0=$(frames_of "$THUMB")
  sleep 5
  f1=$(frames_of "$THUMB")
  echo "5 s hidden: $(( f1 - f0 )) frames on $THUMB" | tee "$log" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

# Real 1.
step_real_shows() {
  cp "$FIXTURE" "$WATCHED/Screenshot_drag.png"
  sill debug screenshot "$WATCHED/Screenshot_drag.png" >"$OUT_DIR/shot.log" 2>&1
  BOX=
  for _ in $(seq 40); do BOX=$(card_geometry) && break; sleep 0.1; done
  sleep 0.6
  BOX=$(card_geometry)
  echo "the card in $THUMB: ${BOX:-none} (surface W H, card box)" >&2
  [ -n "$BOX" ]
}

# Real 2. The surface is flush with the output's bottom-right corner (dock on the left).
step_real_drag_cancelled_over_nothing() {
  local SW SH L T R B X Y mark steps i outcome up kept
  [ -n "$BOX" ] || { echo "no card box from the first step" >&2; return 1; }
  read -r SW SH L T R B <<<"$BOX"
  X=$(( WIDTH - SW + (L + R) / 2 ))
  Y=$(( HEIGHT - SH + (T + B) / 2 ))
  mark=$(wc -l <"$OUT_DIR/daemon.log")
  steps=("move $X $Y" "sleep 300" "press left" "sleep 150" "move $(( X - 12 )) $Y" "sleep 250")
  for i in $(seq 1 10); do
    steps+=("move $(( X - 12 - (X - 12 - 700) * i / 10 )) $(( Y - (Y - 400) * i / 10 ))" "sleep 60")
  done
  steps+=("sleep 300" "release left" "sleep 600")
  input "${steps[@]}" || return 1
  RELEASED=$(now_ms)
  outcome=
  for _ in $(seq 30); do
    outcome=$(tail -n +$(( mark + 1 )) "$OUT_DIR/daemon.log" | sed -n 's/^sill: screenshot drag: //p' | head -n 1)
    [ -n "$outcome" ] && break
    sleep 0.1
  done
  up=0; mapped "$THUMB" && up=1
  kept=0; [ -f "$WATCHED/Screenshot_drag.png" ] && kept=1
  echo "pressed at $X $Y, released at 700 400: outcome ${outcome:-none}; card up after: $up; file there: $kept" >&2
  [ "$outcome" = "Cancelled" ] && [ "$up" = 1 ] && [ "$kept" = 1 ]
}

# Real 3.
step_real_hold_resumes() {
  local gone=-1 kept=0
  while [ $(( $(now_ms) - RELEASED )) -le "$LIMIT_MS" ]; do
    mapped "$THUMB" || { gone=$(( $(now_ms) - RELEASED )); break; }
    sleep 0.05
  done
  [ -f "$WATCHED/Screenshot_drag.png" ] && kept=1
  echo "unmapped ${gone} ms after the release (rest $REST_MS, limit $LIMIT_MS); file kept: $kept" >&2
  [ "$gone" -ge 0 ] && [ "$gone" -ge $(( REST_MS - 400 )) ] && [ "$kept" = 1 ]
}

# Ends everything the run started; safe to call twice.
cleanup() {
  local result=0
  stop_input
  stop_daemon || result=1
  if [ "$KWIN_UP" = yes ]; then stop_kwin; KWIN_UP=no; fi
  stop_private_bus
  scratch_drop
  return "$result"
}

# The daemon on the nested compositor, on its own private bus, the stand-ins in place.
setup_nested() {
  use_nested_socket || return 1
  command -v grim >/dev/null 2>&1 || { echo "grim is needed for the nested screenshot" >&2; return 1; }
  build || return 1
  await_no_process 'target/[a-z]*/sill run' || echo "going on with it still running" >&2
  stubs
  start_private_bus || return 1
  start_nested_daemon
}

# The headless KWin, ONE fake-input client and the daemon; the pointer rests on the desktop.
setup_real() {
  build || return 1
  if ! start_kwin; then
    echo "KWin did not start (see $OUT_DIR/kwin.log)" >&2
    return 1
  fi
  KWIN_UP=yes
  start_input || return 1
  start_real_daemon || return 1
  input "move 900 500" "sleep 200"
}

main() {
  local status
  trap cleanup EXIT
  if [ "$MODE" = real ]; then setup_real || exit 1; else setup_nested || exit 1; fi
  if [ "$MODE" = real ]; then
    run_steps step_real_shows step_real_drag_cancelled_over_nothing step_real_hold_resumes
  else
    run_steps step_warm step_debug_shot_shows_then_hides step_tool_then_click step_hover_pauses \
      step_idle
  fi
  status=$STATUS
  cleanup || status=1
  trap - EXIT
  echo "logs in $OUT_DIR"
  [ "$MODE" = real ] || echo "screenshot $SHOTS/screenshot-thumb-cosmic.png"
  exit "$status"
}

main

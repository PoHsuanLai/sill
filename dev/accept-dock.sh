#!/usr/bin/env bash
# Milestone 3 acceptance for the dock, on the nested cosmic-comp (dev/nested.sh; a running
# wayland-1 is reused and never stopped). Starts `sill run` from this checkout's debug build
# (`--features debug`) with a scratch XDG_CONFIG_HOME, so the pins it writes never touch the
# real ~/.config/sill, and quits it at the end.
#
#   1. The dock reserves base + margin: a maximised probe is output_h - bar_h - 72.
#   2. Hover sweep at 60 Hz for 1 s through `sill debug pointer`: frames >= 0.9 x callbacks;
#      then, after `sill debug leave` and the 200 ms shrink, 5 s idle: 0 frames.
#   3. A click on the pinned, not-running COSMIC Terminal tile launches it: its running dot
#      (the dock's view of a mapped toplevel) shows within 3 s.
#   4. `badge_emitter` for that app shows a count badge within 1 s.
#   5. `sill debug click --button secondary` on that tile opens its context menu popup.
# Screenshots: dev/out/dock-cosmic.png (rest), dock-cosmic-magnified.png, dock-cosmic-menu.png.
#
# Usage: dev/nested.sh && dev/accept-dock.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-dock"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
HOST_REPO="$(cd "$SHELL_HOST_DEV/../.." && pwd)"
PROBE="$HOST_REPO/target/release/examples/probe_toplevel"
EMITTER="$SILL_REPO/target/debug/examples/badge_emitter"
BAR_H="${BAR_H:-32}"
RESERVE=72
TERM_ID=com.system76.CosmicTerm
# Surface-local geometry of the default dock (design/10 §10.3.1): 160 tall, the pill's middle
# at 120, the baseline at 144, the running dot's centre 4 below it.
PILL_Y=120
DOT_Y=148

sill() { "$SILL_BIN" "$@"; }
nested() { "$SHELL_HOST_DEV/run.sh" "$@"; }

# The nested output's logical size, "W H".
output_size() {
  nested wayland-info | sed -n 's/.*logical_width: \([0-9]*\), logical_height: \([0-9]*\).*/\1 \2/p' | head -1
}

# The daemon's dock surface: the first `sill-dock-*` row of `sill debug stats`.
dock_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"].startswith("sill-dock-"):
        print(row["surface"]); break'
}

# The dock's cumulative `frames callbacks` (`debug stats` drains the timing samples; the
# counters stay cumulative, shell-host F29).
dock_counters() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"], row["callbacks"])' "$DOCK"
}

# Kill every COSMIC Terminal running in the nested session (never the host's).
kill_nested_terms() {
  local pid
  for pid in $(pgrep -x cosmic-term); do
    if tr '\0' '\n' <"/proc/$pid/environ" 2>/dev/null | grep -qx "WAYLAND_DISPLAY=$WAYLAND"; then
      kill "$pid" 2>/dev/null
    fi
  done
}

# The rest centre x of the first tile when the dock holds it and the Trash (dock/frame.rs:
# rest_centers(2) re-centred around the 13 px separator block).
first_tile_x() {
  echo $(( (OUT_W - 116) / 2 + 32 - 6 ))
}

# Capture the dock surface to $1 and print the RGBA at ($2, $3), surface-local logical px.
pixel() {
  sill debug capture "$DOCK" "$1" >/dev/null 2>&1 || return 1
  python3 - "$1" "$2" "$3" "$OUT_W" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
scale = img.width / int(sys.argv[4])
x, y = int(float(sys.argv[2]) * scale), int(float(sys.argv[3]) * scale)
print(*img.getpixel((x, y)))
EOF
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug \
    && cargo build -p sill-services --example badge_emitter) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
  [ -x "$PROBE" ] || (cd "$HOST_REPO" && cargo build --release -p shell-host --features debug-inject \
    --example probe_toplevel >>"$OUT_DIR/build.log" 2>&1)
}

start_daemon() {
  CONFIG=$(mktemp -d)
  XDG_CONFIG_HOME="$CONFIG" nested "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  sill dock pin "$TERM_ID" >"$OUT_DIR/pin.log" 2>&1 || return 1
  for _ in $(seq 50); do DOCK=$(dock_name); [ -n "$DOCK" ] && break; sleep 0.1; done
  [ -n "${DOCK:-}" ] || { echo "no dock surface in 'sill debug stats'" >&2; return 1; }
  sleep 1
}

# 1.
step_exclusive_zone() {
  local probe_log="$OUT_DIR/probe.log" height
  sleep 20 | nested "$PROBE" >"$probe_log" 2>&1 &
  local probe=$!
  for _ in $(seq 100); do grep -q '"maximized":true' "$probe_log" && break; sleep 0.1; done
  kill "$probe" 2>/dev/null; wait "$probe" 2>/dev/null
  height=$(grep '"maximized":true' "$probe_log" | tail -1 | python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["height"])')
  echo "output ${OUT_H}, maximised window ${height}, bar ${BAR_H}, dock ${RESERVE}" | tee "$OUT_DIR/exclusive-zone.log" >&2
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/dock-cosmic.png" >/dev/null 2>&1 || true
  [ "$height" = "$(( OUT_H - BAR_H - RESERVE ))" ]
}

# 2.
step_sweep_and_idle() {
  local log="$OUT_DIR/sweep.log" f0 c0 f1 c1 f2 f3 frames callbacks idle
  read -r f0 c0 < <(dock_counters)
  python3 - "$SILL_BIN" "$DOCK" "$OUT_W" "$PILL_Y" <<'EOF' >"$log" 2>&1
import subprocess, sys, time
sill, dock, width, y = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
start, span, hz = width // 2 - 160, 320, 60
t0 = time.monotonic()
for i in range(hz):
    x = start + span * i // (hz - 1)
    subprocess.run([sill, "debug", "pointer", dock, str(x), y], check=True, capture_output=True)
    wait = t0 + (i + 1) / hz - time.monotonic()
    if wait > 0:
        time.sleep(wait)
print(f"sweep: {hz} moves in {time.monotonic() - t0:.3f}s")
EOF
  sleep 0.05
  sill debug pointer "$DOCK" $(( OUT_W / 2 )) "$PILL_Y" >/dev/null
  sleep 0.3
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/dock-cosmic-magnified.png" >/dev/null 2>&1 || true
  read -r f1 c1 < <(dock_counters)
  frames=$(( f1 - f0 )); callbacks=$(( c1 - c0 ))
  echo "sweep: frames $frames callbacks $callbacks" | tee -a "$log" >&2
  sill debug leave "$DOCK" >/dev/null
  sleep 1
  read -r f2 _ < <(dock_counters)
  echo "the leave's shrink: frames $(( f2 - f1 ))" | tee -a "$log" >&2
  sleep 5
  read -r f3 _ < <(dock_counters)
  idle=$(( f3 - f2 ))
  echo "idle 5 s after the leave settled: frames $idle" | tee -a "$log" >&2
  [ "$frames" -gt 0 ] && [ $(( frames * 10 )) -ge $(( callbacks * 9 )) ] && [ "$idle" = 0 ]
}

# 3.
step_launch() {
  local log="$OUT_DIR/launch.log" x before after start
  kill_nested_terms
  sleep 1
  x=$(first_tile_x)
  before=$(pixel "$OUT_DIR/launch-before.png" "$x" "$DOT_Y")
  echo "click the COSMIC Terminal tile at $x,$PILL_Y; dot pixel before: $before" >"$log"
  start=$(date +%s%N)
  sill debug click "$DOCK" "$x" "$PILL_Y" >>"$log" 2>&1 || return 1
  sill debug leave "$DOCK" >/dev/null
  local took found=no
  for _ in $(seq 60); do
    after=$(pixel "$OUT_DIR/launch-after.png" "$x" "$DOT_Y")
    dark $after && { found=yes; break; }
    sleep 0.05
  done
  took=$(( ($(date +%s%N) - start) / 1000000 ))
  echo "dot pixel after ${took} ms: $after (dot: $found)" | tee -a "$log" >&2
  # Let the launch bounce land before the badge step looks at the tile.
  sleep 1
  "$SHELL_HOST_DEV/shot.sh" "$OUT_DIR/launched.png" >/dev/null 2>&1 || true
  [ "$found" = yes ] && [ "$took" -le 3000 ]
}

# Whether an RGBA pixel is the running dot's ink (dark on the light pill).
dark() { [ $(( $1 + $2 + $3 )) -lt 330 ] && [ "$4" -gt 128 ]; }
# Whether an RGBA pixel is the badge's red.
red() { [ "$1" -gt 150 ] && [ "$2" -lt 110 ] && [ "$4" -gt 128 ]; }

# 4.
step_badge() {
  local log="$OUT_DIR/badge.log" x y before after start
  # Inside the badge's right padding, clear of its digit: the capsule's right edge is 2 px
  # right of the plate's (80.5 % of a 48 px tile), its middle 7 px below the plate's top.
  x=$(( $(first_tile_x) + 19 )); y=$(( 144 - 48 + 5 + 6 ))
  before=$(pixel "$OUT_DIR/badge-before.png" "$x" "$y")
  start=$(date +%s%N)
  "$EMITTER" "$TERM_ID" --count 5 >"$OUT_DIR/emitter.log" 2>&1 &
  local emitter=$!
  local found=no
  for _ in $(seq 40); do
    after=$(pixel "$OUT_DIR/badge-after.png" "$x" "$y")
    red $after && { found=yes; break; }
    sleep 0.025
  done
  local took=$(( ($(date +%s%N) - start) / 1000000 ))
  kill "$emitter" 2>/dev/null; wait "$emitter" 2>/dev/null
  echo "badge pixel at $x,$y: $before -> $after after ${took} ms (red: $found)" | tee "$log" >&2
  [ "$found" = yes ] && [ "$took" -le 1000 ]
}

# 5.
step_menu() {
  local log="$OUT_DIR/menu.log" x opened=no
  x=$(first_tile_x)
  sill debug click "$DOCK" "$x" "$PILL_Y" --button secondary >"$log" 2>&1 || return 1
  for _ in $(seq 40); do
    sill debug capture "$DOCK/popup" "$OUT_DIR/menu-popup.png" >>"$log" 2>&1 && { opened=yes; break; }
    sleep 0.05
  done
  sleep 0.5
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/dock-cosmic-menu.png" >/dev/null 2>&1 || true
  echo "popup $DOCK/popup opened: $opened" | tee -a "$log" >&2
  [ "$opened" = yes ]
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  kill_nested_terms
  rm -rf "$CONFIG"
}

source "$SHELL_HOST_DEV/.nested-env"
WAYLAND="$WAYLAND_DISPLAY"
build || exit 1
read -r OUT_W OUT_H < <(output_size)
[ -n "${OUT_W:-}" ] || { echo "no nested output (dev/nested.sh first)" >&2; exit 1; }
if sill ping >/dev/null 2>&1; then
  echo "a sill daemon is already running; quit it first (this script starts its own)" >&2
  exit 1
fi
start_daemon || { stop_daemon; exit 1; }
status=0
for step in step_exclusive_zone step_sweep_and_idle step_launch step_badge step_menu; do
  if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; fi
done
stop_daemon
echo "logs in $OUT_DIR; screenshots in $SHOTS"
exit "$status"

#!/usr/bin/env bash
# Milestone 4 launcher-ui acceptance, on the nested cosmic-comp (dev/nested.sh; a running
# wayland-1 is reused and never stopped). Starts `sill run` from this checkout's debug build
# (`--features debug`) with a scratch XDG_CONFIG_HOME and quits it at the end.
#
#   1. Warm: both launcher surfaces exist from the start, hidden (no capture of either).
#   2. `sill launcher toggle` shows the panel: a capture of `sill-launcher-<output>` within
#      100 ms of the toggle has the panel's pixels; the daemon logs `open_latency_ms`.
#   3. `sill launcher toggle` again hides it (no capture), and a click on the catcher closes a
#      reopened one.
#   4. 5 s idle after the hide: 0 frames on the panel and the catcher.
#   5. On `examples/launcher_demo.rs` (fake results; shell-host injects no keys, so the demo
#      types through its stdin): `open`, `type fi` shows rows; `key actions` opens the
#      selected row's actions menu; `key escape` twice closes the menu, then the launcher.
# Screenshots: dev/out/launcher-cosmic.png (open over "fi"), launcher-cosmic-actions.png.
#
# Usage: dev/nested.sh && dev/accept-launcher-ui.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-launcher-ui"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
DEMO="$SILL_REPO/target/debug/examples/launcher_demo"

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
print(sum(1 for p in img.getdata() if p[3] > 0))
EOF
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug \
    && cargo build -p sill-surfaces --example launcher_demo) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

start_daemon() {
  CONFIG=$(mktemp -d)
  XDG_CONFIG_HOME="$CONFIG" nested "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  PANEL="sill-launcher-$NAME"; CATCHER="sill-launcher-catcher-$NAME"
  sleep 1
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  rm -rf "$CONFIG"
}

# 1.
step_warm() {
  local log="$OUT_DIR/warm.log"
  sill debug stats >"$log" 2>&1
  grep -q "\"$PANEL\"" "$log" && grep -q "\"$CATCHER\"" "$log" || {
    echo "the launcher surfaces are not in 'sill debug stats'" >&2; return 1; }
  if sill debug capture "$PANEL" "$OUT_DIR/hidden.png" >>"$log" 2>&1; then
    echo "the panel is mapped before any toggle" >&2; return 1
  fi
  echo "both launcher surfaces spawned at start, hidden" | tee -a "$log" >&2
}

# 2.
step_toggle_shows() {
  local log="$OUT_DIR/toggle.log" start took shown=no px=0
  start=$(date +%s%N)
  sill launcher toggle >"$log" 2>&1 || return 1
  for _ in $(seq 200); do
    if sill debug capture "$PANEL" "$OUT_DIR/open.png" >>"$log" 2>&1; then
      px=$(opaque_pixels "$OUT_DIR/open.png")
      [ "$px" -gt 1000 ] && { shown=yes; break; }
    fi
  done
  took=$(( ($(date +%s%N) - start) / 1000000 ))
  sleep 0.6
  local latency
  latency=$(grep -o 'open_latency_ms=[0-9]*' "$OUT_DIR/daemon.log" | tail -1)
  echo "toggle -> panel capture with $px opaque px after ${took} ms (shown: $shown); daemon $latency" \
    | tee -a "$log" >&2
  "$SHELL_HOST_DEV/shot.sh" "$OUT_DIR/daemon-open.png" >/dev/null 2>&1 || true
  [ "$shown" = yes ] && [ "$took" -le 100 ]
}

# 3.
step_hide() {
  local log="$OUT_DIR/hide.log" hidden=no closed=no
  sill launcher toggle >"$log" 2>&1 || return 1
  for _ in $(seq 40); do
    sill debug capture "$PANEL" "$OUT_DIR/after-hide.png" >>"$log" 2>&1 || { hidden=yes; break; }
    sleep 0.025
  done
  sill launcher toggle >>"$log" 2>&1
  sleep 0.5
  sill debug click "$CATCHER" 20 "$(( OUT_H - 20 ))" >>"$log" 2>&1
  for _ in $(seq 40); do
    sill debug capture "$PANEL" "$OUT_DIR/after-catch.png" >>"$log" 2>&1 || { closed=yes; break; }
    sleep 0.025
  done
  echo "toggle hides: $hidden; a catcher click closes: $closed" | tee -a "$log" >&2
  [ "$hidden" = yes ] && [ "$closed" = yes ]
}

# 4.
step_idle() {
  local log="$OUT_DIR/idle.log" p0 c0 p1 c1
  sleep 1
  p0=$(frames_of "$PANEL"); c0=$(frames_of "$CATCHER")
  sleep 5
  p1=$(frames_of "$PANEL"); c1=$(frames_of "$CATCHER")
  echo "5 s idle after the hide: panel $(( p1 - p0 )) frames, catcher $(( c1 - c0 )) frames" \
    | tee "$log" >&2
  [ "$(( p1 - p0 ))" = 0 ] && [ "$(( c1 - c0 ))" = 0 ]
}

# 5.
step_demo() {
  local log="$OUT_DIR/demo.log" fifo="$OUT_DIR/demo.in" ok=yes
  rm -f "$fifo"; mkfifo "$fifo"
  nested "$DEMO" <"$fifo" >"$log" 2>&1 &
  local demo=$!
  exec 7>"$fifo"
  sleep 2
  local name
  name=$(grep -o 'sill-launcher-[A-Za-z0-9-]*' "$log" | head -1)
  echo "open" >&7; sleep 0.8
  echo "type fi" >&7; sleep 1.2
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/launcher-cosmic.png" >/dev/null 2>&1 || ok=no
  echo "sill-launcher-$NAME capture $OUT_DIR/demo-open.png" >&7; sleep 0.5
  echo "key actions" >&7; sleep 1.0
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/launcher-cosmic-actions.png" >/dev/null 2>&1 || ok=no
  echo "sill-launcher-$NAME capture $OUT_DIR/demo-actions.png" >&7; sleep 0.5
  echo "key escape" >&7; sleep 0.5
  echo "key escape" >&7; sleep 0.8
  echo "sill-launcher-$NAME capture $OUT_DIR/demo-closed.png" >&7; sleep 0.5
  echo "quit" >&7
  exec 7>&-
  wait "$demo" 2>/dev/null
  rm -f "$fifo"
  local open_px actions_px
  open_px=$(opaque_pixels "$OUT_DIR/demo-open.png" 2>/dev/null || echo 0)
  actions_px=$(opaque_pixels "$OUT_DIR/demo-actions.png" 2>/dev/null || echo 0)
  echo "demo: open ${open_px} px, actions open ${actions_px} px (${name:-?})" | tee -a "$log" >&2
  [ "$ok" = yes ] && [ "$open_px" -gt 1000 ] && [ "$actions_px" -gt "$open_px" ]
}

source "$SHELL_HOST_DEV/.nested-env"
build || exit 1
read -r OUT_W OUT_H < <("$SHELL_HOST_DEV/run.sh" wayland-info | sed -n 's/.*logical_width: \([0-9]*\), logical_height: \([0-9]*\).*/\1 \2/p' | head -1)
[ -n "${OUT_W:-}" ] || { echo "no nested output (dev/nested.sh first)" >&2; exit 1; }
if sill ping >/dev/null 2>&1; then
  echo "a sill daemon is already running; quit it first (this script starts its own)" >&2
  exit 1
fi
start_daemon || { stop_daemon; exit 1; }
status=0
for step in step_warm step_toggle_shows step_hide step_idle; do
  if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; fi
done
stop_daemon
if step_demo; then echo "PASS step_demo"; else echo "FAIL step_demo"; status=1; fi
echo "logs in $OUT_DIR; screenshots in $SHOTS"
exit "$status"

#!/usr/bin/env bash
# Screenshot thumbnail acceptance (design/20 §1.13), on the nested cosmic-comp (dev/nested.sh; a
# running wayland-1 is reused and never stopped). Starts
# `sill run` from this checkout's debug build (`--features debug`) with a scratch
# XDG_CONFIG_HOME, XDG_STATE_HOME and XDG_CACHE_HOME, its own XDG_RUNTIME_DIR (the nested socket
# as an absolute WAYLAND_DISPLAY), under a private session bus (dev/private-bus.conf).
#
# Nothing here captures a real screen or touches Pictures or the real trash: the scratch
# config's `screenshot.folders` is a temp folder and `screenshot.tool` a stand-in that copies the
# services' fixture PNG there (never cosmic-screenshot); `xdg-open` and `gio` are stand-ins first
# on the daemon's PATH that log their arguments; the output shot is grim on the nested socket
# only (no fallback).
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
# Exit 0 when every step passed, 1 on a failure. Another sill daemon started from a
# `target/*/sill run` is waited for (up to 10 minutes).
#
# Usage: dev/nested.sh && dev/accept-screenshot.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-screenshot"
SHOTS="$SILL_REPO/dev/out"
FIXTURE="$SILL_REPO/crates/sill-services/tests/fixtures/shots/Screenshot_fixture.png"
HOLD_MS=5200
REST_MS=1500
mkdir -p "$OUT_DIR" "$SHOTS"

source "$SHELL_HOST_DEV/.nested-env"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
NESTED_SOCKET="$HOST_RUNTIME/$WAYLAND_DISPLAY"
RUNTIME=$(mktemp -d)
chmod 700 "$RUNTIME"
WATCHED=$(mktemp -d)
ELSEWHERE=$(mktemp -d)
STUBS=$(mktemp -d)
OPENED="$OUT_DIR/opened.log"
TRASHED="$OUT_DIR/trashed.log"

sill() { XDG_RUNTIME_DIR="$RUNTIME" "$SILL_BIN" "$@"; }
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }

# The nested output's connector name (the surfaces' namespace suffix).
output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

# SURFACE's cumulative frame counter from `sill debug stats`.
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

# Whether the thumbnail is mapped with its card drawn.
shows() {
  sill debug capture "$THUMB" "$OUT_DIR/probe.png" >/dev/null 2>&1 || return 1
  [ "$(opaque_pixels "$OUT_DIR/probe.png")" -gt 5000 ]
}
mapped() { sill debug capture "$THUMB" "$OUT_DIR/probe.png" >/dev/null 2>&1; }

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
    mapped || { echo "$t"; return; }
    sleep 0.05
  done
}

# The nested output only: grim on the nested socket, never a host screenshot.
nested_shot() {
  WAYLAND_DISPLAY="$NESTED_SOCKET" grim "$1"
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

stubs() {
  printf '#!/bin/sh\necho "$@" >>"%s"\n' "$OPENED" >"$STUBS/xdg-open"
  printf '#!/bin/sh\necho "$@" >>"%s"\n' "$TRASHED" >"$STUBS/gio"
  chmod +x "$STUBS/xdg-open" "$STUBS/gio"
  : >"$OPENED"
  : >"$TRASHED"
}

start_daemon() {
  CONFIG=$(mktemp -d)
  STATE=$(mktemp -d)
  CACHE=$(mktemp -d)
  mkdir -p "$CONFIG/sill"
  cat >"$CONFIG/sill/settings.toml" <<TOML
[screenshot]
folders = "$WATCHED"
tool = 'cp "$FIXTURE" "\$2/Screenshot_\$1_\$(date +%s%N).png"'
TOML
  backdrop_seed "$CONFIG" || return 1
  use_fake_ddc "$CONFIG" >/dev/null
  require_fake_ddc || return 1
  PATH="$STUBS:$PATH" XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" \
    XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$NESTED_SOCKET" \
    dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- \
    sh -c "$PRIVATE_BOTH_BUSES" "$SILL_BIN" \
    >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  THUMB="sill-screenshot-thumb-$NAME"
  sleep 1.5
  echo "daemon on a private bus and runtime dir $RUNTIME; watching $WATCHED; $THUMB" >&2
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  rm -rf "$CONFIG" "$STATE" "$CACHE"
}

# The card's centre in the surface: the surface's own size less the room around the card.
card_centre() {
  sill debug capture "$THUMB" "$OUT_DIR/card.png" >/dev/null 2>&1 || return 1
  python3 - "$OUT_DIR/card.png" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
box = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
print((box[0] + box[2]) // 2, (box[1] + box[3]) // 2) if box else sys.exit(1)
EOF
}

# 1.
step_warm() {
  local log="$OUT_DIR/warm.log"
  sill debug stats >"$log" 2>&1
  grep -q "\"$THUMB\"" "$log" || { echo "$THUMB is not in 'sill debug stats'" >&2; return 1; }
  if mapped; then echo "the thumbnail is mapped before any screenshot" >&2; return 1; fi
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

"$SHELL_HOST_DEV/run.sh" wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
command -v grim >/dev/null 2>&1 || { echo "grim is needed for the nested screenshot" >&2; exit 1; }
build || exit 1
for _ in $(seq 600); do
  pgrep -f 'target/[a-z]*/sill run' >/dev/null || break
  [ -n "${WAITED:-}" ] || { echo "another sill daemon is running; waiting for it to quit" >&2; WAITED=1; }
  sleep 1
done
stubs
start_daemon || { stop_daemon; exit 1; }
status=0
for step in step_warm step_debug_shot_shows_then_hides step_tool_then_click step_hover_pauses step_idle; do
  if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; fi
done
stop_daemon
rm -rf "$RUNTIME" "$WATCHED" "$ELSEWHERE" "$STUBS"
echo "logs in $OUT_DIR; screenshot $SHOTS/screenshot-thumb-cosmic.png"
exit "$status"

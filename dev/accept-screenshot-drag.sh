#!/usr/bin/env bash
# Real-input acceptance for dragging the screenshot thumbnail out (G120; shell-host F60's
# `SurfaceHandle::start_drag`), on shell-host's private headless KWin (scripts/dev/kwin-virtual.sh)
# driven by ONE fake-input client (scripts/dev/fake-input.sh). Never the user's session
# (wayland-0), never the nested compositor, never ~/Pictures: the daemon runs from this
# checkout's debug build on KWin's socket and private bus, with its own XDG_RUNTIME_DIR, a scratch
# XDG_CONFIG_HOME/STATE/CACHE, `screenshot.folders` a temp folder, `screenshot.tool = "off"`, and
# `dock.position = "left"` (so nothing reserves the bottom edge and the thumbnail sits at the
# output's true bottom-right corner).
#
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
# Then the input client, the daemon (`sill quit`) and the headless KWin are stopped.
# Exit 0 only when every step passed; 1 on a failure.
#
# Usage: dev/accept-screenshot-drag.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-screenshot-drag"
mkdir -p "$OUT_DIR"
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
SIZE=1920x1080
WIDTH=${SIZE%x*}
HEIGHT=${SIZE#*x}
REST_MS=1500
LIMIT_MS=3200
FIXTURE="$SILL_REPO/crates/sill-services/tests/fixtures/shots/Screenshot_fixture.png"

SCRATCH=$(mktemp -d)
RUNTIME="$SCRATCH/run"
CONFIG="$SCRATCH/config"
STATE="$SCRATCH/state"
CACHE="$SCRATCH/cache"
WATCHED="$SCRATCH/shots"
mkdir -p "$RUNTIME" "$CONFIG/sill" "$STATE" "$CACHE" "$WATCHED"
chmod 700 "$RUNTIME"
DAEMON=
INPUT=
WL=
BUS=

in_env() {
  env XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$WL" DBUS_SESSION_BUS_ADDRESS="$BUS" \
    DBUS_SYSTEM_BUS_ADDRESS="$BUS" \
    XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" "$@"
}
sill() { in_env "$SILL_BIN" "$@"; }
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

# The headless KWin, its socket (absolute) and bus read from its env file without sourcing it.
start_kwin() {
  "$SHELL_HOST_DEV/kwin-virtual.sh" start "$SIZE" >"$OUT_DIR/kwin.log" 2>&1 || return 1
  local runtime socket
  runtime=$(sed -n 's/^export XDG_RUNTIME_DIR=//p' "$KWIN_ENV")
  socket=$(sed -n 's/^export WAYLAND_DISPLAY=//p' "$KWIN_ENV")
  BUS=$(sed -n "s/^export DBUS_SESSION_BUS_ADDRESS='\\(.*\\)'$/\\1/p" "$KWIN_ENV")
  WL="$runtime/$socket"
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $KWIN_ENV" >&2; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "KWin's bus is the session's" >&2; return 1; }
  require_private_system_bus "$BUS" || return 1
}
stop_kwin() { "$SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1; }

start_input() {
  rm -f "$OUT_DIR/input.fifo"; mkfifo "$OUT_DIR/input.fifo" || return 1
  "$SHELL_HOST_DEV/fake-input.sh" <"$OUT_DIR/input.fifo" >"$OUT_DIR/input.log" 2>&1 &
  INPUT=$!
  exec 3>"$OUT_DIR/input.fifo"
}
stop_input() {
  [ -n "$INPUT" ] || return 0
  exec 3>&-
  for _ in $(seq 50); do kill -0 "$INPUT" 2>/dev/null || break; sleep 0.1; done
  kill "$INPUT" 2>/dev/null
  INPUT=
  rm -f "$OUT_DIR/input.fifo"
}

# The fake-input commands "$@", one per line; returns once each has been answered.
input() {
  local before
  before=$(grep -c '^ok ' "$OUT_DIR/input.log")
  printf '%s\n' "$@" >&3
  for _ in $(seq 400); do
    [ "$(grep -c '^ok ' "$OUT_DIR/input.log")" -ge $(( before + $# )) ] && return 0
    sleep 0.05
  done
  echo "fake input did not answer: $*" >&2
  return 1
}

output_name() {
  sill debug stats 2>/dev/null | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

start_daemon() {
  cat >"$CONFIG/sill/settings.toml" <<TOML
[dock]
position = "left"
[screenshot]
folders = "$WATCHED"
tool = "off"
TOML
  use_fake_ddc "$SCRATCH" >/dev/null
  require_fake_ddc || return 1
  in_env "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  THUMB="sill-screenshot-thumb-$NAME"
  sleep 1.5
}

# Ask the daemon to quit and wait up to 10 s; never kill it (FINDINGS G51).
stop_daemon() {
  [ -n "$DAEMON" ] || return 0
  in_env timeout 10 "$SILL_BIN" quit >/dev/null 2>&1
  for _ in $(seq 100); do kill -0 "$DAEMON" 2>/dev/null || break; sleep 0.1; done
  DAEMON=
}

# The card's bounding box and the surface's size in a capture: "W H LEFT TOP RIGHT BOTTOM".
card_box() {
  sill debug capture "$THUMB" "$OUT_DIR/card.png" >/dev/null 2>&1 || return 1
  python3 - "$OUT_DIR/card.png" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
box = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
print(img.width, img.height, *box) if box else sys.exit(1)
EOF
}
mapped() { sill debug capture "$THUMB" "$OUT_DIR/probe.png" >/dev/null 2>&1; }

status=0
check() {
  if [ "$2" = 1 ]; then echo "PASS $1: $3"; else echo "FAIL $1: $3"; status=1; fi
}

cleanup() {
  stop_input
  stop_daemon
  stop_kwin
  rm -rf "$SCRATCH"
}
trap cleanup EXIT

build || exit 1
start_kwin || { echo "KWin did not start (see $OUT_DIR/kwin.log)" >&2; exit 1; }
start_input || exit 1
start_daemon || exit 1
input "move 900 500" "sleep 200" || exit 1

# 1.
cp "$FIXTURE" "$WATCHED/Screenshot_drag.png"
sill debug screenshot "$WATCHED/Screenshot_drag.png" >"$OUT_DIR/shot.log" 2>&1
box=
for _ in $(seq 40); do box=$(card_box) && break; sleep 0.1; done
sleep 0.6
box=$(card_box)
ok=0; [ -n "$box" ] && ok=1
check shows "$ok" "the card in $THUMB: ${box:-none} (surface W H, card box)"
[ "$ok" = 1 ] || exit 1

# 2. The surface is flush with the output's bottom-right corner (dock on the left).
read -r SW SH L T R B <<<"$box"
X=$(( WIDTH - SW + (L + R) / 2 ))
Y=$(( HEIGHT - SH + (T + B) / 2 ))
mark=$(wc -l <"$OUT_DIR/daemon.log")
steps=("move $X $Y" "sleep 300" "press left" "sleep 150" "move $(( X - 12 )) $Y" "sleep 250")
for i in $(seq 1 10); do
  steps+=("move $(( X - 12 - (X - 12 - 700) * i / 10 )) $(( Y - (Y - 400) * i / 10 ))" "sleep 60")
done
steps+=("sleep 300" "release left" "sleep 600")
input "${steps[@]}" || exit 1
released=$(now_ms)
outcome=
for _ in $(seq 30); do
  outcome=$(tail -n +$(( mark + 1 )) "$OUT_DIR/daemon.log" | sed -n 's/^sill: screenshot drag: //p' | head -n 1)
  [ -n "$outcome" ] && break
  sleep 0.1
done
up=0; mapped && up=1
kept=0; [ -f "$WATCHED/Screenshot_drag.png" ] && kept=1
ok=0; [ "$outcome" = "Cancelled" ] && [ "$up" = 1 ] && [ "$kept" = 1 ] && ok=1
check drag_cancelled_over_nothing "$ok" "pressed at $X $Y, released at 700 400: outcome ${outcome:-none}; card up after: $up; file there: $kept"

# 3.
gone=-1
while [ $(( $(now_ms) - released )) -le "$LIMIT_MS" ]; do
  mapped || { gone=$(( $(now_ms) - released )); break; }
  sleep 0.05
done
kept=0; [ -f "$WATCHED/Screenshot_drag.png" ] && kept=1
ok=0; [ "$gone" -ge 0 ] && [ "$gone" -ge $(( REST_MS - 400 )) ] && [ "$kept" = 1 ] && ok=1
check hold_resumes "$ok" "unmapped ${gone} ms after the release (rest $REST_MS, limit $LIMIT_MS); file kept: $kept"

echo "logs in $OUT_DIR"
exit $status

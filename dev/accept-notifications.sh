#!/usr/bin/env bash
# The banners' acceptance (lane b, FINDINGS "The M6 fill-in" and "M6 banners"), on the nested
# cosmic-comp (dev/nested.sh; a running wayland-1 is reused and never stopped).
#
# Every notification comes from `sill debug notify` (the service's own reducer, no D-Bus). The
# daemon still runs on a private bus (dev/private-bus.conf: no service directories, F264), with
# its own XDG_RUNTIME_DIR (WAYLAND_DISPLAY an absolute socket path) and a scratch
# XDG_CONFIG_HOME and XDG_STATE_HOME, so nothing here reaches the real session bus's
# org.freedesktop.Notifications or another sill's IPC socket. Another sill daemon drawing on the
# same nested output would be in the screenshots, so the script waits for it to finish first.
#
#   1. Warm: `sill-banners-<out>` exists from the start, hidden (no capture).
#   2. A notification shows it within 300 ms (a capture with the card's pixels);
#      dev/out/banners-cosmic.png.
#   3. It hides between the hold (notifications.banner_hold_ms, 5200) and the hold plus the exit
#      (--t-move) plus 400 ms; `status` then says banners 0, kept 1.
#   4. Hovered at 4000 ms (`sill debug pointer` on the card) it is still up at 7000, and gone by
#      1500 ms plus the exit plus 400 ms after `sill debug leave`.
#   5. Three from one app within 2 s make one card (one card's height, `status` banners 3); five
#      from five apps make three cards (three cards' height). Then three from one app and two
#      others: dev/out/banners-stack-cosmic.png, a stack with a group.
#   6. notifications.dnd = "on" and `sill reload`: a normal one shows no banner but kept rises;
#      a critical one shows and is still up after 10 s.
#   7. `sill debug click` on the body of one sent with `--action default=Open`: the card leaves
#      and kept falls by one.
#   8. 5 s after the last card leaves: 0 frames on the column.
#
# The drag and the swipe are the real-input pass's (lane d): debug injection has no drag.
#
# Usage: dev/nested.sh && dev/accept-notifications.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-notifications"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
HOLD_MS=5200
EXIT_MS=250
SLACK_MS=400
# The card's centre in the column (surface-local): layer::ROOM plus half a 360 x 72 card.
CARD_X=220
CARD_Y=76
# The capture's drawn height, the cards' shadows included: one card (a group adds its layers)
# stays under 170; three stack to 240-360 (a card and its gap are about 80 px).
ONE_CARD_MAX=170
THREE_CARDS_MIN=240

SCRATCH=$(mktemp -d)
RUNTIME="$SCRATCH/run"
CONFIG="$SCRATCH/config"
STATE="$SCRATCH/state"
mkdir -p "$RUNTIME" "$CONFIG/sill" "$STATE"
chmod 700 "$RUNTIME"
BUS=
BUS_PID=
DAEMON=

in_env() {
  env XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$WL" DBUS_SESSION_BUS_ADDRESS="$BUS" \
    XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" "$@"
}
sill() { in_env "$SILL_BIN" "$@"; }
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }
since() { echo $(( $(now_ms) - $1 )); }
notify() { sill debug notify "$@" >>"$OUT_DIR/notify.log" 2>&1; }

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

start_bus() {
  local out
  out=$(dbus-daemon --config-file="$SILL_REPO/dev/private-bus.conf" --fork \
    --print-address=1 --print-pid=1) || return 1
  BUS=$(sed -n 1p <<<"$out")
  BUS_PID=$(sed -n 2p <<<"$out")
  case "$BUS" in unix:*) ;; *) echo "no private bus address: $out" >&2; return 1 ;; esac
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "not a private bus" >&2; return 1; }
}

settings() {
  printf '[notifications]\nserver = "own"\ndnd = "%s"\n' "$1" >"$CONFIG/sill/settings.toml"
}

# The nested output's connector name (the surfaces' namespace suffix).
output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-"):
        print(s[len("sill-bar-"):]); break'
}

frames_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"])' "$1"
}

# The height of the capture $1's drawn part (rows with any opaque pixel), 0 for none.
drawn_height() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
box = img.getchannel("A").point(lambda a: 255 if a > 8 else 0).getbbox()
print(0 if box is None else box[3] - box[1])
EOF
}

opaque_pixels() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
print(sum(1 for a in img.getchannel("A").tobytes() if a > 0))
EOF
}

# Whether the column captures (it is mapped) with a card's pixels; the capture is $1.
# A capture right after the surface maps can answer before its first frame is presented and
# write nothing, so a missing file is asked again for up to half a second.
shows() {
  local png="${1:-$OUT_DIR/banners.png}"
  for _ in $(seq 10); do
    rm -f "$png"
    sill debug capture "$BANNERS" "$png" >>"$OUT_DIR/capture.log" 2>&1 || return 1
    [ -s "$png" ] && break
    sleep 0.05
  done
  [ -s "$png" ] && [ "$(opaque_pixels "$png")" -gt 1000 ]
}

# Wait up to $1 ms for the column to unmap; prints the ms it took (or -1).
await_hidden() {
  local start t
  start=$(now_ms)
  while [ "$(since "$start")" -lt "$1" ]; do
    if ! sill debug capture "$BANNERS" "$OUT_DIR/late.png" >>"$OUT_DIR/capture.log" 2>&1; then
      since "$start"; return 0
    fi
    sleep 0.05
  done
  echo -1
}

status_field() {
  sill notifications status 2>/dev/null | python3 -c '
import json, sys
def find(v):
    if isinstance(v, dict):
        if "holder" in v and "kept" in v:
            return v
        for x in v.values():
            r = find(x)
            if r: return r
    return None
r = find(json.loads(sys.stdin.read())) or {}
print(r.get(sys.argv[1], ""))' "$1"
}

await_status() {
  local got=
  for _ in $(seq 50); do
    got=$(status_field "$1")
    [ "$got" = "$2" ] && return 0
    sleep 0.1
  done
  echo "status $1 is '$got', wanted '$2'" >&2
  return 1
}

# Clear the banners and wait for the column to unmap.
clear_banners() {
  sill notifications dismiss-all >>"$OUT_DIR/notify.log" 2>&1
  [ "$(await_hidden 3000)" -ge 0 ]
}

# Another sill daemon on the nested output would show up in the shots: wait for it to finish.
await_alone() {
  local waited=0
  while pgrep -u "$USER" -f '/sill run$' >/dev/null; do
    [ "$waited" = 0 ] && echo "another sill daemon is running; waiting for it to finish" >&2
    waited=$((waited + 1))
    [ "$waited" -gt 600 ] && { echo "it is still running after 10 min" >&2; return 1; }
    sleep 1
  done
}

start_daemon() {
  backdrop_seed "$CONFIG" || return 1
  in_env "$SILL_BIN" run >>"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  BANNERS="sill-banners-$NAME"
  sleep 1
}

stop_daemon() {
  [ -n "$DAEMON" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
}

cleanup() {
  stop_daemon
  [ -n "$BUS_PID" ] && kill "$BUS_PID" 2>/dev/null
  rm -rf "$SCRATCH"
}
trap cleanup EXIT

# 1.
step_warm() {
  sill debug stats >"$OUT_DIR/warm.log" 2>&1
  grep -q "\"$BANNERS\"" "$OUT_DIR/warm.log" || { echo "$BANNERS is not in 'sill debug stats'" >&2; return 1; }
  if sill debug capture "$BANNERS" "$OUT_DIR/hidden.png" >>"$OUT_DIR/warm.log" 2>&1; then
    echo "the column is mapped before any notification" >&2; return 1
  fi
  echo "$BANNERS spawned at start, hidden" >&2
}

# 2. and 3.
step_shows_holds_hides() {
  local start shown=-1 gone t
  start=$(now_ms)
  notify "Lunch on Thursday?" --app accept \
    --body "Are you free at noon on Thursday? The new place on the corner has a table for four." \
    || return 1
  for _ in $(seq 60); do
    if shows; then shown=$(since "$start"); break; fi
    sleep 0.01
  done
  sleep 0.6
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/banners-cosmic.png" >>"$OUT_DIR/shot.log" 2>&1 \
    || echo "screenshot failed" >&2
  while [ "$(since "$start")" -lt $((HOLD_MS - 300)) ]; do sleep 0.05; done
  t=$(await_hidden 3000)
  gone=$(( t < 0 ? -1 : $(since "$start") ))
  local limit=$((HOLD_MS + EXIT_MS + SLACK_MS))
  echo "shown ${shown} ms after the notification; hidden at ${gone} ms (hold ${HOLD_MS}, limit ${limit})" >&2
  [ "$shown" -ge 0 ] && [ "$shown" -le 300 ] && [ "$gone" -ge "$HOLD_MS" ] && [ "$gone" -le "$limit" ] \
    || return 1
  await_status banners 0 && await_status kept 1 || return 1
  echo "status: banners 0, kept 1" >&2
}

# 4.
step_hover() {
  local start up=no gone
  start=$(now_ms)
  notify "Hover me" --app accept --body "Held while the pointer rests on it." || return 1
  while [ "$(since "$start")" -lt 4000 ]; do sleep 0.02; done
  sill debug pointer "$BANNERS" "$CARD_X" "$CARD_Y" >>"$OUT_DIR/pointer.log" 2>&1 || return 1
  while [ "$(since "$start")" -lt 7000 ]; do sleep 0.02; done
  shows && up=yes
  sill debug leave "$BANNERS" >>"$OUT_DIR/pointer.log" 2>&1 || return 1
  local left
  left=$(now_ms)
  gone=$(await_hidden 4000)
  local limit=$((1500 + EXIT_MS + SLACK_MS))
  echo "hovered at 4000 ms: still up at 7000 ms: $up; gone ${gone} ms after the pointer left (limit ${limit})" >&2
  [ "$up" = yes ] && [ "$gone" -ge 1500 ] && [ "$gone" -le "$limit" ]
}

# 5.
step_grouping() {
  local h1 h3
  notify "One" --app chat || return 1
  sleep 0.5
  notify "Two" --app chat || return 1
  sleep 0.5
  notify "Three" --app chat || return 1
  sleep 0.8
  shows "$OUT_DIR/group.png" || { echo "no card for the group" >&2; return 1; }
  h1=$(drawn_height "$OUT_DIR/group.png")
  await_status banners 3 || return 1
  echo "three from one app: drawn ${h1} px tall (one card under ${ONE_CARD_MAX}), status banners 3" >&2
  [ "$h1" -gt 0 ] && [ "$h1" -le "$ONE_CARD_MAX" ] || return 1
  clear_banners || return 1
  for app in alpha bravo charlie delta echo; do notify "From $app" --app "$app" || return 1; done
  sleep 0.8
  shows "$OUT_DIR/five.png" || return 1
  h3=$(drawn_height "$OUT_DIR/five.png")
  echo "five from five apps: drawn ${h3} px tall (three cards past ${THREE_CARDS_MIN}, under four)" >&2
  [ "$h3" -ge "$THREE_CARDS_MIN" ] && [ "$h3" -le $((THREE_CARDS_MIN + 120)) ] || return 1
  clear_banners || return 1
  notify "Ada Lovelace" --app chat --body "Hi" || return 1
  notify "Ada Lovelace" --app chat --body "Are you there?" || return 1
  notify "Ada Lovelace" --app chat --body "The engine is running again." || return 1
  notify "Build finished" --app builder --body "sill: 0 warnings, all tests passed." || return 1
  notify "Lunch on Thursday?" --app mail --body "Are you free at noon on Thursday?" \
    --action default=Open --action reply=Reply || return 1
  sleep 0.8
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/banners-stack-cosmic.png" >>"$OUT_DIR/shot.log" 2>&1 \
    || echo "stack screenshot failed" >&2
  clear_banners
}

# 6.
step_dnd() {
  local kept up=no
  settings on
  sill reload >>"$OUT_DIR/notify.log" 2>&1 || return 1
  sleep 0.5
  kept=$(status_field kept)
  notify "Quiet" --app accept || return 1
  sleep 1
  if shows; then echo "a banner showed under Do Not Disturb" >&2; return 1; fi
  await_status kept $((kept + 1)) || return 1
  notify "Critical" --app accept --urgency critical --body "Shows through Do Not Disturb." || return 1
  sleep 0.8
  shows || { echo "the critical one did not show" >&2; return 1; }
  sleep 10
  shows && up=yes
  echo "DND: a normal one kept without a banner (kept $kept -> $((kept + 1))); a critical one up after 10 s: $up" >&2
  settings off
  sill reload >>"$OUT_DIR/notify.log" 2>&1
  [ "$up" = yes ] && clear_banners
}

# 7.
step_click() {
  local kept after gone
  notify "Open me" --app accept --body "A click opens it." --action default=Open || return 1
  sleep 0.8
  shows || return 1
  kept=$(status_field kept)
  sill debug click "$BANNERS" "$CARD_X" "$CARD_Y" >>"$OUT_DIR/pointer.log" 2>&1 || return 1
  gone=$(await_hidden 2000)
  await_status kept $((kept - 1)) || return 1
  after=$(status_field kept)
  echo "a click on the body: the card left in ${gone} ms, kept $kept -> $after" >&2
  [ "$gone" -ge 0 ]
}

# 8.
step_idle() {
  local f0 f1
  sleep 5
  f0=$(frames_of "$BANNERS")
  sleep 5
  f1=$(frames_of "$BANNERS")
  echo "5 s idle after the last card left: $BANNERS $((f1 - f0)) frames" >&2
  [ "$((f1 - f0))" = 0 ]
}

source "$SHELL_HOST_DEV/.nested-env"
case "$WAYLAND_DISPLAY" in
  /*) WL="$WAYLAND_DISPLAY" ;;
  *) WL="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$WAYLAND_DISPLAY" ;;
esac
[ -S "$WL" ] || { echo "no nested compositor at $WL (dev/nested.sh first)" >&2; exit 1; }
build || exit 1
await_alone || exit 1
start_bus || exit 1
: >"$OUT_DIR/daemon.log"
: >"$OUT_DIR/notify.log"
settings off
start_daemon || exit 1
status=0
for step in step_warm step_shows_holds_hides step_hover step_grouping step_dnd step_click step_idle; do
  if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; clear_banners; fi
done
echo "logs in $OUT_DIR; screenshots $SHOTS/banners-cosmic.png, $SHOTS/banners-stack-cosmic.png"
exit "$status"

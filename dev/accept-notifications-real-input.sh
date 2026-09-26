#!/usr/bin/env bash
# Real input acceptance for the banners and the notification center (M6 lane d, FINDINGS
# "The M6 fill-in" (d) and "M6 lane d"), on shell-host's private headless KWin
# (scripts/dev/kwin-virtual.sh, FINDINGS F52) driven by shell-host's scripts/dev/fake-input.sh:
# every hover, press, drag and key is a real Wayland event with a real serial and keyboard focus.
#
# The daemon runs from this checkout's debug build (`--features debug`) on the headless KWin and
# its private D-Bus bus (shell-host's private-bus.conf: nothing activatable, FINDINGS F56), with
# its own XDG_RUNTIME_DIR (WAYLAND_DISPLAY an absolute socket path) and a scratch
# XDG_CONFIG_HOME and XDG_STATE_HOME, `notifications.server = "own"`. It is checked to hold
# org.freedesktop.Notifications on that bus before anything is posted (G61). Every notification
# comes from `sill debug notify`; nothing here reaches the real session bus, the user's session
# (wayland-0) or the nested compositor (wayland-1). ONE fake-input client serves the whole run
# (each client is a new input device, and KWin re-announces the seat's pointer for each).
#
#   1. `sill notifications status`: holder sill, mode own.
#   2. Hover: a real pointer on the card at 4000 ms keeps it past the hold (5200) at 7000 ms; it
#      leaves 1500 ms (plus the exit and slack) after the pointer does; `kept` rises by one.
#   3. A real click on the body of one sent with `--action default=Open`: the card leaves, kept
#      falls by one, and the bus carries ActionInvoked (<id>, 'default').
#   4. Hovered (after that click's card left under the pointer), the card shows its close
#      button; a real click on it: the card leaves, kept falls by one.
#   5. A real 90 px drag to the right swipes it away: the column unmaps, banners 0, and it stays in
#      the notification center (kept unchanged; notifications.swipe = keep_in_center).
#   6. A real 50 px drag, slowed before the release: the card springs back and stays up.
#   7. A real click on the clock opens the center; real Escape closes it.
#   8. Opened again by a click on the clock, a real click outside the column closes it.
#   9. In the center, a real Down focuses the first group's header (two from one app) and a real
#      Delete clears the group: kept falls by two.
#  10. Overflowed with 15 cards, a real wheel over the column (fake-input's `axis`, shell-host
#      FINDINGS F63) scrolls it: the whole column moves, title included. Rubber-band at the top
#      is not exercised: wheel scrolling never stretches (F63 §11.3.7's last row and its own "no
#      stretch at an edge" test); only a finger gesture does, and fake-input has no finger source
#      (F63's own live gap) — recorded, not proven.
#  11. 5 s idle: 0 frames on the banners and the center.
#  12. `status` afterwards: holder still sill.
#
# Then the input client, the daemon (`sill quit`) and the headless KWin are stopped.
#
# Two shell-host gaps once shaped the order (FINDINGS G100, G101, closed by shell-host 902a12c):
# an enter now moves the document's pointer and an unmap withdraws it, so a "move onto" is one
# motion and the steps run in their natural order. `repro` keeps both as regression checks.
#
# Exit 0 only when every step passed; 1 on a failure.
#
# Usage: dev/accept-notifications-real-input.sh           (the acceptance)
#        dev/accept-notifications-real-input.sh repro     (G100, G101 closed: regression checks;
#                                                          Q170: FAILS while it stands)
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-notifications-real-input"
mkdir -p "$OUT_DIR"
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
SIZE=1920x1080
WIDTH=${SIZE%x*}
HEIGHT=${SIZE#*x}
HOLD_MS=5200
EXIT_MS=250
SLACK_MS=400
# Somewhere on the desktop, away from every sill surface but the wallpaper.
REST_X=$(( WIDTH / 3 ))
REST_Y=$(( HEIGHT / 2 ))

SCRATCH=$(mktemp -d)
RUNTIME="$SCRATCH/run"
CONFIG="$SCRATCH/config"
STATE="$SCRATCH/state"
mkdir -p "$RUNTIME" "$CONFIG/sill" "$STATE"
chmod 700 "$RUNTIME"
DAEMON=
INPUT=
MONITOR=
WL=
BUS=

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

# The headless KWin, and its socket (absolute) and bus from its env file, read without sourcing
# it into this shell: `kwin-virtual.sh stop` must never find this script naming the bus.
start_kwin() {
  "$SHELL_HOST_DEV/kwin-virtual.sh" start "$SIZE" >"$OUT_DIR/kwin.log" 2>&1 || return 1
  local runtime socket
  runtime=$(sed -n 's/^export XDG_RUNTIME_DIR=//p' "$KWIN_ENV")
  socket=$(sed -n 's/^export WAYLAND_DISPLAY=//p' "$KWIN_ENV")
  BUS=$(sed -n "s/^export DBUS_SESSION_BUS_ADDRESS='\\(.*\\)'$/\\1/p" "$KWIN_ENV")
  WL="$runtime/$socket"
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $KWIN_ENV" >&2; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "KWin's bus is the session's" >&2; return 1; }
}

stop_kwin() {
  "$SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1
}

# Real input through ONE fake-input client for the whole run.
start_input() {
  rm -f "$OUT_DIR/input.fifo"; mkfifo "$OUT_DIR/input.fifo" || return 1
  "$SHELL_HOST_DEV/fake-input.sh" <"$OUT_DIR/input.fifo" >"$OUT_DIR/input.log" 2>&1 &
  INPUT=$!
  exec 3>"$OUT_DIR/input.fifo"
}

# Wait up to 5 s for process $1 (named $2) to end, then end it.
reap() {
  for _ in $(seq 50); do kill -0 "$1" 2>/dev/null || return 0; sleep 0.1; done
  echo "the $2 ($1) did not end; killing it" >&2
  kill "$1" 2>/dev/null
  sleep 0.5
  kill -9 "$1" 2>/dev/null
  return 0
}

stop_input() {
  [ -n "$INPUT" ] || return 0
  exec 3>&-
  reap "$INPUT" input
  INPUT=
  rm -f "$OUT_DIR/input.fifo"
}

# The fake-input commands "$@", one per line; returns once each has been answered (`ok <line>`).
input() {
  local before
  before=$(grep -c '^ok ' "$OUT_DIR/input.log")
  printf '%s\n' "$@" >&3
  for _ in $(seq 200); do
    [ "$(grep -c '^ok ' "$OUT_DIR/input.log")" -ge $(( before + $# )) ] && return 0
    sleep 0.05
  done
  echo "fake input did not answer: $*" >&2
  return 1
}
key() { input "key $1 down" "sleep 30" "key $1 up" "sleep 150"; }
# A move onto $1 $2: one motion (G100 closed: the enter hovers).
point() { input "move $1 $2" "sleep 80"; }
click() { point "$1" "$2"; input "press left" "sleep 40" "release left" "sleep 200"; }
rest() { point "$REST_X" "$REST_Y"; }

# A drag from $1 $2, $3 px to the right in steps of $4 px every $5 ms, held $6 ms before the
# release (a pause makes the release slow: the card springs back).
drag() {
  local x="$1" y="$2" dx="$3" step="$4" every="$5" hold="$6" moved=0 lines=()
  lines+=("press left" "sleep 60")
  while [ "$moved" -lt "$dx" ]; do
    moved=$(( moved + step > dx ? dx : moved + step ))
    lines+=("move $(( x + moved )) $y" "sleep $every")
  done
  lines+=("sleep $hold" "release left" "sleep 200")
  point "$x" "$y" && input "${lines[@]}"
}

# The bus's signals from org.freedesktop.Notifications, for the whole run.
start_monitor() {
  in_env gdbus monitor --session --dest org.freedesktop.Notifications \
    >"$OUT_DIR/monitor.log" 2>&1 &
  MONITOR=$!
}

stop_monitor() {
  [ -n "$MONITOR" ] || return 0
  kill "$MONITOR" 2>/dev/null
  wait "$MONITOR" 2>/dev/null
  MONITOR=
}

start_daemon() {
  printf '[notifications]\nserver = "own"\n[control_center]\nmenu_bar_sound = "hide"\nmenu_bar_battery = "hide"\n' \
    >"$CONFIG/sill/settings.toml"
  in_env "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  BAR="sill-bar-$NAME"
  BANNERS="sill-banners-$NAME"
  CENTER="sill-notification-center-$NAME"
  sleep 1.5
}

# Ask the daemon to quit and wait up to 10 s; never kill it (FINDINGS G51).
stop_daemon() {
  [ -n "$DAEMON" ] || return 0
  in_env timeout 10 "$SILL_BIN" quit >/dev/null 2>&1
  local result=1
  for _ in $(seq 100); do kill -0 "$DAEMON" 2>/dev/null || { result=0; break; }; sleep 0.1; done
  [ "$result" = 0 ] || echo "the daemon ($DAEMON) did not end after quit" >&2
  DAEMON=
  return "$result"
}

output_name() {
  sill debug stats 2>/dev/null | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

frames_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"])' "$1"
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
v = r.get(sys.argv[1], "")
print(v.get("kind", "") if isinstance(v, dict) else v)' "$1"
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

# Whether SURFACE is mapped (it captures), the capture in $2.
mapped() { sill debug capture "$1" "${2:-$OUT_DIR/probe.png}" >/dev/null 2>&1; }

# Wait up to $2 tenths of a second for SURFACE $1 to be up (or, with $3 = down, gone).
wait_surface() {
  local surface="$1" tenths="$2" want="${3:-up}"
  for _ in $(seq "$tenths"); do
    if mapped "$surface"; then [ "$want" = up ] && return 0
    else [ "$want" = down ] && return 0; fi
    sleep 0.1
  done
  return 1
}

# Wait up to $1 ms for the column to unmap; prints the ms it took (or -1).
await_hidden() {
  local start
  start=$(now_ms)
  while [ "$(since "$start")" -lt "$1" ]; do
    mapped "$BANNERS" || { since "$start"; return 0; }
    sleep 0.05
  done
  echo -1
}

# The banner column's origin on the output, "x y", from KWin's window list: the sill window as
# wide as the column (notifications.banner_width_px, 360, plus layer::ROOM each side).
column_origin() {
  "$SHELL_HOST_DEV/kwin-windows.sh" --class sill 2>/dev/null | python3 -c '
import json, sys
for line in sys.stdin:
    w = json.loads(line)
    if w["frame"][2] == 440:
        print(w["frame"][0], w["frame"][1]); break'
}

# The top card's box on the output, "x0 y0 x1 y1": the opaque rows of a capture of the column,
# offset by the column's origin. Also sets CARD_X, CARD_Y (its centre).
card_box() {
  local png="$OUT_DIR/card.png" origin box
  for _ in $(seq 20); do
    rm -f "$png"
    mapped "$BANNERS" "$png" && [ -s "$png" ] && break
    sleep 0.1
  done
  [ -s "$png" ] || { echo "no capture of the column" >&2; return 1; }
  origin=$(column_origin)
  [ -n "$origin" ] || { echo "no column in KWin's window list" >&2; return 1; }
  box=$(python3 - "$png" $origin <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
ox, oy = int(sys.argv[2]), int(sys.argv[3])
box = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
if box:
    print(box[0] + ox, box[1] + oy, box[2] + ox, box[3] + oy)
EOF
)
  [ -n "$box" ] || { echo "the column shows no card" >&2; return 1; }
  read -r CARD_X0 CARD_Y0 CARD_X1 CARD_Y1 <<<"$box"
  CARD_X=$(( (CARD_X0 + CARD_X1) / 2 ))
  CARD_Y=$(( (CARD_Y0 + CARD_Y1) / 2 ))
  echo "$box"
}

# Whether the top card shows its close button (it is hovered): "yes" or "no", from a capture
# named $1. quire's 18 px button stands 6 px out past the plate's top-left corner, so its middle
# is 3 px inside the corner (layer::ROOM + 3): opaque when shown, the corner's round is not.
close_shown() {
  sleep 0.4
  mapped "$BANNERS" "$OUT_DIR/$1.png" || { echo "no"; return 0; }
  python3 - "$OUT_DIR/$1.png" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
print("yes" if img.getpixel((43, 43))[3] > 200 else "no")
EOF
}

# Post "$@" and wait for its card; sets the CARD_* box.
post_card() {
  notify "$@" || return 1
  wait_surface "$BANNERS" 30 up || { echo "no banner for: $*" >&2; return 1; }
  sleep 0.6
  card_box >>"$OUT_DIR/cards.log" || return 1
}

# The status items' centres, right to left, from a capture of the bar: runs of inked columns
# (dev/accept-real-input.sh's reading; the clock's is first). Prints "x" per item.
status_items() {
  sill debug capture "$BAR" "$OUT_DIR/bar.png" >/dev/null 2>&1 || return 1
  python3 - "$OUT_DIR/bar.png" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
w, h = img.size
ground = img.getpixel((w // 2, h // 2))
def inked(x):
    return any(sum(abs(a - b) for a, b in zip(img.getpixel((x, y)), ground)) > 90
               for y in range(4, h - 4))
runs, run, gap = [], None, 0
for x in range(w - 1, w // 2, -1):
    if inked(x):
        run = [x, x] if run is None else [x, run[1]]
        gap = 0
    elif run is not None:
        gap += 1
        if gap > 7:
            runs.append(run); run = None
for left, right in runs:
    print((left + right) // 2)
EOF
}

clock_click() {
  local items
  mapfile -t items < <(status_items)
  [ "${#items[@]}" -ge 1 ] || { echo "no clock on the bar" >&2; return 1; }
  CLOCK_X=${items[0]}
  click "$CLOCK_X" 16
}

clear_all() {
  sill notifications dismiss-all >>"$OUT_DIR/notify.log" 2>&1
  wait_surface "$BANNERS" 30 down
}

# 1. and 12.
step_holder() {
  await_status holder sill || return 1
  await_status mode own || return 1
  echo "status: holder sill, mode own" >&2
}

# 2.
step_hover() {
  local start kept up=no gone left limit
  rest || return 1
  kept=$(status_field kept)
  start=$(now_ms)
  post_card "Hover me" --app accept --body "Held while the pointer rests on it." || return 1
  while [ "$(since "$start")" -lt 4000 ]; do sleep 0.02; done
  point "$CARD_X" "$CARD_Y" || return 1
  while [ "$(since "$start")" -lt 7000 ]; do sleep 0.02; done
  mapped "$BANNERS" "$OUT_DIR/hovered.png" && up=yes
  rest || return 1
  left=$(now_ms)
  gone=$(await_hidden 4000)
  limit=$((1500 + EXIT_MS + SLACK_MS))
  await_status kept $((kept + 1)) || return 1
  echo "hovered at 4000 ms: still up at 7000 ms: $up; gone ${gone} ms after the pointer left" \
    "(limit ${limit}); kept $kept -> $((kept + 1))" >&2
  [ "$up" = yes ] && [ "$gone" -ge 1300 ] && [ "$gone" -le "$limit" ]
}

# 3.
step_click_default() {
  local kept gone id
  rest || return 1
  post_card "Open me" --app accept --body "A click opens it." --action default=Open \
    --expire never || return 1
  kept=$(status_field kept)
  click "$CARD_X" "$CARD_Y" || return 1
  gone=$(await_hidden 3000)
  await_status kept $((kept - 1)) || return 1
  sleep 0.3
  id=$(grep -A1 "ActionInvoked" "$OUT_DIR/monitor.log" | grep -o "(uint32 [0-9]*, 'default')" | tail -1)
  echo "a real click on the body: the card left in ${gone} ms, kept $kept -> $((kept - 1));" \
    "the bus: ActionInvoked ${id:-(none)}" >&2
  [ "$gone" -ge 0 ] && [ -n "$id" ]
}

# 4.
step_close_button() {
  local kept gone cx cy shown
  rest || return 1
  post_card "Close me" --app accept --body "Its close button dismisses it." --expire never \
    || return 1
  kept=$(status_field kept)
  point "$CARD_X" "$CARD_Y" || return 1
  shown=$(close_shown close-hover)
  cx=$(( CARD_X0 + 3 )); cy=$(( CARD_Y0 + 3 ))
  click "$cx" "$cy" || return 1
  gone=$(await_hidden 3000)
  await_status kept $((kept - 1)) || return 1
  echo "hovered, the close button shows: $shown; a real click on it at $cx,$cy: gone in ${gone} ms," \
    "kept $kept -> $((kept - 1))" >&2
  rest
  [ "$shown" = yes ] && [ "$gone" -ge 0 ]
}

# 5.
step_swipe() {
  local kept gone
  rest || return 1
  post_card "Swipe me" --app accept --body "A drag to the right sends it to the center." \
    --expire never || return 1
  kept=$(status_field kept)
  drag $(( CARD_X - 60 )) "$CARD_Y" 90 15 30 0 || return 1
  gone=$(await_hidden 3000)
  await_status banners 0 || return 1
  sleep 0.5
  [ "$(status_field kept)" = "$kept" ] || { echo "kept $kept -> $(status_field kept)" >&2; return 1; }
  echo "a real 90 px drag right: gone in ${gone} ms; banners 0, kept $kept (kept in the center)" >&2
  rest
  [ "$gone" -ge 0 ]
}

# 6.
step_spring_back() {
  local x0 banners
  rest || return 1
  post_card "Spring back" --app accept --body "A short drag lets it go back." --expire never \
    || return 1
  x0=$CARD_X0
  drag $(( CARD_X - 60 )) "$CARD_Y" 50 10 60 300 || return 1
  sleep 1
  mapped "$BANNERS" || { echo "the card left after a 50 px drag" >&2; return 1; }
  # Off the card first: hovered, quire's close button stands 6 px out past its corner.
  rest || return 1
  sleep 0.4
  card_box >>"$OUT_DIR/cards.log" || return 1
  banners=$(status_field banners)
  echo "a real 50 px drag, slowed: the card is back at x $CARD_X0 (was $x0), banners $banners" >&2
  clear_all
  [ "$CARD_X0" = "$x0" ] && [ "$banners" = 1 ]
}

# 7.
step_center_clock_escape() {
  clock_click || return 1
  wait_surface "$CENTER" 20 up || { echo "a click on the clock ($CLOCK_X) opened nothing" >&2; return 1; }
  sleep 0.6
  mapped "$CENTER" "$OUT_DIR/center.png"
  key Escape
  wait_surface "$CENTER" 20 down || { echo "Escape left the center up" >&2; return 1; }
  echo "a real click on the clock ($CLOCK_X 16) opened the center; real Escape closed it" >&2
}

# 8.
step_center_outside() {
  clock_click || return 1
  wait_surface "$CENTER" 20 up || { echo "the second clock click opened nothing" >&2; return 1; }
  sleep 0.6
  click 200 "$REST_Y" || return 1
  wait_surface "$CENTER" 20 down || { echo "a click outside left the center up" >&2; return 1; }
  echo "a real click outside the column (200 $REST_Y) closed the center" >&2
}

# 9.
step_center_delete() {
  local kept
  notify "Ada" --app chat --body "Hi" || return 1
  notify "Ada" --app chat --body "Are you there?" || return 1
  # Both stay in the center after their hold.
  sleep $(( (HOLD_MS + EXIT_MS + SLACK_MS) / 1000 + 1 ))
  wait_surface "$BANNERS" 30 down || return 1
  kept=$(status_field kept)
  clock_click || return 1
  wait_surface "$CENTER" 20 up || return 1
  sleep 0.6
  key Down || return 1
  sleep 0.3
  mapped "$CENTER" "$OUT_DIR/center-focus.png"
  key Delete || return 1
  await_status kept $((kept - 2)) || return 1
  key Escape
  wait_surface "$CENTER" 20 down || { echo "Escape left the center up" >&2; return 1; }
  echo "in the center, real Down then Delete cleared chat's group: kept $kept -> $((kept - 2))" >&2
}

# A hash of a capture's top band: the whole column scrolls (the title along with the history,
# notification_center.css's own comment), so wheeling it changes this band if shell-host's scroll
# engine is wired to `.sill-nc-column`.
column_signature() {
  python3 - "$1" <<'EOF'
import sys, hashlib
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
w, h = img.size
band = img.crop((0, 0, w, min(220, h)))
print(hashlib.sha256(band.tobytes()).hexdigest())
EOF
}

# 10.
step_center_scroll() {
  local i before after cx cy
  cx=$(( WIDTH - 100 )); cy=$(( HEIGHT / 2 ))
  for i in $(seq 1 15); do
    notify "Overflow $i" --app "overflow$i" --body "Card $i, so the column overflows." \
      --expire never || return 1
  done
  clock_click || return 1
  wait_surface "$CENTER" 20 up || { echo "the center never opened for the scroll check" >&2; return 1; }
  sleep 0.5
  mapped "$CENTER" "$OUT_DIR/scroll-before.png" || { echo "no capture before scrolling" >&2; return 1; }
  before=$(column_signature "$OUT_DIR/scroll-before.png")
  point "$cx" "$cy" || return 1
  input "axis vertical 15" "sleep 40" "axis vertical 15" "sleep 40" "axis vertical 15" "sleep 40" \
    "axis vertical 15" "sleep 40" "axis vertical 15" "sleep 200" || return 1
  sleep 0.3
  mapped "$CENTER" "$OUT_DIR/scroll-after.png" || { echo "no capture after scrolling" >&2; return 1; }
  after=$(column_signature "$OUT_DIR/scroll-after.png")
  echo "5 real wheel detents over the column: top band ${before:0:12} -> ${after:0:12}" >&2
  echo "rubber-band at the top not exercised: wheel never stretches (F63 §11.3.7's last row" \
    "and its own 'no stretch at an edge' test); only a finger gesture does, and fake-input has" \
    "no finger source (F63's own gap)" >&2
  key Escape
  wait_surface "$CENTER" 20 down
  clear_all
  [ "$before" != "$after" ]
}

# 11.
step_idle() {
  local b0 b1 c0 c1
  rest
  sleep 2
  b0=$(frames_of "$BANNERS"); c0=$(frames_of "$CENTER")
  sleep 5
  b1=$(frames_of "$BANNERS"); c1=$(frames_of "$CENTER")
  echo "5 s idle: $BANNERS $((b1 - b0)) frames, $CENTER $((c1 - c0)) frames" >&2
  [ "$((b1 - b0))" = 0 ] && [ "$((c1 - c0))" = 0 ]
}

# Repros of G100 and G101 (shell-host, closed by 902a12c), `repro` mode: each step states what
# should happen and failed while the gap stood.
#
# G100: one motion onto a card from outside the column hovers it. Failed: shell-host took the
# wl_pointer.enter (with its position) without moving the document's pointer there, so nothing
# was hovered until the next motion. The step also shows that a second motion hovers it.
repro_enter_one_motion() {
  local one two
  rest || return 1
  post_card "One motion" --app accept --expire never || return 1
  input "move $CARD_X $CARD_Y" "sleep 80" || return 1
  one=$(close_shown repro-one-motion)
  input "move $(( CARD_X + 1 )) $CARD_Y" "sleep 80" || return 1
  two=$(close_shown repro-two-motions)
  echo "G100: one motion onto the card: hovered $one; after a second motion: $two" >&2
  rest; clear_all
  [ "$one" = yes ]
}

# G101: after a card left under the pointer and the column unmapped, the next card is hovered
# by one motion onto it from the desktop. Failed: shell-host's unmap rebuilt the wl_surface
# without `pointer_left` on its document, so the document kept a hover chain on the removed card
# and the next card was never announced as entered (no pause, no close button) even after the
# pointer went elsewhere while the column was unmapped.
repro_hover_after_unmap() {
  local hovered
  rest || return 1
  post_card "First" --app accept --action default=Open --expire never || return 1
  click "$CARD_X" "$CARD_Y" || return 1
  wait_surface "$BANNERS" 30 down || return 1
  rest || return 1
  post_card "After the unmap" --app accept --expire never || return 1
  point "$CARD_X" "$CARD_Y" || return 1
  hovered=$(close_shown repro-after-unmap)
  echo "G101: a card after the column unmapped under the pointer, one motion onto it:" \
    "hovered $hovered" >&2
  rest; clear_all
  [ "$hovered" = yes ]
}

# Q170 (quire; Blitz's `refresh_hover`): a card that slides in under a resting pointer is not
# hovered when the pointer then moves on it. Fails: after each layout Blitz re-resolves the
# hovered node against the last pointer position without dispatching pointerenter (its own
# TODO), so the card's node is already the hovered one when the motion comes and no enter ever
# reaches quire's `NotificationCard`: no pause, no close button until the pointer leaves the
# card and comes back.
repro_card_under_resting_pointer() {
  local moved back
  rest || return 1
  post_card "First" --app accept --action default=Open --expire never || return 1
  click "$CARD_X" "$CARD_Y" || return 1
  wait_surface "$BANNERS" 30 down || return 1
  post_card "Under the pointer" --app accept --expire never || return 1
  point $(( CARD_X - 20 )) "$CARD_Y" || return 1
  moved=$(close_shown repro-under-pointer)
  rest && point "$CARD_X" "$CARD_Y" || return 1
  back=$(close_shown repro-back)
  echo "Q170: a card slid in under the resting pointer, moved on: hovered $moved;" \
    "out and back: $back" >&2
  rest; clear_all
  [ "$moved" = yes ]
}

cleanup() {
  stop_input
  stop_monitor
  stop_daemon
  stop_kwin
  rm -rf "$SCRATCH"
}

build || exit 1
: >"$OUT_DIR/notify.log"
: >"$OUT_DIR/cards.log"
start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; stop_kwin; exit 1; }
trap cleanup EXIT
start_daemon || exit 1
start_monitor
start_input || { echo "no fake input client" >&2; exit 1; }
# The seat's pointer is announced with the input client's device: the first motion only makes
# it known. Rest it on the desktop.
rest; rest
STEPS="step_holder step_hover step_click_default step_close_button step_swipe step_spring_back
  step_center_clock_escape step_center_outside step_center_delete step_center_scroll step_idle
  step_holder"
[ "${1:-}" = repro ] && STEPS="step_holder repro_enter_one_motion repro_hover_after_unmap repro_card_under_resting_pointer"
status=0
for step in $STEPS; do
  if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; clear_all; rest; fi
done
echo "stopping the input client, the daemon and the headless KWin" >&2
cleanup
trap - EXIT
echo "logs in $OUT_DIR"
exit "$status"

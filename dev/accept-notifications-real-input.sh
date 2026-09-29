#!/usr/bin/env bash
# Real input acceptance for the banners and the notification center, on shell-host's private
# headless KWin (scripts/dev/kwin-virtual.sh) driven by shell-host's scripts/dev/fake-input.sh:
# every hover, press, drag and key is a real Wayland event with a real serial and keyboard focus.
#
# The daemon runs from this checkout's debug build (`--features debug`) on the headless KWin and
# its private D-Bus bus (shell-host's private-bus.conf: nothing activatable), with
# its own XDG_RUNTIME_DIR (WAYLAND_DISPLAY an absolute socket path) and a scratch
# XDG_CONFIG_HOME, XDG_STATE_HOME and XDG_CACHE_HOME, `notifications.server = "own"`. It is
# checked to hold org.freedesktop.Notifications on that bus before anything is posted. Every
# notification comes from `sill debug notify`; nothing here reaches the real session bus, the
# user's session (wayland-0) or the nested compositor (wayland-1). ONE fake-input client serves
# the whole run (each client is a new input device, and KWin re-announces the seat's pointer
# for each).
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
#  6b. Ghosts: after a real click on a card, and after a real swipe, the card's box on the output,
#      as KWin composites it (spectacle), is the wallpaper as it was before the card came.
#   7. A real click on the clock opens the center; real Escape closes it.
#   8. Opened again by a click on the clock, a real click outside the column closes it.
#   9. In the center, a real Down focuses the first group's header (two from one app) and a real
#      Delete clears the group: kept falls by two.
#  9b. A real click on Clear All (found in a capture of the center): kept falls to 0 and the
#      center then rests, 0 frames over 3 s.
#  9c. Clear and Clear All play their exit (quire's LeavingList): Chat (two) and
#      CI (one) posted with `sill debug notify`, the center opened by a real click on the clock;
#      a real click on Chat's Clear (found in a capture): kept falls by two, the list settles
#      Chat's head and card after their fold (the daemon logs each settled row), the center
#      draws frames while they fold and CI heals up into Chat's place, then rests at 0 frames
#      over 3 s; the same for a real click on Clear All.
#  10. Overflowed with 15 cards, a real wheel over the column (fake-input's `axis`) scrolls it: the
#      whole column moves, title included. Rubber-band at the top is not exercised: wheel scrolling
#      never stretches (quire's design/11 §11.3.7's last row and its own "no stretch at an edge"
#      test); only a finger gesture does, and fake-input has no finger source — recorded, not
#      proven.
#  11. 5 s idle: 0 frames on the banners and the center.
#  12. `status` afterwards: holder still sill.
#
# Then the input client, the daemon (`sill quit`) and the headless KWin are stopped.
#
# In shell-host an enter moves the document's pointer and an unmap withdraws it, so a "move onto"
# is one motion and the steps run in their natural order. `repro` keeps both as regression checks.
#
# Exit 0 only when every step passed; 1 on a failure.
#
# The wallpaper is a fine checkerboard (`seed_checks`) so blur shows as grey. KWIN_SCALE=2 runs
# the output at 3840 x 2160 and scale 2, the user's KDE setup. ACCEPT_STEPS="step_a
# step_b" runs just those steps.
#
# Usage: dev/accept-notifications-real-input.sh           (the acceptance)
#        dev/accept-notifications-real-input.sh ghost     (the blur left behind mid-drag; FAILS
#                                                          while shell-host leaves it)
#        dev/accept-notifications-real-input.sh clear     (9b alone)
#        dev/accept-notifications-real-input.sh exit      (9c alone)
#        dev/accept-notifications-real-input.sh repro     (the enter and unmap regression checks;
#                                                          the resting-pointer one FAILS while
#                                                          Blitz skips the enter)
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-notifications-real-input"
mkdir -p "$OUT_DIR"
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
SIZE=1920x1080
WIDTH=${SIZE%x*}
HEIGHT=${SIZE#*x}
# KWIN_SCALE=2 runs the output at twice the pixels and scale 2 (the user's 4K at 200 %):
# every coordinate here stays logical; captures are read back at their own density.
SCALE=${KWIN_SCALE:-1}
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
CACHE="$SCRATCH/cache"
mkdir -p "$RUNTIME" "$CONFIG/sill" "$STATE"
chmod 700 "$RUNTIME"
DAEMON=
INPUT=
MONITOR=
WL=
BUS=

in_env() {
  env XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$WL" DBUS_SESSION_BUS_ADDRESS="$BUS" \
    DBUS_SYSTEM_BUS_ADDRESS="$BUS" \
    XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" "$@"
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
  "$SHELL_HOST_DEV/kwin-virtual.sh" start "$(( WIDTH * SCALE ))x$(( HEIGHT * SCALE ))" \
    >"$OUT_DIR/kwin.log" 2>&1 || return 1
  local runtime socket
  runtime=$(sed -n 's/^export XDG_RUNTIME_DIR=//p' "$KWIN_ENV")
  socket=$(sed -n 's/^export WAYLAND_DISPLAY=//p' "$KWIN_ENV")
  BUS=$(sed -n "s/^export DBUS_SESSION_BUS_ADDRESS='\\(.*\\)'$/\\1/p" "$KWIN_ENV")
  WL="$runtime/$socket"
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $KWIN_ENV" >&2; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "KWin's bus is the session's" >&2; return 1; }
  require_private_system_bus "$BUS" || return 1
  [ "$SCALE" = 1 ] && return 0
  local output
  output=$(in_env kscreen-doctor -j 2>/dev/null | python3 -c '
import json, sys
print(json.load(sys.stdin)["outputs"][0]["name"])')
  in_env kscreen-doctor "output.$output.scale.$SCALE" >>"$OUT_DIR/kwin.log" 2>&1 || return 1
}

# A wallpaper of fine checks (2 px), seeded as the COSMIC background record sill's wallpaper reads:
# blur over it shows as flat grey where the checks were, which a flat wallpaper hides.
seed_checks() {
  local png="$OUT_DIR/checks.png" record="$CONFIG/cosmic/com.system76.CosmicBackground/v1"
  python3 - "$png" "$WIDTH" "$HEIGHT" "$SCALE" <<'EOF' || return 1
import sys
from PIL import Image
out, w, h, k = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
cell = 2 * k
img = Image.new("RGB", (w * k, h * k))
img.putdata([(40, 40, 40) if ((x // cell) + (y // cell)) % 2 else (220, 220, 220)
             for y in range(h * k) for x in range(w * k)])
img.save(out)
EOF
  mkdir -p "$record"
  cat >"$record/all" <<RON
(
    output: "all",
    source: Path("$png"),
    filter_by_theme: false,
    rotation_frequency: 3600,
    filter_method: Lanczos,
    scaling_mode: Zoom,
    sampling_method: Alphanumeric,
)
RON
  echo true >"$record/same-on-all"
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
# A move onto $1 $2: one motion (the enter hovers).
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
  seed_checks || { echo "no checks wallpaper" >&2; return 1; }
  use_fake_ddc "$SCRATCH" >/dev/null
  require_fake_ddc || return 1
  in_env "$SILL_BIN" run "${FAKE_DDC_FLAGS[@]}" >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  BAR="sill-bar-$NAME"
  BANNERS="sill-banners-$NAME"
  CENTER="sill-notification-center-$NAME"
  sleep 1.5
}

# Ask the daemon to quit and wait up to 10 s; never kill it.
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
k = img.width / 440
box = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
if box:
    box = [round(v / k) for v in box]
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
k = img.width / 440
print("yes" if img.getpixel((round(43 * k), round(43 * k)))[3] > 200 else "no")
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
  python3 - "$OUT_DIR/bar.png" "$WIDTH" <<'EOF'
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
    print(round((left + right) / 2 * int(sys.argv[2]) / w))
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
  echo "rubber-band at the top not exercised: wheel never stretches (quire's design/11 §11.3.7's last row" \
    "and its own 'no stretch at an edge' test); only a finger gesture does, and fake-input has" \
    "no finger source" >&2
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

# Repros of two shell-host gaps, `repro` mode: each step states what should happen and failed
# while the gap stood.
#
# The enter: one motion onto a card from outside the column hovers it. Failed: shell-host took the
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
  echo "enter: one motion onto the card: hovered $one; after a second motion: $two" >&2
  rest; clear_all
  [ "$one" = yes ]
}

# The unmap: after a card left under the pointer and the column unmapped, the next card is hovered
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
  echo "unmap: a card after the column unmapped under the pointer, one motion onto it:" \
    "hovered $hovered" >&2
  rest; clear_all
  [ "$hovered" = yes ]
}

# The resting pointer (quire; Blitz's `refresh_hover`): a card that slides in under a resting
# pointer is not hovered when the pointer then moves on it. Fails: after each layout Blitz
# re-resolves the hovered node against the last pointer position without dispatching pointerenter
# (its own TODO), so the card's node is already the hovered one when the motion comes and no enter
# ever reaches quire's `NotificationCard`: no pause, no close button until the pointer leaves the
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
  echo "resting pointer: a card slid in under the resting pointer, moved on: hovered $moved;" \
    "out and back: $back" >&2
  rest; clear_all
  [ "$moved" = yes ]
}

# The whole output as KWin composites it (blur, stale buffers and all), to $1: spectacle on the
# headless KWin's socket and private bus (KWin's ScreenShot2 answers only a client whose desktop
# file lists it, which spectacle's does). `sill debug capture` renders a surface offscreen and so
# cannot see what the compositor still draws for it.
screen_shot() {
  rm -f "$1"
  in_env QT_QPA_PLATFORM=wayland timeout 20 spectacle -b -n -f -o "$1" >/dev/null 2>&1
  [ -s "$1" ]
}

# How far the box "x0 y0 x1 y1" of shot $2 differs from shot $1: the largest per-pixel channel
# sum, and how many pixels differ by more than 24.
shot_diff() {
  python3 - "$1" "$2" "$3" "$WIDTH" <<'EOF'
import sys
from PIL import Image, ImageChops
a = Image.open(sys.argv[1]).convert("RGB")
b = Image.open(sys.argv[2]).convert("RGB")
k = a.width / int(sys.argv[4])
box = tuple(round(int(v) * k) for v in sys.argv[3].split())
d = ImageChops.difference(a.crop(box), b.crop(box))
sums = [r + g + bl for r, g, bl in d.get_flattened_data()]
print(max(sums), sum(1 for s in sums if s > 24))
EOF
}

# Wait up to 10 s for the checks wallpaper to show on the output (sill's wallpaper surface loads
# it after the daemon is up).
await_checks() {
  local png="$OUT_DIR/checks-up.png"
  for _ in $(seq 10); do
    screen_shot "$png" && python3 - "$png" <<'EOF' && return 0
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("L")
w, h = img.size
row = [img.getpixel((x, h // 2)) for x in range(w // 2, w // 2 + 64)]
sys.exit(0 if max(row) - min(row) > 100 else 1)
EOF
    sleep 1
  done
  echo "the checks wallpaper never showed" >&2
  return 1
}

# Ghosts: after a card leaves (by $1: click or swipe), the output where it stood is what it was
# before the card came, as KWin composites it. $2 names the captures.
ghost_check() {
  local how="$1" name="$2" box worst count
  rest || return 1
  wait_surface "$BANNERS" 50 down || { echo "a card from before is still up" >&2; return 1; }
  await_checks || return 1
  screen_shot "$OUT_DIR/$name-before.png" || { echo "no compositor shot" >&2; return 1; }
  case "$how" in
    click) post_card "Ghost $name" --app accept --body "Leaves nothing behind." \
      --action default=Open --expire never || return 1 ;;
    swipe) post_card "Ghost $name" --app accept --body "Leaves nothing behind." \
      --expire never || return 1 ;;
  esac
  screen_shot "$OUT_DIR/$name-up.png"
  box="$(( CARD_X0 - 8 )) $(( CARD_Y0 - 8 )) $(( CARD_X1 + 8 )) $(( CARD_Y1 + 8 ))"
  case "$how" in
    click) click "$CARD_X" "$CARD_Y" || return 1 ;;
    swipe) drag $(( CARD_X - 60 )) "$CARD_Y" 90 15 30 0 || return 1 ;;
  esac
  [ "$(await_hidden 3000)" -ge 0 ] || { echo "the column stayed up after the $how" >&2; return 1; }
  rest || return 1
  sleep 0.5
  screen_shot "$OUT_DIR/$name-after.png" || return 1
  read -r worst count <<<"$(shot_diff "$OUT_DIR/$name-before.png" "$OUT_DIR/$name-after.png" "$box")"
  echo "ghost $how: the card's box ($box) after it left vs before it came: worst $worst," \
    "$count px off (captures $name-{before,up,after}.png)" >&2
  [ "$count" = 0 ]
}

# Ghosts, mid-drag: with a real press held 60 px right of where it went down, nothing but the card
# is grey. The band the card has moved off (its old left 60 px, from its top to its bottom)
# must show the wallpaper as it was before the card came.
step_ghost_held() {
  local box worst count x y
  rest || return 1
  await_checks || return 1
  screen_shot "$OUT_DIR/ghost-held-before.png" || return 1
  post_card "Ghost held" --app accept --body "Dragged, and held." --expire never || return 1
  x=$(( CARD_X - 60 )); y=$CARD_Y
  point "$x" "$y" || return 1
  input "press left" "sleep 60" "move $(( x + 20 )) $y" "sleep 30" "move $(( x + 40 )) $y" \
    "sleep 30" "move $(( x + 60 )) $y" "sleep 400" || return 1
  screen_shot "$OUT_DIR/ghost-held-mid.png" || return 1
  input "release left" "sleep 200" || return 1
  box="$(( CARD_X0 + 8 )) $(( CARD_Y0 + 16 )) $(( CARD_X0 + 52 )) $(( CARD_Y1 - 16 ))"
  read -r worst count <<<"$(shot_diff "$OUT_DIR/ghost-held-before.png" "$OUT_DIR/ghost-held-mid.png" "$box")"
  echo "ghost held: the band the card moved off ($box) mid-drag vs before it came: worst $worst," \
    "$count px off (ghost-held-{before,mid}.png)" >&2
  rest; clear_all
  [ "$count" = 0 ]
}

# The center's Clear All, "x y" on the output: the rightmost run of ink in the head row of a
# capture of the center (the title is the leftmost), offset by nothing (the center's surface
# covers the output below the bar, so its capture's origin is the bar's height down).
clear_all_at() {
  python3 - "$1" "$(( WIDTH ))" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
k = img.width / int(sys.argv[2])
a = img.getchannel("A")
box = a.point(lambda v: 255 if v > 200 else 0).getbbox()
x0, y0, x1, y1 = box
top = y0 + round(12 * k)
band = range(top, top + round(28 * k))
lum = img.convert("L")
ground = sorted(lum.getpixel((x, top + round(14 * k))) for x in range(x0 + 20, x1 - 20))
ground = ground[len(ground) // 2]
def inked(x):
    return any(abs(lum.getpixel((x, y)) - ground) > 60 for y in band)
runs, run, gap = [], None, 0
for x in range(x1 - round(14 * k), x0, -1):
    if inked(x):
        run = [x, x] if run is None else [x, run[1]]
        gap = 0
    elif run is not None:
        gap += 1
        if gap > round(10 * k):
            runs.append(run); run = None
            break
if run is not None:
    runs.append(run)
left, right = runs[0]
print(round((left + right) / 2 / k), round((top + 14 * k) / k))
EOF
}

# The top of the sill window as large as capture $1 (logical px), from KWin's window list.
surface_top() {
  "$SHELL_HOST_DEV/kwin-windows.sh" --class sill 2>/dev/null | python3 -c '
import json, sys
from PIL import Image
img = Image.open(sys.argv[1]); k = img.width / int(sys.argv[2])
w, h = round(img.width / k), round(img.height / k)
for line in sys.stdin:
    f = json.loads(line)["frame"]
    if round(f[2]) == w and round(f[3]) == h:
        print(round(f[1])); sys.exit(0)
sys.exit(1)' "$1" "$WIDTH"
}

# 9b. Clear All with a real click in the center: the history empties and the center then rests
# at 0 frames.
step_center_clear_all() {
  local at x y bar c0 c1
  notify "Ada" --app chat --body "Hi" || return 1
  notify "Build" --app ci --body "Passed" || return 1
  sleep $(( (HOLD_MS + EXIT_MS + SLACK_MS) / 1000 + 1 ))
  wait_surface "$BANNERS" 50 down || { echo "the banners stayed up" >&2; return 1; }
  clock_click || return 1
  wait_surface "$CENTER" 20 up || { echo "a click on the clock opened nothing" >&2; return 1; }
  sleep 0.8
  mapped "$CENTER" "$OUT_DIR/clear-all-before.png" || { echo "no capture of the center" >&2; return 1; }
  at=$(clear_all_at "$OUT_DIR/clear-all-before.png") || { echo "no Clear All found" >&2; return 1; }
  read -r x y <<<"$at"
  bar=$(surface_top "$OUT_DIR/clear-all-before.png") || { echo "no center in KWin's list" >&2; return 1; }
  click "$x" $(( y + bar )) || return 1
  await_status kept 0 || { echo "a click at $x $(( y + bar )) cleared nothing" >&2; return 1; }
  sleep 1
  mapped "$CENTER" "$OUT_DIR/clear-all-after.png"
  c0=$(frames_of "$CENTER"); sleep 3; c1=$(frames_of "$CENTER")
  echo "a real click on Clear All at $x $(( y + bar )): kept 0; then 3 s idle:" \
    "$CENTER $((c1 - c0)) frames" >&2
  key Escape
  wait_surface "$CENTER" 20 down || { echo "real Escape after Clear All left the center up" >&2; return 1; }
  echo "real Escape then closed it" >&2
  [ "$((c1 - c0))" = 0 ]
}

# The first group header's Clear in capture $1 of the center, "x y" on the surface (logical):
# the rows of ink in the panel grouped into bands (the title row with Clear All first, the first
# group's header next), and in the header's band the rightmost run of ink, which is "Clear".
# Also prints the header band's top as a third number.
header_clear_at() {
  python3 - "$1" "$WIDTH" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
k = img.width / int(sys.argv[2])
x0, y0, x1, y1 = img.getchannel("A").point(lambda v: 255 if v > 200 else 0).getbbox()
lum = img.convert("L")
left, right = x0 + round(20 * k), x1 - round(14 * k)
def ground(y):
    row = sorted(lum.getpixel((x, y)) for x in range(left, right, 2))
    return row[len(row) // 2]
def inked_row(y):
    g = ground(y)
    return any(abs(lum.getpixel((x, y)) - g) > 60 for x in range(left, right, 2))
bands, start, gap = [], None, 0
for y in range(y0 + round(8 * k), y1):
    if inked_row(y):
        start = y if start is None else start
        gap = 0
    elif start is not None:
        gap += 1
        if gap > round(4 * k):
            bands.append((start, y - gap)); start = None
    if len(bands) == 2:
        break
top, bottom = bands[1]
g = ground((top + bottom) // 2)
def inked(x):
    return any(abs(lum.getpixel((x, y)) - g) > 60 for y in range(top, bottom + 1))
run, gap = None, 0
for x in range(right, left, -1):
    if inked(x):
        run = [x, x] if run is None else [x, run[1]]
        gap = 0
    elif run is not None:
        gap += 1
        if gap > round(10 * k):
            break
print(round((run[0] + run[1]) / 2 / k), round((top + bottom) / 2 / k), round(top / k))
EOF
}

settles() { grep -c "notification_center.settled=" "$OUT_DIR/daemon.log"; }

# Click "x y" (surface px, the surface's top $3 down) and prove the exit plays: the history
# falls to $4 kept, the rows the model dropped settle ($5 of them, logged by the list as each
# batch settles, after its fold), the center draws frames while they fold and heal, and then
# rests at 0 frames. Captures: $6-{mid,after}.png.
exit_plays() {
  local x="$1" y="$2" top="$3" kept="$4" rows="$5" name="$6" s0 c0 c1 c2 start dropped settled
  s0=$(settles); c0=$(frames_of "$CENTER")
  start=$(now_ms)
  click "$x" $(( y + top )) || return 1
  await_status kept "$kept" || { echo "a click at $x $(( y + top )) cleared nothing" >&2; return 1; }
  dropped=$(since "$start")
  mapped "$CENTER" "$OUT_DIR/$name-mid.png"
  for _ in $(seq 50); do [ "$(settles)" -ge $(( s0 + rows )) ] && break; sleep 0.1; done
  settled=$(since "$start")
  [ "$(settles)" -ge $(( s0 + rows )) ] || { echo "the list settled $(( $(settles) - s0 )) of $rows rows" >&2; return 1; }
  sleep 1
  c1=$(frames_of "$CENTER")
  mapped "$CENTER" "$OUT_DIR/$name-after.png"
  sleep 3
  c2=$(frames_of "$CENTER")
  echo "  the model dropped them ${dropped} ms after the click; the list settled $rows rows by" \
    "${settled} ms; $((c1 - c0)) frames while they folded and healed; then 3 s idle: $((c2 - c1))" \
    "frames" >&2
  grep "notification_center.settled=" "$OUT_DIR/daemon.log" | tail -n "$rows" | sed 's/^/  /' >&2
  [ "$((c1 - c0))" -ge 5 ] && [ "$((c2 - c1))" = 0 ]
}

# 9c. Clear and Clear All play their exit: two groups posted, the center opened
# by a real click on the clock, a real click on the first group's Clear folds its head and card
# and the group below heals up; a real click on Clear All folds the rest; the center then rests.
step_center_clear_exit() {
  local at x y bar before after
  [ "$(status_field kept)" = 0 ] || { echo "the history is not empty before the step" >&2; return 1; }
  notify "Build" --app ci --body "Passed" || return 1
  notify "Ada" --app chat --body "Hi" || return 1
  notify "Ada" --app chat --body "Still there?" || return 1
  sleep $(( (HOLD_MS + EXIT_MS + SLACK_MS) / 1000 + 1 ))
  wait_surface "$BANNERS" 50 down || { echo "the banners stayed up" >&2; return 1; }
  wait_surface "$CENTER" 30 down || { echo "the center was still up" >&2; return 1; }
  clock_click || return 1
  wait_surface "$CENTER" 20 up || { echo "a click on the clock opened nothing" >&2; return 1; }
  sleep 1
  mapped "$CENTER" "$OUT_DIR/exit-before.png" || { echo "no capture of the center" >&2; return 1; }
  bar=$(surface_top "$OUT_DIR/exit-before.png") || { echo "no center in KWin's list" >&2; return 1; }
  at=$(header_clear_at "$OUT_DIR/exit-before.png") || { echo "no group header found" >&2; return 1; }
  read -r x y before <<<"$at"
  echo "a real click on Chat's Clear at $x $(( y + bar )):" >&2
  exit_plays "$x" "$y" "$bar" 1 2 exit-clear || return 1
  # CI's header healed up into the first header's place.
  at=$(header_clear_at "$OUT_DIR/exit-clear-after.png") || { echo "no header after the Clear" >&2; return 1; }
  read -r _ _ after <<<"$at"
  echo "  the first header's top: $before before, $after after (CI healed into Chat's place)" >&2
  [ $(( after - before )) -le 1 ] && [ $(( before - after )) -le 1 ] || return 1
  at=$(clear_all_at "$OUT_DIR/exit-clear-after.png") || { echo "no Clear All found" >&2; return 1; }
  read -r x y <<<"$at"
  echo "a real click on Clear All at $x $(( y + bar )):" >&2
  exit_plays "$x" "$y" "$bar" 0 2 exit-clear-all || return 1
  key Escape
  wait_surface "$CENTER" 20 down || { echo "real Escape after Clear All left the center up" >&2; return 1; }
  echo "real Escape then closed the center" >&2
}

# 5b, 5c.
step_ghost_click() { ghost_check click ghost-click; }
step_ghost_swipe() { ghost_check swipe ghost-swipe; }

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
  step_ghost_click step_ghost_swipe step_center_clock_escape step_center_outside
  step_center_delete step_center_clear_all step_center_clear_exit step_center_scroll step_idle
  step_holder"
[ "${1:-}" = ghost ] && STEPS="step_holder step_ghost_held step_ghost_click step_ghost_swipe"
[ "${1:-}" = clear ] && STEPS="step_holder step_center_clear_all"
[ "${1:-}" = exit ] && STEPS="step_holder step_center_clear_exit step_idle"
[ -n "${ACCEPT_STEPS:-}" ] && STEPS="$ACCEPT_STEPS"
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

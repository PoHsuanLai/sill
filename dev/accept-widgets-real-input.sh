#!/usr/bin/env bash
# Widgets with real input at scale 2, on shell-host's private headless KWin
# (scripts/dev/kwin-virtual.sh, 3840 x 2160, its one output set to scale 2 with kscreen-doctor on
# that KWin's own socket: the BenQ MA270U's 4K at 2x, the user's session) driven by shell-host's
# scripts/dev/fake-input.sh: every click and key is a real Wayland event with a real serial and
# keyboard focus. Nothing here reaches the user's session (wayland-0), the nested compositor
# (wayland-1) or the real session or system bus: the daemon runs from this checkout's debug build
# (`--features debug`) on the headless KWin's private bus, which stands in for BOTH the daemon's
# session and system bus (start_kwin refuses to go on if it would be the real one), with its own
# XDG_RUNTIME_DIR (WAYLAND_DISPLAY an absolute socket path) and scratch XDG_CONFIG_HOME,
# XDG_STATE_HOME and XDG_CACHE_HOME. ONE fake-input client serves the whole run. UPower and BlueZ
# are simply absent on the private bus, so the Batteries widget always reads "No batteries" here:
# every step below only checks the widget's structure (card count and size), never a real device's
# name or level.
#
#   1. Scale: kscreen-doctor reports the output at scale 2, and the bar's capture is 2x.
#   2. The desktop at scale 2 (`widgets.desktop = "on"`, the default Calendar and Batteries):
#      two frames, each one 164 px cell (328 device px) square at the top right; the Calendar
#      and the Batteries cropped out (dev/out/widgets-kwin-s2-calendar.png,
#      widgets-kwin-s2-battery.png) and the layer over a plain backdrop
#      (dev/out/widgets-kwin-s2-desktop.png).
#   3. A real click on the bar's clock opens the notification center: three tiles, each Medium
#      (164 tall: the month is Medium in the center), "Edit Widgets" at their foot
#      (dev/out/widgets-kwin-s2-center.png).
#   4. A real click on "Edit Widgets": the center unmaps and Edit Widgets maps on the output as
#      a sheet at the bottom, centred, 1040 x 400 (dev/out/widgets-kwin-s2-edit.png).
#   4b. A real click on "World Clock" in the gallery's list: one preview, at Medium (344 wide),
#      and no size control (widgets-kwin-s2-one-size.png).
#   5. A real click on "Add to Desktop": the button says "Added" with its check (captured
#      at once, widgets-kwin-s2-added.png), the daemon writes `world_clock` into
#      `widgets.desktop_widgets`, and the card rises on the desktop above the sheet: the layer
#      paints the rise, then shows three frames (widgets-kwin-s2-desktop-added.png).
#   6. A real click on the Batteries row's "Remove" in the Desktop list: `battery` leaves
#      `widgets.desktop_widgets`, its card plays its exit (the layer paints it), and the
#      layer shows two frames, the Calendar and the clock (widgets-kwin-s2-desktop-removed.png,
#      widgets-kwin-s2-removed.png).
#   7. A real Escape closes Edit Widgets (it unmaps).
#   8. 5 s idle: 0 frames on the desktop layer.
#
# Then the input client, the daemon (`sill quit`) and the headless KWin are stopped.
# Exit 0 only when every step passed; 1 on a failure.
#
# Usage: dev/accept-widgets-real-input.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-widgets-real-input"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
SIZE=3840x2160
SCALE=2

SCRATCH=$(mktemp -d)
RUNTIME="$SCRATCH/run"
CONFIG="$SCRATCH/config"
STATE="$SCRATCH/state"
CACHE="$SCRATCH/cache"
mkdir -p "$RUNTIME" "$CONFIG/sill" "$STATE" "$CACHE"
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

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

# The headless KWin, and its socket (absolute) and bus from its env file, read without sourcing
# it into this shell: `kwin-virtual.sh stop` must never find this script naming the bus.
start_kwin() {
  if [ -f "$KWIN_ENV" ]; then
    echo "a headless KWin is already running ($KWIN_ENV); not sharing it" >&2
    return 1
  fi
  "$SHELL_HOST_DEV/kwin-virtual.sh" start "$SIZE" >"$OUT_DIR/kwin.log" 2>&1 || return 1
  local runtime socket
  runtime=$(sed -n 's/^export XDG_RUNTIME_DIR=//p' "$KWIN_ENV")
  socket=$(sed -n 's/^export WAYLAND_DISPLAY=//p' "$KWIN_ENV")
  BUS=$(sed -n "s/^export DBUS_SESSION_BUS_ADDRESS='\\(.*\\)'$/\\1/p" "$KWIN_ENV")
  WL="$runtime/$socket"
  OUR_SCOPE=$(sed -n 's/^\(export \)\{0,1\}KWIN_SCOPE=//p' "$KWIN_ENV")
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $KWIN_ENV" >&2; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "KWin's bus is the session's" >&2; return 1; }
  require_private_system_bus "$BUS" || return 1
}

# Stop our KWin only: kwin-virtual.sh keeps one env file, so another session's start may have
# replaced it since; then our scope is stopped directly and theirs is left alone.
stop_kwin() {
  if [ -n "$WL" ] && [ -f "$KWIN_ENV" ] && grep -q "${WL##*/}" "$KWIN_ENV"; then
    "$SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1
  elif [ -n "${OUR_SCOPE:-}" ]; then
    systemctl --user stop "$OUR_SCOPE" >>"$OUT_DIR/kwin.log" 2>&1
  fi
}

# The headless output's name and scale, from kscreen-doctor on that KWin's socket.
kscreen() { in_env kscreen-doctor "$@" 2>&1 | sed 's/\x1b\[[0-9;]*m//g'; }
output_of_kwin() { kscreen -o | awk '/^Output:/{print $3; exit}'; }
scale_of_kwin() { kscreen -o | awk '/Scale:/{print $2; exit}'; }

set_scale() {
  local out
  out=$(output_of_kwin)
  [ -n "$out" ] || { echo "kscreen-doctor names no output" >&2; return 1; }
  kscreen "output.$out.scale.$SCALE" >>"$OUT_DIR/kscreen.log"
  for _ in $(seq 30); do [ "$(scale_of_kwin)" = "$SCALE" ] && return 0; sleep 0.1; done
  echo "the output's scale is $(scale_of_kwin), not $SCALE" >&2
  return 1
}

start_input() {
  grep -q "${WL##*/}" "$KWIN_ENV" 2>/dev/null || { echo "$KWIN_ENV no longer names our KWin" >&2; return 1; }
  rm -f "$OUT_DIR/input.fifo"; mkfifo "$OUT_DIR/input.fifo" || return 1
  "$SHELL_HOST_DEV/fake-input.sh" <"$OUT_DIR/input.fifo" >"$OUT_DIR/input.log" 2>&1 &
  INPUT=$!
  exec 3>"$OUT_DIR/input.fifo"
}

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

# The fake-input commands "$@", one per line; returns once each has been answered.
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
click() { input "move $1 $2" "sleep 120" "press left" "sleep 40" "release left" "sleep 250"; }

start_daemon() {
  printf '[widgets]\ndesktop = "on"\n[notifications]\nserver = "passive"\n' \
    >"$CONFIG/sill/settings.toml"
  use_fake_ddc "$SCRATCH" >/dev/null
  require_fake_ddc || return 1
  in_env "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  BAR="sill-bar-$NAME"
  CENTER="sill-notification-center-$NAME"
  LAYER="sill-desktop-widgets-$NAME"
  EDIT="sill-edit-widgets-$NAME"
  sleep 3
}

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

# SURFACE's counters now: "frames callbacks commits skipped".
counters_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"], row["callbacks"], row["commits"], row["skipped"])' "$1"
}

# SURFACE's counters every 100 ms for about 1.5 s, one line each, into FILE (in the background
# while a motion plays): the frames it paints over time. The headless KWin at 4K presents the
# layer about every 150-200 ms here, so a 280 ms motion is two or three frames.
sample_counters() {
  local surface="$1" file="$2"
  : >"$file"
  for _ in $(seq 15); do
    echo "$(date +%s.%N | cut -c1-14) $(counters_of "$surface")" >>"$file"
    sleep 0.1
  done
}

# A capture of SURFACE into FILE. A debug build paints a 4K surface in more than the IPC's 2 s
# reply time, so a timed-out request still writes the file: wait for it. An unmapped surface
# fails at once ("not presented").
capture() {
  local surface="$1" file="$2" said
  rm -f "$file"
  said=$(sill debug capture "$surface" "$file" 2>&1)
  case "$said" in *"not been presented"*|*"no surface"*|*"unknown surface"*) return 1 ;; esac
  for _ in $(seq 100); do
    [ -s "$file" ] && python3 -c 'import sys; from PIL import Image; Image.open(sys.argv[1]).load()' \
      "$file" 2>/dev/null && return 0
    sleep 0.1
  done
  return 1
}

# Whether SURFACE is mapped (a capture succeeds).
mapped() { capture "$1" "$OUT_DIR/probe.png"; }

wait_surface() {
  local surface="$1" tries="$2" want="$3"
  for _ in $(seq "$tries"); do
    if mapped "$surface"; then [ "$want" = up ] && return 0
    else [ "$want" = down ] && return 0; fi
    sleep 0.2
  done
  return 1
}

# The bar's height in logical px (every Auto-exclusive surface starts under it).
bar_height() {
  python3 -c 'import sys; from PIL import Image; print(round(Image.open(sys.argv[1]).size[1] / float(sys.argv[2])))' \
    "$OUT_DIR/bar.png" "$SCALE"
}

# The opaque blobs of a capture (the desktop layer's cards), "L T R B" each in device px.
frames() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
# The desktop card is see-through (the Widget material's .48 tint): any real coverage counts.
a = img.getchannel("A").point(lambda v: 255 if v > 40 else 0)
w, h = img.size
cols = [x for x in range(0, w) if a.crop((x, 0, x + 1, h)).getbbox()]
bands, band = [], None
for x in cols:
    if band and x - band[1] <= 2:
        band[1] = x
    else:
        band and bands.append(band); band = [x, x]
band and bands.append(band)
for l, r in bands:
    strip = a.crop((l, 0, r + 1, h))
    run = None
    for y in range(h):
        on = strip.crop((0, y, r + 1 - l, y + 1)).getbbox() is not None
        if on and run is None: run = [y, y]
        elif on: run[1] = y
        elif run is not None:
            print(l, run[0], r, run[1]); run = None
    if run is not None: print(l, run[0], r, run[1])
EOF
}

# The kinds in `widgets.desktop_widgets` in the scratch settings, space-separated.
desktop_kinds() {
  python3 -c '
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    print(" ".join(tomllib.load(f).get("widgets", {}).get("desktop_widgets", ["calendar", "battery"])))' \
    "$CONFIG/sill/settings.toml"
}

# A capture over a plain backdrop, cropped to BOX ("L T R B", device px), into FILE.
backdrop_crop() {
  python3 - "$1" "$2" "$3" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
l, t, r, b = map(int, sys.argv[3].split())
ground = Image.new("RGBA", img.size, (74, 98, 138, 255))
ground.alpha_composite(img)
ground.crop((max(0, l), max(0, t), min(img.size[0], r), min(img.size[1], b))).save(sys.argv[2])
EOF
}

# 1.
step_scale() {
  local scale shot
  scale=$(scale_of_kwin)
  capture "$BAR" "$OUT_DIR/bar.png" || { echo "no bar capture" >&2; return 1; }
  shot=$(python3 -c 'import sys; from PIL import Image; print(*Image.open(sys.argv[1]).size)' "$OUT_DIR/bar.png")
  TOP=$(bar_height)
  echo "kscreen-doctor: scale $scale; the bar's capture is $shot device px ($TOP logical tall)" >&2
  [ "$scale" = "$SCALE" ] && [ "${shot%% *}" = 3840 ]
}

# 2.
step_desktop() {
  local cal bat ok=0
  capture "$LAYER" "$OUT_DIR/desktop.png" || { echo "no desktop layer" >&2; return 1; }
  frames "$OUT_DIR/desktop.png" >"$OUT_DIR/desktop-frames.txt"
  cal=$(sed -n 1p "$OUT_DIR/desktop-frames.txt"); bat=$(sed -n 2p "$OUT_DIR/desktop-frames.txt")
  echo "desktop at scale 2: $(wc -l <"$OUT_DIR/desktop-frames.txt") frames: Calendar $cal, Batteries $bat (device px)" >&2
  backdrop_crop "$OUT_DIR/desktop.png" "$SHOTS/widgets-kwin-s2-desktop.png" "3000 0 3840 900"
  read -r l t r b <<<"$cal"
  backdrop_crop "$OUT_DIR/desktop.png" "$SHOTS/widgets-kwin-s2-calendar.png" "$((l - 16)) $((t - 16)) $((r + 16)) $((b + 16))"
  read -r l t r b <<<"$bat"
  backdrop_crop "$OUT_DIR/desktop.png" "$SHOTS/widgets-kwin-s2-battery.png" "$((l - 16)) $((t - 16)) $((r + 16)) $((b + 16))"
  for f in "$cal" "$bat"; do
    python3 -c '
import sys
l, t, r, b = map(int, sys.argv[1].split())
# One 164 px cell at 2x, give or take the card rim (quire card 328 device px).
sys.exit(0 if abs(r - l + 1 - 328) <= 6 and abs(b - t + 1 - 328) <= 6 else 1)' "$f" || ok=1
  done
  [ "$(wc -l <"$OUT_DIR/desktop-frames.txt")" = 2 ] && [ "$ok" = 0 ]
}

# The clock's centre in logical px: the rightmost run of ink in the bar's capture.
clock_at() {
  python3 - "$OUT_DIR/bar.png" "$SCALE" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
scale = float(sys.argv[2])
w, h = img.size
ground = img.getpixel((w // 2, 2))
def ink(x):
    return any(sum(abs(p - q) for p, q in zip(img.getpixel((x, y)), ground)) > 120 for y in range(h // 4, 3 * h // 4))
runs, run = [], None
for x in range(w - 1, w // 2, -1):
    if ink(x):
        run = [x, x] if run is None else [x, run[1]]
    elif run is not None and run[1] - x > 24:
        runs.append(run); run = None
        break
if run is not None: runs.append(run)
l, r = runs[0]
print(round((l + r) / 2 / scale), round(h / 2 / scale))
EOF
}

# The center's tiles, "TOP BOTTOM" device px each, and the Edit Widgets text's centre
# ("X Y" logical, on the surface) below the last.
center_layout() {
  python3 - "$OUT_DIR/center.png" "$SCALE" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
scale = float(sys.argv[2])
left, top, right, bottom = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
outside, inside = left + round(5 * scale), left + round(24 * scale)
def off(y):
    a, b = img.getpixel((outside, y)), img.getpixel((inside, y))
    return sum(abs(p - q) for p, q in zip(a[:3], b[:3])) > 6
runs, run = [], None
for y in range(top + round(60 * scale), bottom - 4):
    if off(y):
        run = [y, y] if run is None else [run[0], y]
    elif run is not None:
        if run[1] - run[0] > 12 * scale: runs.append(run)
        run = None
for r in runs:
    print("tile", *r)
# "Edit Widgets": the first row of ink under the last tile, in the panel's middle.
mid = (left + right) // 2
ground = img.getpixel((mid, runs[-1][1] + round(8 * scale)))
ys = []
for y in range(runs[-1][1] + round(4 * scale), min(bottom, runs[-1][1] + round(80 * scale))):
    if any(sum(abs(p - q) for p, q in zip(img.getpixel((x, y))[:3], ground[:3])) > 90
           for x in range(mid - round(60 * scale), mid + round(60 * scale), 2)):
        ys.append(y)
if ys:
    print("edit", round(mid / scale), round((ys[0] + ys[-1]) / 2 / scale))
EOF
}

# 3.
step_center() {
  local cx cy tiles
  read -r cx cy <<<"$(clock_at)"
  echo "the clock at ($cx, $cy); a real click on it" >&2
  click "$cx" "$cy" || return 1
  wait_surface "$CENTER" 25 up || { echo "the center never mapped" >&2; return 1; }
  sleep 1.2
  capture "$CENTER" "$OUT_DIR/center.png" || return 1
  backdrop_crop "$OUT_DIR/center.png" "$SHOTS/widgets-kwin-s2-center.png" "2980 0 3840 1960"
  center_layout >"$OUT_DIR/center-layout.txt"
  tiles=$(grep -c '^tile' "$OUT_DIR/center-layout.txt")
  echo "the center: $tiles tiles ($(grep '^tile' "$OUT_DIR/center-layout.txt" | awk '{print $2"-"$3}' | tr '\n' ' ')device px); Edit Widgets at $(grep '^edit' "$OUT_DIR/center-layout.txt" | cut -d' ' -f2-)" >&2
  [ "$tiles" = 3 ] || return 1
  # Each tile is Medium: 164 logical px tall.
  grep '^tile' "$OUT_DIR/center-layout.txt" | awk -v s="$SCALE" '{h=($3-$2+1)/s; if (h < 150 || h > 176) bad=1} END {exit bad}'
}

# 4.
step_open_edit() {
  local ex ey
  read -r ex ey <<<"$(grep '^edit' "$OUT_DIR/center-layout.txt" | cut -d' ' -f2-)"
  [ -n "${ey:-}" ] || { echo "no Edit Widgets under the tiles" >&2; return 1; }
  echo "a real click on Edit Widgets at ($ex, $(( ey + TOP )))" >&2
  click "$ex" $(( ey + TOP )) || return 1
  wait_surface "$EDIT" 25 up || { echo "Edit Widgets never mapped" >&2; return 1; }
  wait_surface "$CENTER" 25 down || { echo "the center is still up" >&2; return 1; }
  sleep 1.5
  capture "$EDIT" "$OUT_DIR/edit.png" || return 1
  # The sheet's plate ("L T R B" logical, on the surface) and the surface's own size.
  PANEL=$(python3 -c '
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
l, t, r, b = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
s = float(sys.argv[2])
print(round(l / s), round(t / s), round(r / s), round(b / s), round(img.size[0] / s), round(img.size[1] / s))' \
    "$OUT_DIR/edit.png" "$SCALE")
  backdrop_crop "$OUT_DIR/edit.png" "$SHOTS/widgets-kwin-s2-edit.png" "0 0 3840 2160"
  echo "Edit Widgets mapped, the center unmapped; the sheet spans $PANEL (L T R B, then the surface's W H; logical)" >&2
  # A sheet at the bottom, centred, 1040 wide, 400 tall, `--s-8` above the bottom.
  python3 -c '
import sys
l, t, r, b, w, h = map(int, sys.argv[1].split())
ok = abs(b - (h - 8)) <= 2 and abs((l + r) / 2 - w / 2) <= 2 and abs(r - l - 1040) <= 4 and abs(b - t - 400) <= 4
sys.exit(0 if ok else 1)' "$PANEL" || { echo "the sheet is not at the bottom, 1040 x 400" >&2; return 1; }
}

# The sheet's parts, in logical px from its plate's top-left corner (quire's gallery in the
# 1040 x 400 sheet, laid out by a Harness at this output's logical size): the list's second
# entry (World Clock) at (+112, +170), "Add to Desktop" under the preview at (+382, +311), the
# preview's card from +120 to +284 down (a row through it at +135, 344 wide at Medium), and the
# Desktop list's second row's (the Batteries') Remove at (+991, +132). quire 5c3fb6db's sizes.
panel_left() { echo "${PANEL%% *}"; }
panel_top() { echo "$PANEL" | cut -d' ' -f2; }

# A capture's crop, BOX "L T R B" in logical px on the surface, into FILE, over a plain ground.
logical_crop() {
  local l t r b
  read -r l t r b <<<"$3"
  backdrop_crop "$1" "$2" "$(( l * SCALE )) $(( t * SCALE )) $(( r * SCALE )) $(( b * SCALE ))"
}

# 4b. World Clock picked, the gallery shows one preview, at Medium (344 wide), and no
# size control. The automated check reads the preview row: one run of card across the detail
# plate, 344 +- 6 wide, a row through its top above the dials; the capture shows the rest.
step_one_size() {
  local left top run
  left=$(panel_left); top=$(panel_top)
  click $(( left + 112 )) $(( top + 170 + TOP )) || return 1
  sleep 1.5
  capture "$EDIT" "$OUT_DIR/edit-one-size.png" || return 1
  logical_crop "$OUT_DIR/edit-one-size.png" "$SHOTS/widgets-kwin-s2-one-size.png" \
    "$(( left - 10 )) $(( top - 10 )) $(( left + 1050 )) $(( top + 410 ))"
  run=$(python3 - "$OUT_DIR/edit-one-size.png" "$SCALE" "$left" "$top" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
s, left, top = float(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
y = round((top + 135) * s)
x0, x1 = round((left + 232) * s), round((left + 724) * s)
ground = img.getpixel((x0 + 4, y))
off = [x for x in range(x0, x1) if sum(abs(p - q) for p, q in zip(img.getpixel((x, y)), ground)) > 6]
runs, run = [], None
for x in off:
    if run and x - run[1] <= 40: run[1] = x
    else:
        run and runs.append(run); run = [x, x]
run and runs.append(run)
print(len(runs), *[round((r - l + 1) / s) for l, r in runs])
EOF
)
  echo "the preview row: $run (runs of card, then each one's width, logical)" >&2
  python3 -c '
import sys
n, *w = map(int, sys.argv[1].split())
sys.exit(0 if n == 1 and abs(w[0] - 344) <= 6 else 1)' "$run"
}

# 5. A real click on "Add to Desktop": the button says "Added" with its check (captured at
# once, inside quire's 900 ms hold), the daemon writes `world_clock`, and the card rises into
# its place on the desktop above the sheet: the layer paints the rise (frames), then 3 frames.
step_add() {
  local left top before after frames_now f0 f1
  left=$(panel_left); top=$(panel_top)
  before=$(desktop_kinds)
  f0=$(frames_of "$LAYER")
  sample_counters "$LAYER" "$OUT_DIR/rise-frames.txt" &
  local sampler=$!
  click $(( left + 382 )) $(( top + 311 + TOP )) || return 1
  capture "$EDIT" "$OUT_DIR/edit-added.png" || return 1
  for _ in $(seq 30); do after=$(desktop_kinds); [ "$after" != "$before" ] && break; sleep 0.2; done
  sleep 2
  wait "$sampler"
  f1=$(frames_of "$LAYER")
  logical_crop "$OUT_DIR/edit-added.png" "$SHOTS/widgets-kwin-s2-added.png" \
    "$(( left - 10 )) $(( top - 10 )) $(( left + 1050 )) $(( top + 410 ))"
  logical_crop "$OUT_DIR/edit-added.png" "$OUT_DIR/added-button.png" \
    "$(( left + 300 )) $(( top + 290 )) $(( left + 456 )) $(( top + 346 ))"
  logical_crop "$OUT_DIR/edit-one-size.png" "$OUT_DIR/add-button.png" \
    "$(( left + 300 )) $(( top + 290 )) $(( left + 456 )) $(( top + 346 ))"
  capture "$LAYER" "$OUT_DIR/desktop-added.png" || return 1
  frames "$OUT_DIR/desktop-added.png" >"$OUT_DIR/desktop-added.txt"
  frames_now=$(wc -l <"$OUT_DIR/desktop-added.txt")
  backdrop_crop "$OUT_DIR/desktop-added.png" "$SHOTS/widgets-kwin-s2-desktop-added.png" "2400 0 3840 1000"
  grep -o 'edit_widgets.write=.*' "$OUT_DIR/daemon.log" | tail -1 >"$OUT_DIR/added-write.txt"
  local changed
  changed=$(python3 -c '
import sys
from PIL import Image, ImageChops
a = Image.open(sys.argv[1]).convert("RGB"); b = Image.open(sys.argv[2]).convert("RGB")
d = ImageChops.difference(a, b).convert("L").point(lambda v: 255 if v > 40 else 0)
print(sum(1 for v in d.getdata() if v))' "$OUT_DIR/add-button.png" "$OUT_DIR/added-button.png")
  echo "added: desktop_widgets '$before' -> '$after'; the button's pixels changed: $changed; the layer painted $(( f1 - f0 )) frames (the rise: a settings change alone paints 1; over time in rise-frames.txt) and shows $frames_now; $(cat "$OUT_DIR/added-write.txt")" >&2
  [[ "$after" == *world_clock* ]] && [ "$frames_now" = 3 ] && grep -q 'shown: On' "$OUT_DIR/added-write.txt" \
    && [ "$changed" -gt 200 ] && [ $(( f1 - f0 )) -ge 3 ]
}

# 6. A real click on the Batteries row's Remove: `battery` leaves the key, its card plays its
# exit where it stood (the layer paints it: frames), and then the layer shows 2 frames.
step_remove() {
  local left top before after frames_now f0 f1
  left=$(panel_left); top=$(panel_top)
  before=$(desktop_kinds)
  f0=$(frames_of "$LAYER")
  sample_counters "$LAYER" "$OUT_DIR/exit-frames.txt" &
  local sampler=$!
  click $(( left + 991 )) $(( top + 132 + TOP )) || return 1
  for _ in $(seq 30); do after=$(desktop_kinds); [ "$after" != "$before" ] && break; sleep 0.2; done
  sleep 2
  wait "$sampler"
  f1=$(frames_of "$LAYER")
  capture "$LAYER" "$OUT_DIR/desktop-removed.png" || return 1
  frames "$OUT_DIR/desktop-removed.png" >"$OUT_DIR/desktop-removed.txt"
  frames_now=$(wc -l <"$OUT_DIR/desktop-removed.txt")
  backdrop_crop "$OUT_DIR/desktop-removed.png" "$SHOTS/widgets-kwin-s2-desktop-removed.png" "2400 0 3840 1000"
  capture "$EDIT" "$OUT_DIR/edit-removed.png" && \
    logical_crop "$OUT_DIR/edit-removed.png" "$SHOTS/widgets-kwin-s2-removed.png" \
      "$(( left - 10 )) $(( top - 10 )) $(( left + 1050 )) $(( top + 410 ))"
  echo "removed: desktop_widgets '$before' -> '$after'; the layer painted $(( f1 - f0 )) frames (the exit: its start and its drop paint 2, so more is the exit playing; over time in exit-frames.txt) and shows $frames_now" >&2
  [[ "$after" != *battery* ]] && [[ "$after" == *world_clock* ]] && [ "$frames_now" = 2 ] \
    && [ $(( f1 - f0 )) -ge 3 ]
}

# 7.
step_escape() {
  key Escape || return 1
  wait_surface "$EDIT" 25 down || { echo "Edit Widgets is still up after Escape" >&2; return 1; }
  echo "a real Escape closed Edit Widgets" >&2
}

# 8.
step_idle() {
  local f0 f1
  sleep 1
  f0=$(frames_of "$LAYER")
  sleep 5
  f1=$(frames_of "$LAYER")
  echo "5 s idle: $(( f1 - f0 )) frames on $LAYER" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

cleanup() {
  stop_input
  stop_daemon
  stop_kwin
  rm -rf "$SCRATCH"
}

build || exit 1
start_kwin || { stop_kwin; exit 1; }
set_scale || { stop_kwin; exit 1; }
start_input || { cleanup; exit 1; }
start_daemon || { cleanup; exit 1; }
status=0
for step in step_scale step_desktop step_center step_open_edit step_one_size step_add step_remove \
  step_escape step_idle; do
  if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; fi
done
cleanup
echo "logs and captures in $OUT_DIR; screenshots $SHOTS/widgets-kwin-s2-*.png"
exit "$status"

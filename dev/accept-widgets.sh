#!/usr/bin/env bash
# Widgets acceptance (design/20 §1.14, FINDINGS "M10 freeze" F404/F405, lane b), on the nested
# cosmic-comp (dev/nested.sh; a running wayland-1 is reused and never stopped). Starts `sill run`
# from this checkout's debug build (`--features debug`) with a scratch XDG_CONFIG_HOME,
# XDG_STATE_HOME and XDG_CACHE_HOME, under a private session bus (dev/private-bus.conf: no
# service directories, so no UPower, BlueZ or real MPRIS player is ever reached) holding one
# fake MPRIS player (sill-services' `mpris_player` example), and with its own XDG_RUNTIME_DIR
# (the nested socket as an absolute WAYLAND_DISPLAY). The daemon is quit at the end.
#
#   1. With the default `widgets.center` (Calendar, Up Next, Battery), `sill notifications open`:
#      a capture of the center shows three widget tiles under the history (Calendar and Up Next
#      are empty until lane a is merged; Battery says "No batteries": the private bus has no
#      UPower and no BlueZ). Screenshot dev/out/widgets-center-cosmic.png.
#   2. `widgets.center` = all five and two world clocks, `sill reload`: five tiles, Now Playing
#      showing the fake player (dev/out/widgets-center-all-cosmic.png).
#   3. 5 s idle while open, clear of the minute's turn: 0 frames on the center.
#   4. `sill widgets toggle-desktop`: `sill-desktop-widgets-<out>` appears in `sill debug stats`,
#      `widgets.desktop = "on"` in the file, and a capture of the layer shows two frames
#      (Calendar and Battery) at the top right with nothing painted between them; a COSMIC
#      Terminal opened in the nested session beside them (dev/out/widgets-desktop-cosmic.png:
#      the layer over the wallpaper, with a window). Then shell-host's probe window, maximised,
#      must hide the first frame (the layer is under windows). The placements are saved in the
#      scratch `$XDG_STATE_HOME/sill/desktop-widgets.json`.
#   5. A real drag through shell-host's real pointer path (FINDINGS "Real-pointer drag", G330
#      closed: `sill debug pointer-press`, `pointer` moved while it is held, `pointer-release`,
#      the same delivery a real `wl_pointer` press/motion/release uses): the Calendar is pressed
#      at its centre and carried left and down; mid-drag it is lifted (larger than a cell, under
#      the pointer); dropped, it snaps to column 8 row 2. Then the Battery is carried onto that
#      cell and held: the Calendar makes room, stepping up to row 1; dropped, the swap is kept.
#      The file then names the output. Captures of the layer before, lifted, dropped, making
#      room and after (target/accept-widgets/drag-*.png) and a screenshot after
#      (dev/out/widgets-drag-cosmic.png).
#   6. 5 s idle: 0 frames on the desktop layer.
#   7. `sill widgets edit` (FINDINGS "Widget interface port"): `sill-edit-widgets-<out>` maps,
#      showing quire's widget gallery beside the desktop's widgets (a capture, and
#      dev/out/widgets-edit-cosmic.png); 3 s idle on it: 0 frames; `sill widgets edit-close`
#      unmaps it again.
#   8. A second toggle removes the surface and writes `widgets.desktop = "off"`.
#
# Exit 0 when every step passed, 1 on a failure. Another sill daemon started from a
# `target/*/sill run` is waited for (up to 10 minutes), so two shells never share the nested
# output's screenshots.
#
# Usage: dev/nested.sh && dev/accept-widgets.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-widgets"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
PLAYER="$SILL_REPO/target/debug/examples/mpris_player"
HOST_REPO="$(cd "$SHELL_HOST_DEV/../.." && pwd)"
PROBE="$HOST_REPO/target/release/examples/probe_toplevel"
# The bar's reserve: the desktop layer (exclusive `Auto`) starts under it, so a capture of the
# layer is this far above the same pixels in a screenshot of the output (scale 1).
BAR_H="${BAR_H:-32}"

source "$SHELL_HOST_DEV/.nested-env"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
NESTED_SOCKET="$HOST_RUNTIME/$WAYLAND_DISPLAY"
RUNTIME=$(mktemp -d)
chmod 700 "$RUNTIME"
TERM_PID=

sill() { XDG_RUNTIME_DIR="$RUNTIME" "$SILL_BIN" "$@"; }

# A screenshot of the nested output only: shot.sh falls back to the host's session when no
# nested compositor answers, so it is never called without one.
shot() {
  "$SHELL_HOST_DEV/run.sh" wayland-info >/dev/null 2>&1 || { echo "no nested compositor" >&2; return 1; }
  "$SHELL_HOST_DEV/shot.sh" "$1" >/dev/null 2>&1
}

output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

# SURFACE's cumulative frame counter from `sill debug stats` (empty when it does not exist).
frames_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"])' "$1"
}

has_surface() { [ -n "$(frames_of "$1")" ]; }

center_up() { sill debug capture "$CENTER" "$OUT_DIR/probe.png" >/dev/null 2>&1; }

wait_center() {
  for _ in $(seq 40); do
    if [ "$1" = up ] && center_up; then return 0; fi
    if [ "$1" = down ] && ! center_up; then return 0; fi
    sleep 0.05
  done
  return 1
}

# The widget tiles in a capture of the center: quire's WidgetFrame tiles (`--surface-2` with a
# hairline, 344 wide and centred in the column, so 20 in), found as the rows where a pixel just
# inside a tile's left edge differs from one in
# the panel's padding beside it (the panel's own ground, whatever its gradient). Prints one
# "TOP BOTTOM" a tile, top to bottom.
center_tiles() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
left, top, right, bottom = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
# The panel is `notifications.center_width_px` (384) wide; its padding is 12 logical px.
scale = (right - left) / 384
outside, inside = left + round(5 * scale), left + round(24 * scale)
def off(y):
    a, b = img.getpixel((outside, y)), img.getpixel((inside, y))
    return sum(abs(p - q) for p, q in zip(a[:3], b[:3])) > 6
runs, run = [], None
for y in range(top + round(60 * scale), bottom - 4):
    if off(y):
        run = [y, y] if run is None else [run[0], y]
    elif run is not None:
        if run[1] - run[0] > 12 * scale:
            runs.append(run)
        run = None
if run is not None and run[1] - run[0] > 12 * scale:
    runs.append(run)
for r in runs:
    print(*r)
EOF
}

# The opaque blobs in a capture of the desktop layer, as "LEFT TOP RIGHT BOTTOM" each (the
# widget frames; everything between them is clear).
desktop_frames() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
a = img.getchannel("A")
w, h = img.size
cols = [x for x in range(w) if any(a.getpixel((x, y)) > 40 for y in range(0, h, 2))]
if not cols:
    sys.exit(1)
# Split into column bands, then each band into row runs.
bands, band = [], [cols[0], cols[0]]
for x in cols[1:]:
    if x - band[1] > 2:
        bands.append(band); band = [x, x]
    else:
        band[1] = x
bands.append(band)
for l, r in bands:
    mid = (l + r) // 2
    run = None
    for y in range(h):
        on = a.getpixel((mid, y)) > 40
        if on and run is None:
            run = [y, y]
        elif on:
            run[1] = y
        elif run is not None:
            print(l, run[0], r, run[1]); run = None
    if run is not None:
        print(l, run[0], r, run[1])
EOF
}

setting_desktop() {
  python3 -c '
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    print(tomllib.load(f).get("widgets", {}).get("desktop", "off"))' "$CONFIG/sill/settings.toml" 2>/dev/null \
    || echo off
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug \
    && cargo build -p sill-services --example mpris_player) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
  [ -x "$PROBE" ] || (cd "$HOST_REPO" && cargo build --release -p shell-host --features debug-inject \
    --example probe_toplevel >>"$OUT_DIR/build.log" 2>&1)
}

start_daemon() {
  CONFIG=$(mktemp -d)
  STATE=$(mktemp -d)
  CACHE=$(mktemp -d)
  backdrop_seed "$CONFIG" || return 1
  XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" \
    XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$NESTED_SOCKET" \
    dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- sh -c '
      "$1" sillaccept --title "So What" --artist "Miles Davis" --album "Kind of Blue" &
      exec "$2" run' sh "$PLAYER" "$SILL_BIN" >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  CENTER="sill-notification-center-$NAME"
  LAYER="sill-desktop-widgets-$NAME"
  sleep 1.5
  if ! sill debug stats | grep -q '"sill-wallpaper-'; then
    XDG_CONFIG_HOME="$CONFIG" WAYLAND_DISPLAY="$WAYLAND_DISPLAY" cosmic-bg >/dev/null 2>&1 &
    BACKDROP_PID=$!
  fi
  # Let the wallpaper paint before the first screenshot.
  for _ in $(seq 50); do
    [ "$(frames_of "sill-wallpaper-$NAME")" -gt 0 ] 2>/dev/null && break
    sleep 0.1
  done
  sleep 1
  echo "daemon on a private bus and runtime dir $RUNTIME; $CENTER, $LAYER" >&2
}

# Kill every COSMIC Terminal on the nested socket (never the host's).
kill_terms() {
  local pid
  for pid in $(pgrep -x cosmic-term); do
    if tr '\0' '\n' <"/proc/$pid/environ" 2>/dev/null | grep -qx "WAYLAND_DISPLAY=$NESTED_SOCKET"; then
      kill "$pid" 2>/dev/null
    fi
  done
  [ -n "$TERM_PID" ] && kill "$TERM_PID" 2>/dev/null
  TERM_PID=
}

stop_daemon() {
  kill_terms
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  pkill -f "$PLAYER sillaccept" 2>/dev/null
  backdrop_down
  rm -rf "$CONFIG" "$STATE" "$CACHE"
}

# 1.
step_center_default() {
  local log="$OUT_DIR/center.log" tiles
  : >"$log"
  sill notifications open >>"$log" 2>&1 || return 1
  wait_center up || { echo "the center never mapped" >&2; return 1; }
  sleep 0.8
  sill debug capture "$CENTER" "$OUT_DIR/center.png" >>"$log" 2>&1 || return 1
  shot "$SHOTS/widgets-center-cosmic.png" || echo "screenshot failed" >&2
  center_tiles "$OUT_DIR/center.png" >"$OUT_DIR/center-tiles.txt"
  tiles=$(wc -l <"$OUT_DIR/center-tiles.txt")
  echo "default widgets.center: $tiles tiles under the history ($(tr '\n' ';' <"$OUT_DIR/center-tiles.txt"))" \
    | tee -a "$log" >&2
  [ "$tiles" = 3 ]
}

# 2.
step_center_all() {
  local log="$OUT_DIR/center-all.log" tiles
  : >"$log"
  mkdir -p "$CONFIG/sill"
  cat >"$CONFIG/sill/settings.toml" <<'TOML'
[widgets]
center = ["calendar", "up_next", "battery", "now_playing", "world_clock"]
world_clocks = "Europe/London; America/New_York"
TOML
  sill reload >>"$log" 2>&1 || return 1
  sleep 1
  sill debug capture "$CENTER" "$OUT_DIR/center-all.png" >>"$log" 2>&1 || return 1
  shot "$SHOTS/widgets-center-all-cosmic.png" || echo "screenshot failed" >&2
  center_tiles "$OUT_DIR/center-all.png" >"$OUT_DIR/center-all-tiles.txt"
  tiles=$(wc -l <"$OUT_DIR/center-all-tiles.txt")
  echo "all five: $tiles tiles ($(tr '\n' ';' <"$OUT_DIR/center-all-tiles.txt"))" | tee -a "$log" >&2
  [ "$tiles" = 5 ]
}

# Sleep past the minute's turn when it is close: the clock redraws the ages and the world clock.
clear_of_the_minute() {
  if [ "$((10#$(date +%S)))" -ge 50 ]; then sleep $((62 - 10#$(date +%S))); fi
}

# 3.
step_center_idle() {
  local log="$OUT_DIR/center-idle.log" f0 f1
  clear_of_the_minute
  f0=$(frames_of "$CENTER")
  sleep 5
  f1=$(frames_of "$CENTER")
  echo "5 s idle open with five widgets: $(( f1 - f0 )) frames on $CENTER" | tee "$log" >&2
  sill notifications close >/dev/null 2>&1
  wait_center down
  [ "$(( f1 - f0 ))" = 0 ]
}

# 4.
step_toggle_on() {
  local log="$OUT_DIR/toggle-on.log" frames saved
  : >"$log"
  has_surface "$LAYER" && { echo "$LAYER exists before any toggle" >&2; return 1; }
  sill widgets toggle-desktop >>"$log" 2>&1 || { cat "$log" >&2; return 1; }
  for _ in $(seq 40); do has_surface "$LAYER" && break; sleep 0.05; done
  has_surface "$LAYER" || { echo "$LAYER never appeared" | tee -a "$log" >&2; return 1; }
  [ "$(setting_desktop)" = on ] || { echo "widgets.desktop is not on in the file" >&2; return 1; }
  sleep 1
  sill debug capture "$LAYER" "$OUT_DIR/desktop.png" >>"$log" 2>&1 || return 1
  desktop_frames "$OUT_DIR/desktop.png" >"$OUT_DIR/desktop-frames.txt"
  frames=$(wc -l <"$OUT_DIR/desktop-frames.txt")
  saved=$(tr -d '\n ' <"$STATE/sill/desktop-widgets.json" 2>/dev/null)
  # A COSMIC Terminal in the nested session (its own scratch config and private bus), over part
  # of the layer.
  XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" \
    WAYLAND_DISPLAY="$NESTED_SOCKET" \
    dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- cosmic-term \
    >"$OUT_DIR/term.log" 2>&1 &
  TERM_PID=$!
  sleep 3
  shot "$SHOTS/widgets-desktop-cosmic.png" || echo "screenshot failed" >&2
  kill_terms
  # Under windows: shell-host's probe window, maximised, covers the whole work area; the first
  # frame's box in a screenshot then shows the probe's checkerboard, not the frame.
  sleep 20 | "$SHELL_HOST_DEV/run.sh" "$PROBE" >"$OUT_DIR/probe.log" 2>&1 &
  local probe=$! covered
  for _ in $(seq 100); do grep -q '"maximized":true' "$OUT_DIR/probe.log" && break; sleep 0.1; done
  sleep 0.5
  shot "$OUT_DIR/covered.png" || echo "screenshot failed" >&2
  kill "$probe" 2>/dev/null; wait "$probe" 2>/dev/null
  covered=$(python3 - "$SHOTS/widgets-desktop-cosmic.png" "$OUT_DIR/covered.png" \
    "$(head -1 "$OUT_DIR/desktop-frames.txt")" "$BAR_H" <<'EOF'
import sys
from PIL import Image
shown, covered = (Image.open(p).convert("RGB") for p in sys.argv[1:3])
l, t, r, b = map(int, sys.argv[3].split())
dy = int(sys.argv[4])
box = (l + 8, t + dy + 8, r - 8, b + dy - 8)
diff = sum(abs(p - q) for p, q in zip(shown.crop(box).tobytes(), covered.crop(box).tobytes()))
print(diff // max(1, (box[2] - box[0]) * (box[3] - box[1])))
EOF
)
  echo "toggle-desktop: $LAYER up, widgets.desktop = on, $frames frames ($(tr '\n' ';' <"$OUT_DIR/desktop-frames.txt")), saved $saved; a maximised window over the first frame changes it by $covered per pixel" \
    | tee -a "$log" >&2
  [ "$frames" = 2 ] && [[ "$saved" == *calendar* ]] && [[ "$saved" == *battery* ]] \
    && [ "${covered:-0}" -gt 30 ]
}

# A real drag on the desktop layer, through shell-host's real pointer path (FINDINGS
# "Real-pointer drag", G330 closed): press, move (the existing `sill debug pointer`, which
# carries the held button on every move) and release at X Y (layer px).
pointer_press() { sill debug pointer-press "$LAYER" "$@" >/dev/null; }
pointer_move() { sill debug pointer "$LAYER" "$@" >/dev/null; }
pointer_release() { sill debug pointer-release "$LAYER" "$@" >/dev/null; }

# Carry a drag from X0 Y0 to X1 Y1 in 12 moves a frame apart, without releasing.
drag_to() {
  local x0=$1 y0=$2 x1=$3 y1=$4 i
  pointer_press "$x0" "$y0" || return 1
  for i in $(seq 12); do
    pointer_move $(( x0 + (x1 - x0) * i / 12 )) $(( y0 + (y1 - y0) * i / 12 )) || return 1
    sleep 0.016
  done
}

# Whether frame line $1 ("L T R B") is at left $2 top $3, 164 square, give or take 3 px.
frame_at() {
  python3 -c '
import sys
l, t, r, b = map(int, sys.argv[1].split())
x, y = int(sys.argv[2]), int(sys.argv[3])
ok = abs(l - x) <= 3 and abs(t - y) <= 3 and abs(r - l + 1 - 164) <= 3 and abs(b - t + 1 - 164) <= 3
sys.exit(0 if ok else 1)' "$1" "$2" "$3"
}

# 5.
step_drag() {
  local log="$OUT_DIR/drag.log" cal bat l t r b cx cy bx by lifted saved
  : >"$log"
  sill debug capture "$LAYER" "$OUT_DIR/drag-before.png" >>"$log" 2>&1 || return 1
  desktop_frames "$OUT_DIR/drag-before.png" >"$OUT_DIR/drag-before.txt"
  cal=$(sed -n 1p "$OUT_DIR/drag-before.txt"); bat=$(sed -n 2p "$OUT_DIR/drag-before.txt")
  read -r l t r b <<<"$cal"
  cx=$(( (l + r) / 2 )); cy=$(( (t + b) / 2 ))
  read -r bx by _ _ <<<"$bat"; bx=$(( bx + 82 )); by=$(( by + 82 ))
  echo "before: Calendar $cal, Battery $bat" | tee -a "$log" >&2
  # The Calendar five cells left and two down (a cell and its gap are 180 px).
  drag_to "$cx" "$cy" $(( cx - 900 )) $(( cy + 360 )) || return 1
  sleep 0.5
  sill debug capture "$LAYER" "$OUT_DIR/drag-lifted.png" >>"$log" 2>&1 || return 1
  shot "$SHOTS/widgets-drag-lifted-cosmic.png" || echo "screenshot failed" >&2
  lifted=$(desktop_frames "$OUT_DIR/drag-lifted.png" | python3 -c '
import sys
rows = [list(map(int, l.split())) for l in sys.stdin if l.strip()]
print(max(r - l + 1 for l, t, r, b in rows))')
  echo "lifted: the widest frame is $lifted px (a cell is 164)" | tee -a "$log" >&2
  pointer_release $(( cx - 900 )) $(( cy + 360 )) || return 1
  sleep 0.8
  sill debug capture "$LAYER" "$OUT_DIR/drag-dropped.png" >>"$log" 2>&1 || return 1
  desktop_frames "$OUT_DIR/drag-dropped.png" >"$OUT_DIR/drag-dropped.txt"
  echo "dropped: $(tr '\n' ';' <"$OUT_DIR/drag-dropped.txt")" | tee -a "$log" >&2
  frame_at "$(grep -v "^$l " "$OUT_DIR/drag-dropped.txt" | head -1)" $(( l - 900 )) $(( t + 360 )) \
    || { echo "the Calendar did not snap to $(( l - 900 )),$(( t + 360 ))" | tee -a "$log" >&2; return 1; }
  # The Battery onto the Calendar's new cell, held there: the Calendar makes room (row 1).
  drag_to "$bx" "$by" $(( cx - 900 )) $(( cy + 360 )) || return 1
  sleep 0.8
  sill debug capture "$LAYER" "$OUT_DIR/drag-room.png" >>"$log" 2>&1 || return 1
  shot "$SHOTS/widgets-drag-room-cosmic.png" || echo "screenshot failed" >&2
  pointer_release $(( cx - 900 )) $(( cy + 360 )) || return 1
  sleep 0.8
  sill debug capture "$LAYER" "$OUT_DIR/drag-after.png" >>"$log" 2>&1 || return 1
  shot "$SHOTS/widgets-drag-cosmic.png" || echo "screenshot failed" >&2
  desktop_frames "$OUT_DIR/drag-after.png" >"$OUT_DIR/drag-after.txt"
  saved=$(tr -d '\n ' <"$STATE/sill/desktop-widgets.json" 2>/dev/null)
  echo "after: $(tr '\n' ';' <"$OUT_DIR/drag-after.txt") saved $saved" | tee -a "$log" >&2
  [ "${lifted:-0}" -ge 168 ] \
    && frame_at "$(sed -n 1p "$OUT_DIR/drag-after.txt")" $(( l - 900 )) $(( t + 180 )) \
    && frame_at "$(sed -n 2p "$OUT_DIR/drag-after.txt")" $(( l - 900 )) $(( t + 360 )) \
    && [[ "$saved" == *"\"output\":\"$NAME\""* ]]
}

# 6.
step_desktop_idle() {
  local log="$OUT_DIR/desktop-idle.log" f0 f1
  clear_of_the_minute
  f0=$(frames_of "$LAYER")
  sleep 5
  f1=$(frames_of "$LAYER")
  echo "5 s idle: $(( f1 - f0 )) frames on $LAYER" | tee "$log" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

# 7.
step_edit() {
  local log="$OUT_DIR/edit.log" edit="sill-edit-widgets-$NAME" f0 f1 up=
  : >"$log"
  has_surface "$edit" || { echo "$edit was never spawned" | tee -a "$log" >&2; return 1; }
  sill widgets edit >>"$log" 2>&1 || { cat "$log" >&2; return 1; }
  for _ in $(seq 40); do
    sill debug capture "$edit" "$OUT_DIR/edit.png" >>"$log" 2>&1 && { up=1; break; }
    sleep 0.05
  done
  [ -n "$up" ] || { echo "$edit never mapped" | tee -a "$log" >&2; return 1; }
  sleep 1
  sill debug capture "$edit" "$OUT_DIR/edit.png" >>"$log" 2>&1 || return 1
  shot "$SHOTS/widgets-edit-cosmic.png" || echo "screenshot failed" >&2
  clear_of_the_minute
  f0=$(frames_of "$edit")
  sleep 3
  f1=$(frames_of "$edit")
  sill widgets edit-close >>"$log" 2>&1 || return 1
  sleep 0.8
  if sill debug capture "$edit" "$OUT_DIR/edit-closed.png" >>"$log" 2>&1; then
    echo "$edit is still mapped after edit-close" | tee -a "$log" >&2; return 1
  fi
  echo "Edit Widgets: $edit mapped on 'sill widgets edit', $(( f1 - f0 )) frames in 3 s idle, unmapped on edit-close" \
    | tee -a "$log" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

# 8.
step_toggle_off() {
  local log="$OUT_DIR/toggle-off.log"
  kill_terms
  sill widgets toggle-desktop >"$log" 2>&1 || { cat "$log" >&2; return 1; }
  for _ in $(seq 40); do has_surface "$LAYER" || break; sleep 0.05; done
  has_surface "$LAYER" && { echo "$LAYER is still there" | tee -a "$log" >&2; return 1; }
  [ "$(setting_desktop)" = off ] || { echo "widgets.desktop is not off in the file" >&2; return 1; }
  echo "a second toggle removes $LAYER and writes widgets.desktop = off" | tee -a "$log" >&2
}

"$SHELL_HOST_DEV/run.sh" wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
build || exit 1
for _ in $(seq 600); do
  pgrep -f 'target/[a-z]*/sill run' >/dev/null || break
  [ -n "${WAITED:-}" ] || { echo "another sill daemon is running; waiting for it to quit" >&2; WAITED=1; }
  sleep 1
done
start_daemon || { stop_daemon; exit 1; }
status=0
for step in step_center_default step_center_all step_center_idle step_toggle_on \
  step_drag step_desktop_idle step_edit step_toggle_off; do
  if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; fi
done
stop_daemon
rm -rf "$RUNTIME"
echo "logs in $OUT_DIR; screenshots $SHOTS/widgets-center-cosmic.png, $SHOTS/widgets-center-all-cosmic.png, $SHOTS/widgets-desktop-cosmic.png, $SHOTS/widgets-edit-cosmic.png"
exit "$status"

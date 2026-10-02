#!/usr/bin/env bash
# Widgets acceptance. Starts `sill run` from this checkout's debug build (`--features debug`)
# on a private bus (the daemon's session AND system bus, so no UPower, BlueZ or real MPRIS player
# is ever reached), with a scratch HOME and XDG directories, its own XDG_RUNTIME_DIR and a fake
# DDC, and quits it at the end.
#
# Default: the nested cosmic-comp (dev/nested.sh; its socket is reused and never stopped, design/20
# §1.14), with one fake MPRIS player (sill-testkit's `mpris_player` example) on the private bus.
# Another sill daemon started from a `target/*/sill run` is waited for (up to 10 minutes), so two
# shells never share the nested output's screenshots.
#
#   1. With the default `widgets.center` (Calendar, Up Next, Battery), `sill notifications open`: a
#      capture of the center shows three widget tiles under the history (Battery says "No
#      batteries": the private bus has no UPower and no BlueZ). Screenshot
#      dev/out/widgets-center-cosmic.png.
#   2. `widgets.center` = all five and two world clocks, `sill reload`: five tiles, Now Playing
#      showing the fake player (dev/out/widgets-center-all-cosmic.png).
#   3. 5 s idle while open, clear of the minute's turn, with the fake player paused (over MPRIS
#      on the private bus: a playing track's position steps once a second, which is motion,
#      not idle): 0 frames on the center.
#   4. `sill widgets toggle-desktop`: `sill-desktop-widgets-<out>` appears in `sill debug stats`,
#      `widgets.desktop = "on"` in the file, and a capture of the layer shows two frames
#      (Calendar and Battery) at the top right with nothing painted between them; a COSMIC
#      Terminal opened in the nested session beside them (dev/out/widgets-desktop-cosmic.png:
#      the layer over the wallpaper, with a window). Then shell-host's probe window, maximised,
#      must hide the first frame (the layer is under windows). The placements are saved in the
#      scratch `$XDG_STATE_HOME/sill/desktop-widgets.json`.
#   5. A real drag through shell-host's real pointer path (`sill debug pointer-press`, `pointer`
#      moved while it is held, `pointer-release`, the same delivery a real `wl_pointer`
#      press/motion/release uses): the Calendar is pressed at its centre and carried left and down;
#      mid-drag it is lifted (a cell wide plus its drag shadow, under the pointer); dropped, it snaps to column 8
#      row 2. Then the Battery is carried onto that cell and held: the Calendar makes room, stepping
#      up to row 1; dropped, the swap is kept. The file then names the output. Captures of the layer
#      before, lifted, dropped, making room and after (target/accept-widgets/drag-*.png), one held
#      half a cell off its snap cell so quire's slot guide shows beside the lifted card
#      (dev/out/widgets-drag-guide-cosmic.png), and a screenshot after
#      (dev/out/widgets-drag-cosmic.png).
#   6. 5 s idle: 0 frames on the desktop layer.
#   7. `sill widgets edit`: `sill-edit-widgets-<out>` maps, showing quire's widget gallery beside
#      the desktop's widgets (a capture, and dev/out/widgets-edit-cosmic.png); 3 s idle on it: 0
#      frames; `sill widgets edit-close` unmaps it again.
#   8. A second toggle removes the surface and writes `widgets.desktop = "off"`.
#
# --real-input: real input at scale 2, on shell-host's private headless KWin
# (scripts/dev/kwin-virtual.sh, 3840 x 2160, its one output set to scale 2 with kscreen-doctor on
# that KWin's own socket: the BenQ MA270U's 4K at 2x, the user's session) driven by shell-host's
# scripts/dev/fake-input.sh: every click and key is a real Wayland event with a real serial and
# keyboard focus. Nothing reaches the user's session (wayland-0), the nested compositor
# (wayland-1) or the real session or system bus: the headless KWin's private bus stands in for
# BOTH the daemon's session and system bus (start_kwin refuses to go on if it would be the real
# one). ONE fake-input client serves the whole run. UPower and BlueZ are simply absent on the
# private bus, so the Batteries widget always reads "No batteries" here: every step below only
# checks the widget's structure (card count and size), never a real device's name or level.
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
# Exit 0 only when every step passed; 1 on a failure.
#
# Usage: dev/nested.sh && dev/accept/widgets.sh
#        dev/accept/widgets.sh --real-input
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
case "${1:-}" in
  --real-input) MODE=real ;;
  "") MODE=nested ;;
  *) echo "usage: dev/accept/widgets.sh [--real-input]" >&2; exit 2 ;;
esac
OUT_DIR="$SILL_TARGET/accept-widgets"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
PLAYER="$SILL_OUT/examples/mpris_player"
PLAYER_PID=
TERM_PID=
# The bar's reserve: the desktop layer (exclusive `Auto`) starts under it, so a capture of the
# layer is this far above the same pixels in a screenshot of the output (scale 1).
BAR_H="${BAR_H:-24}"
if [ "$MODE" = real ]; then
  SIZE=3840x2160
  SCALE=2
  INPUT_TICKS=200
fi
scratch_new
trap scratch_drop EXIT

# --- nested compositor ---

# A screenshot of the nested output only: shot.sh falls back to the host's session when no
# nested compositor answers, so it is never called without one.
shot() {
  "$SHELL_HOST_DEV/run.sh" wayland-info >/dev/null 2>&1 || { echo "no nested compositor" >&2; return 1; }
  "$SHELL_HOST_DEV/shot.sh" "$1" >/dev/null 2>&1
}

has_surface() { [ -n "$(frames_of "$1")" ]; }

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

build_nested() {
  build "-p sill-testkit --example mpris_player" || return 1
  [ -x "$PROBE" ] || (cd "$HOST_REPO" && cargo build --release -p shell-host --features debug-inject \
    --example probe_toplevel >>"$OUT_DIR/build.log" 2>&1)
}

start_nested_daemon() {
  backdrop_seed "$CONFIG" || return 1
  in_env "$PLAYER" sillaccept --title "So What" --artist "Miles Davis" --album "Kind of Blue" &
  PLAYER_PID=$!
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  CENTER="sill-notification-center-$NAME"
  LAYER="sill-desktop-widgets-$NAME"
  sleep 1.5
  backdrop_up "$CONFIG" || echo "backdrop: none (the shots show the bare compositor)" >&2
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
    if tr '\0' '\n' <"/proc/$pid/environ" 2>/dev/null | grep -qx "WAYLAND_DISPLAY=$WL"; then
      kill "$pid" 2>/dev/null
    fi
  done
  [ -n "$TERM_PID" ] && kill "$TERM_PID" 2>/dev/null
  TERM_PID=
}

# 1.
step_center_default() {
  local log="$OUT_DIR/center.log" tiles
  : >"$log"
  sill notifications open >>"$log" 2>&1 || return 1
  wait_surface "$CENTER" 20 up || { echo "the center never mapped" >&2; return 1; }
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

# Toggle the fake player's playback over MPRIS on the private bus.
player_play_pause() {
  DBUS_SESSION_BUS_ADDRESS="$BUS" dbus-send --session --type=method_call \
    --dest=org.mpris.MediaPlayer2.sillaccept /org/mpris/MediaPlayer2 \
    org.mpris.MediaPlayer2.Player.PlayPause
}

# 3.
step_center_idle() {
  local log="$OUT_DIR/center-idle.log" f0 f1 p0 p1
  clear_of_the_minute
  p0=$(frames_of "$CENTER")
  sleep 3
  p1=$(frames_of "$CENTER")
  echo "3 s open while the fake track plays: $(( p1 - p0 )) frames (its position steps each second)" \
    | tee "$log" >&2
  player_play_pause || { echo "could not pause the fake player" | tee -a "$log" >&2; return 1; }
  sleep 1
  clear_of_the_minute
  f0=$(frames_of "$CENTER")
  sleep 5
  f1=$(frames_of "$CENTER")
  echo "5 s idle open with five widgets, the track paused: $(( f1 - f0 )) frames on $CENTER" \
    | tee -a "$log" >&2
  sill notifications close >/dev/null 2>&1
  wait_surface "$CENTER" 20 down
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
  in_env dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- cosmic-term \
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

# A real drag on the desktop layer, through shell-host's real pointer path: press, move (the
# existing `sill debug pointer`, which carries the held button on every move) and release at X Y
# (layer px).
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
  local log="$OUT_DIR/drag.log" cal bat l t r b cx cy bx by lifted lifted_w lifted_x lifted_y saved
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
  # quire's lift is the drag shadow, not a scale (`.ds-widget[data-lift=lifted]` in
  # ds-shell/src/widget/frame.css): the card keeps its cell's size and the detected frame is at
  # most a few px wider where the shadow starts. So "lifted" is: the frame under the pointer, about a
  # cell wide (the cell, up to 24 px of shadow).
  lifted=$(desktop_frames "$OUT_DIR/drag-lifted.png" | python3 -c '
import sys
px, py = int(sys.argv[1]), int(sys.argv[2])
rows = [list(map(int, l.split())) for l in sys.stdin if l.strip()]
# The lifted card is the frame nearest the pointer (the other widget has not moved).
l, t, r, b = min(rows, key=lambda row: abs((row[0] + row[2]) // 2 - px) + abs((row[1] + row[3]) // 2 - py))
print(r - l + 1, (l + r) // 2, (t + b) // 2)' $(( cx - 900 )) $(( cy + 360 )))
  read -r lifted_w lifted_x lifted_y <<<"$lifted"
  echo "lifted: the frame under the pointer is ${lifted_w} px (a cell is 164) centred at $lifted_x,$lifted_y (pointer $(( cx - 900 )),$(( cy + 360 )))" \
    | tee -a "$log" >&2
  # Held off its cell by half a cell, the card no longer covers quire's slot guide at the cell
  # it would land in; then back over the cell before the release.
  pointer_move $(( cx - 900 + 70 )) $(( cy + 360 + 50 )) || return 1
  sleep 0.5
  sill debug capture "$LAYER" "$OUT_DIR/drag-guide.png" >>"$log" 2>&1 || return 1
  shot "$SHOTS/widgets-drag-guide-cosmic.png" || echo "screenshot failed" >&2
  pointer_move $(( cx - 900 )) $(( cy + 360 )) || return 1
  sleep 0.3
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
  [ "${lifted_w:-0}" -ge 164 ] && [ "${lifted_w:-999}" -le 188 ] \
    && [ "${lifted_x:-9999}" -ge $(( cx - 900 - 12 )) ] && [ "${lifted_x:-0}" -le $(( cx - 900 + 12 )) ] \
    && [ "${lifted_y:-9999}" -ge $(( cy + 360 - 12 )) ] && [ "${lifted_y:-0}" -le $(( cy + 360 + 12 )) ] \
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

cleanup_nested() {
  local result=0
  kill_terms
  stop_daemon || result=1
  [ -z "$PLAYER_PID" ] || kill "$PLAYER_PID" 2>/dev/null
  pkill -f "$PLAYER sillaccept" 2>/dev/null
  backdrop_down
  stop_private_bus
  scratch_drop
  return "$result"
}

run_nested() {
  use_nested_socket || exit 1
  nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
  build_nested || exit 1
  await_no_process || exit 1
  start_private_bus || exit 1
  start_nested_daemon || { cleanup_nested; exit 1; }
  run_steps step_center_default step_center_all step_center_idle step_toggle_on \
    step_drag step_desktop_idle step_edit step_toggle_off
  status=$STATUS
  cleanup_nested || status=1
  echo "logs in $OUT_DIR; screenshots $SHOTS/widgets-center-cosmic.png, $SHOTS/widgets-center-all-cosmic.png, $SHOTS/widgets-desktop-cosmic.png, $SHOTS/widgets-edit-cosmic.png"
  exit "$status"
}

# --- headless KWin, real input, scale 2 ---

# SURFACE's counters now: "frames callbacks commits skipped".
full_counters_of() {
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
    echo "$(date +%s.%N | cut -c1-14) $(full_counters_of "$surface")" >>"$file"
    sleep 0.1
  done
}

# A capture of SURFACE into FILE. A debug build paints a 4K surface in more than the IPC's 2 s
# reply time, so a timed-out request still writes the file: wait for it. An unmapped surface
# fails at once ("not presented").
capture_patient() {
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
mapped_patient() { capture_patient "$1" "$OUT_DIR/probe.png"; }

wait_presented() {
  local surface="$1" tries="$2" want="$3"
  for _ in $(seq "$tries"); do
    if mapped_patient "$surface"; then [ "$want" = up ] && return 0
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
desktop_cards() {
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
crop_over_ground() {
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
step_real_scale() {
  local scale shot
  scale=$(kwin_scale)
  capture_patient "$BAR" "$OUT_DIR/bar.png" || { echo "no bar capture" >&2; return 1; }
  shot=$(python3 -c 'import sys; from PIL import Image; print(*Image.open(sys.argv[1]).size)' "$OUT_DIR/bar.png")
  TOP=$(bar_height)
  echo "kscreen-doctor: scale $scale; the bar's capture is $shot device px ($TOP logical tall)" >&2
  python3 -c 'import sys; sys.exit(0 if abs(float(sys.argv[1]) - float(sys.argv[2])) < 0.01 else 1)' \
    "$scale" "$SCALE" && [ "${shot%% *}" = 3840 ]
}

# 2.
step_real_desktop() {
  local cal bat ok=0
  capture_patient "$LAYER" "$OUT_DIR/desktop.png" || { echo "no desktop layer" >&2; return 1; }
  desktop_cards "$OUT_DIR/desktop.png" >"$OUT_DIR/desktop-frames.txt"
  cal=$(sed -n 1p "$OUT_DIR/desktop-frames.txt"); bat=$(sed -n 2p "$OUT_DIR/desktop-frames.txt")
  echo "desktop at scale 2: $(wc -l <"$OUT_DIR/desktop-frames.txt") frames: Calendar $cal, Batteries $bat (device px)" >&2
  crop_over_ground "$OUT_DIR/desktop.png" "$SHOTS/widgets-kwin-s2-desktop.png" "3000 0 3840 900"
  read -r l t r b <<<"$cal"
  crop_over_ground "$OUT_DIR/desktop.png" "$SHOTS/widgets-kwin-s2-calendar.png" "$((l - 16)) $((t - 16)) $((r + 16)) $((b + 16))"
  read -r l t r b <<<"$bat"
  crop_over_ground "$OUT_DIR/desktop.png" "$SHOTS/widgets-kwin-s2-battery.png" "$((l - 16)) $((t - 16)) $((r + 16)) $((b + 16))"
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
step_real_center() {
  local cx cy tiles
  read -r cx cy <<<"$(clock_at)"
  echo "the clock at ($cx, $cy); a real click on it" >&2
  click "$cx" "$cy" || return 1
  wait_presented "$CENTER" 25 up || { echo "the center never mapped" >&2; return 1; }
  sleep 1.2
  capture_patient "$CENTER" "$OUT_DIR/center.png" || return 1
  crop_over_ground "$OUT_DIR/center.png" "$SHOTS/widgets-kwin-s2-center.png" "2980 0 3840 1960"
  center_layout >"$OUT_DIR/center-layout.txt"
  tiles=$(grep -c '^tile' "$OUT_DIR/center-layout.txt")
  echo "the center: $tiles tiles ($(grep '^tile' "$OUT_DIR/center-layout.txt" | awk '{print $2"-"$3}' | tr '\n' ' ')device px); Edit Widgets at $(grep '^edit' "$OUT_DIR/center-layout.txt" | cut -d' ' -f2-)" >&2
  [ "$tiles" = 3 ] || return 1
  # Each tile is Medium: 164 logical px tall.
  grep '^tile' "$OUT_DIR/center-layout.txt" | awk -v s="$SCALE" '{h=($3-$2+1)/s; if (h < 150 || h > 176) bad=1} END {exit bad}'
}

# 4.
step_real_open_edit() {
  local ex ey
  read -r ex ey <<<"$(grep '^edit' "$OUT_DIR/center-layout.txt" | cut -d' ' -f2-)"
  [ -n "${ey:-}" ] || { echo "no Edit Widgets under the tiles" >&2; return 1; }
  echo "a real click on Edit Widgets at ($ex, $(( ey + TOP )))" >&2
  click "$ex" $(( ey + TOP )) || return 1
  wait_presented "$EDIT" 25 up || { echo "Edit Widgets never mapped" >&2; return 1; }
  wait_presented "$CENTER" 25 down || { echo "the center is still up" >&2; return 1; }
  sleep 1.5
  capture_patient "$EDIT" "$OUT_DIR/edit.png" || return 1
  # The sheet's plate ("L T R B" logical, on the surface) and the surface's own size.
  PANEL=$(python3 -c '
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
l, t, r, b = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
s = float(sys.argv[2])
print(round(l / s), round(t / s), round(r / s), round(b / s), round(img.size[0] / s), round(img.size[1] / s))' \
    "$OUT_DIR/edit.png" "$SCALE")
  crop_over_ground "$OUT_DIR/edit.png" "$SHOTS/widgets-kwin-s2-edit.png" "0 0 3840 2160"
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
  crop_over_ground "$1" "$2" "$(( l * SCALE )) $(( t * SCALE )) $(( r * SCALE )) $(( b * SCALE ))"
}

# 4b. World Clock picked, the gallery shows one preview, at Medium (344 wide), and no
# size control. The automated check reads the preview row: one run of card across the detail
# plate, 344 +- 6 wide, a row through its top above the dials; the capture shows the rest.
step_real_one_size() {
  local left top run
  left=$(panel_left); top=$(panel_top)
  click $(( left + 112 )) $(( top + 170 + TOP )) || return 1
  sleep 1.5
  capture_patient "$EDIT" "$OUT_DIR/edit-one-size.png" || return 1
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
step_real_add() {
  local left top before after frames_now f0 f1
  left=$(panel_left); top=$(panel_top)
  before=$(desktop_kinds)
  f0=$(frames_of "$LAYER")
  sample_counters "$LAYER" "$OUT_DIR/rise-frames.txt" &
  local sampler=$!
  click $(( left + 382 )) $(( top + 311 + TOP )) || return 1
  capture_patient "$EDIT" "$OUT_DIR/edit-added.png" || return 1
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
  capture_patient "$LAYER" "$OUT_DIR/desktop-added.png" || return 1
  desktop_cards "$OUT_DIR/desktop-added.png" >"$OUT_DIR/desktop-added.txt"
  frames_now=$(wc -l <"$OUT_DIR/desktop-added.txt")
  crop_over_ground "$OUT_DIR/desktop-added.png" "$SHOTS/widgets-kwin-s2-desktop-added.png" "2400 0 3840 1000"
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
step_real_remove() {
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
  capture_patient "$LAYER" "$OUT_DIR/desktop-removed.png" || return 1
  desktop_cards "$OUT_DIR/desktop-removed.png" >"$OUT_DIR/desktop-removed.txt"
  frames_now=$(wc -l <"$OUT_DIR/desktop-removed.txt")
  crop_over_ground "$OUT_DIR/desktop-removed.png" "$SHOTS/widgets-kwin-s2-desktop-removed.png" "2400 0 3840 1000"
  capture_patient "$EDIT" "$OUT_DIR/edit-removed.png" && \
    logical_crop "$OUT_DIR/edit-removed.png" "$SHOTS/widgets-kwin-s2-removed.png" \
      "$(( left - 10 )) $(( top - 10 )) $(( left + 1050 )) $(( top + 410 ))"
  echo "removed: desktop_widgets '$before' -> '$after'; the layer painted $(( f1 - f0 )) frames (the exit: its start and its drop paint 2, so more is the exit playing; over time in exit-frames.txt) and shows $frames_now" >&2
  [[ "$after" != *battery* ]] && [[ "$after" == *world_clock* ]] && [ "$frames_now" = 2 ] \
    && [ $(( f1 - f0 )) -ge 3 ]
}

# 7.
step_real_escape() {
  key Escape || return 1
  wait_presented "$EDIT" 25 down || { echo "Edit Widgets is still up after Escape" >&2; return 1; }
  echo "a real Escape closed Edit Widgets" >&2
}

# 8.
step_real_idle() {
  local f0 f1
  sleep 1
  f0=$(frames_of "$LAYER")
  sleep 5
  f1=$(frames_of "$LAYER")
  echo "5 s idle: $(( f1 - f0 )) frames on $LAYER" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

cleanup_real() {
  stop_input
  stop_daemon
  stop_kwin
  scratch_drop
}

start_real_daemon() {
  mkdir -p "$CONFIG/sill"
  printf '[widgets]\ndesktop = "on"\n[notifications]\nserver = "passive"\n' \
    >"$CONFIG/sill/settings.toml"
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  BAR="sill-bar-$NAME"
  CENTER="sill-notification-center-$NAME"
  LAYER="sill-desktop-widgets-$NAME"
  EDIT="sill-edit-widgets-$NAME"
  sleep 3
}

run_real() {
  build || exit 1
  start_kwin || { stop_kwin; exit 1; }
  require_kwin_scale "$SCALE" || { stop_kwin; scratch_drop; exit 1; }
  grep -q "${WL##*/}" "$KWIN_ENV" 2>/dev/null || {
    echo "$KWIN_ENV no longer names our KWin" >&2; cleanup_real; exit 1; }
  start_input || { cleanup_real; exit 1; }
  start_real_daemon || { cleanup_real; exit 1; }
  run_steps step_real_scale step_real_desktop step_real_center step_real_open_edit step_real_one_size \
    step_real_add step_real_remove step_real_escape step_real_idle
  status=$STATUS
  cleanup_real
  echo "logs and captures in $OUT_DIR; screenshots $SHOTS/widgets-kwin-s2-*.png"
  exit "$status"
}

if [ "$MODE" = real ]; then run_real; else run_nested; fi

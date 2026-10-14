#!/usr/bin/env bash
# Acceptance for the dock, on casement (brought up and ended by start_compositor). Starts `sill run` from this checkout's debug build (`--features debug`) with a
# scratch HOME and XDG tree (the pins, the appearance file and the icon cache it writes never touch
# the real ones), its own XDG_RUNTIME_DIR (the nested socket as an absolute WAYLAND_DISPLAY) and a
# private dbus-daemon as both its session and its system bus (refused if it would be the real one),
# so NetworkManager, BlueZ, UPower and the rest are simply absent. Quits it at the end.
#
# Default steps (the dock's behaviour):
#   1. The dock reserves base + margin: a maximised probe is output_h - bar_h - 68 (gut 8 + tile
#      48 + 2 x the 6 px padding).
#   2. Hover sweep at 60 Hz for 1 s through `sill debug pointer`: frames >= 0.9 x callbacks;
#      then, after `sill debug leave` and the 200 ms shrink, 5 s idle: 0 frames.
#   3. A click on the pinned, not-running foot tile launches it: its running dot
#      (the dock's view of a mapped toplevel) shows within 3 s.
#   4. `badge_emitter` for that app shows a count badge within 1 s.
#   5. A secondary click on that tile (`surface_click --button secondary`: `sill debug click` on KWin,
#      a real right press through casement's inject on casement) opens its context
#      menu popup.
#   6. The pill's corner is a squircle of `dock.pill_radius_px` from the scratch settings
#      (PILL_RADIUS, 12 by default, not the key's 22): the pixel centred 6.5 px in from the
#      pill's left edge and 1.5 px down from its top is the pill's (inside a 12 px squircle,
#      whose edge crosses that row 5.4 px in, and outside a 22 px one, 7.7 px in: its extent is
#      capped at half the 60 px pill).
# Screenshots: dev/out/dock-casement.png (rest), dock-casement-magnified.png, dock-casement-menu.png,
# over the test wallpaper (dev/lib/backdrop.sh).
#
# --icons steps (the icon style on the dock, its own daemon and pins):
# The dock holds foot, Files, Google Chrome and btop, every one on the neutral
# plate with its own theme icon, tinted along with the plate under Muted and Monochrome. With
# `icons.style` set in the scratch `quire/appearance.toml` to colour, then muted, then monochrome
# (the running dock follows the file live), it checks that the Muted and Monochrome runs each
# write re-tinted rasters into the scratch cache, one per pinned app, and captures the dock
# surface each time: dev/out/dock-icons-{colour,muted,monochrome}.png. A last switch to
# `appearance.theme = "dark"` in Monochrome (no new raster: the dark scheme changes the plate's
# own stops, not the icon's) captures dev/out/dock-icons-monochrome-dark.png. The launcher
# (queried for "foot", Colour style) is captured too: dev/out/launcher-icons.png,
# foot as a row drawing its own hicolor icon, the same fallback path.
#   1. The compositor answers; the daemon starts and the dock is up with its four pins.
#   2. Colour: a capture, and no Muted or Monochrome raster in the cache.
#   3. The launcher's icons capture.
#   4. Muted, then Monochrome: four re-tinted rasters each, a capture each.
#   5. Five idle seconds in Monochrome: 0 frames on the dock.
#   6. The dark scheme in Monochrome: a capture.
#
# Exit 0 when every step passed, 1 on a failure.
#
# Usage: dev/accept/dock.sh [--icons]
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
case "${1:-}" in
  --icons) MODE=icons; OUT_DIR="$SILL_OUT_ROOT/accept-icons" ;;
  "") MODE=dock; OUT_DIR="$SILL_OUT_ROOT/accept-dock" ;;
  *) echo "usage: dev/accept/dock.sh [--icons]" >&2; exit 2 ;;
esac
SHOTS="$SILL_SHOTS"
mkdir -p "$OUT_DIR" "$SHOTS"
EMITTER="$SILL_OUT/examples/badge_emitter"
BAR_H="${BAR_H:-24}"
RESERVE=68
PILL_RADIUS="${PILL_RADIUS:-12}"
TERM_ID=foot
PINS=(foot org.gnome.Nautilus google-chrome btop)
# Surface-local geometry of the default dock (design/10 §10.3.1): the surface is H_s tall (gut 8 +
# pad 6 + magnified tile 72 + tile 48 = 134, read off a capture by `read_geometry` rather than
# written down: it was 158 when the magnified tile was 96), the pill's bottom edge `gut` above the
# surface's bottom, its middle half `H_p` above that, the tiles' baseline `gut + pad` above the
# bottom, the running dot's centre 3 below the baseline (`dock.running_dot_gap_px`). Set by
# `read_geometry` once the dock is up.
GUT=8
PAD=6
TILE=48
PILL_Y=0 BASELINE_Y=0 DOT_Y=0 SURFACE_H=0

# Kill every foot running on casement's socket (never the host's).
kill_nested_terms() {
  local pid
  for pid in $(pgrep -x foot); do
    if tr '\0' '\n' <"/proc/$pid/environ" 2>/dev/null \
      | grep -qxE "WAYLAND_DISPLAY=($WL|${WL##*/})"; then
      kill "$pid" 2>/dev/null
    fi
  done
}

# The rest centre x of the first tile when the dock holds it and the Trash (dock/frame.rs:
# rest_centers(2) re-centred around the 13 px separator block): W0 = 2 x 48 + 8 + 2 x 6 = 116,
# the first centre the padding and half a tile in, less half the separator block.
first_tile_x() {
  echo $(( (OUT_W - 116) / 2 + 30 - 6 ))
}

# Waits up to 5 s for the dock's surface and sets DOCK to its name.
await_dock() {
  for _ in $(seq 50); do DOCK=$(surface_named sill-dock-); [ -n "$DOCK" ] && break; sleep 0.1; done
  [ -n "${DOCK:-}" ] || { echo "no dock surface in 'sill debug stats'" >&2; return 1; }
}

# The dock's height in logical px, from a capture of it (its width is OUT_W logical px), and the
# surface-local rows derived from it.
read_geometry() {
  local png="$OUT_DIR/geometry.png"
  sill debug capture "$DOCK" "$png" >/dev/null 2>&1 || { echo "no capture of $DOCK" >&2; return 1; }
  SURFACE_H=$(python3 - "$png" "$OUT_W" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1])
print(round(img.height * int(sys.argv[2]) / img.width))
PY
  ) || return 1
  PILL_Y=$(( SURFACE_H - GUT - (TILE + 2 * PAD) / 2 ))
  BASELINE_Y=$(( SURFACE_H - GUT - PAD ))
  DOT_Y=$(( BASELINE_Y + 3 ))
  echo "dock surface ${OUT_W}x${SURFACE_H}: pill middle $PILL_Y, baseline $BASELINE_Y, dot $DOT_Y" | tee "$OUT_DIR/geometry.log" >&2
}

start_dock_daemon() {
  mkdir -p "$CONFIG/sill"
  printf '[dock]\npill_radius_px = %s\n' "$PILL_RADIUS" >"$CONFIG/sill/settings.toml"
  backdrop_seed "$CONFIG" || return 1
  launch_daemon || return 1
  sill dock pin "$TERM_ID" >"$OUT_DIR/pin.log" 2>&1 || return 1
  await_dock || return 1
  sleep 1
  read_geometry || return 1
}

# 1.
step_exclusive_zone() {
  local height
  height=$(maximised_height) || return $?
  echo "output ${OUT_H}, maximised window ${height}, bar ${BAR_H}, dock ${RESERVE}" | tee "$OUT_DIR/exclusive-zone.log" >&2
  shot "$SHOTS/dock-casement.png" >/dev/null 2>&1 || true
  [ "$height" = "$(( OUT_H - BAR_H - RESERVE ))" ]
}

# 2.
step_sweep_and_idle() {
  local log="$OUT_DIR/sweep.log" f0 c0 f1 c1 f2 f3 frames callbacks idle
  read -r f0 c0 < <(counters_of "$DOCK")
  in_env python3 - "$SILL_BIN" "$DOCK" "$OUT_W" "$PILL_Y" <<'EOF' >"$log" 2>&1
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
  shot "$SHOTS/dock-casement-magnified.png" >/dev/null 2>&1 || true
  read -r f1 c1 < <(counters_of "$DOCK")
  frames=$(( f1 - f0 )); callbacks=$(( c1 - c0 ))
  echo "sweep: frames $frames callbacks $callbacks" | tee -a "$log" >&2
  sill debug leave "$DOCK" >/dev/null
  sleep 1
  await_frames_settled "$DOCK" || echo "the dock was still drawing 20 s after the leave" >&2
  read -r f2 _ < <(counters_of "$DOCK")
  echo "the leave's shrink: frames $(( f2 - f1 ))" | tee -a "$log" >&2
  sleep 5
  read -r f3 _ < <(counters_of "$DOCK")
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
  before=$(pixel "$DOCK" "$x" "$DOT_Y" "$OUT_W")
  cp "$OUT_DIR/pixel.png" "$OUT_DIR/launch-before.png"
  echo "click the foot tile at $x,$PILL_Y; dot pixel before: $before" >"$log"
  start=$(date +%s%N)
  sill debug click "$DOCK" "$x" "$PILL_Y" >>"$log" 2>&1 || return 1
  sill debug leave "$DOCK" >/dev/null
  local took found=no
  for _ in $(seq 60); do
    after=$(pixel "$DOCK" "$x" "$DOT_Y" "$OUT_W")
    dark $after && { found=yes; break; }
    sleep 0.05
  done
  cp "$OUT_DIR/pixel.png" "$OUT_DIR/launch-after.png"
  took=$(( ($(date +%s%N) - start) / 1000000 ))
  echo "dot pixel after ${took} ms: $after (dot: $found)" | tee -a "$log" >&2
  # Let the launch bounce land before the badge step looks at the tile.
  sleep 1
  shot "$OUT_DIR/launched.png" >/dev/null 2>&1 || true
  [ "$found" = yes ] && budget_check "the launched app's dot after ${took} ms (less the capture's own $(capture_cost_ms) ms) against 3000 ms" "$(( $(captured_ms "$took") <= 3000 ))"
}

# Whether an RGBA pixel is the running dot's ink (dark on the light pill).
dark() { [ $(( $1 + $2 + $3 )) -lt 330 ] && [ "$4" -gt 128 ]; }
# Whether an RGBA pixel is the badge's red.
red() { [ "$1" -gt 150 ] && [ "$2" -lt 110 ] && [ "$4" -gt 128 ]; }

# 4.
step_badge() {
  local log="$OUT_DIR/badge.log" x y before after start
  # Inside the badge's right padding, clear of its digit: quire's Small `Badge` is a 14 px tall
  # capsule hung off the plate's top right corner (`DockTile`), its right edge 2 px right of the
  # plate's (80.5 % of a 48 px tile) and its middle 7 px below the tile's top.
  x=$(( $(first_tile_x) + 21 )); y=$(( BASELINE_Y - TILE + 7 ))
  before=$(pixel "$DOCK" "$x" "$y" "$OUT_W")
  cp "$OUT_DIR/pixel.png" "$OUT_DIR/badge-before.png"
  start=$(date +%s%N)
  in_env "$EMITTER" "$TERM_ID" --count 5 >"$OUT_DIR/emitter.log" 2>&1 &
  local emitter=$!
  local found=no
  for _ in $(seq 40); do
    after=$(pixel "$DOCK" "$x" "$y" "$OUT_W")
    red $after && { found=yes; break; }
    sleep 0.025
  done
  cp "$OUT_DIR/pixel.png" "$OUT_DIR/badge-after.png"
  local took=$(( ($(date +%s%N) - start) / 1000000 ))
  kill "$emitter" 2>/dev/null; wait "$emitter" 2>/dev/null
  echo "badge pixel at $x,$y: $before -> $after after ${took} ms (red: $found)" | tee "$log" >&2
  [ "$found" = yes ] && budget_check "the badge after ${took} ms (less the capture's own $(capture_cost_ms) ms) against 1000 ms" "$(( $(captured_ms "$took") <= 1000 ))"
}

# 5.
step_menu() {
  local log="$OUT_DIR/menu.log" x opened=no
  x=$(first_tile_x)
  # A real right press at the tile's global point: casement dismisses a popup grabbed on a
  # synthetic click's serial. The dock is anchored to the bottom of the output, full width.
  surface_click "$DOCK" "$x" "$PILL_Y" --button secondary --origin 0 $(( OUT_H - SURFACE_H )) >"$log" 2>&1 || return 1
  for _ in $(seq 40); do
    sill debug capture "$DOCK/popup" "$OUT_DIR/menu-popup.png" >>"$log" 2>&1 && { opened=yes; break; }
    sleep 0.05
  done
  sleep 0.5
  shot "$SHOTS/dock-casement-menu.png" >/dev/null 2>&1 || true
  echo "popup $DOCK/popup opened: $opened" | tee -a "$log" >&2
  [ "$opened" = yes ]
}

# 6.
step_radius() {
  local log="$OUT_DIR/radius.log"
  sill debug leave "$DOCK" >/dev/null 2>&1
  sleep 0.5
  sill debug capture "$DOCK" "$OUT_DIR/radius.png" >"$log" 2>&1 || return 1
  python3 - "$OUT_DIR/radius.png" "$OUT_W" "$PILL_Y" "$PILL_RADIUS" <<'EOF' | tee -a "$log" >&2
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
scale = img.width / int(sys.argv[2])
mid = int(int(sys.argv[3]) * scale)
radius = int(sys.argv[4])
solid = lambda x, y: img.getpixel((x, y))[3] > 100  # the pill is ~150 (Dock tint), its shadow < 20
left = next(x for x in range(img.width) if solid(x, mid))
right = max(x for x in range(img.width) if solid(x, mid))
cx = (left + right) // 2
top = min(y for y in range(mid + 1) if all(solid(cx, yy) for yy in range(y, mid + 1)))
# The device pixel whose centre is 6.5 logical px in from the left edge and 1.5 down from the top.
probe = (left + int(6.5 * scale), top + int(1.5 * scale))
alpha = img.getpixel(probe)[3]
print(f"pill {left},{top}..{right} (device px); at 6.5, 1.5 px inside its corner alpha {alpha}; "
      f"squircle pill_radius_px {radius}: {'PASS' if (alpha > 100) == (radius <= 13) else 'FAIL'}")
EOF
  grep -q ': PASS$' "$log"
}

# Monochrome again, but `appearance.theme = "dark"`: no new raster (the dark scheme changes the
# plate alone, through quire's own stylesheet, not a re-tinted file), so no wait, only a shot.
style_monochrome_dark() {
  mkdir -p "$CONFIG/quire"
  printf '[appearance]\ntheme = "dark"\n\n[icons]\nstyle = "monochrome"\nmonochrome_tint = "space"\n' \
    >"$CONFIG/quire/appearance.toml"
}

style() {
  mkdir -p "$CONFIG/quire"
  printf '[icons]\nstyle = "%s"\nmonochrome_tint = "space"\n' "$1" >"$CONFIG/quire/appearance.toml"
}

# How many re-tinted rasters of style $1 the scratch cache holds.
cached() {
  find "$CACHE/sill/icons" -name "*-$1-*.png" 2>/dev/null | wc -l
}

step_icons_start() {
  style colour
  launch_daemon || return 1
  local id
  for id in "${PINS[@]}"; do sill dock pin "$id" >>"$OUT_DIR/pin.log" 2>&1 || return 1; done
  await_dock || return 1
  sleep 2
}

step_ping_compositor() {
  nested wayland-info >"$OUT_DIR/wayland-info.log" 2>&1
}

shoot_dock() {
  sill debug capture "$DOCK" "$SHOTS/dock-icons-$1.png" >"$OUT_DIR/capture-$1.log" 2>&1
  grep -q '"kind":"done"' "$OUT_DIR/capture-$1.log"
}

step_colour() {
  shoot_dock colour && [ "$(cached muted)" = 0 ] && [ "$(cached monochrome)" = 0 ]
}

# The launcher, queried for "foot" so foot shows as a row: its own
# hicolor icon on the row's plate, the fallback every entry takes (dev/out/launcher-icons.png).
step_launcher() {
  sill launcher open --query foot >>"$OUT_DIR/launcher.log" 2>&1
  local name="" rc=1
  for _ in $(seq 50); do name=$(surface_named sill-launcher-); [ -n "$name" ] && break; sleep 0.1; done
  if [ -n "$name" ]; then
    # The surface exists in `debug stats` before its first frame is presented; retry the
    # capture rather than racing it with one fixed sleep.
    for _ in $(seq 30); do
      sill debug capture "$name" "$SHOTS/launcher-icons.png" >"$OUT_DIR/capture-launcher.log" 2>&1
      grep -q '"kind":"done"' "$OUT_DIR/capture-launcher.log" && { rc=0; break; }
      sleep 0.2
    done
  else
    echo "no launcher surface in 'sill debug stats'" >&2
  fi
  sill launcher close >>"$OUT_DIR/launcher.log" 2>&1
  return "$rc"
}

# Switch to style $1 and wait for its rasters: one per pinned app (every app takes the theme
# path, so Muted and Monochrome both re-tint all four).
switch_and_wait() {
  local want=$2 n=0
  style "$1"
  for _ in $(seq 100); do
    n=$(cached "$1")
    [ "$n" -ge "$want" ] && break
    sleep 0.1
  done
  echo "$1: $n re-tinted rasters in the cache" | tee -a "$OUT_DIR/cache.log" >&2
  # One more paint after the last raster lands.
  sleep 1
  shoot_dock "$1" && [ "$n" -ge "$want" ]
}

step_muted() { switch_and_wait muted 4; }
step_monochrome() { switch_and_wait monochrome 4; }

# Once the rasters have landed nothing re-tints or repaints: five idle seconds, zero frames.
step_icons_idle() {
  local before after
  before=$(frames_of "$DOCK")
  sleep 5
  after=$(frames_of "$DOCK")
  echo "idle 5 s in Monochrome: frames $(( after - before ))" | tee "$OUT_DIR/idle.log" >&2
  [ "$before" = "$after" ]
}

# The plate under the dark scheme: the tinted plate's own dark stops, not
# the light ones the other shots show.
step_monochrome_dark() {
  style_monochrome_dark
  sleep 1
  shoot_dock monochrome-dark
}

cleanup() {
  local result=0
  stop_daemon || result=1
  kill_nested_terms
  stop_private_bus
  stop_compositor
  scratch_drop
  return "$result"
}

start_compositor || exit 1
nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (start_compositor failed)" >&2; exit 1; }
if [ "$MODE" = icons ]; then
  build || exit 1
else
  build "-p sill-testkit --example badge_emitter" || exit 1
  read -r OUT_W OUT_H < <(output_size)
  [ -n "${OUT_W:-}" ] || { echo "no nested output (start_compositor failed)" >&2; exit 1; }
fi
await_no_process || exit 1
scratch_new
start_private_bus || { scratch_drop; exit 1; }
if [ "$MODE" = icons ]; then
  run_steps step_ping_compositor step_icons_start
  status=$STATUS
  if [ "$status" = 0 ]; then
    run_steps step_colour step_launcher step_muted step_monochrome step_icons_idle step_monochrome_dark
    status=$STATUS
  fi
  ls -l "$CACHE/sill/icons" >"$OUT_DIR/cache-files.log" 2>&1
  cleanup || status=1
  echo "output dir: $OUT_DIR; shots: $SHOTS/dock-icons-{colour,muted,monochrome,monochrome-dark}.png, $SHOTS/launcher-icons.png" >&2
else
  start_dock_daemon || { cleanup; exit 1; }
  run_steps step_exclusive_zone step_sweep_and_idle step_launch step_badge step_menu step_radius
  status=$STATUS
  cleanup || status=1
  echo "logs in $OUT_DIR; screenshots in $SHOTS"
fi
exit "$status"

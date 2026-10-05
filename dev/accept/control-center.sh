#!/usr/bin/env bash
# Control center acceptance. Starts `sill run` from this checkout's debug build (`--features
# debug`) on a private bus (the daemon's session AND system bus), with a scratch HOME and XDG
# directories, its own XDG_RUNTIME_DIR and a fake DDC, and quits it at the end. Its settings hide
# the Sound and Battery bar items, so the status area reads, from the right: the clock, the
# control center, Wi-Fi (the one module item left).
#
# Default: the nested cosmic-comp (dev/nested.sh; its socket is reused and never stopped; design/20
# §1.5, design/13 §13.3.7). Nothing changes a real radio, level or profile: no switch or slider is
# ever pressed, and the only list the Wi-Fi pane asks for is a scan.
#
#   1. A debug click on the control center's item opens `sill-bar-<out>/popup` at the panel's
#      size (320 + the shadow room), its card 8 px inside the output's right edge (read from a
#      screenshot); a second click closes it. Screenshot dev/out/control-center-cosmic.png.
#   2. An outside click (the bar's empty middle) closes it; Escape (`sill debug key`: shell-host's
#      `Inject::Key`, which needs no grab) closes it too, or the step says why it could not try.
#   3. The Wi-Fi tile's chevron slides the Wi-Fi pane in: the popup takes the pane's size (a
#      Wi-Fi pane with a long list of networks may be as tall as the grid; then it only has to
#      change). Its only request is a scan.
#      Screenshot dev/out/control-center-wifi-cosmic.png.
#   4. The Wi-Fi bar item opens the Wi-Fi pane directly: its popup is about the pane's size from
#      step 3 (within 24 px: no back header), not the grid's.
#   5. `sill control-center toggle` opens it with no pointer event behind it, and again closes
#      it; whether cosmic-comp maps a popup opened without a fresh input serial, and whether its
#      grab holds (an outside click dismisses it), is printed. Not mapped: exit 2, naming it.
#   6. 5 s idle after closing: 0 frames on the bar.
#
# --real-input [bluetooth]: the Display module and the Bluetooth confirmation under real input,
# on shell-host's private headless KWin (scripts/dev/kwin-virtual.sh) driven by shell-host's
# scripts/dev/fake-input.sh: the control center is opened by a real click on its bar item, and its
# brightness capsule is dragged and clicked by a real pointer, with real serials, through KWin's
# own popup grab. No `sill debug` opener, click or pointer is used; `sill debug capture` and
# `sill debug stats` only look. The headless output is 3840 x 2160 at scale 2, as the user's BenQ
# MA270U (CC_SIZE and CC_SCALE change it), set through kscreen-doctor on KWin's own socket and bus.
#
# SAFETY (--real-input). No real monitor, no real outputs list, no real Bluetooth:
# - Bluetooth is a FAKE: dev/fake-bluez.py owns org.bluez on KWin's private bus (never the
#   system bus; it refuses one) with a made-up adapter and keyboard, and logs every Powered set
#   to a scratch file. The daemon's Bluetooth service reads it as its system bus.
# - DDC is a FAKE: the daemon runs with `sill run --fake-ddc FILE` naming a JSON file of one
#   made-up monitor on the headless output. The daemon's log must say so ("DDC is the fake in
#   ...") before any input is sent; otherwise the script stops. The fake never opens /dev/i2c-*.
# - The outputs list is FAKE: a stand-in `cosmic-randr` that never answers is first on the
#   daemon's PATH.
# - KWin's private bus is the daemon's session AND system bus, so the real BlueZ,
#   NetworkManager and the rest are simply absent; the power menu is a dry run.
#
#   1. A real click on the control center's bar item opens its popup; real Escape closes it.
#   2. Opened again by a click, a real click outside it closes it.
#   3. A real click on the item opens it again. The fake monitor did not answer the startup
#      probe (its file lists none): the capsule is disabled at 0 %, as on the user's run.
#   3b. The monitor answers now (40 %): with the control center open the service probes again
#      and the capsule comes alive at 40 % within its follow period (2 s).
#   4. External change: the fake monitor's brightness is rewritten 40 -> 90 (its own buttons);
#      within the service's follow period (2 s) the capsule shows it (two captures differ in
#      the capsule's band). The two captures also give the capsule's rail on screen.
#   5. Real drag: press at 30 %, eight moves to 75 % over about half a second, release. The fake
#      monitor receives writes DURING the drag (not only at the release), rising, and the last
#      is 75 (+-4).
#   6. Real click on the track at 20 %: the fake monitor receives 20 (+-4).
#   B1. A real click on the Bluetooth tile (its lit disc, found in a capture) shows the alert
#       "Turn Bluetooth off?" over the panel (the daemon logs `bluetooth_ask=shown`); the fake
#       adapter stays powered.
#   B2. Real Escape cancels it: the popup stays, the adapter stays powered.
#   B3. Asked again, a real click beside the alert cancels nothing (the alert is modal and dims
#       nothing); a real click on Cancel then cancels it.
#   B4. Asked again, a real click on Cancel cancels it.
#   B5. Asked again, a real click on Turn Off (the accent button in the capture) powers the fake
#       adapter off.
#   B6. A real click on the tile again powers it on, with no alert.
#   7. A real click outside closes the popup. (Real Escape first, recorded only: the control
#      center's keyboard focus after a pointer press is not checked here.)
#
# `--real-input bluetooth` runs step 3, B1-B6 and 7 only. The power menu's real-input steps are
# in dev/accept/power-menu.sh.
#
# Captures: dev/out/ (screenshots) and target/accept-control-center/*.png. Exit 0 only when every
# step passed; 1 when one failed; 2 when a step could not run safely (named).
#
# Usage: dev/nested.sh && dev/accept/control-center.sh
#        dev/accept/control-center.sh --real-input [bluetooth]
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
usage() { echo "usage: dev/accept/control-center.sh [--real-input [bluetooth]]" >&2; exit 2; }
SELECT=all
case "${1:-}" in
  --real-input)
    MODE=real
    case "${2:-}" in "") ;; bluetooth) SELECT=bluetooth ;; *) usage ;; esac ;;
  "") MODE=nested ;;
  *) usage ;;
esac
OUT_DIR="$SILL_OUT_ROOT/accept-control-center"
SHOTS="$SILL_SHOTS"
mkdir -p "$OUT_DIR" "$SHOTS"
# The panel's popup: the card (control_center.width_px, 320 by default) plus the transparent
# shadow room on each side (`SIDE` in crates/sill-bar/src/bar/popup.rs, read from the source so
# the script follows it) plus the card's own hairline border, a pixel or two a side. A pixel
# count is not a design value, so the check is a range: [card + 2 * room, + 2 * 4], the 8 being
# the most a hairline plus rounding can add. (It was a hard 376 and failed at 378.)
PANEL_CARD_W=320
PANEL_SIDE=$(sed -n 's/^pub(crate) const SIDE: i32 = \([0-9]*\);.*/\1/p' \
  "$SILL_REPO/crates/sill-bar/src/bar/popup.rs")
PANEL_SIDE="${PANEL_SIDE:-28}"
PANEL_POPUP_MIN=$(( PANEL_CARD_W + 2 * PANEL_SIDE ))
PANEL_POPUP_MAX=$(( PANEL_POPUP_MIN + 8 ))
panel_width_ok() { [ "${1:-0}" -ge "$PANEL_POPUP_MIN" ] && [ "${1:-0}" -le "$PANEL_POPUP_MAX" ]; }
# The output's scale: the headless KWin's is set (the user's BenQ MA270U: 3840 x 2160 at 2,
# CC_SCALE=1 CC_SIZE=1920x1080 for 1x), the nested compositor's is read as 1.
SCALE=1
if [ "$MODE" = real ]; then
  SIZE="${CC_SIZE:-3840x2160}"
  SCALE="${CC_SCALE:-2}"
  INPUT_TICKS=400
fi
scratch_new
trap scratch_drop EXIT
STANDINS="$SCRATCH/bin"
RECORD="$SCRATCH/power-menu.record"
BTLOG="$SCRATCH/bluez.powered"
BLUEZ=
NM_PID=
NMLOG="$SCRATCH/nm.requests"
NAME=
POPUP=
BAR=
# The Bluetooth tile's disc and the alert's buttons on screen (global logical px), from B1.
DISC=
TURN_OFF=
CANCEL=
SCRIM=
# The capsule's rail on screen (global logical px), from step 4.
RAIL_LEFT=
RAIL_WIDTH=
RAIL_Y=
SAFE=unproven

# The settings both modes run on.
write_settings() {
  mkdir -p "$CONFIG/sill"
  printf '[control_center]\nmenu_bar_sound = "hide"\nmenu_bar_battery = "hide"\n' \
    >"$CONFIG/sill/settings.toml"
}

# The status items' centres, right to left (the clock's first), from a capture of the bar: runs of
# inked columns, in logical px. Prints "x y" per item.
status_items() {
  capture "$BAR" "$OUT_DIR/bar.png" || return 1
  python3 - "$OUT_DIR/bar.png" "$SCALE" <<'EOF'
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
        if gap > 7 * float(sys.argv[2]):
            runs.append(run); run = None
scale = float(sys.argv[2])
for left, right in runs:
    print(round((left + right) / 2 / scale), round(h / 2 / scale))
EOF
}

# --- nested compositor ---

# SURFACE's size as "WIDTH HEIGHT", read from a capture (a popup is not in `debug stats`, but
# shell-host names it `<bar>/popup` for `debug capture`); nothing when it is not mapped.
size_of() {
  sill debug capture "$1" "$OUT_DIR/size.png" >/dev/null 2>&1 || return 1
  python3 -c 'import sys; from PIL import Image; print(*Image.open(sys.argv[1]).size)' \
    "$OUT_DIR/size.png"
}

popup_width() { size_of "$POPUP" | cut -d' ' -f1; }
popup_height() { size_of "$POPUP" | cut -d' ' -f2; }
popup_up() { mapped "$POPUP"; }

click_bar() { # x y
  surface_click "$BAR" "$1" "$2" >/dev/null 2>&1
}

close_popup() {
  popup_up || return 0
  click_bar $((BAR_W / 2)) $((BAR_H / 2))
  wait_surface "$POPUP" 20 down
}

# The card's right edge in a screenshot: the last column, on a row through the panel's first
# tiles, that differs from the column past the panel (the backdrop).
card_right_gap() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
w, h = img.size
y = 60
ground = img.getpixel((w - 1, y))
right = max((x for x in range(w - 60, w)
             if sum(abs(a - b) for a, b in zip(img.getpixel((x, y)), ground)) > 40), default=-1)
print(w - 1 - right)
EOF
}

# 1.
step_item_click() {
  local log="$OUT_DIR/item.log" width gap
  : >"$log"
  # shellcheck disable=SC2086
  click_bar $CC_ITEM
  wait_surface "$POPUP" 20 up || { echo "a click on the item opened no popup" | tee -a "$log" >&2; return 1; }
  sleep 0.6
  width=$(popup_width)
  sill debug capture "$POPUP" "$OUT_DIR/panel.png" >>"$log" 2>&1 || return 1
  # A screenshot taken before the compositor shows the popup has no card in it (gap = the whole
  # width): take it again, up to four times.
  for _ in 1 2 3 4; do
    [ "$COMPOSITOR" != casement ] || break # no output screenshot there yet (below)
    shot "$SHOTS/control-center-cosmic.png" >>"$log" 2>&1 \
      || echo "screenshot failed" >&2
    gap=$(card_right_gap "$SHOTS/control-center-cosmic.png")
    [ "$gap" -lt 100 ] && break
    sleep 0.5
  done
  # shellcheck disable=SC2086
  click_bar $CC_ITEM
  wait_surface "$POPUP" 20 down || { echo "a second click left it open" | tee -a "$log" >&2; return 1; }
  if [ "$COMPOSITOR" = casement ]; then
    echo "item click: popup ${width} wide (want $PANEL_POPUP_MIN to $PANEL_POPUP_MAX); a second click closed it" | tee -a "$log" >&2
    panel_width_ok "$width" || return 1
    expected_skip "the card's distance from the output's right edge is read off an output screenshot (casementctl shot, P1-C1)"
    return 2
  fi
  echo "item click: popup ${width} wide (want $PANEL_POPUP_MIN to $PANEL_POPUP_MAX), card ${gap} px from the output's right edge (want about 8); a second click closed it" \
    | tee -a "$log" >&2
  panel_width_ok "$width" && [ "$gap" -ge 4 ] && [ "$gap" -le 12 ]
}

# 2.
step_dismiss() {
  local log="$OUT_DIR/dismiss.log" escape
  : >"$log"
  # shellcheck disable=SC2086
  click_bar $CC_ITEM
  wait_surface "$POPUP" 20 up || return 1
  sleep 0.3
  close_popup || { echo "an outside click left it open" | tee -a "$log" >&2; return 1; }
  # shellcheck disable=SC2086
  click_bar $CC_ITEM
  wait_surface "$POPUP" 20 up || return 1
  sleep 0.3
  if sill debug key "$POPUP" Escape >>"$log" 2>&1; then
    if wait_surface "$POPUP" 20 down; then escape="Escape closed it"; else
      escape="Escape did not reach it: left open"; close_popup; fi
  else
    escape="Escape not tried: the daemon refused the key"; close_popup
  fi
  echo "an outside click closed it; $escape" | tee -a "$log" >&2
  ! popup_up
}

# 3.
step_wifi_pane() {
  local log="$OUT_DIR/wifi.log" before after
  : >"$log"
  # shellcheck disable=SC2086
  click_bar $CC_ITEM
  wait_surface "$POPUP" 20 up || return 1
  sleep 0.5
  before=$(popup_height)
  # The Wi-Fi tile's chevron, popup-local: the card at (28, 8) with a hairline, the panel's
  # padding 12, a half tile 144 wide and 52 tall, its 18 px chevron 4 in from the right.
  sill debug click "$POPUP" 171 47 >>"$log" 2>&1
  sleep 0.8
  after=$(popup_height)
  shot "$SHOTS/control-center-wifi-cosmic.png" >>"$log" 2>&1 \
    || echo "screenshot failed" >&2
  close_popup
  PANE_H=$after
  GRID_H=$before
  echo "the chevron: popup ${before} -> ${after} tall; the fake NetworkManager was asked: $(tr '\n' ' ' <"$NMLOG")" \
    | tee -a "$log" >&2
  # The pane's only request is a scan: no radio, join or disconnect reached the fake.
  ! grep -qv '^scan$' "$NMLOG" || { echo "the Wi-Fi pane asked for more than a scan" >&2; return 1; }
  [ -n "$after" ] && [ "$after" != "$before" ]
}

# 4.
step_module_item() {
  local log="$OUT_DIR/module.log" width height
  : >"$log"
  # shellcheck disable=SC2086
  click_bar $WIFI_ITEM
  wait_surface "$POPUP" 20 up || { echo "the Wi-Fi item opened no popup" | tee -a "$log" >&2; return 1; }
  sleep 0.5
  width=$(popup_width)
  height=$(popup_height)
  sill debug capture "$POPUP" "$OUT_DIR/wifi-item.png" >>"$log" 2>&1
  close_popup
  echo "the Wi-Fi item: a ${width}x${height} popup (the pane from step 3 is ${PANE_H:-?} tall, the grid ${GRID_H:-?})" \
    | tee -a "$log" >&2
  # The bar item's dropdown is the pane without the in-panel page's back header: a few px
  # shorter than the chevron's page (281 against 287 at the time of writing), so "the pane's
  # size" is within 24 px of it, and far from the grid's.
  panel_width_ok "$width" && [ -n "${PANE_H:-}" ] && [ $(( height - PANE_H )) -ge -24 ] \
    && [ $(( height - PANE_H )) -le 24 ] && [ "$height" != "${GRID_H:-}" ]
}

# 5.
step_toggle() {
  local log="$OUT_DIR/toggle.log" held grab
  : >"$log"
  sill control-center toggle >>"$log" 2>&1 || { echo "the toggle request failed" >&2; return 1; }
  if ! wait_surface "$POPUP" 20 up; then
    echo "sill control-center toggle: cosmic-comp did not map the popup (grab without a fresh input serial refused)" \
      | tee -a "$log" >&2
    return 2
  fi
  sleep 1
  if popup_up; then held="it stays mapped"; else held="it was dismissed at once (popup_done)"; fi
  sill debug capture "$POPUP" "$OUT_DIR/toggled.png" >/dev/null 2>&1
  sill control-center toggle >>"$log" 2>&1
  wait_surface "$POPUP" 20 down || { echo "a second toggle left it open" | tee -a "$log" >&2; return 1; }
  sill control-center toggle >>"$log" 2>&1
  wait_surface "$POPUP" 20 up || return 1
  sleep 0.5
  if close_popup; then grab="an outside click on the bar closes it"; else
    grab="an outside click left it open"; fi
  echo "sill control-center toggle: mapped with no pointer event behind it, $held; a second toggle closed it; $grab" \
    | tee -a "$log" >&2
  ! popup_up
}

# 6.
step_idle() {
  local log="$OUT_DIR/idle.log" f0 f1
  close_popup
  sleep 1
  await_frames_settled "$BAR" || echo "the bar was still drawing 20 s after the popup closed" >&2
  clear_of_the_minute
  f0=$(frames_of "$BAR")
  sleep 5
  f1=$(frames_of "$BAR")
  echo "5 s idle after closing: $BAR $(( f1 - f0 )) frames" | tee "$log" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

start_nested_daemon() {
  backdrop_seed "$CONFIG" || return 1
  write_settings
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  BAR="sill-bar-$NAME"
  POPUP="$BAR/popup"
  sleep 1.5
  mapfile -t ITEMS < <(status_items)
  [ "${#ITEMS[@]}" -ge 3 ] || { echo "found ${#ITEMS[@]} status items, want 3" >&2; return 1; }
  CC_ITEM=${ITEMS[1]}
  WIFI_ITEM=${ITEMS[2]}
  read -r BAR_W BAR_H < <(size_of "$BAR")
  echo "bar $BAR ${BAR_W}x${BAR_H}; control center item at $CC_ITEM, Wi-Fi at $WIFI_ITEM" >&2
}

cleanup_nested() {
  local result=0
  backdrop_down
  stop_daemon || result=1
  stop_nm
  stop_private_bus
  stop_compositor
  scratch_drop
  return "$result"
}

run_nested() {
  start_compositor || exit 1
  nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
  build || exit 1
  start_private_bus || exit 1
  start_nm || { cleanup_nested; exit 1; }
  start_nested_daemon || { cleanup_nested; exit 1; }
  backdrop_up "$SCRATCH/backdrop" || echo "backdrop: none (the shot shows the bare compositor)" >&2
  run_steps step_item_click step_dismiss step_wifi_pane step_module_item step_toggle step_idle
  status=$STATUS
  cleanup_nested || status=1
  echo "logs in $OUT_DIR; screenshots $SHOTS/control-center-cosmic.png, $SHOTS/control-center-wifi-cosmic.png"
  exit "$status"
}

# --- headless KWin, real input ---

# The headless output's connector name, from KWin's wl_output (v4 name).
headless_output() {
  env WAYLAND_DISPLAY="$WL" wayland-info 2>/dev/null \
    | sed -n "s/.*name: '\\{0,1\\}\\([A-Za-z]*-[0-9A-Za-z-]*\\)'\\{0,1\\}$/\\1/p" | head -n 1
}

# The fake monitor at brightness $1; written whole, then renamed into place.
fake_monitor() {
  printf '{"displays":[{"id":7,"connector":"%s","product":"Fake Monitor","brightness":{"value":%s,"max":100}}]}\n' \
    "$NAME" "$1" >"$DDC.tmp" && mv "$DDC.tmp" "$DDC"
}

# The stand-ins first on the daemon's PATH: a cosmic-randr that never answers (KWin has no
# zwlr_output_manager_v1), so the displays service reports no outputs.
standins() {
  mkdir -p "$STANDINS"
  cat >"$STANDINS/cosmic-randr" <<'EOF'
#!/bin/sh
echo "accept-control-center: stand-in cosmic-randr, no answer" >&2
exit 1
EOF
  chmod +x "$STANDINS/cosmic-randr"
}

start_real_daemon() {
  write_settings
  : >"$RECORD"
  IN_ENV_EXTRA=("PATH=$STANDINS:$PATH")
  launch_daemon --session-dry-run "$RECORD" || return 1
  DDC="$FAKE_DDC"
  WRITES="$DDC.writes"
  local bar
  bar=$(output_name) || return 1
  [ "$bar" = "$NAME" ] || { echo "the bar is on '$bar', not '$NAME'" >&2; return 1; }
  BAR="sill-bar-$NAME"
  POPUP="$BAR/popup"
  sleep 1.5
}

# The fake BlueZ on KWin's private bus, before the daemon (its Bluetooth service seeds once).
start_bluez() {
  : >"$BTLOG"
  python3 "$SILL_REPO/dev/fake-bluez.py" "$BUS" "$BTLOG" >"$OUT_DIR/bluez.log" 2>&1 &
  BLUEZ=$!
  for _ in $(seq 50); do
    grep -q "fake-bluez: org.bluez on $BUS" "$OUT_DIR/bluez.log" && return 0
    kill -0 "$BLUEZ" 2>/dev/null || break
    sleep 0.1
  done
  echo "the fake BlueZ did not start (see $OUT_DIR/bluez.log)" >&2
  return 1
}

# The fake NetworkManager on the private bus, before the daemon (the Network and Wi-Fi services
# seed once). It logs scans, radio and connection requests to $NMLOG and changes nothing real.
start_nm() {
  : >"$NMLOG"
  python3 "$SILL_REPO/dev/fake-nm.py" "$BUS" "$NMLOG" >"$OUT_DIR/nm.log" 2>&1 &
  NM_PID=$!
  for _ in $(seq 50); do
    grep -q "fake-nm: org.freedesktop.NetworkManager on $BUS" "$OUT_DIR/nm.log" && return 0
    kill -0 "$NM_PID" 2>/dev/null || break
    sleep 0.1
  done
  echo "the fake NetworkManager did not start (see $OUT_DIR/nm.log)" >&2
  return 1
}

stop_nm() {
  [ -n "${NM_PID:-}" ] || return 0
  kill "$NM_PID" 2>/dev/null
  reap "$NM_PID" "fake NetworkManager"
  NM_PID=
}

stop_bluez() {
  [ -n "$BLUEZ" ] || return 0
  kill "$BLUEZ" 2>/dev/null
  reap "$BLUEZ" "fake BlueZ"
  BLUEZ=
}

# The fake DDC and the fake outputs are in force, from the daemon's own log. Sets SAFE=proven.
prove_safe() {
  local log="$OUT_DIR/safety.log"
  {
    grep "sill: displays: DDC is the fake in $DDC" "$OUT_DIR/daemon.log" || return 1
    grep "power_menu.backend=dry-run" "$OUT_DIR/daemon.log" || return 1
    grep "fake-bluez: org.bluez on $BUS" "$OUT_DIR/bluez.log" || return 1
  } >"$log" 2>&1
  SAFE=proven
  echo "safety proven from the daemon's log: fake DDC ($DDC), dry-run power menu," \
    "fake BlueZ on the private bus" >&2
}

# The popup's top-left on screen, "x y": KWin's window whose buffer (logical) is the popup
# capture's size over the scale.
popup_origin() {
  capture "$POPUP" "$OUT_DIR/popup-size.png" || return 1
  local size
  size=$(python3 -c 'import sys; from PIL import Image; print(*Image.open(sys.argv[1]).size)' \
    "$OUT_DIR/popup-size.png")
  "$SHELL_HOST_DEV/kwin-windows.sh" >"$OUT_DIR/windows.log" 2>&1
  # shellcheck disable=SC2086
  python3 - "$OUT_DIR/windows.log" $size "$SCALE" <<'EOF'
import json, sys
path, w, h, scale = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), float(sys.argv[4])
for line in open(path):
    line = line.strip()
    if not line.startswith("{"):
        continue
    win = json.loads(line)
    bx, by, bw, bh = win["buffer"]
    if abs(bw * scale - w) <= 1 and abs(bh * scale - h) <= 1:
        print(round(bx), round(by)); break
EOF
}

# The fake monitor's brightness now, and its writes so far (space-separated values).
monitor_level() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["displays"][0]["brightness"]["value"])' "$DDC"; }
writes() { [ -f "$WRITES" ] && sed -n 's/^write 7 Brightness //p' "$WRITES" | tr '\n' ' '; }
write_count() { [ -f "$WRITES" ] && grep -c '^write 7 Brightness' "$WRITES" || echo 0; }

# Real 1. A real click on the control center's item opens it; real Escape closes it.
step_real_click_escape() {
  local items
  mapfile -t items < <(status_items)
  [ "${#items[@]}" -ge 2 ] || { echo "found ${#items[@]} status items" >&2; return 1; }
  CC_ITEM=${items[1]}
  # shellcheck disable=SC2086
  click $CC_ITEM
  wait_surface "$POPUP" 20 up || { echo "a click on the item ($CC_ITEM) opened nothing" >&2; return 1; }
  sleep 0.5
  capture "$POPUP" "$OUT_DIR/control-center.png"
  key Escape
  wait_surface "$POPUP" 20 down || { echo "Escape left the control center up" >&2; return 1; }
  echo "a real click on the item ($CC_ITEM) opened the control center; real Escape closed it" >&2
}

# Real 2.
step_real_outside_click() {
  # shellcheck disable=SC2086
  click $CC_ITEM
  wait_surface "$POPUP" 20 up || { echo "the second click opened nothing" >&2; return 1; }
  sleep 0.5
  click 200 $(( ${SIZE#*x} / SCALE / 2 ))
  wait_surface "$POPUP" 20 down || { echo "an outside click left the control center up" >&2; return 1; }
  echo "a real click outside closed the control center" >&2
}

# 3.
step_real_open() {
  local items
  mapfile -t items < <(status_items)
  [ "${#items[@]}" -ge 2 ] || { echo "found ${#items[@]} status items" >&2; return 1; }
  CC_ITEM=${items[1]}
  # shellcheck disable=SC2086
  click $CC_ITEM || return 1
  wait_surface "$POPUP" 30 up || { echo "a real click on the item ($CC_ITEM) opened nothing" >&2; return 1; }
  sleep 0.8
  capture "$POPUP" "$OUT_DIR/open.png"
  echo "a real click on the control center's item ($CC_ITEM) opened it" >&2
}

# The band two popup captures differ in, "x0 x1 y height": the changed rows grouped into
# bands; in the widest band, the longest run of changed columns (the fill between two levels,
# not the glyph beside it). Nothing when they are the same.
changed_band() {
  python3 - "$OUT_DIR/$1.png" "$OUT_DIR/$2.png" <<'EOF'
import sys
import numpy as np
from PIL import Image
a, b = (np.asarray(Image.open(p).convert("RGB"), dtype=int) for p in sys.argv[1:3])
if a.shape != b.shape:
    sys.exit(0)
diff = np.abs(a - b).sum(axis=2) > 40
rows = np.where(diff.sum(axis=1) > 0)[0]
if rows.size == 0:
    sys.exit(0)
bands, start = [], rows[0]
for r0, r1 in zip(rows, rows[1:]):
    if r1 > r0 + 2:
        bands.append((start, r0)); start = r1
bands.append((start, rows[-1]))
def longest_run(cols):
    best, run = (cols[0], cols[0]), [cols[0], cols[0]]
    for c in cols[1:]:
        if c <= run[1] + 3:
            run[1] = c
        else:
            run = [c, c]
        if run[1] - run[0] > best[1] - best[0]:
            best = (run[0], run[1])
    return best
found = None
for top, bottom in bands:
    cols = np.where(diff[top:bottom + 1].sum(axis=0) > 0)[0]
    x0, x1 = longest_run(list(cols))
    if found is None or x1 - x0 > found[1] - found[0]:
        found = (x0, x1, (top + bottom) // 2, bottom - top + 1)
print(*found)
EOF
}

# The rail's ends on row $3 of the popup, "left right": from the 40 % edge ($1) left while the
# 40 % capture shows the fill's colour, from the 90 % edge ($2) right while the 90 % capture
# shows the well's.
rail_ends() {
  python3 - "$OUT_DIR/level-40.png" "$OUT_DIR/level-90.png" "$1" "$2" "$3" <<'EOF'
import sys
from PIL import Image
low, high = (Image.open(p).convert("RGB") for p in sys.argv[1:3])
x0, x1, y = (int(v) for v in sys.argv[3:6])
def far(a, b, by):
    return sum(abs(p - q) for p, q in zip(a, b)) > by
well = high.getpixel((x1 + 4, y))
right = x1 + 4
while right < high.size[0] - 1 and not far(high.getpixel((right + 1, y)), well, 24):
    right += 1
# The module's plate just past the rail's end; the rail starts where the plate stops, left of
# the fill (and the glyph inside it). The knob's soft shadow at the fill's end is plate-coloured
# for a pixel or two, so the plate must run for 8 pixels before it counts as the rail's end.
plate = high.getpixel((right + 3, y))
left = x0
while left > 8 and any(far(low.getpixel((left - k, y)), plate, 8) for k in range(1, 9)):
    left -= 1
print(left, right + 1)
EOF
}

# 4. Also measures the rail: RAIL_LEFT, RAIL_WIDTH, RAIL_Y (global).
step_real_external() {
  capture "$POPUP" "$OUT_DIR/level-40.png" || return 1
  fake_monitor 90
  local band= waited=0
  for _ in $(seq 30); do
    sleep 0.2
    waited=$(( waited + 200 ))
    capture "$POPUP" "$OUT_DIR/level-90.png" || continue
    band=$(changed_band level-40 level-90)
    [ -n "$band" ] && break
  done
  [ -n "$band" ] || { echo "the capsule never showed the monitor's 90 (no change in the popup)" >&2; return 1; }
  # Let the slide to the new level finish, then measure.
  sleep 0.8
  capture "$POPUP" "$OUT_DIR/level-90.png" || return 1
  band=$(changed_band level-40 level-90)
  echo "the capsule followed the monitor's own 90 within about ${waited} ms" >&2
  local x0 x1 y h origin ox oy left right
  read -r x0 x1 y h <<<"$band"
  # The rail's ends on that row: left of the 40 % edge the fill runs to the rail's start (the
  # 40 % capture), right of the 90 % edge the well runs to its end (the 90 % capture).
  read -r left right <<<"$(rail_ends "$x0" "$x1" "$y")"
  [ -n "$right" ] || { echo "the rail's ends were not found on row $y" >&2; return 1; }
  origin=$(popup_origin)
  [ -n "$origin" ] || { echo "no KWin window the popup's size (see $OUT_DIR/windows.log)" >&2; return 1; }
  read -r ox oy <<<"$origin"
  # The captures are in the output's pixels; the pointer moves in logical ones.
  RAIL_WIDTH=$(( (right - left) / SCALE ))
  RAIL_LEFT=$(( ox + left / SCALE ))
  RAIL_Y=$(( oy + y / SCALE ))
  echo "the fake monitor's own buttons 40 -> 90: the capsule followed (changed band x $x0-$x1," \
    "y $y, $h px tall, in the popup at $ox,$oy); rail on screen x $RAIL_LEFT + $RAIL_WIDTH, y $RAIL_Y" >&2
  [ "$RAIL_WIDTH" -gt 60 ]
}

# $1 % along the knob's travel: the control center's level is a capsule with a knob
# (design/29), whose centre travels from half the capsule's height (a Regular control's 22) in
# at each end.
KNOB_HALF=11
at() { echo $(( RAIL_LEFT + KNOB_HALF + (RAIL_WIDTH - 2 * KNOB_HALF) * $1 / 100 )) "$RAIL_Y"; }

# 3b.
step_real_late_monitor() {
  capture "$POPUP" "$OUT_DIR/absent.png" || return 1
  fake_monitor 40
  local waited=0
  for _ in $(seq 30); do
    sleep 0.2
    waited=$(( waited + 200 ))
    capture "$POPUP" "$OUT_DIR/alive.png" || continue
    [ -n "$(changed_band absent alive)" ] && break
  done
  [ -n "$(changed_band absent alive)" ] || {
    echo "the capsule never came alive after the monitor answered (still as at the start)" >&2
    return 1; }
  sleep 0.8
  echo "the monitor answered late: the open control center probed again and the capsule came" \
    "alive within about ${waited} ms" >&2
}

# 5.
step_real_drag() {
  local from to before during after last p x y
  read -r from y <<<"$(at 30)"
  read -r to _ <<<"$(at 75)"
  before=$(write_count)
  input "move $from $y" "sleep 80" "press left" "sleep 60" || return 1
  for p in 1 2 3 4 5 6 7 8; do
    x=$(( from + (to - from) * p / 8 ))
    input "move $x $y" "sleep 70" || return 1
  done
  during=$(write_count)
  input "release left" "sleep 600" || return 1
  after=$(write_count)
  last=$(monitor_level)
  capture "$POPUP" "$OUT_DIR/dragged.png"
  echo "real drag 30 % -> 75 % (x $from -> $to, y $y): the fake monitor got $(( during - before ))" \
    "write(s) during the drag, $(( after - before )) in all: [$(writes)]; it is at $last" >&2
  [ $(( during - before )) -ge 1 ] && [ "$last" -ge 71 ] && [ "$last" -le 79 ] || return 1
  python3 - "$(writes)" "$before" <<'EOF' || { echo "the writes did not rise with the drag" >&2; return 1; }
import sys
values = [int(v) for v in sys.argv[1].split()][int(sys.argv[2]):]
sys.exit(0 if values and all(a < b for a, b in zip(values, values[1:])) else 1)
EOF
}

# 6.
step_real_click() {
  local x y last
  read -r x y <<<"$(at 20)"
  click "$x" "$y" || return 1
  sleep 0.5
  last=$(monitor_level)
  capture "$POPUP" "$OUT_DIR/clicked.png"
  echo "real click on the track at 20 % (x $x): the fake monitor is at $last; writes [$(writes)]" >&2
  [ "$last" -ge 16 ] && [ "$last" -le 24 ]
}

# The blue blobs of popup capture $1, one per line, "cx cy w h" in the capture's logical px:
# pixels whose blue beats their red by more than $2 (60: the accent itself, a lit disc or a
# filled button, about (148, 192, 254); 20 also takes a tint of it).
accent_blobs() {
  python3 - "$OUT_DIR/$1.png" "$SCALE" "$2" <<'EOF'
import sys
import numpy as np
from PIL import Image
k = float(sys.argv[2])
img = np.asarray(Image.open(sys.argv[1]).convert("RGB"), dtype=int)
r, g, b = img[..., 0], img[..., 1], img[..., 2]
mask = (b > 180) & (b - r > int(sys.argv[3]))
h, w = mask.shape
seen = np.zeros_like(mask)
for y0 in range(0, h, 2):
    for x0 in range(0, w, 2):
        if not mask[y0, x0] or seen[y0, x0]:
            continue
        stack, xs, ys = [(y0, x0)], [], []
        seen[y0, x0] = True
        while stack:
            y, x = stack.pop()
            xs.append(x); ys.append(y)
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                ny, nx = y + dy, x + dx
                if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                    seen[ny, nx] = True
                    stack.append((ny, nx))
        bw, bh = (max(xs) - min(xs) + 1) / k, (max(ys) - min(ys) + 1) / k
        if bw >= 12 and bh >= 12:
            print(round((min(xs) + max(xs)) / 2 / k), round((min(ys) + max(ys)) / 2 / k),
                  round(bw), round(bh))
EOF
}

asks() { grep -c "control_center.bluetooth_ask=shown" "$OUT_DIR/daemon.log"; }
cancels() { grep -c "control_center.bluetooth_ask=cancel" "$OUT_DIR/daemon.log"; }
powered_offs() { grep -c '^powered false' "$BTLOG"; }
powered_ons() { grep -c '^powered true' "$BTLOG"; }

# Wait up to $2 tenths for command $1 to print at least $3.
await_count() {
  for _ in $(seq "$2"); do [ "$($1)" -ge "$3" ] && return 0; sleep 0.1; done
  return 1
}

# Whether B1 found the tile and the buttons.
found() { [ -n "$DISC" ] && [ -n "$TURN_OFF" ] || { echo "B1 found no tile or buttons" >&2; return 1; }; }

# A real click on the tile's disc raises the alert (the ask count rises by one).
raise_alert() {
  local before
  before=$(asks)
  # shellcheck disable=SC2086
  click $DISC || return 1
  await_count asks 30 $(( before + 1 )) || { echo "a real click on the tile ($DISC) asked nothing" >&2; return 1; }
  sleep 0.5
}

# The blobs of capture $1 not in capture $2 (none within 4 px of one there).
new_blobs() {
  local bx by bw bh
  accent_blobs "$1" 60 | while read -r bx by bw bh; do
    accent_blobs "$2" 60 | awk -v x="$bx" -v y="$by" \
      '($1-x)^2 + ($2-y)^2 < 16 { found = 1 } END { exit found }' && echo "$bx $by $bw $bh"
  done
}

# B1. Also finds the buttons: DISC, TURN_OFF, CANCEL, SCRIM (global logical px).
step_real_bt_ask() {
  local origin ox oy disc blob cx cy bw bh
  capture "$POPUP" "$OUT_DIR/bt-before.png" || return 1
  origin=$(popup_origin)
  [ -n "$origin" ] || { echo "no KWin window the popup's size" >&2; return 1; }
  read -r ox oy <<<"$origin"
  # A lit tile's disc is the accent (its plate is not tinted): Bluetooth's is the only lit disc
  # here (Wi-Fi has no NetworkManager, Focus is off), the topmost accent blob about a Large
  # control (28) across.
  accent_blobs bt-before 60 >"$OUT_DIR/bt-before.blobs"
  disc=$(awk '$3 >= 24 && $3 <= 34 && $4 >= 24 && $4 <= 34' "$OUT_DIR/bt-before.blobs" | sort -k2 -n | head -n 1)
  [ -n "$disc" ] || { echo "no lit tile in the popup (bt-before.png)" >&2; return 1; }
  read -r cx cy bw _ <<<"$disc"
  DISC="$(( ox + cx )) $(( oy + cy ))"
  # Just under the disc, still on the tile: the scrim covers it while the alert is up.
  SCRIM="$(( ox + cx )) $(( oy + cy + 20 ))"
  raise_alert || return 1
  capture "$POPUP" "$OUT_DIR/bt-alert.png" || return 1
  # Turn Off: the wide accent blob the alert added.
  new_blobs bt-alert bt-before >"$OUT_DIR/bt-alert.blobs"
  blob=$(awk '$3 > 2.5 * $4 && $4 >= 18 && $4 <= 44' "$OUT_DIR/bt-alert.blobs" | head -n 1)
  [ -n "$blob" ] || { echo "no Turn Off button in the capture (bt-alert.png)" >&2; return 1; }
  read -r cx cy bw bh <<<"$blob"
  TURN_OFF="$(( ox + cx )) $(( oy + cy ))"
  CANCEL="$(( ox + cx - bw - 10 )) $(( oy + cy ))"
  [ "$(powered_offs)" = 0 ] || { echo "the adapter went off on the tile press" >&2; return 1; }
  echo "a real click on the Bluetooth tile ($DISC) showed the alert (bt-alert.png): Turn Off at" \
    "$TURN_OFF (${bw}x${bh}), Cancel at $CANCEL; the fake adapter is still powered" >&2
}

# Wait for a cancel after the cancels were $1; the popup stays, the adapter stays on.
cancelled() {
  await_count cancels 30 $(( $1 + 1 )) || return 1
  sleep 0.4
  mapped "$POPUP" || { echo "it closed the control center too" >&2; return 1; }
  [ "$(powered_offs)" = 0 ] || { echo "the adapter went off" >&2; return 1; }
}

# B2.
step_real_bt_escape() {
  local before
  before=$(cancels)
  key Escape || return 1
  cancelled "$before" || { echo "real Escape did not cancel the alert" >&2; return 1; }
  capture "$POPUP" "$OUT_DIR/bt-escaped.png"
  echo "real Escape cancelled the alert; the control center stays; the adapter stays powered" >&2
}

# B3.
step_real_bt_beside() {
  local before
  found && raise_alert || return 1
  before=$(cancels)
  # shellcheck disable=SC2086
  click $SCRIM || return 1
  sleep 1
  [ "$(cancels)" = "$before" ] || { echo "a real click beside the alert ($SCRIM) cancelled it" >&2; return 1; }
  mapped "$POPUP" || { echo "a click beside the alert closed the control center" >&2; return 1; }
  # shellcheck disable=SC2086
  click $CANCEL || return 1
  cancelled "$before" || { echo "a real click on Cancel ($CANCEL) did not cancel" >&2; return 1; }
  echo "a real click beside the alert ($SCRIM) cancelled nothing; Cancel then cancelled it" >&2
}

# B4.
step_real_bt_cancel() {
  local before
  found && raise_alert || return 1
  before=$(cancels)
  # shellcheck disable=SC2086
  click $CANCEL || return 1
  cancelled "$before" || { echo "a real click on Cancel ($CANCEL) did not cancel" >&2; return 1; }
  echo "a real click on Cancel ($CANCEL) cancelled the alert; the adapter stays powered" >&2
}

# B5.
step_real_bt_turn_off() {
  found && raise_alert || return 1
  # shellcheck disable=SC2086
  click $TURN_OFF || return 1
  await_count powered_offs 30 1 || { echo "a real click on Turn Off ($TURN_OFF) left the adapter on" >&2; return 1; }
  sleep 0.6
  capture "$POPUP" "$OUT_DIR/bt-off.png"
  echo "a real click on Turn Off ($TURN_OFF) powered the fake adapter off (bt-off.png)" >&2
}

# B6.
step_real_bt_on() {
  local asked ons
  found || return 1
  asked=$(asks); ons=$(powered_ons)
  # shellcheck disable=SC2086
  click $DISC || return 1
  await_count powered_ons 30 $(( ons + 1 )) || { echo "a real click on the tile left the adapter off" >&2; return 1; }
  sleep 0.6
  capture "$POPUP" "$OUT_DIR/bt-on.png"
  [ "$(asks)" = "$asked" ] || { echo "turning Bluetooth on asked first" >&2; return 1; }
  echo "a real click on the tile powered the fake adapter on, with no alert (bt-on.png)" >&2
}

# 7.
step_real_close() {
  key Escape || return 1
  if wait_surface "$POPUP" 20 down; then
    echo "real Escape (recorded only) closed the control center" >&2
    return 0
  fi
  echo "real Escape (recorded only) left the control center up after the pointer presses" >&2
  click 200 $(( ${SIZE#*x} / SCALE / 2 )) || return 1
  wait_surface "$POPUP" 20 down || { echo "an outside click left the control center up" >&2; return 1; }
  echo "a real click outside closed the control center" >&2
}

cleanup_real() {
  stop_input
  stop_daemon
  stop_bluez
  stop_compositor
  scratch_drop
}

run_real() {
  command -v wayland-info >/dev/null || { echo "wayland-info is missing; not installed by this script" >&2; exit 1; }
  build || exit 1
  start_compositor --default kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; scratch_drop; exit 1; }
  trap cleanup_real EXIT
  : >"$OUT_DIR/capture.log"
  NAME=$(headless_output)
  [ -n "$NAME" ] || { echo "no output name from wayland-info on $WL" >&2; exit 1; }
  echo "headless KWin at $WL, output $NAME" >&2
  if [ "$SCALE" != 1 ]; then
    require_scale "$SCALE" || { echo "the headless output is not at scale $SCALE (see $OUT_DIR/kwin.log)" >&2; exit 1; }
  fi
  standins
  start_bluez || exit 1
  start_real_daemon || exit 1
  prove_safe || { echo "SKIP: the fake DDC or the stand-in outputs are not proven from the daemon's log; no input sent" >&2; exit 2; }
  start_input || { echo "no fake input client" >&2; exit 1; }
  # The seat's devices are announced with the input client's: the first events only make them known.
  input "move 300 900" "sleep 80" "move 310 900" "sleep 80" || exit 1
  if [ "$SELECT" = bluetooth ]; then
    run_steps step_real_open step_real_bt_ask step_real_bt_escape step_real_bt_beside step_real_bt_cancel \
      step_real_bt_turn_off step_real_bt_on step_real_close
  else
    run_steps step_real_click_escape step_real_outside_click step_real_open step_real_late_monitor \
      step_real_external step_real_drag step_real_click step_real_bt_ask step_real_bt_escape \
      step_real_bt_beside step_real_bt_cancel step_real_bt_turn_off step_real_bt_on step_real_close
  fi
  status=$STATUS
  echo "stopping the input client, the daemon and the headless KWin" >&2
  cleanup_real
  trap - EXIT
  echo "logs and captures in $OUT_DIR"
  exit "$status"
}

if [ "$MODE" = real ]; then run_real; else run_nested; fi

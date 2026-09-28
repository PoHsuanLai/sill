#!/usr/bin/env bash
# Control center acceptance (design/20 §1.5, design/13 §13.3.7), on the nested cosmic-comp
# (dev/nested.sh; a running wayland-1 is reused and never stopped). Starts `sill run` from this
# checkout's debug build (`--features debug`) with a scratch XDG_CONFIG_HOME and quits it at the
# end. Its settings hide the Sound and Battery bar items, so the status area reads, from the
# right: the clock, the control center, Wi-Fi (the one module item left).
#
# Nothing here changes a real radio, level or profile: no switch or slider is ever pressed, and
# the only list the Wi-Fi pane asks for is a scan.
#
#   1. A debug click on the control center's item opens `sill-bar-<out>/popup` at the panel's
#      size (320 + the shadow room), its card 8 px inside the output's right edge (read from a
#      screenshot); a second click closes it. Screenshot dev/out/control-center-cosmic.png.
#   2. An outside click (the bar's empty middle) closes it; Escape (wtype, into the popup's grab)
#      closes it too, or the step says why it could not try.
#   3. The Wi-Fi tile's chevron slides the Wi-Fi pane in: the popup takes the pane's size (a
#      Wi-Fi pane with a long list of networks may be as tall as the grid; then it only has to
#      change). Its only request is a scan.
#      Screenshot dev/out/control-center-wifi-cosmic.png.
#   4. The Wi-Fi bar item opens the Wi-Fi pane directly: its popup is the pane's size from
#      step 3, not the grid's.
#   5. `sill control-center toggle` opens it with no pointer event behind it, and again closes
#      it; whether cosmic-comp maps a popup opened without a fresh input serial, and whether its
#      grab holds (an outside click dismisses it), is printed. Not mapped: exit 2, naming it.
#   6. 5 s idle after closing: 0 frames on the bar.
#
# Usage: dev/nested.sh && dev/accept-control-center.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-control-center"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
# The panel's popup: control_center.width_px (320) plus the shadow room (28 each side).
PANEL_POPUP_W=376
BUS=
BUS_PID=

sill() { "$SILL_BIN" "$@"; }
nested() { "$SHELL_HOST_DEV/run.sh" "$@"; }

# The nested output's connector name (the surfaces' namespace suffix).
output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

# SURFACE's cumulative frame counter from `sill debug stats` (the daemon's own surfaces).
frames_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"])' "$1"
}

# SURFACE's size as "WIDTH HEIGHT", read from a capture (a popup is not in `debug stats`, but
# shell-host names it `<bar>/popup` for `debug capture`); nothing when it is not mapped.
size_of() {
  sill debug capture "$1" "$OUT_DIR/size.png" >/dev/null 2>&1 || return 1
  python3 -c 'import sys; from PIL import Image; print(*Image.open(sys.argv[1]).size)' \
    "$OUT_DIR/size.png"
}

popup_up() { size_of "$POPUP" >/dev/null; }
popup_width() { size_of "$POPUP" | cut -d' ' -f1; }
popup_height() { size_of "$POPUP" | cut -d' ' -f2; }

# Wait up to 2 s for the popup to be up (or, with `down`, gone).
wait_popup() {
  for _ in $(seq 40); do
    if [ "$1" = up ] && popup_up; then return 0; fi
    if [ "$1" = down ] && ! popup_up; then return 0; fi
    sleep 0.05
  done
  return 1
}

# The status items' centres, right to left, found in a capture of the bar: runs of inked columns
# (the clock's text is the widest run and comes first). Prints "x y" per item.
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
    print((left + right) // 2, h // 2)
EOF
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

start_daemon() {
  CONFIG=$(mktemp -d)
  backdrop_seed "$CONFIG" || return 1
  mkdir -p "$CONFIG/sill"
  printf '[control_center]\nmenu_bar_sound = "hide"\nmenu_bar_battery = "hide"\n' \
    >"$CONFIG/sill/settings.toml"
  use_fake_ddc "$CONFIG" >/dev/null
  require_fake_ddc || return 1
  XDG_CONFIG_HOME="$CONFIG" DBUS_SESSION_BUS_ADDRESS="$BUS" DBUS_SYSTEM_BUS_ADDRESS="$BUS" \
    nested "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
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

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  stop_private_bus
  rm -rf "$CONFIG"
}

click_bar() { # x y
  sill debug click "$BAR" "$1" "$2" >/dev/null 2>&1
}

close_popup() {
  popup_up || return 0
  click_bar $((BAR_W / 2)) $((BAR_H / 2))
  wait_popup down
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
  wait_popup up || { echo "a click on the item opened no popup" | tee -a "$log" >&2; return 1; }
  sleep 0.6
  width=$(popup_width)
  sill debug capture "$POPUP" "$OUT_DIR/panel.png" >>"$log" 2>&1 || return 1
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/control-center-cosmic.png" >>"$log" 2>&1 \
    || echo "screenshot failed" >&2
  gap=$(card_right_gap "$SHOTS/control-center-cosmic.png")
  # shellcheck disable=SC2086
  click_bar $CC_ITEM
  wait_popup down || { echo "a second click left it open" | tee -a "$log" >&2; return 1; }
  echo "item click: popup ${width} wide (want $PANEL_POPUP_W), card ${gap} px from the output's right edge (want about 8); a second click closed it" \
    | tee -a "$log" >&2
  [ "$width" = "$PANEL_POPUP_W" ] && [ "$gap" -ge 4 ] && [ "$gap" -le 12 ]
}

# 2.
step_dismiss() {
  local log="$OUT_DIR/dismiss.log" escape
  : >"$log"
  # shellcheck disable=SC2086
  click_bar $CC_ITEM
  wait_popup up || return 1
  sleep 0.3
  close_popup || { echo "an outside click left it open" | tee -a "$log" >&2; return 1; }
  # shellcheck disable=SC2086
  click_bar $CC_ITEM
  wait_popup up || return 1
  sleep 0.3
  if nested wtype -k Escape >>"$log" 2>&1; then
    if wait_popup down; then escape="Escape closed it"; else
      escape="Escape (wtype) did not reach it: left open"; close_popup; fi
  else
    escape="Escape not tried: wtype failed in the nested session"; close_popup
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
  wait_popup up || return 1
  sleep 0.5
  before=$(popup_height)
  # The Wi-Fi tile's chevron, popup-local: the card at (28, 8) with a hairline, the panel's
  # padding 12, a half tile 144 wide and 52 tall, its 18 px chevron 4 in from the right.
  sill debug click "$POPUP" 171 47 >>"$log" 2>&1
  sleep 0.8
  after=$(popup_height)
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/control-center-wifi-cosmic.png" >>"$log" 2>&1 \
    || echo "screenshot failed" >&2
  close_popup
  PANE_H=$after
  GRID_H=$before
  echo "the chevron: popup ${before} -> ${after} tall" | tee -a "$log" >&2
  [ -n "$after" ] && [ "$after" != "$before" ]
}

# 4.
step_module_item() {
  local log="$OUT_DIR/module.log" width height
  : >"$log"
  # shellcheck disable=SC2086
  click_bar $WIFI_ITEM
  wait_popup up || { echo "the Wi-Fi item opened no popup" | tee -a "$log" >&2; return 1; }
  sleep 0.5
  width=$(popup_width)
  height=$(popup_height)
  sill debug capture "$POPUP" "$OUT_DIR/wifi-item.png" >>"$log" 2>&1
  close_popup
  echo "the Wi-Fi item: a ${width}x${height} popup (the pane from step 3 is ${PANE_H:-?} tall, the grid ${GRID_H:-?})" \
    | tee -a "$log" >&2
  [ "$width" = "$PANEL_POPUP_W" ] && [ "$height" = "${PANE_H:-}" ] && [ "$height" != "${GRID_H:-}" ]
}

# 5.
step_toggle() {
  local log="$OUT_DIR/toggle.log" held grab
  : >"$log"
  sill control-center toggle >>"$log" 2>&1 || { echo "the toggle request failed" >&2; return 1; }
  if ! wait_popup up; then
    echo "sill control-center toggle: cosmic-comp did not map the popup (grab without a fresh input serial refused)" \
      | tee -a "$log" >&2
    return 2
  fi
  sleep 1
  if popup_up; then held="it stays mapped"; else held="it was dismissed at once (popup_done)"; fi
  sill debug capture "$POPUP" "$OUT_DIR/toggled.png" >/dev/null 2>&1
  sill control-center toggle >>"$log" 2>&1
  wait_popup down || { echo "a second toggle left it open" | tee -a "$log" >&2; return 1; }
  sill control-center toggle >>"$log" 2>&1
  wait_popup up || return 1
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
  # The clock redraws the bar once a minute: keep the window clear of the minute's turn.
  if [ "$((10#$(date +%S)))" -ge 50 ]; then sleep $((62 - 10#$(date +%S))); fi
  f0=$(frames_of "$BAR")
  sleep 5
  f1=$(frames_of "$BAR")
  echo "5 s idle after closing: $BAR $(( f1 - f0 )) frames" | tee "$log" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

source "$SHELL_HOST_DEV/.nested-env"
nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
build || exit 1
if sill ping >/dev/null 2>&1; then
  echo "a sill daemon is already running; quit it first (this script starts its own)" >&2
  exit 1
fi
start_private_bus || exit 1
start_daemon || { stop_daemon; stop_private_bus; exit 1; }
BACKDROP_DIR=$(mktemp -d)
backdrop_up "$BACKDROP_DIR" || echo "backdrop: none (the shot shows the bare compositor)" >&2
status=0
for step in step_item_click step_dismiss step_wifi_pane step_module_item step_toggle step_idle; do
  "$step"
  code=$?
  case "$code" in
    0) echo "PASS $step" ;;
    2) echo "SKIP $step (see above)"; [ "$status" = 0 ] && status=2 ;;
    *) echo "FAIL $step"; status=1 ;;
  esac
done
backdrop_down
rm -rf "$BACKDROP_DIR"
stop_daemon
echo "logs in $OUT_DIR; screenshots $SHOTS/control-center-cosmic.png, $SHOTS/control-center-wifi-cosmic.png"
exit "$status"

#!/usr/bin/env bash
# The bar's menus and the keyboard of the control center and the notification center under
# REAL input (FINDINGS "Manual run 2026-09-28: menus and keyboard"), on shell-host's private
# headless KWin (scripts/dev/kwin-virtual.sh, FINDINGS F52) driven by shell-host's
# scripts/dev/fake-input.sh. Every menu and panel here is opened by a real pointer click on its
# bar item, never by `sill control-center toggle` or `sill notifications open`; every key is a
# real Wayland key event. `sill debug capture` and `sill debug notify` only look and seed.
#
# As on the user's session, an app window has the keyboard first: dev/key-sink.py, a plain GTK 4
# window clicked into focus, which logs every key it is sent. A shell panel opened from the bar
# must take the keyboard from it; a key the sink logs went to the app instead.
#
# The daemon runs from this checkout's debug build (`--features debug`) on the headless KWin's
# socket (an absolute WAYLAND_DISPLAY) and its private D-Bus bus (nothing activatable), with its
# own XDG_RUNTIME_DIR and a scratch HOME and XDG dirs. KWin's private bus is the daemon's
# session AND system bus (start_kwin refuses to go on if it would be the real one, FINDINGS
# F1068), so NetworkManager, BlueZ, UPower and the rest are simply absent: the bar's Wi-Fi and
# Bluetooth items read Unavailable, never a real neighbour SSID or device. The bus goes to the
# daemon and the sink only, never to this shell (G52). ONE fake-input client serves the whole
# run (G50). Nothing here reaches the user's session (wayland-0) or the real session bus.
#
#   0. A real click on the sink gives it the keyboard; a real "k" reaches it.
#   1. A real click on bar item A (Wi-Fi), then one on item B (Sound): A's menu closes and B's
#      opens (B's pill on, A's off; the popup shows what B alone shows).
#   2. Hover switch: with B's menu open, the real pointer onto A switches the popup to A's menu;
#      a real click on A keeps it (the click the person meant, F991), a second one closes it.
#   3. A real click on the control center's item opens it with the keyboard: real Tab moves the
#      focus ring (each Tab changes the panel, a Shift+Tab goes back), real Space toggles the
#      focused tile (Focus, first in `control_center.modules`: `notifications.dnd` turns on, and
#      off again on a second Space; the tile changes on its own, no `sill reload` needed, F993
#      fixed), real Escape closes the panel. The sink gets none of these keys.
#   4. Two notifications seeded; a real click on the clock opens the notification center with
#      the keyboard: real Down moves the focus ring onto a row, real Escape closes the center.
#      The sink gets none of these keys.
#   5. The notification center open, a real click on A closes it and opens A's menu (one
#      menu-bar panel at a time, as on macOS).
#   6. A's menu open, a real click on the clock closes the menu and opens the center, which has
#      the keyboard: real Down moves its ring, real Escape closes it; the sink gets no key.
#   G391. After a bar menu closes, does the app have the keyboard back (a real "j" reaches the
#      sink)? Recorded, not asserted: shell-host's side (FINDINGS G391). Only on a full run.
#
# Captures: target/accept-menus-keyboard-real-input/*.png. Then the input client, the sink, the
# daemon (`sill quit`) and the headless KWin are stopped.
#
# Exit 0 only when steps 0-6 passed; 1 when one failed. The G391 line never changes it.
#
# SHELL_HOST_DEV may name a private copy of shell-host's scripts/dev (with tools/ beside its
# scripts/): kwin-virtual.sh keeps one .kwin-env per scripts/dev, so a copy runs beside another
# agent's headless KWin.
#
# Usage: dev/accept-menus-keyboard-real-input.sh [STEP...]   (default: every step, in order)
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-menus-keyboard-real-input"
mkdir -p "$OUT_DIR"
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
SIZE=1920x1080

SCRATCH=$(mktemp -d)
# A short runtime folder of its own: the agent socket's path must fit in 108 bytes.
RUNTIME=$(mktemp -d /tmp/sill-rt.XXXXXX)
HOME_DIR="$SCRATCH/home"
CONFIG="$SCRATCH/config"
mkdir -p "$HOME_DIR" "$CONFIG/sill" "$SCRATCH/data" "$SCRATCH/state" "$SCRATCH/cache"
chmod 700 "$RUNTIME"
DAEMON=
INPUT=
SINK=
WL=
BUS=
NAME=

in_env() {
  env HOME="$HOME_DIR" XDG_RUNTIME_DIR="$RUNTIME" XDG_CONFIG_HOME="$CONFIG" \
    XDG_DATA_HOME="$SCRATCH/data" XDG_STATE_HOME="$SCRATCH/state" \
    XDG_CACHE_HOME="$SCRATCH/cache" WAYLAND_DISPLAY="$WL" DBUS_SESSION_BUS_ADDRESS="$BUS" \
    DBUS_SYSTEM_BUS_ADDRESS="$BUS" "$@"
}
sill() { in_env "$SILL_BIN" "$@"; }

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

# The headless KWin, its socket (absolute) and bus read from its env file without sourcing it
# into this shell: `kwin-virtual.sh stop` must never find this script naming the bus (G52).
start_kwin() {
  # Never borrow (and later stop) a headless KWin someone else started.
  [ ! -e "$KWIN_ENV" ] || { echo "$KWIN_ENV exists: another headless KWin is running" >&2; return 1; }
  # No key repeat: under load a key's release can reach the shell after its repeat delay, and a
  # repeated Tab or Space would walk or toggle past what the step looks at.
  mkdir -p "$SCRATCH/kwin-seed"
  printf '[Keyboard]\nKeyRepeat=nothing\nRepeatDelay=5000\n' >"$SCRATCH/kwin-seed/kcminputrc"
  KWIN_VIRTUAL_CONFIG_SEED="$SCRATCH/kwin-seed" \
    "$SHELL_HOST_DEV/kwin-virtual.sh" start "$SIZE" >"$OUT_DIR/kwin.log" 2>&1 || return 1
  local runtime socket
  runtime=$(sed -n 's/^export XDG_RUNTIME_DIR=//p' "$KWIN_ENV")
  socket=$(sed -n 's/^export WAYLAND_DISPLAY=//p' "$KWIN_ENV")
  BUS=$(sed -n "s/^export DBUS_SESSION_BUS_ADDRESS='\\(.*\\)'$/\\1/p" "$KWIN_ENV")
  case "$socket" in /*) WL="$socket" ;; *) WL="$runtime/$socket" ;; esac
  case "$WL" in
    *wayland-kwinvirt-*) ;;
    *) echo "refusing $WL: not the headless KWin's socket" >&2; stop_kwin; return 1 ;;
  esac
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $KWIN_ENV" >&2; stop_kwin; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "KWin's bus is the session's" >&2; stop_kwin; return 1; }
  require_private_system_bus "$BUS" || { stop_kwin; return 1; }
}

stop_kwin() { "$SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1; }

# Real input through ONE fake-input client for the whole run (G50).
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
  for _ in $(seq 400); do
    [ "$(grep -c '^ok ' "$OUT_DIR/input.log")" -ge $(( before + $# )) ] && return 0
    sleep 0.05
  done
  echo "fake input did not answer: $*" >&2
  return 1
}
# Real keys, one after another: each pressed, held 30 ms, released, then 250 ms to settle.
key() { local k; for k in "$@"; do input "key $k down" "sleep 30" "key $k up" "sleep 250" || return 1; done; }
chord() {
  input "key $1 down" "sleep 30" "key $2 down" "sleep 30" "key $2 up" "sleep 30" "key $1 up" \
    "sleep 250"
}
point() { input "move $1 $2" "sleep 80"; }
click() { point "$1" "$2" && input "press left" "sleep 40" "release left" "sleep 300"; }

# The key sink: a GTK 4 window on KWin's socket and bus that logs the keys it gets.
start_sink() {
  in_env GDK_BACKEND=wayland python3 "$SILL_REPO/dev/key-sink.py" >"$OUT_DIR/sink.log" 2>&1 &
  SINK=$!
  for _ in $(seq 100); do grep -q ' ready$' "$OUT_DIR/sink.log" && return 0; sleep 0.1; done
  echo "the key sink never started" >&2
  return 1
}

stop_sink() {
  [ -n "$SINK" ] || return 0
  kill "$SINK" 2>/dev/null
  reap "$SINK" sink
  SINK=
}

sink_keys() { grep -c ' key ' "$OUT_DIR/sink.log"; }

# The sink's frame "x y w h" from KWin's scripting API.
sink_frame() {
  "$SHELL_HOST_DEV/kwin-windows.sh" 2>/dev/null | python3 -c '
import json, sys
for line in sys.stdin:
    w = json.loads(line)
    if w.get("caption") == "sill key sink":
        print(*w["frame"]); break'
}

start_daemon() {
  cat >"$CONFIG/sill/settings.toml" <<'EOF'
[control_center]
modules = ["focus", "appearance", "wifi", "bluetooth", "sound"]
menu_bar_wifi = "show"
menu_bar_sound = "show"
menu_bar_battery = "hide"

[notifications]
server = "own"
EOF
  in_env "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "$NAME" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  BAR="sill-bar-$NAME"
  POPUP="$BAR/popup"
  CENTER="sill-notification-center-$NAME"
  sleep 1.5
  echo "daemon on KWin's bus, runtime $RUNTIME, home $HOME_DIR, output $NAME" >&2
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

# Capture SURFACE $1 into $2.png; fails while it is not mapped.
capture() {
  sill debug capture "$1" "$OUT_DIR/$2.png" >>"$OUT_DIR/capture.log" 2>&1 && return 0
  echo "no capture of $1 for $2 (not mapped)" >&2
  return 1
}

# Capture SURFACE $1 into $2.png once two captures 150 ms apart agree (up to 3 s): a key's
# change may reach the screen a frame or two after the key, more under load.
settled() {
  local previous="$2.settle"
  capture "$1" "$previous" || return 1
  for _ in $(seq 20); do
    sleep 0.15
    capture "$1" "$2" || return 1
    [ "$(diff_box "$previous" "$2")" = none ] && return 0
    cp "$OUT_DIR/$2.png" "$OUT_DIR/$previous.png"
  done
  return 0
}
mapped() { sill debug capture "$1" "$OUT_DIR/probe.png" >/dev/null 2>&1; }

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

# The box two captures differ in, "x0 y0 x1 y1" or "none" ("size" when their sizes differ).
diff_box() {
  python3 - "$OUT_DIR/$1.png" "$OUT_DIR/$2.png" <<'EOF'
import sys
from PIL import Image, ImageChops
a, b = (Image.open(p).convert("RGBA") for p in sys.argv[1:3])
if a.size != b.size:
    print("size"); sys.exit(0)
box = ImageChops.difference(a, b).getbbox()
print(*box) if box else print("none")
EOF
}

# The status items' centres, right to left, from a capture of the bar: runs of inked columns
# (the clock's first). Prints "x y" per item.
status_items() {
  capture "$BAR" bar || return 1
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

# The bar's pixels under item "x y" (a 30 px square), as a hash: the item's pill on or off.
item_look() {
  capture "$BAR" "$1" || return 1
  python3 - "$OUT_DIR/$1.png" "$2" "$3" <<'EOF'
import hashlib, sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
x, y = int(sys.argv[2]), int(sys.argv[3])
print(hashlib.sha1(img.crop((x - 15, 1, x + 15, img.size[1] - 1)).tobytes()).hexdigest()[:12])
EOF
}

# Close whatever the bar shows: real Escape (the popup has the keyboard) and a wait.
close_popup() {
  mapped "$POPUP" || return 0
  key Escape
  wait_surface "$POPUP" 20 down && return 0
  echo "(real Escape left the popup up; a click on the sink closes it)" >&2
  click "$SINK_X" "$SINK_Y"
  wait_surface "$POPUP" 20 down
}

# 0.
step_sink_focus() {
  local frame x y w h before
  frame=$(sink_frame)
  [ -n "$frame" ] || { echo "the sink's window is not among KWin's" >&2; return 1; }
  read -r x y w h <<<"$frame"
  SINK_X=$(( x + w / 2 )); SINK_Y=$(( y + h / 2 ))
  click "$SINK_X" "$SINK_Y" || return 1
  sleep 0.3
  before=$(sink_keys)
  key k || return 1
  sleep 0.3
  [ "$(sink_keys)" -gt "$before" ] || { echo "the sink got no key: $(tail -3 "$OUT_DIR/sink.log")" >&2; return 1; }
  echo "a real click on the sink ($SINK_X $SINK_Y) gave it the keyboard; a real 'k' reached it" >&2
}

# 1.
step_click_switch() {
  local items
  mapfile -t items < <(status_items)
  # Right to left: the clock, the control center, Sound, Wi-Fi.
  [ "${#items[@]}" -ge 4 ] || { echo "found ${#items[@]} status items, wanted 4" >&2; return 1; }
  CLOCK_AT=${items[0]}; CC_AT=${items[1]}; B_AT=${items[2]}; A_AT=${items[3]}
  # References: A's and B's menus each opened alone.
  local a_off b_off a_on b_on
  # shellcheck disable=SC2086
  a_off=$(item_look bar-a-off $A_AT); b_off=$(item_look bar-b-off $B_AT)
  # shellcheck disable=SC2086
  click $A_AT || return 1
  wait_surface "$POPUP" 20 up || { echo "a click on A ($A_AT) opened nothing" >&2; return 1; }
  sleep 0.5
  capture "$POPUP" menu-a-alone || return 1
  # shellcheck disable=SC2086
  a_on=$(item_look bar-a-on $A_AT)
  close_popup || { echo "Escape left A's menu up" >&2; return 1; }
  # shellcheck disable=SC2086
  click $B_AT || return 1
  wait_surface "$POPUP" 20 up || { echo "a click on B ($B_AT) opened nothing" >&2; return 1; }
  sleep 0.5
  capture "$POPUP" menu-b-alone || return 1
  # shellcheck disable=SC2086
  b_on=$(item_look bar-b-on $B_AT)
  close_popup || { echo "Escape left B's menu up" >&2; return 1; }
  [ "$(diff_box menu-a-alone menu-b-alone)" != none ] || { echo "A's and B's menus look alike" >&2; return 1; }
  [ "$a_on" != "$a_off" ] && [ "$b_on" != "$b_off" ] || { echo "no pill on an open item" >&2; return 1; }
  # A, then B.
  # shellcheck disable=SC2086
  click $A_AT || return 1
  wait_surface "$POPUP" 20 up || return 1
  sleep 0.5
  # shellcheck disable=SC2086
  click $B_AT || return 1
  sleep 0.6
  local popup a_now b_now shown
  popup=$(mapped "$POPUP" && echo up || echo down)
  capture "$POPUP" menu-a-then-b
  # shellcheck disable=SC2086
  a_now=$(item_look bar-a-then-b $A_AT)
  # shellcheck disable=SC2086
  b_now=$(item_look bar-a-then-b $B_AT)
  shown=none
  [ "$popup" = up ] && [ "$(diff_box menu-a-then-b menu-b-alone)" = none ] && shown=B
  [ "$popup" = up ] && [ "$(diff_box menu-a-then-b menu-a-alone)" = none ] && shown=A
  echo "real click on A ($A_AT) then on B ($B_AT): popup $popup showing $shown; A's pill" \
    "$([ "$a_now" = "$a_on" ] && echo on || echo off), B's pill" \
    "$([ "$b_now" = "$b_on" ] && echo on || echo off)" >&2
  [ "$shown" = B ] && [ "$a_now" = "$a_off" ] && [ "$b_now" = "$b_on" ]
}

# 2.
step_hover_switch() {
  mapped "$POPUP" || {
    # shellcheck disable=SC2086
    click $B_AT || return 1
    wait_surface "$POPUP" 20 up || return 1
  }
  sleep 0.3
  # shellcheck disable=SC2086
  point $A_AT || return 1
  sleep 0.6
  local shown=other
  capture "$POPUP" menu-hover-a || shown=down
  [ "$shown" = other ] && [ "$(diff_box menu-hover-a menu-a-alone)" = none ] && shown=A
  echo "B's menu open, the real pointer onto A: the popup shows $shown" >&2
  [ "$shown" = A ] || return 1
  # The click the person meant on the title the pointer switched to: A stays.
  # shellcheck disable=SC2086
  click $A_AT || return 1
  sleep 0.4
  mapped "$POPUP" || { echo "a click on A's title, switched to by hover, closed its menu" >&2; return 1; }
  # shellcheck disable=SC2086
  click $A_AT || return 1
  wait_surface "$POPUP" 20 down || { echo "a second click on A's title left its menu up" >&2; return 1; }
  echo "a real click on A's title keeps its menu; a second one closes it" >&2
}

# `notifications.dnd` in the scratch settings file once it reads $1 (up to 3 s), else what it
# reads then: the Focus tile's switch, stored by the daemon's focus task.
await_dnd() {
  local now=
  for _ in $(seq 30); do
    now=$(sed -n 's/^dnd *= *"\(.*\)"$/\1/p' "$CONFIG/sill/settings.toml")
    [ "$now" = "$1" ] && break
    sleep 0.1
  done
  echo "${now:-unset}"
}

# 3.
step_control_center_keys() {
  local sunk b1 b2 back space
  sunk=$(sink_keys)
  # shellcheck disable=SC2086
  click $CC_AT || return 1
  wait_surface "$POPUP" 20 up || { echo "a click on the control center ($CC_AT) opened nothing" >&2; return 1; }
  sleep 0.8
  settled "$POPUP" cc-0 || return 1
  key Tab || return 1
  settled "$POPUP" cc-tab1 || return 1
  key Tab || return 1
  settled "$POPUP" cc-tab2 || return 1
  chord Shift_L Tab || return 1
  settled "$POPUP" cc-back || return 1
  b1=$(diff_box cc-0 cc-tab1); b2=$(diff_box cc-tab1 cc-tab2); back=$(diff_box cc-tab1 cc-back)
  echo "control center opened by a real click on its item ($CC_AT): real Tab changed [$b1]," \
    "a second Tab [$b2], Shift+Tab back to the first Tab's look: [$back]" >&2
  # Space on the focused Focus tile stores `notifications.dnd`. The daemon's file watch
  # republishes the settings feed the tile reads on its own (FINDINGS F993, fixed by F1030): no
  # `sill reload` here any more; the key itself is all real, and `settled` waits out the watch's
  # debounce plus the redraw.
  key space || return 1
  local dnd_on dnd_off
  dnd_on=$(await_dnd on)
  settled "$POPUP" cc-space || return 1
  space=$(diff_box cc-back cc-space)
  echo "real Space on the focused tile (Focus): notifications.dnd $dnd_on; the tile changed" \
    "[$space] with no 'sill reload'" >&2
  key space || return 1
  dnd_off=$(await_dnd off)
  echo "real Space again: notifications.dnd $dnd_off" >&2
  key Escape || return 1
  wait_surface "$POPUP" 20 down || { echo "real Escape left the control center up" >&2; return 1; }
  echo "real Escape closed it; keys the sink got meanwhile: $(( $(sink_keys) - sunk ))" >&2
  [ "$b1" != none ] && [ "$b2" != none ] && [ "$back" = none ] && [ "$space" != none ] \
    && [ "$dnd_on" = on ] && [ "$dnd_off" = off ] && [ "$(sink_keys)" = "$sunk" ]
}

# Two made-up notifications for the center's rows, once per run; both stay in the center after
# their banners leave.
SEEDED=no
seed_notes() {
  [ "$SEEDED" = no ] || return 0
  sill debug notify "Ada" --app chat --body "Hi" >>"$OUT_DIR/notify.log" 2>&1 || return 1
  sill debug notify "Grace" --app mail --body "The report" >>"$OUT_DIR/notify.log" 2>&1 || return 1
  sleep 8
  SEEDED=yes
}

# 4.
step_center_keys() {
  local sunk down tab esc
  seed_notes || return 1
  sunk=$(sink_keys)
  # shellcheck disable=SC2086
  click $CLOCK_AT || return 1
  wait_surface "$CENTER" 20 up || { echo "a click on the clock ($CLOCK_AT) opened nothing" >&2; return 1; }
  sleep 0.8
  settled "$CENTER" nc-0 || return 1
  key Down || return 1
  settled "$CENTER" nc-down || return 1
  down=$(diff_box nc-0 nc-down)
  echo "notification center opened by a real click on the clock ($CLOCK_AT): real Down" \
    "changed [$down]" >&2
  # G390/F74: Tab (unhandled by the center's own key map) reaches Clear All, a quire `Button`;
  # the modality bridge (`sill_surfaces::host_modality::HostModalityBridge`) now provides
  # `ds::HostModality` there too, so the ring should draw.
  key Tab || return 1
  settled "$CENTER" nc-tab || return 1
  tab=$(diff_box nc-down nc-tab)
  echo "real Tab (after Down) reaches a quire control (Clear All): changed [$tab]" >&2
  key Escape || return 1
  esc=down
  wait_surface "$CENTER" 20 down || esc=up
  echo "real Escape: the center is $esc; keys the sink got meanwhile: $(( $(sink_keys) - sunk ))" >&2
  [ "$esc" = up ] && { click "$SINK_X" "$SINK_Y"; wait_surface "$CENTER" 20 down; }
  [ "$down" != none ] && [ "$tab" != none ] && [ "$esc" = down ] && [ "$(sink_keys)" = "$sunk" ]
}

center_open() { mapped "$CENTER" && echo open || echo closed; }

# 5.
step_center_then_menu() {
  # shellcheck disable=SC2086
  click $CLOCK_AT || return 1
  wait_surface "$CENTER" 20 up || { echo "a click on the clock opened nothing" >&2; return 1; }
  sleep 0.6
  # shellcheck disable=SC2086
  click $A_AT || return 1
  sleep 0.8
  local popup center shown=other
  popup=$(mapped "$POPUP" && echo up || echo down)
  center=$(center_open)
  capture "$POPUP" menu-after-center && [ "$(diff_box menu-after-center menu-a-alone)" = none ] && shown=A
  echo "notification center open, a real click on A: the center is $center, the popup" \
    "$popup showing $shown" >&2
  [ "$center" = closed ] && [ "$shown" = A ]
}

# 6.
step_menu_then_center() {
  seed_notes || return 1
  mapped "$POPUP" || {
    # shellcheck disable=SC2086
    click $A_AT || return 1
    wait_surface "$POPUP" 20 up || return 1
  }
  sleep 0.4
  local sunk down popup
  sunk=$(sink_keys)
  # shellcheck disable=SC2086
  click $CLOCK_AT || return 1
  wait_surface "$CENTER" 20 up || { echo "A's menu open, a click on the clock opened no center" >&2; return 1; }
  sleep 0.8
  popup=$(mapped "$POPUP" && echo up || echo down)
  settled "$CENTER" nc2-0 || return 1
  key Down || return 1
  settled "$CENTER" nc2-down || return 1
  down=$(diff_box nc2-0 nc2-down)
  key Escape || return 1
  wait_surface "$CENTER" 20 down || { echo "real Escape left the center up" >&2; click "$SINK_X" "$SINK_Y"; return 1; }
  echo "A's menu open, a real click on the clock: the popup is $popup, the center opened;" \
    "real Down changed [$down]; real Escape closed it; keys the sink got: $(( $(sink_keys) - sunk ))" >&2
  [ "$popup" = down ] && [ "$down" != none ] && [ "$(sink_keys)" = "$sunk" ]
}

# Whether a real "j" now reaches the sink: yes or no.
sink_takes_j() {
  local before
  before=$(sink_keys)
  key j || return 1
  sleep 0.3
  [ "$(sink_keys)" -gt "$before" ] && echo yes || echo no
}

# G391. Recorded, not asserted (shell-host's side, FINDINGS G391): the sink clicked back into
# focus, a bar menu opened by a real click and closed by real Escape, then a real "j": does the
# app have the keyboard back? Then the same around the notification center. OPEN while the
# menu leaves no window with the keyboard.
check_g391() {
  local after_menu after_center state=OPEN
  click "$SINK_X" "$SINK_Y" && sleep 0.3
  # shellcheck disable=SC2086
  click $A_AT && wait_surface "$POPUP" 20 up && sleep 0.3
  key Escape && wait_surface "$POPUP" 20 down && sleep 0.3
  after_menu=$(sink_takes_j)
  click "$SINK_X" "$SINK_Y" && sleep 0.3
  # shellcheck disable=SC2086
  click $CLOCK_AT && wait_surface "$CENTER" 20 up && sleep 0.3
  key Escape && wait_surface "$CENTER" 20 down && sleep 0.3
  after_center=$(sink_takes_j)
  [ "$after_menu" = yes ] && state=CLOSED
  echo "G391 $state: the app has the keyboard back after a bar menu closes: $after_menu; after" \
    "the notification center closes: $after_center" >&2
}

cleanup() {
  stop_input
  stop_sink
  stop_daemon
  stop_kwin
  rm -rf "$SCRATCH" "$RUNTIME"
}

build || exit 1
start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; rm -rf "$SCRATCH" "$RUNTIME"; exit 1; }
trap cleanup EXIT
: >"$OUT_DIR/capture.log"
: >"$OUT_DIR/notify.log"
start_daemon || exit 1
start_sink || exit 1
sleep 1
start_input || { echo "no fake input client" >&2; exit 1; }
# The seat's devices are announced with the input client's: the first events only make them known.
input "move 300 900" "sleep 80" "move 310 900" "sleep 80" || exit 1
status=0
steps=("$@")
[ "${#steps[@]}" -gt 0 ] || steps=(step_sink_focus step_click_switch step_hover_switch \
  step_control_center_keys step_center_keys step_center_then_menu step_menu_then_center)
for step in "${steps[@]}"; do
  "$step"
  case "$?" in
    0) echo "PASS $step" ;;
    *) echo "FAIL $step"; status=1 ;;
  esac
  close_popup
done
[ "$#" -gt 0 ] || check_g391 || echo "G391 could not be checked" >&2
echo "stopping the input client, the sink, the daemon and the headless KWin" >&2
cleanup
trap - EXIT
echo "logs and captures in $OUT_DIR"
exit "$status"

#!/usr/bin/env bash
# The control center's Display module under real pointer input (FINDINGS "Manual run
# 2026-09-28: control center"), on shell-host's private headless KWin (scripts/dev/kwin-virtual.sh,
# FINDINGS F52) driven by shell-host's scripts/dev/fake-input.sh: the control center is opened by
# a real click on its bar item, and its brightness capsule is dragged and clicked by a real
# pointer, with real serials, through KWin's own popup grab. No `sill debug` opener, click or
# pointer is used; `sill debug capture` and `sill debug stats` only look.
#
# The headless output is 3840 x 2160 at scale 2, as the user's BenQ MA270U (CC_SIZE and CC_SCALE
# change it), set through kscreen-doctor's in-process backend on KWin's own socket and bus.
#
# SAFETY. No real monitor, no real outputs list, no real Bluetooth:
# - DDC is a FAKE: the daemon is this checkout's debug build (`--features debug`, which turns on
#   sill-services' `fake-ddc`) started with SILL_FAKE_DDC naming a JSON file of one made-up
#   monitor on the headless output. The daemon's log must say so ("DDC is the fake in ...")
#   before any input is sent; otherwise the script stops. The fake never opens /dev/i2c-*.
# - The outputs list is FAKE: a stand-in `cosmic-randr` that never answers and a stand-in
#   `kscreen-doctor` that prints the headless output are first on the daemon's PATH.
# - KWin's private bus is the daemon's session AND system bus, so BlueZ, NetworkManager and
#   the rest are simply absent (Bluetooth shows Unavailable); the power menu is a dry run.
# - Its own XDG_RUNTIME_DIR (no PipeWire), a scratch HOME and XDG dirs. Nothing reaches
#   wayland-0 or the user's session bus.
#
#   1. A real click on the control center's bar item opens its popup. The fake monitor did not
#      answer the startup probe (its file lists none): the capsule is disabled at 0 %, as on
#      the user's run.
#   1b. The monitor answers now (40 %): with the control center open the service probes again
#      and the capsule comes alive at 40 % within its follow period (2 s).
#   2. External change: the fake monitor's brightness is rewritten 40 -> 90 (its own buttons);
#      within the service's follow period (2 s) the capsule shows it (two captures differ in
#      the capsule's band). The two captures also give the capsule's rail on screen.
#   3. Real drag: press at 30 %, eight moves to 75 % over about half a second, release. The fake
#      monitor receives writes DURING the drag (not only at the release), rising, and the last
#      is 75 (+-4).
#   4. Real click on the track at 20 %: the fake monitor receives 20 (+-4).
#   5. A real click outside closes the popup. (Real Escape first, recorded only: the control
#      center's keyboard focus after a pointer press is another lane's.)
#
# Captures: target/accept-cc-real-input/*.png. Exit 0 only when every step passed; 1 when one
# failed; 2 when a step could not run safely (named).
#
# Usage: dev/accept-cc-real-input.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-cc-real-input"
mkdir -p "$OUT_DIR"
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
# The user's BenQ MA270U: 3840 x 2160 at scale 2 (CC_SCALE=1 CC_SIZE=1920x1080 for 1x).
SIZE="${CC_SIZE:-3840x2160}"
SCALE="${CC_SCALE:-2}"

SCRATCH=$(mktemp -d)
# A short runtime folder of its own: the agent socket's path must fit in 108 bytes.
RUNTIME=$(mktemp -d /tmp/sill-rt.XXXXXX)
chmod 700 "$RUNTIME"
HOME_DIR="$SCRATCH/home"
CONFIG="$SCRATCH/config"
STANDINS="$SCRATCH/bin"
DDC="$SCRATCH/ddc.json"
WRITES="$SCRATCH/ddc.json.writes"
RECORD="$SCRATCH/power-menu.record"
mkdir -p "$HOME_DIR" "$CONFIG/sill" "$STANDINS" "$SCRATCH/data" "$SCRATCH/state" "$SCRATCH/cache"
DAEMON=
INPUT=
WL=
BUS=
NAME=
POPUP=
SAFE=unproven
# The capsule's rail on screen (global logical px), from step 2.
RAIL_LEFT=
RAIL_WIDTH=
RAIL_Y=

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

# The headless output's connector name, from KWin's wl_output (v4 name).
headless_output() {
  env WAYLAND_DISPLAY="$WL" wayland-info 2>/dev/null \
    | sed -n "s/.*name: '\\{0,1\\}\\([A-Za-z]*-[0-9A-Za-z-]*\\)'\\{0,1\\}$/\\1/p" | head -n 1
}

# The headless output at scale $SCALE, through kscreen-doctor's in-process backend on KWin's own
# socket and bus (nothing else is reachable from that environment), then read back.
set_scale() {
  local env=(env -i PATH="$PATH" HOME="$HOME_DIR" XDG_RUNTIME_DIR="$RUNTIME"
    XDG_CONFIG_HOME="$CONFIG" XDG_DATA_HOME="$SCRATCH/data" XDG_CACHE_HOME="$SCRATCH/cache"
    WAYLAND_DISPLAY="$WL" DBUS_SESSION_BUS_ADDRESS="$BUS" KSCREEN_BACKEND_INPROCESS=1
    QT_QPA_PLATFORM=wayland)
  [ "$SCALE" = 1 ] && return 0
  "${env[@]}" timeout 20 kscreen-doctor "output.$NAME.scale.$SCALE" >"$OUT_DIR/scale.log" 2>&1
  local now
  now=$("${env[@]}" timeout 20 kscreen-doctor --json 2>>"$OUT_DIR/scale.log" | python3 -c '
import json, sys
for o in json.load(sys.stdin)["outputs"]:
    if o["name"] == sys.argv[1]:
        print(o.get("scale"))' "$NAME")
  echo "headless output $NAME at scale ${now:-?} (wanted $SCALE)" >&2
  python3 -c 'import sys; sys.exit(0 if abs(float(sys.argv[1]) - float(sys.argv[2])) < 0.01 else 1)' \
    "${now:-0}" "$SCALE"
}

# The fake monitor at brightness $1, or (`none`) no monitor answering DDC; written whole, then
# renamed into place.
fake_monitor() {
  if [ "$1" = none ]; then
    printf '{"displays":[]}\n' >"$DDC.tmp" && mv "$DDC.tmp" "$DDC"
    return
  fi
  printf '{"displays":[{"id":7,"connector":"%s","product":"Fake Monitor","brightness":{"value":%s,"max":100}}]}\n' \
    "$NAME" "$1" >"$DDC.tmp" && mv "$DDC.tmp" "$DDC"
}

# The stand-ins first on the daemon's PATH: a cosmic-randr that never answers (KWin has no
# zwlr_output_manager_v1) and a kscreen-doctor that prints only the headless output.
standins() {
  cat >"$STANDINS/cosmic-randr" <<'EOF'
#!/bin/sh
echo "accept-cc-real-input: stand-in cosmic-randr, no answer" >&2
exit 1
EOF
  cat >"$STANDINS/kscreen-doctor" <<EOF
#!/bin/sh
[ "\$1" = --json ] || { echo "accept-cc-real-input: stand-in kscreen-doctor, --json only" >&2; exit 1; }
cat <<'JSON'
{"outputs":[{"name":"$NAME","connected":true,"enabled":true,"currentModeId":"1",
 "modes":[{"id":"1","name":"1920x1080@60","refreshRate":60.0,"size":{"width":1920,"height":1080}}],
 "scale":1,"sizeMM":{"width":597,"height":336}}]}
JSON
EOF
  chmod +x "$STANDINS/cosmic-randr" "$STANDINS/kscreen-doctor"
}

start_daemon() {
  printf '[control_center]\nmenu_bar_sound = "hide"\nmenu_bar_battery = "hide"\n' \
    >"$CONFIG/sill/settings.toml"
  : >"$RECORD"
  in_env PATH="$STANDINS:$PATH" SILL_FAKE_DDC="$DDC" SILL_POWER_MENU_DRY_RUN="$RECORD" \
    "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  local bar=
  for _ in $(seq 50); do bar=$(bar_name); [ -n "$bar" ] && break; sleep 0.1; done
  [ "$bar" = "$NAME" ] || { echo "the bar is on '$bar', not '$NAME'" >&2; return 1; }
  POPUP="sill-bar-$NAME/popup"
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

bar_name() {
  sill debug stats 2>/dev/null | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

# The fake DDC and the fake outputs are in force, from the daemon's own log. Sets SAFE=proven.
prove_safe() {
  local log="$OUT_DIR/safety.log"
  {
    grep "sill: displays: DDC is the fake in $DDC" "$OUT_DIR/daemon.log" || return 1
    grep "power_menu.backend=dry-run" "$OUT_DIR/daemon.log" || return 1
  } >"$log" 2>&1
  for _ in $(seq 60); do
    grep -q "outputs read from kscreen-doctor" "$OUT_DIR/daemon.log" && break
    sleep 0.1
  done
  grep "outputs read from kscreen-doctor" "$OUT_DIR/daemon.log" >>"$log" || return 1
  SAFE=proven
  echo "safety proven from the daemon's log: fake DDC ($DDC), the stand-in outputs, dry-run power menu" >&2
}

# Real input through ONE fake-input client for the whole run (G50).
start_input() {
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
key() { input "key $1 down" "sleep 30" "key $1 up" "sleep 200"; }
click() { input "move $1 $2" "sleep 80" "press left" "sleep 40" "release left" "sleep 250"; }

mapped() { sill debug capture "$1" "$OUT_DIR/probe.png" >/dev/null 2>&1; }

wait_surface() {
  local surface="$1" tenths="$2" want="${3:-up}"
  for _ in $(seq "$tenths"); do
    if mapped "$surface"; then [ "$want" = up ] && return 0
    else [ "$want" = down ] && return 0; fi
    sleep 0.1
  done
  return 1
}

capture() { sill debug capture "$1" "$OUT_DIR/$2.png" >>"$OUT_DIR/capture.log" 2>&1; }

# The status items' centres, right to left (the clock's first), from a capture of the bar.
status_items() {
  capture "sill-bar-$NAME" bar || return 1
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

# The popup's top-left on screen, "x y": KWin's window whose buffer (logical) is the popup
# capture's size over the scale.
popup_origin() {
  capture "$POPUP" popup-size || return 1
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

# 1.
step_open() {
  local items
  mapfile -t items < <(status_items)
  [ "${#items[@]}" -ge 2 ] || { echo "found ${#items[@]} status items" >&2; return 1; }
  CC_ITEM=${items[1]}
  # shellcheck disable=SC2086
  click $CC_ITEM || return 1
  wait_surface "$POPUP" 30 up || { echo "a real click on the item ($CC_ITEM) opened nothing" >&2; return 1; }
  sleep 0.8
  capture "$POPUP" open
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
# the fill (and the glyph inside it).
plate = high.getpixel((right + 3, y))
left = x0
while left > 1 and not (not far(low.getpixel((left - 1, y)), plate, 8)
                        and not far(low.getpixel((left - 2, y)), plate, 8)):
    left -= 1
print(left, right + 1)
EOF
}

# 2. Also measures the rail: RAIL_LEFT, RAIL_WIDTH, RAIL_Y (global).
step_external() {
  capture "$POPUP" level-40 || return 1
  fake_monitor 90
  local band= waited=0
  for _ in $(seq 30); do
    sleep 0.2
    waited=$(( waited + 200 ))
    capture "$POPUP" level-90 || continue
    band=$(changed_band level-40 level-90)
    [ -n "$band" ] && break
  done
  [ -n "$band" ] || { echo "the capsule never showed the monitor's 90 (no change in the popup)" >&2; return 1; }
  # Let the slide to the new level finish, then measure.
  sleep 0.8
  capture "$POPUP" level-90 || return 1
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

at() { echo $(( RAIL_LEFT + RAIL_WIDTH * $1 / 100 )) "$RAIL_Y"; }

# 1b.
step_late_monitor() {
  capture "$POPUP" absent || return 1
  fake_monitor 40
  local waited=0
  for _ in $(seq 30); do
    sleep 0.2
    waited=$(( waited + 200 ))
    capture "$POPUP" alive || continue
    [ -n "$(changed_band absent alive)" ] && break
  done
  [ -n "$(changed_band absent alive)" ] || {
    echo "the capsule never came alive after the monitor answered (still as at the start)" >&2
    return 1; }
  sleep 0.8
  echo "the monitor answered late: the open control center probed again and the capsule came" \
    "alive within about ${waited} ms" >&2
}

# 3.
step_drag() {
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
  capture "$POPUP" dragged
  echo "real drag 30 % -> 75 % (x $from -> $to, y $y): the fake monitor got $(( during - before ))" \
    "write(s) during the drag, $(( after - before )) in all: [$(writes)]; it is at $last" >&2
  [ $(( during - before )) -ge 1 ] && [ "$last" -ge 71 ] && [ "$last" -le 79 ] || return 1
  python3 - "$(writes)" "$before" <<'EOF' || { echo "the writes did not rise with the drag" >&2; return 1; }
import sys
values = [int(v) for v in sys.argv[1].split()][int(sys.argv[2]):]
sys.exit(0 if values and all(a < b for a, b in zip(values, values[1:])) else 1)
EOF
}

# 4.
step_click() {
  local x y last
  read -r x y <<<"$(at 20)"
  click "$x" "$y" || return 1
  sleep 0.5
  last=$(monitor_level)
  capture "$POPUP" clicked
  echo "real click on the track at 20 % (x $x): the fake monitor is at $last; writes [$(writes)]" >&2
  [ "$last" -ge 16 ] && [ "$last" -le 24 ]
}

# 5.
step_close() {
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

cleanup() {
  stop_input
  stop_daemon
  stop_kwin
  rm -rf "$SCRATCH" "$RUNTIME"
}

command -v wayland-info >/dev/null || { echo "wayland-info is missing; not installed by this script" >&2; exit 1; }
build || exit 1
start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; rm -rf "$SCRATCH" "$RUNTIME"; exit 1; }
trap cleanup EXIT
: >"$OUT_DIR/capture.log"
NAME=$(headless_output)
[ -n "$NAME" ] || { echo "no output name from wayland-info on $WL" >&2; exit 1; }
echo "headless KWin at $WL, output $NAME" >&2
set_scale || { echo "the headless output is not at scale $SCALE (see $OUT_DIR/scale.log)" >&2; exit 1; }
fake_monitor none
standins
start_daemon || exit 1
prove_safe || { echo "SKIP: the fake DDC or the stand-in outputs are not proven from the daemon's log; no input sent" >&2; exit 2; }
start_input || { echo "no fake input client" >&2; exit 1; }
# The seat's devices are announced with the input client's: the first events only make them known.
input "move 300 900" "sleep 80" "move 310 900" "sleep 80" || exit 1
status=0
for step in step_open step_late_monitor step_external step_drag step_click step_close; do
  "$step"
  rc=$?
  case "$rc" in
    0) echo "PASS $step" ;;
    2) echo "SKIP $step"; [ "$status" = 1 ] || status=2 ;;
    *) echo "FAIL $step"; status=1 ;;
  esac
done
echo "stopping the input client, the daemon and the headless KWin" >&2
cleanup
trap - EXIT
echo "logs and captures in $OUT_DIR"
exit "$status"

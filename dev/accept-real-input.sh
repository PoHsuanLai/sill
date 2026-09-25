#!/usr/bin/env bash
# Real input acceptance: the power menu and the control center under real Wayland keyboard and
# pointer input, on a private headless KWin (shell-host scripts/dev/kwin-virtual.sh, FINDINGS
# F52) driven by shell-host's scripts/dev/fake-input.sh. The nested cosmic-comp gives the shell's
# surfaces no keyboard and debug clicks carry no input serial (FINDINGS G30, F226); here every key
# and press is a real one, with a real serial and keyboard focus.
#
# Starts `sill run` from this checkout's debug build (`--features debug`) on the headless KWin,
# with a scratch XDG_CONFIG_HOME and KWin's private D-Bus session bus, and quits it at the end;
# stops the headless KWin at the end too.
#
# SAFETY. The daemon is started with SILL_POWER_MENU_DRY_RUN naming a record file, so every power
# menu has the dry-run backend (a chosen action is only recorded, never sent to logind), and the
# script proves that from the daemon's log before it sends any key. Enter is never pressed and no
# button of the menu is clicked: the keys are arrows and Escape, the click is on the scrim.
#
#   1. A real click on the control center's bar item opens its popup; real Escape closes it.
#   2. Opened again by a click, a real click outside it closes it.
#   3. The power menu opens and holds the real keyboard (`power_menu.keyboard=held`).
#   4. Real Right moves the focus Shut Down -> Suspend, Left Left -> Restart.
#   5. Real Escape closes it, recording nothing.
#   6. Opened again, a real click on the scrim closes it, recording nothing.
#
# Exit 0 only when every step passed; 2 when a step could not run safely (named); 1 on a failure.
#
# Usage: dev/accept-real-input.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-real-input"
RECORD="$OUT_DIR/dry-run.record"
mkdir -p "$OUT_DIR"
DRY_RUN=unproven
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
SIZE=1920x1080

sill() { "$SILL_BIN" "$@"; }

# Real input into the headless KWin through ONE fake-input client for the whole run: each client
# is a new fake input device, and KWin re-announces the seat's pointer for each; shell-host then
# holds a wl_pointer per announcement and a press reaches the bar once per pointer, so a click
# toggled a popup open and shut (FINDINGS "Adopt quire v0.1.9").
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
  [ -n "${INPUT:-}" ] || return 0
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
  for _ in $(seq 100); do
    [ "$(grep -c '^ok ' "$OUT_DIR/input.log")" -ge $(( before + $# )) ] && return 0
    sleep 0.05
  done
  echo "fake input did not answer: $*" >&2
  return 1
}
key() { input "key $1 down" "sleep 30" "key $1 up" "sleep 150"; }
click() { input "move $1 $2" "sleep 60" "press left" "sleep 40" "release left" "sleep 200"; }

count_log() { grep -c -- "$1" "$OUT_DIR/daemon.log"; }

# Wait up to $2 tenths of a second for the daemon's log to hold $3 more lines matching $1 than
# it did when counted as $4.
wait_log() {
  local pattern="$1" tenths="$2" more="$3" before="$4"
  for _ in $(seq "$tenths"); do
    [ "$(count_log "$pattern")" -ge $(( before + more )) ] && return 0
    sleep 0.1
  done
  return 1
}

# Whether SURFACE is mapped (it captures).
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

output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

# The status items' centres, right to left, from a capture of the bar (dev/accept-control-center.sh's
# reading: runs of inked columns; the clock's is first). Prints "x y" per item.
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

# The headless KWin. Its socket and bus reach only the daemon (start_daemon): kwin-virtual.sh stop
# kills every process whose environment names the bus, which must not include this script.
start_kwin() {
  "$SHELL_HOST_DEV/kwin-virtual.sh" start "$SIZE" >"$OUT_DIR/kwin.log" 2>&1
}

start_daemon() {
  CONFIG=$(mktemp -d)
  mkdir -p "$CONFIG/sill"
  printf '[control_center]\nmenu_bar_sound = "hide"\nmenu_bar_battery = "hide"\n' \
    >"$CONFIG/sill/settings.toml"
  : >"$RECORD"
  (
    # shellcheck disable=SC1090
    source "$KWIN_ENV"
    export WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS
    XDG_CONFIG_HOME="$CONFIG" SILL_POWER_MENU_DRY_RUN="$RECORD" exec "$SILL_BIN" run
  ) >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  BAR="sill-bar-$NAME"
  POPUP="$BAR/popup"
  MENU="sill-power-menu-$NAME"
  sleep 1.5
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  timeout 10 "$SILL_BIN" quit >/dev/null 2>&1
  local result=0
  await_exit "$DAEMON" daemon || result=1
  DAEMON=
  rm -rf "$CONFIG"
  return "$result"
}

# Wait up to 10 s for process $1 (named $2) to end on its own, once already asked to. Unlike
# `reap` this never kills it: a daemon that goes back to not ending on `quit` (FINDINGS G51)
# fails the script instead of being silently swept away.
await_exit() {
  for _ in $(seq 100); do kill -0 "$1" 2>/dev/null || return 0; sleep 0.1; done
  echo "the $2 ($1) did not end after quit" >&2
  return 1
}

# The dry run is in force, from the daemon's own log. Sets DRY_RUN=proven.
prove_dry_run() {
  local log="$OUT_DIR/dry-run.log"
  {
    grep "sill: power menu: dry run, recording to $RECORD" "$OUT_DIR/daemon.log" || return 1
    grep "power_menu.backend=dry-run" "$OUT_DIR/daemon.log" || return 1
    if grep "power_menu.backend=live" "$OUT_DIR/daemon.log"; then return 1; fi
  } >"$log" 2>&1
  DRY_RUN=proven
  echo "dry run proven from the daemon's log (record file $RECORD)" | tee -a "$log" >&2
}

# 3.
step_menu_keyboard() {
  local before
  [ "$DRY_RUN" = proven ] || { echo "no dry run proven: the menu is not opened" >&2; return 2; }
  before=$(count_log 'power_menu.keyboard=held')
  sill power-menu open >/dev/null 2>&1 || return 1
  wait_surface "$MENU" 30 up || { echo "open: never mapped" >&2; return 1; }
  wait_log 'power_menu.keyboard=held' 30 1 "$before" || {
    echo "the menu never held the real keyboard" >&2; return 1; }
  sleep 0.6
  echo "the power menu opened and holds the real keyboard" >&2
}

# 4.
step_menu_arrows() {
  local before
  [ "$DRY_RUN" = proven ] || return 2
  mapped "$MENU" || { echo "the menu is not up for the arrows" >&2; return 1; }
  before=$(count_log 'power_menu.focus=Suspend')
  key Right
  wait_log 'power_menu.focus=Suspend' 20 1 "$before" || { echo "Right: no focus on Suspend" >&2; return 1; }
  before=$(count_log 'power_menu.focus=Restart')
  key Left; key Left
  wait_log 'power_menu.focus=Restart' 20 1 "$before" || { echo "Left Left: no focus on Restart" >&2; return 1; }
  echo "real Right: Shut Down -> Suspend; real Left Left: -> Restart" >&2
}

# 5.
step_menu_escape() {
  [ "$DRY_RUN" = proven ] || return 2
  mapped "$MENU" || { echo "the menu is not up for Escape" >&2; return 1; }
  key Escape
  wait_surface "$MENU" 20 down || { echo "Escape: the menu stayed up" >&2; return 1; }
  [ ! -s "$RECORD" ] || { echo "Escape recorded: $(cat "$RECORD")" >&2; return 1; }
  echo "real Escape closed the menu, recording nothing" >&2
}

# 6.
step_menu_scrim_click() {
  [ "$DRY_RUN" = proven ] || return 2
  sill power-menu open >/dev/null 2>&1 || return 1
  wait_surface "$MENU" 30 up || { echo "open: never mapped" >&2; return 1; }
  sleep 0.6
  click 40 $(( ${SIZE#*x} - 40 ))
  wait_surface "$MENU" 20 down || { echo "a scrim click left the menu up" >&2; return 1; }
  [ ! -s "$RECORD" ] || { echo "the scrim click recorded: $(cat "$RECORD")" >&2; return 1; }
  echo "a real click on the scrim closed the menu, recording nothing" >&2
}

# 1.
step_cc_click_escape() {
  local items
  mapfile -t items < <(status_items)
  [ "${#items[@]}" -ge 2 ] || { echo "found ${#items[@]} status items" >&2; return 1; }
  CC_ITEM=${items[1]}
  # shellcheck disable=SC2086
  click $CC_ITEM
  wait_surface "$POPUP" 20 up || { echo "a click on the item ($CC_ITEM) opened nothing" >&2; return 1; }
  sleep 0.5
  sill debug capture "$POPUP" "$OUT_DIR/control-center.png" >/dev/null 2>&1
  key Escape
  wait_surface "$POPUP" 20 down || { echo "Escape left the control center up" >&2; return 1; }
  echo "a real click on the item ($CC_ITEM) opened the control center; real Escape closed it" >&2
}

# 2.
step_cc_outside_click() {
  # shellcheck disable=SC2086
  click $CC_ITEM
  wait_surface "$POPUP" 20 up || { echo "the second click opened nothing" >&2; return 1; }
  sleep 0.5
  click 200 $(( ${SIZE#*x} / 2 ))
  wait_surface "$POPUP" 20 down || { echo "an outside click left the control center up" >&2; return 1; }
  echo "a real click outside closed the control center" >&2
}

build || exit 1
# The shell's IPC socket is shared with the nested session's: wait (up to 10 min) for another
# script's daemon to quit.
for _ in $(seq 120); do sill ping >/dev/null 2>&1 || break; echo "a sill daemon is running; waiting" >&2; sleep 5; done
if sill ping >/dev/null 2>&1; then
  echo "a sill daemon is still running; quit it first (this script starts its own)" >&2
  exit 1
fi
start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; exit 1; }
if ! start_daemon; then
  stop_daemon; "$SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1; exit 1
fi
prove_dry_run || echo "the dry run could not be proven from the daemon's log" >&2
start_input || echo "no fake input client" >&2
status=0
for step in step_cc_click_escape step_cc_outside_click step_menu_keyboard step_menu_arrows \
    step_menu_escape step_menu_scrim_click; do
  "$step"; result=$?
  case "$result" in
    0) echo "PASS $step" ;;
    2) echo "SKIP $step"; [ "$status" = 1 ] || status=2 ;;
    *) echo "FAIL $step"; status=1 ;;
  esac
done
sill power-menu close >/dev/null 2>&1
echo "stopping the input client, the daemon and the headless KWin" >&2
stop_input
stop_daemon || status=1
"$SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1
echo "logs in $OUT_DIR"
exit "$status"

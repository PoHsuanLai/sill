#!/usr/bin/env bash
# Notification center acceptance (design/20 §1.6), on the nested cosmic-comp (dev/nested.sh; a
# running wayland-1 is reused and never stopped). Starts
# `sill run` from this checkout's debug build (`--features debug`) with a scratch
# XDG_CONFIG_HOME and XDG_STATE_HOME, under a private session bus (dev/private-bus.conf: no
# service directories, so nothing is activated and the real session's
# org.freedesktop.Notifications is never touched), and with its own XDG_RUNTIME_DIR (the
# nested socket given as an absolute WAYLAND_DISPLAY), so another sill running meanwhile keeps
# its own single-instance lock and IPC socket. Every notification comes from `sill debug
# notify`. The daemon is quit at the end.
#
#   1. `sill-notification-center-<out>` exists from the start, hidden (no capture).
#   2. `sill notifications toggle` shows it (a capture with the column's pixels; the empty state
#      in dev/out/notification-center-empty-cosmic.png), `toggle`
#      again hides it, `open` twice stays open, `close` hides it.
#   3. A debug click on the bar's clock opens it; a second click closes it.
#   4. Three notifications from two apps (Chat, then Mail twice): a capture shows two groups,
#      screenshot dev/out/notification-center-cosmic.png; a click on "1 more" opens Mail's
#      group (dev/out/notification-center-open-cosmic.png); Mail's Clear drops `status` kept
#      by 2.
#   5. A click left of the column closes it.
#   6. 5 s idle while open, and 5 s after it hides: 0 frames on the center surface.
#
# Exit 0 when every step passed, 2 when step 4 could not run (`sill debug notify` refused: the
# notifications service's live loop is not in this build), 1 on a failure. Another sill daemon
# started from a `target/*/sill run` is waited for (up to 10 minutes), so two shells never share
# the nested output's screenshots.
#
# The panel, the group headers and the cards are quire's (`Panel`, `GroupHeader`,
# `NotificationCard`); a group's "N more" and Clear are found in its header's ink.
#
# Usage: dev/nested.sh && dev/accept-notification-center.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-notification-center"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"

source "$SHELL_HOST_DEV/.nested-env"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
NESTED_SOCKET="$HOST_RUNTIME/$WAYLAND_DISPLAY"
RUNTIME=$(mktemp -d)
chmod 700 "$RUNTIME"

sill() { XDG_RUNTIME_DIR="$RUNTIME" "$SILL_BIN" "$@"; }

# The nested output's connector name (the surfaces' namespace suffix).
output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

# SURFACE's cumulative frame counter from `sill debug stats`.
frames_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"])' "$1"
}

# `sill notifications status`'s kept count.
kept() {
  sill notifications status | python3 -c 'import json, sys; print(json.loads(sys.stdin.read())["v"]["kept"])'
}

center_up() { sill debug capture "$CENTER" "$OUT_DIR/probe.png" >/dev/null 2>&1; }

# Wait up to 2 s for the center to be mapped (`up`) or not (`down`); `down` allows for quire's
# exit, which plays before the surface unmaps.
wait_center() {
  for _ in $(seq 40); do
    if [ "$1" = up ] && center_up; then return 0; fi
    if [ "$1" = down ] && ! center_up; then return 0; fi
    sleep 0.05
  done
  return 1
}

# The column's box in a capture of the center: "LEFT TOP RIGHT BOTTOM" of the pixels that are
# not transparent (the rest of the surface is the clear catcher).
column_box() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
box = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
print(*box) if box else sys.exit(1)
EOF
}

# The ink rows inside the column (text and lines darker or lighter than the column's own tint),
# each as "TOP BOTTOM LEFT RIGHT"; the column's first 40 rows (the title) are skipped.
ink_rows() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
left, top, right, bottom = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
left, right, top, bottom = left + 14, right - 14, top + 40, bottom - 14
ground = img.getpixel(((left + right) // 2, bottom - 4))
def inked(x, y):
    return sum(abs(a - b) for a, b in zip(img.getpixel((x, y))[:3], ground[:3])) > 120
rows, run = [], None
for y in range(top, bottom):
    xs = [x for x in range(left, right) if inked(x, y)]
    if xs and run is None:
        run = [y, y, min(xs), max(xs)]
    elif xs:
        run = [run[0], y, min(run[2], min(xs)), max(run[3], max(xs))]
    elif run is not None:
        rows.append(run); run = None
for r in rows:
    print(*r)
EOF
}

# The first group header's two buttons, "N more" (or "Show less") and Clear, from a capture:
# the first ink row (below the title) that reaches both the column's left padding (the app's
# icon and name) and its right padding (Clear); its ink split into words by gaps wider than a
# space, the last word is Clear and the one before it the toggle. Prints "TOGGLE_X Y CLEAR_X Y".
header_buttons() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
left, top, right, bottom = img.getchannel("A").point(lambda a: 255 if a > 200 else 0).getbbox()
left, right, top, bottom = left + 8, right - 8, top + 40, bottom - 14
ground = img.getpixel(((left + right) // 2, bottom - 4))
def inked(x, y):
    return sum(abs(a - b) for a, b in zip(img.getpixel((x, y))[:3], ground[:3])) > 120
rows, run = [], None
for y in range(top, bottom):
    xs = [x for x in range(left, right) if inked(x, y)]
    if xs and run is None:
        run = [y, y, set(xs)]
    elif xs:
        run = [run[0], y, run[2] | set(xs)]
    elif run is not None:
        rows.append(run); run = None
for t, b, xs in rows:
    if min(xs) < left + 40 and max(xs) > right - 40:
        words, word = [], None
        for x in sorted(xs):
            if word and x - word[1] <= 8:
                word[1] = x
            else:
                if word: words.append(word)
                word = [x, x]
        words.append(word)
        # "1 more" is two words a space apart: the toggle is everything after the name's words
        # up to Clear, found as the widest gap before the last two words.
        clear = words[-1]
        gaps = [(words[i + 1][0] - words[i][1], i) for i in range(len(words) - 2)]
        cut = max(gaps)[1] + 1 if gaps else len(words) - 2
        toggle = [words[cut][0], words[-2][1]]
        y = (t + b) // 2
        print((toggle[0] + toggle[1]) // 2, y, (clear[0] + clear[1]) // 2, y)
        break
else:
    sys.exit(1)
EOF
}

# The clock: the first run of inked columns from the bar's right end. Prints "X Y".
clock_centre() {
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
run, gap = None, 0
for x in range(w - 1, w // 2, -1):
    if inked(x):
        run = [x, x] if run is None else [x, run[1]]
        gap = 0
    elif run is not None:
        gap += 1
        if gap > 7:
            break
print((run[0] + run[1]) // 2, h // 2)
EOF
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

start_daemon() {
  CONFIG=$(mktemp -d)
  STATE=$(mktemp -d)
  backdrop_seed "$CONFIG" || return 1
  use_fake_ddc "$CONFIG" >/dev/null
  require_fake_ddc || return 1
  XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_RUNTIME_DIR="$RUNTIME" \
    WAYLAND_DISPLAY="$NESTED_SOCKET" \
    dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- \
    sh -c "$PRIVATE_BOTH_BUSES" "$SILL_BIN" \
    >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  BAR="sill-bar-$NAME"
  CENTER="sill-notification-center-$NAME"
  sleep 1.5
  if ! sill debug stats | grep -q '"sill-wallpaper-'; then
    XDG_CONFIG_HOME="$CONFIG" WAYLAND_DISPLAY="$WAYLAND_DISPLAY" cosmic-bg >/dev/null 2>&1 &
    BACKDROP_PID=$!
  fi
  echo "daemon on a private bus and runtime dir $RUNTIME; $BAR, $CENTER" >&2
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  backdrop_down
  rm -rf "$CONFIG" "$STATE"
}

# 1.
step_warm() {
  local log="$OUT_DIR/warm.log"
  sill debug stats >"$log" 2>&1
  grep -q "\"$CENTER\"" "$log" || { echo "$CENTER is not in 'sill debug stats'" >&2; return 1; }
  if center_up; then echo "the center is mapped before any open" >&2; return 1; fi
  echo "$CENTER spawned at start, hidden" | tee -a "$log" >&2
}

# 2.
step_toggle_open_close() {
  local log="$OUT_DIR/toggle.log" box size shot
  : >"$log"
  sill notifications toggle >>"$log" 2>&1 || return 1
  wait_center up || { echo "toggle: never mapped" >&2; return 1; }
  sleep 0.6
  sill debug capture "$CENTER" "$OUT_DIR/toggled.png" >>"$log" 2>&1 || return 1
  box=$(column_box "$OUT_DIR/toggled.png") || { echo "toggle: no column drawn" >&2; return 1; }
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/notification-center-empty-cosmic.png" >>"$log" 2>&1 \
    || echo "screenshot failed" >&2
  size=$(python3 -c 'import sys; from PIL import Image; print(*Image.open(sys.argv[1]).size)' \
    "$OUT_DIR/toggled.png")
  shot=$(python3 -c 'import sys; from PIL import Image; print(*Image.open(sys.argv[1]).size)' \
    "$SHOTS/notification-center-empty-cosmic.png")
  sill notifications toggle >>"$log" 2>&1 || return 1
  wait_center down || { echo "toggle: still mapped" >&2; return 1; }
  sill notifications open >>"$log" 2>&1 || return 1
  sill notifications open >>"$log" 2>&1 || return 1
  wait_center up || { echo "open: never mapped" >&2; return 1; }
  sleep 0.6
  center_up || { echo "open twice: not open" >&2; return 1; }
  sill notifications close >>"$log" 2>&1 || return 1
  wait_center down || { echo "close: still mapped" >&2; return 1; }
  echo "toggle shows the column (box $box in a $size surface, the output shot $shot: the surface keeps clear of the bar's reserve) and hides it; open twice stays open; close hides it" \
    | tee -a "$log" >&2
}

# 3.
step_clock() {
  local log="$OUT_DIR/clock.log" at
  : >"$log"
  at=$(clock_centre) || { echo "no clock found in the bar" >&2; return 1; }
  # shellcheck disable=SC2086
  sill debug click "$BAR" $at >>"$log" 2>&1
  wait_center up || { echo "a click on the clock ($at) opened nothing" | tee -a "$log" >&2; return 1; }
  sleep 0.6
  # shellcheck disable=SC2086
  sill debug click "$BAR" $at >>"$log" 2>&1
  wait_center down || { echo "a second click on the clock left it open" | tee -a "$log" >&2; return 1; }
  echo "a click on the clock at $at opens the center; a second closes it" | tee -a "$log" >&2
}

# 4.
step_groups() {
  local log="$OUT_DIR/groups.log" before after fold clear rows buttons
  : >"$log"
  if ! sill debug notify "ping" --app Chat --body "are you there?" >>"$log" 2>&1; then
    echo "sill debug notify: $(tail -n 1 "$log") (the service's live loop is not in this build)" >&2
    return 2
  fi
  sill debug notify "Re: plans" --app Mail --body "Saturday works." >>"$log" 2>&1 || return 1
  sill debug notify "Lunch?" --app Mail --body "Noon at the usual place." >>"$log" 2>&1 || return 1
  sleep 0.3
  before=$(kept)
  sill notifications open >>"$log" 2>&1 || return 1
  wait_center up || return 1
  sleep 0.8
  sill debug capture "$CENTER" "$OUT_DIR/groups.png" >>"$log" 2>&1 || return 1
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/notification-center-cosmic.png" >>"$log" 2>&1 \
    || echo "screenshot failed" >&2
  ink_rows "$OUT_DIR/groups.png" >"$OUT_DIR/rows-collapsed.txt"
  read -r -a buttons < <(header_buttons "$OUT_DIR/groups.png") \
    || { echo "no group header found" | tee -a "$log" >&2; return 1; }
  fold="${buttons[0]} ${buttons[1]}"
  # shellcheck disable=SC2086
  sill debug click "$CENTER" $fold >>"$log" 2>&1
  # Off the header: a hovered Quiet button's ground would run its words into Clear's.
  sill debug leave "$CENTER" >>"$log" 2>&1
  sleep 0.5
  sill debug capture "$CENTER" "$OUT_DIR/open.png" >>"$log" 2>&1 || return 1
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/notification-center-open-cosmic.png" >>"$log" 2>&1 \
    || echo "screenshot failed" >&2
  ink_rows "$OUT_DIR/open.png" >"$OUT_DIR/rows-open.txt"
  grep -q 'act=Toggle(AppKey("Mail"))' "$OUT_DIR/daemon.log" \
    || grep -q 'act=Toggle' "$OUT_DIR/daemon.log" \
    || { echo "the click at $fold did not open the group" | tee -a "$log" >&2; return 1; }
  read -r -a buttons < <(header_buttons "$OUT_DIR/open.png") \
    || { echo "no group header found" | tee -a "$log" >&2; return 1; }
  clear="${buttons[2]} ${buttons[3]}"
  # shellcheck disable=SC2086
  sill debug click "$CENTER" $clear >>"$log" 2>&1
  sleep 0.4
  after=$(kept)
  rows="$(wc -l <"$OUT_DIR/rows-collapsed.txt") ink rows collapsed, $(wc -l <"$OUT_DIR/rows-open.txt") opened"
  echo "kept $before; '1 more' at $fold opened Mail's group ($rows); Mail's Clear at $clear: kept $after" \
    | tee -a "$log" >&2
  [ "$before" = 3 ] && [ "$after" = 1 ]
}

# 5.
step_outside() {
  local log="$OUT_DIR/outside.log"
  : >"$log"
  center_up || { sill notifications open >>"$log" 2>&1; wait_center up || return 1; sleep 0.6; }
  sill debug click "$CENTER" 100 300 >>"$log" 2>&1
  wait_center down || { echo "a click outside the column left it open" | tee -a "$log" >&2; return 1; }
  echo "a click left of the column closes it" | tee -a "$log" >&2
}

# 6.
step_idle() {
  local log="$OUT_DIR/idle.log" f0 f1 f2 f3
  sill notifications open >"$log" 2>&1
  wait_center up || return 1
  sleep 1
  # The clock redraws the ages once a minute: keep the windows clear of the minute's turn.
  if [ "$((10#$(date +%S)))" -ge 45 ]; then sleep $((62 - 10#$(date +%S))); fi
  f0=$(frames_of "$CENTER")
  sleep 5
  f1=$(frames_of "$CENTER")
  sill notifications close >>"$log" 2>&1
  wait_center down || return 1
  sleep 1
  f2=$(frames_of "$CENTER")
  sleep 5
  f3=$(frames_of "$CENTER")
  echo "5 s idle open: $(( f1 - f0 )) frames; 5 s hidden: $(( f3 - f2 )) frames on $CENTER" \
    | tee -a "$log" >&2
  [ "$(( f1 - f0 ))" = 0 ] && [ "$(( f3 - f2 ))" = 0 ]
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
for step in step_warm step_toggle_open_close step_clock step_groups step_outside step_idle; do
  "$step"
  code=$?
  case "$code" in
    0) echo "PASS $step" ;;
    2) echo "SKIP $step (see above)"; [ "$status" = 0 ] && status=2 ;;
    *) echo "FAIL $step"; status=1 ;;
  esac
done
stop_daemon
rm -rf "$RUNTIME"
echo "logs in $OUT_DIR; screenshots $SHOTS/notification-center-cosmic.png, $SHOTS/notification-center-open-cosmic.png"
exit "$status"

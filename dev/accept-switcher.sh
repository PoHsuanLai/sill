#!/usr/bin/env bash
# App switcher acceptance (design/13 §13.3.5, design/20 §1.11; FINDINGS "M11 freeze" lane c), on
# the nested cosmic-comp (dev/nested.sh; wayland-1 is pinged first, reused, never stopped or
# restarted). Starts `sill run` from this checkout's debug build (`--features debug`) with its own
# XDG_RUNTIME_DIR (WAYLAND_DISPLAY as an absolute socket path) and a scratch XDG_CONFIG_HOME, and
# three windows of its own inside the nested compositor, never the user's:
#   demo    shell-host's `demo_window` (app id `demo_window`), which prints its own activated
#           state: the script's view of the nested toplevel state;
#   term    `cosmic-term` (com.system76.CosmicTerm);
#   calc    `kcalc` (org.kde.kcalc), the one Q closes.
# Started demo, term, calc, so the most recently used order is calc, term, demo. Every window
# is killed at the end.
#
# The chord is `sill switcher next|prev`, as COSMIC's WindowSwitcher runs it under dist/cosmic.
# Its modifier is held with `wtype -M alt` on the nested compositor's own virtual keyboard
# (zwp_virtual_keyboard_v1; WAYLAND_DISPLAY is the nested socket's absolute path, so nothing
# reaches the user's session): the switcher reads it from shell-host's modifiers on focus
# (`use_modifiers`), and its release is the release. Keys typed while it is held come from the
# same `wtype` (a second virtual keyboard would carry its own, empty, modifiers). Never Tab: the
# nested compositor binds Alt+Tab to cosmic-launcher; Right steps as Tab does.
#
#   1. Warm: `sill-switcher-<out>` exists from the start, hidden (its capture fails).
#   2. Alt held, `sill switcher next` shows the panel after the show delay (150 ms), not before,
#      on the three apps (screenshot dev/out/switcher-cosmic.png); `next` and `prev` move the
#      selection (the panel's pixels change, and come back); `cancel` hides it, and no window's
#      activation changed, then or at Alt's release.
#   3. Alt held, `next`, 300 ms, `next`, `commit` activates the third app in MRU order (demo
#      prints Active).
#   4. The quick tap, nothing held: `next` + `commit` inside the delay switches to the previous
#      app (demo goes Inactive) with no panel frames; `next` alone (the release came before the
#      focus, G180) does too (demo Active again).
#   5. Alt held, `next`, a real Right, Alt's release: the panel showed, Right moved it, and the
#      release activated the third app (demo goes Inactive).
#   6. Alt held, `next`, Right, a real `q` on calc: it closes and the panel stays; Escape hides
#      it and no window's activation changed.
#   7. 5 s idle after it hides: 0 frames on the switcher surface.
#
# Usage: dev/nested.sh && dev/accept-switcher.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-switcher"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
HOST_REPO="$(cd "$SHELL_HOST_DEV/../.." && pwd)"
HOST_TARGET="$SILL_REPO/target/shell-host-examples"
DEMO="$HOST_TARGET/debug/examples/demo_window"
SHOW_DELAY_MS=150

source "$SHELL_HOST_DEV/.nested-env"
NESTED_SOCKET="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$WAYLAND_DISPLAY"
BUS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=/run/user/$(id -u)/bus}"
RUN_DIR=
CONFIG=
DAEMON=
PIDS=()
HOLD=

sill() { XDG_RUNTIME_DIR="$RUN_DIR" WAYLAND_DISPLAY="$NESTED_SOCKET" "$SILL_BIN" "$@"; }
nested() { "$SHELL_HOST_DEV/run.sh" "$@"; }
keys() { WAYLAND_DISPLAY="$NESTED_SOCKET" wtype "$@"; }
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }

# The nested output's connector name (the surfaces' namespace suffix).
output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-"):
        print(s[len("sill-bar-"):]); break'
}

# A surface's cumulative frame counter from `sill debug stats`.
frames_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"])' "$1"
}

# How many pixels of the PNG $1 are not fully transparent.
opaque_pixels() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
print(sum(1 for a in img.getchannel("A").tobytes() if a > 0))
EOF
}

# The mean absolute difference of two PNGs' RGBA bytes (0-255).
diff_of() {
  python3 - "$1" "$2" <<'EOF'
import sys
from PIL import Image, ImageChops
a = Image.open(sys.argv[1]).convert("RGBA")
b = Image.open(sys.argv[2]).convert("RGBA")
if a.size != b.size:
    print(255); sys.exit()
d = ImageChops.difference(a, b)
print(round(sum(d.tobytes()) / len(d.tobytes()), 3))
EOF
}

# Capture the switcher to $1; succeeds only when the panel drew (over 1000 opaque pixels).
panel_to() {
  sill debug capture "$SWITCHER" "$1" >/dev/null 2>&1 || return 1
  [ "$(opaque_pixels "$1")" -gt 1000 ]
}

# Wait up to $1 ms for the panel; prints the ms it took.
wait_panel() {
  local start t
  start=$(now_ms)
  while [ $(( $(now_ms) - start )) -lt "$1" ]; do
    if panel_to "$OUT_DIR/probe.png"; then echo $(( $(now_ms) - start )); return 0; fi
    sleep 0.02
  done
  return 1
}

# Wait up to $1 ms for the switcher to be unmapped (its capture fails).
wait_hidden() {
  local start
  start=$(now_ms)
  while [ $(( $(now_ms) - start )) -lt "$1" ]; do
    sill debug capture "$SWITCHER" "$OUT_DIR/gone.png" >/dev/null 2>&1 || return 0
    sleep 0.02
  done
  return 1
}

# demo_window's activated state, as it last printed it ("Active" or "Inactive").
demo_state() {
  grep '"event":"state"' "$OUT_DIR/demo.log" | tail -1 | python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["activated"])'
}

# How many state lines demo_window has printed.
demo_lines() { grep -c '"event":"state"' "$OUT_DIR/demo.log"; }

# Wait up to $2 ms for demo_window's state to be $1.
wait_demo() {
  local start
  start=$(now_ms)
  while [ $(( $(now_ms) - start )) -lt "$2" ]; do
    [ "$(demo_state)" = "$1" ] && return 0
    sleep 0.05
  done
  return 1
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
  CARGO_TARGET_DIR="$HOST_TARGET" cargo build --manifest-path "$HOST_REPO/Cargo.toml" \
    -p shell-host --features debug-inject --example demo_window >>"$OUT_DIR/build.log" 2>&1 || {
    echo "demo_window build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

start_daemon() {
  RUN_DIR=$(mktemp -d)
  chmod 700 "$RUN_DIR"
  CONFIG=$(mktemp -d)
  backdrop_seed "$CONFIG" || return 1
  XDG_RUNTIME_DIR="$RUN_DIR" WAYLAND_DISPLAY="$NESTED_SOCKET" DBUS_SESSION_BUS_ADDRESS="$BUS" \
    XDG_CONFIG_HOME="$CONFIG" "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  SWITCHER="sill-switcher-$NAME"
  sleep 1
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  rm -rf "$CONFIG" "$RUN_DIR"
}

# The three windows, each waited for; demo's state lines go to demo.log.
start_windows() {
  : >"$OUT_DIR/demo.log"
  rm -f "$OUT_DIR/demo.in"
  mkfifo "$OUT_DIR/demo.in"
  nested "$DEMO" <"$OUT_DIR/demo.in" >"$OUT_DIR/demo.log" 2>&1 &
  PIDS+=($!)
  exec 8>"$OUT_DIR/demo.in"
  wait_demo Active 10000 || { echo "demo_window never became active" >&2; return 1; }
  nested cosmic-term >"$OUT_DIR/term.log" 2>&1 &
  TERM_PID=$!
  PIDS+=("$TERM_PID")
  wait_demo Inactive 10000 || { echo "cosmic-term never took the focus" >&2; return 1; }
  sleep 1
  QT_QPA_PLATFORM=wayland nested kcalc >"$OUT_DIR/calc.log" 2>&1 &
  CALC_PID=$!
  PIDS+=("$CALC_PID")
  sleep 3
  kill -0 "$CALC_PID" 2>/dev/null || { echo "kcalc did not start" >&2; return 1; }
}

stop_windows() {
  local pid
  exec 8>&- 2>/dev/null
  for pid in "${PIDS[@]}"; do
    pkill -P "$pid" 2>/dev/null
    kill "$pid" 2>/dev/null
  done
  for pid in "${PIDS[@]}"; do wait "$pid" 2>/dev/null; done
  PIDS=()
}

# Hold Alt on the nested virtual keyboard for the `wtype` script $@ (in the background; its pid
# in HOLD), then give the compositor a moment to pass the modifiers on.
hold() {
  keys -M alt "$@" -m alt >>"$OUT_DIR/keys.log" 2>&1 &
  HOLD=$!
  sleep 0.15
}

# 1.
step_warm() {
  local log="$OUT_DIR/warm.log"
  sill debug stats >"$log" 2>&1
  grep -q "\"$SWITCHER\"" "$log" || { echo "$SWITCHER is not in 'sill debug stats'" >&2; return 1; }
  if sill debug capture "$SWITCHER" "$OUT_DIR/hidden.png" >>"$log" 2>&1; then
    echo "the switcher is mapped before any chord" >&2; return 1
  fi
  echo "$SWITCHER spawned at start, hidden" | tee -a "$log" >&2
}

# 2. MRU calc, term, demo.
step_show_move_cancel() {
  local log="$OUT_DIR/show.log" early=none shown ab ac before
  : >"$log"
  before=$(demo_lines)
  hold -s 5000
  sill switcher next >>"$log" 2>&1 || return 1
  sleep 0.06
  panel_to "$OUT_DIR/early.png" && early=panel
  shown=$(wait_panel 2000) || { echo "the panel never showed" >&2; wait "$HOLD"; return 1; }
  sleep 0.3
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/switcher-cosmic.png" >>"$log" 2>&1 || echo "screenshot failed" >&2
  panel_to "$OUT_DIR/a.png"
  sill switcher next >>"$log" 2>&1; sleep 0.4; panel_to "$OUT_DIR/b.png"
  sill switcher prev >>"$log" 2>&1; sleep 0.4; panel_to "$OUT_DIR/c.png"
  ab=$(diff_of "$OUT_DIR/a.png" "$OUT_DIR/b.png")
  ac=$(diff_of "$OUT_DIR/a.png" "$OUT_DIR/c.png")
  sill switcher cancel >>"$log" 2>&1
  wait_hidden 500 || { echo "cancel left it up" >&2; wait "$HOLD"; return 1; }
  wait "$HOLD"
  sleep 0.5
  echo "at 60 ms: ${early}; the panel a further ${shown} ms on (delay ${SHOW_DELAY_MS} ms);" \
    "next moved the ring (diff ${ab}), prev brought it back (diff ${ac}); cancel hid it;" \
    "demo's state lines ${before} -> $(demo_lines) ($(demo_state)) after Alt's release" | tee -a "$log" >&2
  [ "$early" = none ] || return 1
  python3 -c "import sys; sys.exit(0 if $ab > 0.05 and $ac < $ab / 4 else 1)" || return 1
  [ "$before" = "$(demo_lines)" ] && [ "$(demo_state)" = Inactive ]
}

# 3. MRU calc, term, demo: the third is demo.
step_commit_third() {
  local log="$OUT_DIR/commit.log"
  : >"$log"
  hold -s 2500
  sill switcher next >>"$log" 2>&1
  sleep 0.3
  sill switcher next >>"$log" 2>&1
  sleep 0.1
  sill switcher commit >>"$log" 2>&1
  wait_hidden 500 || { echo "commit left it up" >&2; wait "$HOLD"; return 1; }
  wait_demo Active 2000 || { echo "the third app (demo) was not activated" >&2; wait "$HOLD"; return 1; }
  wait "$HOLD"
  sleep 0.3
  echo "next, 300 ms, next, commit: demo (third in MRU) is $(demo_state), still after Alt's release" | tee -a "$log" >&2
  [ "$(demo_state)" = Active ]
}

# 4. MRU demo, calc, term.
step_quick_tap() {
  local log="$OUT_DIR/quick.log" f0 f1 start took ok=yes
  : >"$log"
  f0=$(frames_of "$SWITCHER")
  start=$(now_ms)
  sill switcher next >>"$log" 2>&1; sill switcher commit >>"$log" 2>&1
  took=$(( $(now_ms) - start ))
  wait_demo Inactive 2000 || ok=no
  sleep 0.3
  f1=$(frames_of "$SWITCHER")
  echo "quick next+commit in ${took} ms: demo $(demo_state) (calc activated); switcher frames $(( f1 - f0 ))" | tee -a "$log" >&2
  [ "$took" -lt "$SHOW_DELAY_MS" ] || ok=no
  [ $(( f1 - f0 )) -le 2 ] || ok=no
  f0=$f1
  sill switcher next >>"$log" 2>&1
  wait_demo Active 2000 || ok=no
  sleep 0.5
  f1=$(frames_of "$SWITCHER")
  echo "next alone, nothing held (the release before the focus): demo $(demo_state); switcher frames $(( f1 - f0 ))" | tee -a "$log" >&2
  [ $(( f1 - f0 )) -le 2 ] || ok=no
  panel_to "$OUT_DIR/stuck.png" && { echo "the panel is up after the tap" >&2; ok=no; }
  [ "$ok" = yes ]
}

# 5. MRU demo, calc, term: next selects calc, Right term, the release activates term.
step_real_keys() {
  local log="$OUT_DIR/keys-step.log" moved
  : >"$log"
  hold -s 900 -k Right -s 700
  sill switcher next >>"$log" 2>&1
  sleep 0.45
  panel_to "$OUT_DIR/k1.png" || { echo "no panel while Alt is held" >&2; wait "$HOLD"; return 1; }
  sleep 0.6
  panel_to "$OUT_DIR/k2.png" || { echo "the panel closed at Right" >&2; wait "$HOLD"; return 1; }
  moved=$(diff_of "$OUT_DIR/k1.png" "$OUT_DIR/k2.png")
  wait "$HOLD"
  wait_hidden 500 || { echo "Alt's release did not close it" >&2; return 1; }
  wait_demo Inactive 2000 || { echo "the release activated nothing" >&2; return 1; }
  echo "Alt held: a real Right moved the ring (diff ${moved}); Alt's release closed it and activated term (demo $(demo_state))" | tee -a "$log" >&2
  python3 -c "import sys; sys.exit(0 if $moved > 0.05 else 1)"
}

# 6. MRU term, demo, calc: next selects demo, Right calc, q closes it, Escape.
step_quit() {
  local log="$OUT_DIR/quit.log" gone=no stayed=no lines
  : >"$log"
  lines=$(demo_lines)
  hold -s 800 -k Right -s 400 -k q -s 1600 -k Escape -s 500
  sill switcher next >>"$log" 2>&1
  for _ in $(seq 60); do kill -0 "$CALC_PID" 2>/dev/null || { gone=yes; break; }; sleep 0.05; done
  sleep 0.6
  panel_to "$OUT_DIR/after-quit.png" && stayed=yes
  wait "$HOLD"
  wait_hidden 500 || { echo "Escape left it up" >&2; return 1; }
  sleep 0.3
  echo "a real q on calc: closed ${gone}; the panel stayed ${stayed}; Escape hid it; demo's state lines ${lines} -> $(demo_lines)" | tee -a "$log" >&2
  [ "$gone" = yes ] && [ "$stayed" = yes ] && [ "$(demo_lines)" = "$lines" ]
}

# 7.
step_idle() {
  local log="$OUT_DIR/idle.log" f0 f1
  sleep 1
  f0=$(frames_of "$SWITCHER")
  sleep 5
  f1=$(frames_of "$SWITCHER")
  echo "5 s idle after it hides: $SWITCHER $(( f1 - f0 )) frames" | tee "$log" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

cleanup() {
  stop_windows
  stop_daemon
}
trap cleanup EXIT

nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
command -v wtype >/dev/null || { echo "wtype not found (the keys)" >&2; exit 1; }
build || exit 1
if pgrep -f "sill run" >/dev/null; then
  echo "another sill daemon is running; wait for it to quit" >&2
  exit 1
fi
start_daemon || exit 1
start_windows || exit 1
status=0
for step in step_warm step_show_move_cancel step_commit_third step_quick_tap step_real_keys \
  step_quit step_idle; do
  if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; fi
done
echo "logs in $OUT_DIR; screenshot $SHOTS/switcher-cosmic.png"
exit "$status"

#!/usr/bin/env bash
# Live IME check on KDE: open sill's launcher demo on wayland-0 and keep it open until
# `ime-live.sh capture` or `ime-live.sh stop`. The user types into it by hand (KWin has no
# virtual keyboard, so nothing can type for them).
#
# `start` records its own demo's and its command tail's PIDs in $D; `stop` and the one-copy guard
# look only at those, never at other sessions' launcher demos.
set -uo pipefail
D="${IME_LIVE_DIR:-${XDG_RUNTIME_DIR:-/tmp}/sill-ime-live}"; mkdir -p "$D"
IN=$D/ime-cmds
PIPE=$D/ime-pipe
DEMO_PID=$D/demo.pid
TAIL_PID=$D/tail.pid

# The PID in pidfile $1 when that process is still alive and its command line still matches $2
# (so a reused PID is never taken for ours); nothing otherwise.
ours() {
  local pid
  pid=$(cat "$1" 2>/dev/null) || return 1
  [[ $pid =~ ^[0-9]+$ ]] || return 1
  tr '\0' ' ' 2>/dev/null <"/proc/$pid/cmdline" | grep -qF -- "$2" || return 1
  echo "$pid"
}

case "${1:-start}" in
  start)
    # One copy at a time: several copies fight over the keyboard focus.
    if ours "$DEMO_PID" examples/launcher_demo >/dev/null || ours "$TAIL_PID" "tail -f $IN" >/dev/null; then
      echo "ime-live: a copy is already running; use \`$0 stop\` first" >&2
      exit 1
    fi
    : >"$IN"
    rm -f "$PIPE" "$DEMO_PID" "$TAIL_PID"
    mkfifo "$PIPE" || exit 1
    cd "$(dirname "${BASH_SOURCE[0]}")/../.."
    cargo build -q -p sill-launcher-ui --example launcher_demo || exit 1
    WAYLAND_DISPLAY=wayland-0 "${CARGO_TARGET_DIR:-target}/debug/examples/launcher_demo" <"$PIPE" >"$D/ime-demo.log" 2>&1 &
    echo $! >"$DEMO_PID"
    tail -f "$IN" >"$PIPE" &
    echo $! >"$TAIL_PID"
    sleep 4
    echo open >>"$IN"
    sleep 2
    grep -o 'launcher.open_to_frame_ms=[0-9.]*' "$D/ime-demo.log" | tail -1
    ;;
  capture)
    out=$(grep -o 'sill-launcher-[A-Za-z0-9-]*' "$D/ime-demo.log" | grep -v catcher | head -1)
    out=${out:-sill-launcher-HDMI-A-2}
    echo "$out capture $D/ime-after.png" >>"$IN"
    sleep 2
    ls -la "$D/ime-after.png"
    ;;
  stop)
    echo quit >>"$IN"
    sleep 1
    if pid=$(ours "$TAIL_PID" "tail -f $IN"); then kill "$pid" 2>/dev/null; fi
    if pid=$(ours "$DEMO_PID" examples/launcher_demo); then kill "$pid" 2>/dev/null; fi
    rm -f "$DEMO_PID" "$TAIL_PID" "$PIPE"
    ;;
esac

#!/usr/bin/env bash
# Live IME check on KDE: open sill's launcher demo on wayland-0 and keep it open until
# `ime-live.sh capture` or `ime-live.sh stop`. The user types into it by hand (KWin has no
# virtual keyboard, so nothing can type for them).
set -uo pipefail
D="${IME_LIVE_DIR:-${XDG_RUNTIME_DIR:-/tmp}/sill-ime-live}"; mkdir -p "$D"
IN=$D/ime-cmds
case "${1:-start}" in
  start)
    # One copy at a time: several copies fight over the keyboard focus.
    if pgrep -f "tail -f $IN" >/dev/null || pgrep -f examples/launcher_demo >/dev/null; then
      echo "ime-live: a copy is already running; use \`$0 stop\` first" >&2
      exit 1
    fi
    : >"$IN"
    cd "$(dirname "${BASH_SOURCE[0]}")/../.."
    cargo build -q -p sill-surfaces --example launcher_demo || exit 1
    (tail -f "$IN" | WAYLAND_DISPLAY=wayland-0 target/debug/examples/launcher_demo >"$D/ime-demo.log" 2>&1 &)
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
    pkill -f "tail -f $IN" >/dev/null 2>&1 || true
    pkill -f examples/launcher_demo || true
    ;;
esac

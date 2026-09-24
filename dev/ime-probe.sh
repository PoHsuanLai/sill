#!/usr/bin/env bash
# CJK IME acceptance on the live KDE session (`WAYLAND_DISPLAY=wayland-0` by default; shell-host
# FINDINGS.md: text-input-v3 is double-buffered, accumulating `preedit_string`/`commit_string`/
# `delete_surrounding_text` until `done`; fcitx5 is installed; IME has only ever been exercised
# on KWin here). Only meaningful when the session is unlocked — a locked session hands no
# keyboard input to any client, IME included — so this checks `loginctl`'s `LockedHint` first
# and exits 2 with a clear message rather than hanging or failing confusingly.
#
# The rest depends on a real, focused, text-input-v3-enabled field: `sill-surfaces/examples/
# launcher_demo` (M4 launcher-ui) if it exists, else `bar_demo`'s text field if the launcher
# demo is not merged yet. Neither exists in this checkout (`bar_demo` has no text field either),
# so today this script always stops there (exit 2, naming M4 launcher-ui) after proving the two
# checks it can: the session is reachable and unlocked, and fcitx5 answers on the session bus.
# It prints the exact commands for the rest, to run by hand once a field exists.
#
# Usage: dev/ime-probe.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# The brief for this probe named `org.fcitx.Fcitx5.Controller1`; the interface this machine's
# fcitx5 actually answers on is `org.fcitx.Fcitx.Controller1` (no "5"), at bus name
# `org.fcitx.Fcitx5`, path `/controller` — verified live below (`busctl --user introspect
# org.fcitx.Fcitx5 /controller`). FINDINGS.md "M4 perf-ime" records the correction.
FCITX_BUS=org.fcitx.Fcitx5
FCITX_PATH=/controller
FCITX_IFACE=org.fcitx.Fcitx.Controller1
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"

# The seat0 session id (`loginctl list-sessions`' SEAT column), not `$XDG_SESSION_ID`: this
# script may run from a pts session that is not the graphical one it needs to check.
seat0_session() {
  loginctl list-sessions --no-legend | awk '$4 == "seat0" { print $1; exit }'
}

# 1. Exit 2 (not a failure — nothing here can proceed) if there is no seat0 session, or it is
#    locked.
check_unlocked() {
  local session locked
  session=$(seat0_session)
  if [ -z "$session" ]; then
    echo "ime-probe: no seat0 session (loginctl list-sessions); this needs the live KDE" \
      "session, not the nested compositor" >&2
    return 2
  fi
  locked=$(loginctl show-session "$session" -p LockedHint --value 2>/dev/null)
  if [ "$locked" != no ]; then
    echo "ime-probe: seat0 session $session is locked (LockedHint=$locked); unlock the" \
      "session and re-run — a locked session hands no keyboard input to any client" >&2
    return 2
  fi
  echo "ime-probe: seat0 session $session is unlocked" >&2
}

# 2. fcitx5 answers on the session bus.
check_fcitx5() {
  if ! busctl --user call "$FCITX_BUS" "$FCITX_PATH" "$FCITX_IFACE" State >/dev/null 2>&1; then
    echo "ime-probe: $FCITX_BUS $FCITX_PATH $FCITX_IFACE did not answer (is fcitx5 running" \
      "in this session? 'fcitx5 &' starts it)" >&2
    return 2
  fi
  echo "ime-probe: fcitx5 answered on $FCITX_BUS" >&2
}

# 3. A demo with a real text field: `launcher_demo` (M4 launcher-ui), or, failing that,
#    `bar_demo` IF a later wave gives it one (it does not today — the bar has no text input).
find_text_field_demo() {
  for name in launcher_demo bar_demo; do
    for profile in debug release; do
      local bin="$SILL_REPO/target/$profile/examples/$name"
      [ -x "$bin" ] && { echo "$bin"; return 0; }
    done
  done
  return 1
}

# The commands to run by hand once a text-field demo exists — this machine's own facts:
# `wtype` is installed, `ydotool` is not; fcitx5's Default input-method group here is
# `keyboard-us` + `chewing` (Taiwanese Zhuyin/Chewing, this machine's only CJK method).
# `SetCurrentIM`'s effect could not be confirmed live while writing this (no focused
# text-input-v3 context to observe it against — the session was locked); that confirmation is
# exactly what running this against a real field, once one exists, is for.
print_commands() {
  cat >&2 <<EOF

Commands for the rest, once a text-field demo exists:

  1. Build and run the demo on the live session:
       cargo build -p sill-surfaces --example launcher_demo
       WAYLAND_DISPLAY=wayland-0 target/debug/examples/launcher_demo &

  2. Give its text field keyboard focus (through its own debug-inject stdin, the bar_demo/
     dock_demo pattern: "<surface> click X Y" on the process's stdin, or a real click).

  3. Switch fcitx5 to Chewing (this machine's CJK method):
       busctl --user call $FCITX_BUS $FCITX_PATH $FCITX_IFACE SetCurrentIM s chewing

  4. Type Zhuyin keys through the compositor's virtual keyboard (wtype speaks wl_keyboard,
     the same input fcitx5's waylandim frontend watches as a real keyboard would produce) and
     commit — the exact keys are the Zhuyin romanisation of whatever you want typed; Chewing
     commits on tone entry or Enter:
       WAYLAND_DISPLAY=wayland-0 wtype '<zhuyin keys for the CJK text>'
       WAYLAND_DISPLAY=wayland-0 wtype -k Return

  5. Capture the field and read the PNG to confirm the CJK glyphs are there:
       sill debug capture <surface> /tmp/ime-after.png

  6. Switch back:
       busctl --user call $FCITX_BUS $FCITX_PATH $FCITX_IFACE SetCurrentIM s keyboard-us
EOF
}

check_unlocked || exit 2
check_fcitx5 || exit 2

# Both checks this script can run today have passed. The rest needs a real, focused,
# text-input-v3-enabled field, which does not exist in this checkout yet (M4 launcher-ui):
# `launcher_demo` is not built, and `bar_demo` has no text field. Driving one automatically
# needs its surface name, which that wave defines — guessing at one here would be exactly the
# kind of untested claim CONVENTIONS.md warns against ("a quirk without a trace is a rumour").
if demo=$(find_text_field_demo); then
  echo "ime-probe: found a text-field demo ($demo), but this script does not yet know its" \
    "surface name (M4 launcher-ui defines it) so it cannot drive it automatically" >&2
else
  echo "ime-probe: no text-field demo yet (M4 launcher-ui)" >&2
fi
print_commands
exit 2

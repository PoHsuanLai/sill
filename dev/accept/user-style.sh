#!/usr/bin/env bash
# The person's style.css, live (quire ARCHITECTURE.md section 10, sill section 8). One daemon runs
# from this checkout's debug build on shell-host's private headless KWin, on a private bus that is
# both its session and its system bus, with a scratch HOME and XDG tree, its own XDG_RUNTIME_DIR
# and the fake DDC; nothing reaches the user's session. The daemon is started once and never
# restarted: every step below changes the file under it.
#
#   1. Baseline: the bar, the dock and the OSD (shown again just before each of its captures:
#      its hold is a constant 1.5 s) are captured with no style.css.
#   2. `sill style check` on the style to be written prints its notes and exits 0.
#   3. style.css is written with a rule per surface (`.ds[data-surface=NAME]` and a token
#      override: the bar's and the dock's frame alpha to 0, the OSD's ink and shadow ring to magenta). Every
#      surface's capture changes without a restart (the same daemon pid answers ping);
#      `sill style check` on a file that is not CSS exits 1.
#   4. The file is edited (frame alpha .45, the ink and ring green): every surface changes again from the
#      first edit (the live text is replaced, not added to).
#   5. The file is deleted: every surface returns to its baseline (`changed_fraction` under 0.2%).
#
# Captures and logs: target/accept-user-style/. Exit 0 only when every step passed.
#
# Usage: dev/accept/user-style.sh
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_OUT_ROOT/accept-user-style"
mkdir -p "$OUT_DIR"
NAME=
SURFACES=()
DAEMON_PID=
# The share of a surface's pixels that must change, and that may differ once it reverts.
CHANGED=0.004
SETTLED=0.002

# The style, in the public selector surface (`data-surface`, namespaced for Blitz) and token
# overrides, which win over the design system's by coming last: the bar's and the dock's frame
# takes the alpha $1 (`--m-frame-alpha`), and the OSD's ink and card shadow the colour $2
# (`--ink`, `--m-box`).
style_for() {
  printf '.ds[data-surface=bar], .ds[data-surface=dock] { --m-frame-alpha: %s }\n' "$1"
  printf '.ds[data-surface=osd] { --ink: %s; --m-box: 0 0 0 6px %s }\n' "$2" "$2"
}

start_daemon() {
  mkdir -p "$CONFIG/sill"
  launch_daemon || return 1
  DAEMON_PID=$DAEMON
  NAME=$(output_name) || return 1
  SURFACES=("sill-bar-$NAME" "sill-dock-$NAME" "sill-osd-$NAME")
  sill debug osd volume 60 >"$OUT_DIR/osd.log" 2>&1 || return 1
  wait_surface "sill-osd-$NAME" 30 || { echo "the OSD never mapped" >&2; return 1; }
  sleep 1
}

# Captures every surface as $OUT_DIR/$1-<surface>.png. The OSD hides 1.5 s after a change, so it
# is shown again (and given its fade-in) right before its own capture.
capture_all() {
  local surface
  for surface in "${SURFACES[@]}"; do
    case "$surface" in
      sill-osd-*)
        sill debug osd volume 60 >>"$OUT_DIR/osd.log" 2>&1 || return 1
        sleep 0.5 ;;
    esac
    capture "$surface" "$OUT_DIR/$1-$surface.png" || { echo "cannot capture $surface" >&2; return 1; }
  done
}

step_baseline() { capture_all base; }

step_check_clean() {
  style_for 0 "rgb(255, 0, 255)" >"$OUT_DIR/good.css"
  sill style check "$OUT_DIR/good.css" >"$OUT_DIR/check-good.log" 2>&1 || {
    cat "$OUT_DIR/check-good.log" >&2; return 1; }
  cat "$OUT_DIR/check-good.log" >&2
}

step_written() {
  style_for 0 "rgb(255, 0, 255)" >"$CONFIG/sill/style.css"
  expect_all base different written || return 1
  printf '.ds { color: red \n}}}\n' >"$OUT_DIR/bad.css"
  if sill style check "$OUT_DIR/bad.css" >"$OUT_DIR/check-bad.log" 2>&1; then
    echo "sill style check accepted a file that is not CSS" >&2; return 1
  fi
  cat "$OUT_DIR/check-bad.log" >&2
}

step_edited() {
  style_for .45 "rgb(0, 255, 0)" >"$CONFIG/sill/style.css"
  expect_all written different edited || return 1
}

step_deleted() {
  rm -f "$CONFIG/sill/style.css"
  expect_all base same reverted
}

cleanup() {
  local result=0
  stop_daemon || result=1
  stop_compositor
  rm -rf "$RUNTIME"
  scratch_drop
  return "$result"
}

build || exit 1
scratch_new
# A short runtime folder of its own: the agent socket's path must fit in 108 bytes.
rm -rf "$RUNTIME"
RUNTIME=$(mktemp -d /tmp/sill-rt.XXXXXX)
chmod 700 "$RUNTIME"
: >"$OUT_DIR/capture.log"
: >"$OUT_DIR/steps.log"
start_compositor --default kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; rm -rf "$RUNTIME"; scratch_drop; exit 1; }
trap cleanup EXIT
backdrop_seed "$CONFIG" || exit 1
start_daemon || { cleanup; trap - EXIT; exit 1; }
run_steps step_baseline step_check_clean step_written step_edited step_deleted
status=$STATUS
cleanup || status=1
trap - EXIT
echo "logs and captures in $OUT_DIR"
exit "$status"

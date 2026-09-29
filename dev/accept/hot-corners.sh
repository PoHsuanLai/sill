#!/usr/bin/env bash
# Hot corners acceptance (design/20 §1.16, design/13 §13.3.12), on the nested cosmic-comp
# (dev/nested.sh; a running one is reused and never stopped).
# Starts `sill run` from this checkout's debug build (`--features debug`) with a scratch HOME and
# XDG tree, on a private bus (dev/private-bus.conf) that is both its session and its system bus,
# and with its own XDG_RUNTIME_DIR (the nested socket given as an absolute WAYLAND_DISPLAY), so
# another sill running meanwhile keeps its own single-instance lock and IPC socket. The scratch
# config: top_left = launcher, top_right = notification_center, bottom_right = command (touches
# a temp file), bottom_left = none; dwell_ms and rearm_ms are left at their defaults (150, 500),
# so the timing checks below are against the real numbers. Never sets a corner to `lock` (never
# touches the real session's locker). The daemon is quit at the end.
#
#   1. `sill debug stats` lists the three active corners and not `bottom_left`'s.
#   2. `sill debug corner top-left` opens the launcher; `top-right` opens the notification
#      center; `bottom-right` runs the command, touching the temp file.
#   3. A real pointer dwell on the top-left corner's own surface (shell-host's debug-inject, the
#      same path a real pointer takes): entering fires the launcher only after the dwell (checked
#      at 50 ms, too early, and 400 ms, after it); a second pointer with no leave between does
#      not fire again (Toggle needs a fresh dwell: the launcher stays open); a leave and 600 ms
#      (past `rearm_ms`) re-arm it, and a fresh dwell fires again, which Toggle closes (design/13
#      §13.3.12's toggle rule: Launcher, NotificationCenter, ControlCenter, Workspaces and
#      ShowDesktop close on a second hit; Lock and Command never toggle).
#   4. `top_left = "none"` and `sill reload` removes its surface; the other two stay.
#   5. 5 s idle, no debug command sent meanwhile: 0 frames on the surviving corner surfaces (the
#      element is never painted, so even the real dwell in step 3 left no lasting frames; this
#      checks the quiet case).
#
# There is no `hot_corners.modifier`: no compositor gives a keyboard-`None` layer surface a held
# modifier portably, and taking the keyboard to get one would steal focus, which a hot corner must
# never do. The machine has no modifier input, so there is nothing to check here.
#
# Exit 0 when every step passed, 1 on a failure. Another sill daemon started from a
# `target/*/sill run` is waited for (up to 10 minutes), so two shells never share the nested
# output's screenshots.
#
# Usage: dev/nested.sh && dev/accept/hot-corners.sh
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_REPO/target/accept-hot-corners"
mkdir -p "$OUT_DIR"

# Whether SURFACE $1 is listed in `sill debug stats` (popups and corners alike).
has_surface() {
  sill debug stats | python3 -c '
import json, sys
names = {row["surface"] for row in json.loads(sys.stdin.read())["v"]}
sys.exit(0 if sys.argv[1] in names else 1)' "$1"
}

start_hot_corners_daemon() {
  MARK="$OUT_DIR/bottom-right-touched"
  rm -f "$MARK"
  backdrop_seed "$CONFIG" || return 1
  mkdir -p "$CONFIG/sill"
  cat >"$CONFIG/sill/settings.toml" <<TOML
[hot_corners]
top_left = "launcher"
top_right = "notification_center"
bottom_left = "none"
bottom_right = "command"
bottom_right_command = "touch $MARK"
TOML
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  TOP_LEFT="sill-hot-corner-top-left-$NAME"
  TOP_RIGHT="sill-hot-corner-top-right-$NAME"
  BOTTOM_LEFT="sill-hot-corner-bottom-left-$NAME"
  BOTTOM_RIGHT="sill-hot-corner-bottom-right-$NAME"
  LAUNCHER="sill-launcher-$NAME"
  CENTER="sill-notification-center-$NAME"
  sleep 1.5
  backdrop_up "$CONFIG" || echo "backdrop: none (the shots show the bare compositor)" >&2
  echo "daemon on a private bus and runtime dir $RUNTIME; corners $TOP_LEFT, $TOP_RIGHT, $BOTTOM_RIGHT (not $BOTTOM_LEFT)" >&2
}

# 1.
step_stats() {
  local log="$OUT_DIR/stats.log"
  : >"$log"
  sill debug stats >"$log" 2>&1
  has_surface "$TOP_LEFT" || { echo "$TOP_LEFT missing from stats" | tee -a "$log" >&2; return 1; }
  has_surface "$TOP_RIGHT" || { echo "$TOP_RIGHT missing from stats" | tee -a "$log" >&2; return 1; }
  has_surface "$BOTTOM_RIGHT" || { echo "$BOTTOM_RIGHT missing from stats" | tee -a "$log" >&2; return 1; }
  if has_surface "$BOTTOM_LEFT"; then
    echo "$BOTTOM_LEFT present (its action is none)" | tee -a "$log" >&2; return 1
  fi
  echo "three active corners listed ($TOP_LEFT, $TOP_RIGHT, $BOTTOM_RIGHT), not $BOTTOM_LEFT" \
    | tee -a "$log" >&2
}

# 2.
step_fake_hits() {
  local log="$OUT_DIR/fake.log"
  : >"$log"
  sill debug corner top-left >>"$log" 2>&1 || return 1
  wait_surface "$LAUNCHER" 20 up || { echo "top-left did not open the launcher" | tee -a "$log" >&2; return 1; }
  sill launcher close >>"$log" 2>&1
  wait_surface "$LAUNCHER" 20 down || return 1

  sill debug corner top-right >>"$log" 2>&1 || return 1
  wait_surface "$CENTER" 20 up || { echo "top-right did not open the center" | tee -a "$log" >&2; return 1; }
  sill notifications close >>"$log" 2>&1
  wait_surface "$CENTER" 20 down || return 1

  sill debug corner bottom-right >>"$log" 2>&1 || return 1
  for _ in $(seq 40); do [ -e "$MARK" ] && break; sleep 0.05; done
  [ -e "$MARK" ] || { echo "bottom-right's command never touched $MARK" | tee -a "$log" >&2; return 1; }
  echo "top-left opens the launcher, top-right the center, bottom-right touches $MARK" \
    | tee -a "$log" >&2
}

# 3.
step_dwell() {
  local log="$OUT_DIR/dwell.log"
  : >"$log"
  sill launcher close >>"$log" 2>&1
  wait_surface "$LAUNCHER" 20 down || return 1

  sill debug pointer "$TOP_LEFT" 0 0 >>"$log" 2>&1 || return 1
  sleep 0.05
  if mapped "$LAUNCHER"; then
    echo "the launcher opened before the dwell (50 ms)" | tee -a "$log" >&2; return 1
  fi
  sleep 0.35
  mapped "$LAUNCHER" || { echo "the launcher did not open by 400 ms" | tee -a "$log" >&2; return 1; }

  # A second pointer, still inside, with no leave: no fresh dwell, no second fire.
  sill debug pointer "$TOP_LEFT" 0 0 >>"$log" 2>&1
  sleep 0.3
  mapped "$LAUNCHER" || { echo "a second pointer with no leave closed it" | tee -a "$log" >&2; return 1; }

  sill debug leave "$TOP_LEFT" >>"$log" 2>&1 || return 1
  sleep 0.6
  sill debug pointer "$TOP_LEFT" 0 0 >>"$log" 2>&1 || return 1
  sleep 0.3
  wait_surface "$LAUNCHER" 20 down || {
    echo "leaving and 600 ms did not re-arm the corner (the second dwell should have toggled it closed)" \
      | tee -a "$log" >&2
    return 1
  }
  sill debug leave "$TOP_LEFT" >>"$log" 2>&1
  echo "the dwell fires once at ~150 ms, not at 50 ms; a second pointer with no leave does nothing; a leave and 600 ms re-arm it, and the next dwell toggles the launcher closed" \
    | tee -a "$log" >&2
}

# 4.
step_reload() {
  local log="$OUT_DIR/reload.log"
  : >"$log"
  cat >"$CONFIG/sill/settings.toml" <<TOML
[hot_corners]
top_left = "none"
top_right = "notification_center"
bottom_left = "none"
bottom_right = "command"
bottom_right_command = "touch $MARK"
TOML
  sill reload >>"$log" 2>&1 || return 1
  for _ in $(seq 40); do has_surface "$TOP_LEFT" || break; sleep 0.05; done
  if has_surface "$TOP_LEFT"; then
    echo "$TOP_LEFT still listed after top_left = none" | tee -a "$log" >&2; return 1
  fi
  has_surface "$TOP_RIGHT" || { echo "$TOP_RIGHT was removed too" | tee -a "$log" >&2; return 1; }
  has_surface "$BOTTOM_RIGHT" || { echo "$BOTTOM_RIGHT was removed too" | tee -a "$log" >&2; return 1; }
  echo "top_left = none removed $TOP_LEFT; $TOP_RIGHT and $BOTTOM_RIGHT stay" | tee -a "$log" >&2
}

# 5.
step_idle() {
  local log="$OUT_DIR/idle.log" f0r f1r f0b f1b
  : >"$log"
  f0r=$(frames_of "$TOP_RIGHT")
  f0b=$(frames_of "$BOTTOM_RIGHT")
  sleep 5
  f1r=$(frames_of "$TOP_RIGHT")
  f1b=$(frames_of "$BOTTOM_RIGHT")
  echo "5 s idle: $(( f1r - f0r )) frames on $TOP_RIGHT, $(( f1b - f0b )) on $BOTTOM_RIGHT" \
    | tee -a "$log" >&2
  [ "$(( f1r - f0r ))" = 0 ] && [ "$(( f1b - f0b ))" = 0 ]
}

cleanup() {
  local result=0
  stop_daemon || result=1
  backdrop_down
  stop_private_bus
  scratch_drop
  return "$result"
}

nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
use_nested_socket || exit 1
build || exit 1
await_no_process 'target/[a-z]*/sill run' || exit 1
scratch_new
start_private_bus || { scratch_drop; exit 1; }
start_hot_corners_daemon || { cleanup; exit 1; }
run_steps step_stats step_fake_hits step_dwell step_reload step_idle
status=$STATUS
cleanup || status=1
echo "logs in $OUT_DIR"
exit "$status"

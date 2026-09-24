#!/usr/bin/env bash
# Milestone 1 acceptance for the bar (plan: "`accept-bar.sh` (exclusive zone, idle 0 frames,
# menus open/dismiss)"). M1 daemon fills every step it can without the bar component (ping,
# quit, single instance, IPC timeout, reload); the bar-specific steps stay `exit 2` naming
# bar-ui, which owns them.
# Exit 0 only when every step passed; a step not filled yet exits 2, never 0.
#
# Usage: dev/nested.sh && dev/accept-bar.sh [OUTPUT]      (OUTPUT defaults to the first one)
#
# `step_quit` stops the daemon, so it always runs last: everything before it needs one running
# (started separately, e.g. by `dev/run.sh` or `dev/nested.sh`'s own seed).
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-bar"
mkdir -p "$OUT_DIR"
OUTPUT="${1:-}"

sill() { "$SHELL_HOST_DEV/run.sh" "$SILL_BIN" "$@"; }
nested() { "$SHELL_HOST_DEV/run.sh" "$@"; }
HOST_REPO="$(cd "$SHELL_HOST_DEV/../.." && pwd)"
PROBE="$HOST_REPO/target/release/examples/probe_toplevel"
# The bar's height: `bar.height_px`, 32 unless the settings say otherwise.
BAR_H="${BAR_H:-32}"

# Every bar surface's frame and commit counters from `sill debug stats`, as `name frames commits`
# lines (popups, `<bar>/popup`, are left out).
bar_counters() {
  sill debug stats | python3 -c '
import json, sys
reply = json.loads(sys.stdin.read())
for row in reply["v"]:
    if row["surface"].startswith("sill-bar-") and "/" not in row["surface"]:
        print(row["surface"], row["frames"], row["commits"])'
}

# 1. The daemon answers.
step_ping() {
  sill ping >"$OUT_DIR/ping.log" 2>&1
}

# 2. A second `sill run` finds the lock held and exits 0 without touching Wayland (latchkey
#    `AlreadyRunning`), and does so fast: `daemon::run` checks the lock before it does anything
#    slow, so a live daemon never lets a rival get far enough to matter.
step_single_instance() {
  local start elapsed
  start=$(date +%s%N)
  if ! sill run >"$OUT_DIR/single-instance.log" 2>&1; then
    echo "a second 'sill run' did not exit 0 (see $OUT_DIR/single-instance.log)" >&2
    return 1
  fi
  elapsed=$(( ($(date +%s%N) - start) / 1000000 ))
  echo "second 'sill run' exited 0 in ${elapsed}ms" >>"$OUT_DIR/single-instance.log"
  if [ "$elapsed" -gt 3000 ]; then
    echo "took ${elapsed}ms; expected well under the 3 s client start budget" >&2
    return 1
  fi
}

# 3. `sill reload` re-reads settings.toml and answers Done (no bar to confirm the effect on:
#    that is dock-ui's/bar-ui's job through a visible change).
step_reload() {
  sill reload >"$OUT_DIR/reload.log" 2>&1 || return 1
  grep -q '"kind":"done"' "$OUT_DIR/reload.log"
}

# 4. A request in the ordinary case answers well inside the 2 s reply timeout (`ipc_server`'s
#    own tests, `crates/sill/tests/ipc.rs`, exercise the timeout itself with a fake dispatcher
#    that never answers; nothing reachable from this script can be made to hang deliberately).
step_ipc_latency() {
  local start elapsed
  start=$(date +%s%N)
  sill ping >"$OUT_DIR/ipc-latency.log" 2>&1 || return 1
  elapsed=$(( ($(date +%s%N) - start) / 1000000 ))
  echo "ping answered in ${elapsed}ms" >>"$OUT_DIR/ipc-latency.log"
  if [ "$elapsed" -gt 1000 ]; then
    echo "took ${elapsed}ms; expected well under the 2 s reply timeout" >&2
    return 1
  fi
}

# 5. A maximised foot window's height is output_h - bar_h (design/13 §13.3.1: Reserve(32)).
#    shell-host's `probe_toplevel` stands in for foot: it asks to be maximised and prints each
#    configure as JSON.
step_exclusive_zone() {
  [ -x "$PROBE" ] || (cd "$HOST_REPO" && cargo build --release -p shell-host --features debug-inject \
    --example probe_toplevel >&2) || return 1
  local out_h probe_log="$OUT_DIR/probe.log" height
  out_h=$(nested wayland-info | sed -n 's/.*logical_width: [0-9]*, logical_height: \([0-9]*\).*/\1/p' | head -1)
  [ -n "$out_h" ] || { echo "no output height from wayland-info" >&2; return 1; }
  # The probe reads commands on stdin, which must be pollable: a pipe, not /dev/null.
  sleep 20 | nested "$PROBE" >"$probe_log" 2>&1 &
  local probe=$!
  for _ in $(seq 100); do
    grep -q '"maximized":true' "$probe_log" && break
    sleep 0.1
  done
  kill "$probe" 2>/dev/null; wait "$probe" 2>/dev/null
  height=$(grep '"maximized":true' "$probe_log" | tail -1 | python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["height"])')
  # The dock (M3) reserves its base + margin too when the daemon runs one on this output.
  local dock_reserve=0
  if sill debug stats 2>/dev/null | grep -q '"sill-dock-'; then dock_reserve=72; fi
  echo "output ${out_h}, maximised window ${height}, bar ${BAR_H}, dock ${dock_reserve}" | tee -a "$OUT_DIR/exclusive-zone.log" >&2
  [ "$height" = "$(( out_h - BAR_H - dock_reserve ))" ]
}

# 6. Idle: 5 s with the pointer still, the bar's commit counter does not move (plan M0 c).
#    Also shoots the nested screen to dev/out/bar-cosmic.png.
step_idle() {
  local before after
  sleep 1
  before=$(bar_counters) || return 1
  [ -n "$before" ] || { echo "no bar surface in 'sill debug stats'" >&2; return 1; }
  sleep 5
  after=$(bar_counters) || return 1
  printf 'before:\n%s\nafter:\n%s\n' "$before" "$after" | tee "$OUT_DIR/idle.log" >&2
  "$SHELL_HOST_DEV/shot.sh" "$SILL_REPO/dev/out/bar-cosmic.png" >&2 || true
  [ "$before" = "$after" ]
}

# 7. Menus: a press on a title opens its menu; an outside press dismisses it (design/13 §13.8
#    tests 1 and 3), on the daemon's own bar through `sill debug click` (FINDINGS F80). The
#    clock sits at the bar's right end; the bar's empty middle stands in for an outside press
#    (a compositor-level outside press cannot be synthesized, shell-host F17). Whether the popup
#    is up is whether `sill debug capture <bar>/popup` succeeds: `debug stats` lists bars only.
#    Shoots the nested screen with the menu open to dev/out/bar-cosmic-menu.png.
step_menus() {
  local log="$OUT_DIR/menus.log" bar width y popup opened=no closed=no
  : >"$log"
  bar=$(bar_counters | awk 'NR==1 {print $1}')
  [ -n "$bar" ] || { echo "no bar surface in 'sill debug stats'" >&2; return 1; }
  width=$(nested wayland-info | sed -n 's/.*logical_width: \([0-9]*\), logical_height: .*/\1/p' | head -1)
  [ -n "$width" ] || { echo "no output width from wayland-info" >&2; return 1; }
  y=$(( BAR_H / 2 ))
  popup="$bar/popup"
  popup_up() { sill debug capture "$popup" "$OUT_DIR/popup.png" >>"$log" 2>&1; }
  if popup_up; then
    echo "a popup was already open before the click" | tee -a "$log" >&2
    return 1
  fi
  echo "click the clock at $(( width - 60 )),$y on $bar" >>"$log"
  sill debug click "$bar" $(( width - 60 )) "$y" >>"$log" 2>&1 || return 1
  for _ in $(seq 40); do
    popup_up && { opened=yes; break; }
    sleep 0.05
  done
  sleep 0.5
  "$SHELL_HOST_DEV/shot.sh" "$SILL_REPO/dev/out/bar-cosmic-menu.png" >&2 || true
  echo "click the bar's middle at $(( width / 2 )),$y" >>"$log"
  sill debug click "$bar" $(( width / 2 )) "$y" >>"$log" 2>&1 || return 1
  for _ in $(seq 40); do
    popup_up || { closed=yes; break; }
    sleep 0.05
  done
  echo "opened: $opened, dismissed: $closed" | tee -a "$log" >&2
  [ "$opened" = yes ] && [ "$closed" = yes ]
}

# 8. Quit closes every surface and exits; a `ping` after it finds nobody home. Runs last: it
#    stops the daemon every earlier step relied on.
step_quit() {
  sill quit >"$OUT_DIR/quit.log" 2>&1 || return 1
  grep -q '"kind":"done"' "$OUT_DIR/quit.log" || return 1
  sleep 0.2
  if sill ping >"$OUT_DIR/quit-ping.log" 2>&1; then
    echo "'sill ping' still succeeded after quit (see $OUT_DIR/quit-ping.log)" >&2
    return 1
  fi
}

status=0
for step in step_ping step_single_instance step_reload step_ipc_latency step_exclusive_zone \
  step_idle step_menus step_quit; do
  if "$step"; then
    echo "PASS $step"
  else
    rc=$?
    echo "FAIL $step (exit $rc)"
    status=$(( rc > status ? rc : status ))
  fi
done
echo "output: ${OUTPUT:-first}; logs in $OUT_DIR"
exit "$status"

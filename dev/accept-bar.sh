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
step_exclusive_zone() {
  echo "exclusive zone: bar-ui fill (maximise foot, read its configure size)" >&2
  return 2
}

# 6. Idle: 5 s with the pointer still, the bar's commit counter does not move (plan M0 c).
step_idle() {
  echo "idle frames: bar-ui fill (sill debug stats twice, 5 s apart, compare commits)" >&2
  return 2
}

# 7. Menus: a press on a title opens its menu; an outside press dismisses it (design/13 §13.8
#    tests 1 and 3, through sill debug pointer).
step_menus() {
  echo "menus open/dismiss: bar-ui fill" >&2
  return 2
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

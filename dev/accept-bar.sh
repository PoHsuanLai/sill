#!/usr/bin/env bash
# Milestone 1 acceptance for the bar (plan: "`accept-bar.sh` (exclusive zone, idle 0 frames,
# menus open/dismiss)"). SKELETON: the W0 freeze writes the steps; M1 daemon fills each.
# Exit 0 only when every step passed; a step not filled yet exits 2, never 0.
#
# Usage: dev/nested.sh && dev/accept-bar.sh [OUTPUT]      (OUTPUT defaults to the first one)
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

# 2. A maximised foot window's height is output_h - bar_h (design/13 §13.3.1: Reserve(32)).
step_exclusive_zone() {
  echo "exclusive zone: M1 fill (maximise foot, read its configure size)" >&2
  return 2
}

# 3. Idle: 5 s with the pointer still, the bar's commit counter does not move (plan M0 c).
step_idle() {
  echo "idle frames: M1 fill (sill debug stats twice, 5 s apart, compare commits)" >&2
  return 2
}

# 4. Menus: a press on a title opens its menu; an outside press dismisses it (design/13 §13.8
#    tests 1 and 3, through sill debug pointer).
step_menus() {
  echo "menus open/dismiss: M1 fill" >&2
  return 2
}

status=0
for step in step_ping step_exclusive_zone step_idle step_menus; do
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

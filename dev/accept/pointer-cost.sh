#!/usr/bin/env bash
# Pointer cost: how many frames each surface renders while the pointer only MOVES (no click) over
# the wallpaper, over the bar, over the dock, and while it is still. A surface that repaints at the
# pointer rate on a still desktop is the iGPU cost seen on a real 4K machine (a full-surface
# re-raster per pointer event, shell-host handlers/band invalidate).
#
# Prints one line per region: the seconds it ran and, per surface, the frames and callbacks it
# gained. This script only measures (it passes whatever it sees); the verdict is read from the table.
#
# Usage: dev/accept/pointer-cost.sh        SECS=10 per region
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_OUT_ROOT/accept-pointer-cost"
mkdir -p "$OUT_DIR"
SECS="${SECS:-10}"

start_compositor || exit 1
scratch_new
cleanup() {
  stop_input
  stop_daemon
  stop_private_bus
  stop_compositor
  scratch_drop
}
trap cleanup EXIT

build || exit 1
start_private_bus || exit 1
# Started plainly (no fake DDC): a daemon built before `--fake-ddc` existed does not know the flag,
# and in the jail there is no display bus to probe. The private buses are still the only ones.
if ! launch_daemon; then
  require_private_system_bus "$BUS" || exit 1
  in_env "${LAUNCH_PREFIX[@]}" "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  wait_ping || exit 1
fi
read -r OUT_W OUT_H < <(output_size)
start_input || exit 1
echo "output ${OUT_W}x${OUT_H}" >&2
sleep 5 # the first frames and the wallpaper settle

# One line per surface: name frames callbacks.
snapshot() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    print(row["surface"], row["frames"], row["callbacks"])'
}

# Moves the pointer back and forth along y=$2 between x=$1 and x=$3 for SECS seconds.
sweep() {
  local x0=$1 y=$2 x1=$3 end i x
  end=$(($(date +%s) + SECS))
  while [ "$(date +%s)" -lt "$end" ]; do
    local cmds=()
    for i in $(seq 0 19); do
      x=$((x0 + (x1 - x0) * i / 19))
      cmds+=("move $x $y" "sleep 16")
    done
    input "${cmds[@]}" || return 1
  done
}

# measure LABEL COMMAND...: runs the command, prints the frames each surface gained meanwhile.
measure() {
  local label=$1 t0 t1
  shift
  snapshot >"$OUT_DIR/before.txt"
  t0=$(date +%s.%N)
  "$@"
  t1=$(date +%s.%N)
  snapshot >"$OUT_DIR/after.txt"
  python3 - "$label" "$t0" "$t1" "$OUT_DIR/before.txt" "$OUT_DIR/after.txt" <<'PY'
import sys

label, t0, t1, before, after = sys.argv[1:]
secs = float(t1) - float(t0)


def load(path):
    rows = {}
    for line in open(path):
        name, frames, callbacks = line.split()
        rows[name] = (int(frames), int(callbacks))
    return rows


b, a = load(before), load(after)
gained = [
    (n, a[n][0] - b.get(n, (0, 0))[0], a[n][1] - b.get(n, (0, 0))[1]) for n in a
]
gained = [row for row in gained if row[1] or row[2]]
cells = ", ".join(f"{n}: {f} frames/{c} callbacks ({f / secs:.0f} fps)" for n, f, c in gained)
print(f"{label:<10} {secs:5.1f}s  {cells or 'no surface rendered'}")
PY
}

left=$((OUT_W / 8))
right=$((OUT_W * 7 / 8))
measure still sleep "$SECS"
measure wallpaper sweep "$left" $((OUT_H / 2)) "$right"
measure bar sweep "$left" 10 "$right"
measure dock sweep $((OUT_W / 2 - 200)) $((OUT_H - 40)) $((OUT_W / 2 + 200))

#!/usr/bin/env bash
# The icon style on the dock, against the running nested cosmic-comp (`dev/nested.sh`; a
# running wayland-1 is reused and never stopped). Starts `sill run` from this checkout's debug
# build with a scratch XDG_CONFIG_HOME and XDG_CACHE_HOME, so neither quire's appearance file
# nor sill's pins nor the icon cache in the real home are touched, and quits it at the end.
#
# The dock holds COSMIC Terminal and COSMIC Files (quire's shipped icons) and Google Chrome and
# btop (theme icons on the neutral plate). With `icons.style` set in the scratch
# `quire/appearance.toml` to colour, then muted, then monochrome (the running dock follows the
# file live), it checks that the Muted and Monochrome runs each write re-tinted rasters into
# the scratch cache, and captures the dock surface each time:
# dev/out/dock-icons-{colour,muted,monochrome}.png.
#
# Usage: dev/nested.sh && dev/accept-icons.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-icons"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
PINS=(com.system76.CosmicTerm com.system76.CosmicFiles google-chrome btop)

# shellcheck disable=SC1091
source "$SHELL_HOST_DEV/.nested-env"
export WAYLAND_DISPLAY

sill() { "$SILL_BIN" "$@"; }
nested() { "$SHELL_HOST_DEV/run.sh" "$@"; }

CONFIG="" CACHE="" DAEMON=""
cleanup() {
  if [ -n "$DAEMON" ] && kill -0 "$DAEMON" 2>/dev/null; then
    sill quit >/dev/null 2>&1 || true
    for _ in $(seq 20); do kill -0 "$DAEMON" 2>/dev/null || break; sleep 0.1; done
    kill -0 "$DAEMON" 2>/dev/null && kill "$DAEMON" 2>/dev/null || true
  fi
  [ -n "$CONFIG" ] && rm -rf "$CONFIG"
  [ -n "$CACHE" ] && rm -rf "$CACHE"
}
trap cleanup EXIT

style() {
  mkdir -p "$CONFIG/quire"
  printf '[icons]\nstyle = "%s"\nmonochrome_tint = "space"\n' "$1" >"$CONFIG/quire/appearance.toml"
}

dock_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"].startswith("sill-dock-"):
        print(row["surface"]); break'
}

# How many re-tinted rasters of style $1 the scratch cache holds.
cached() {
  find "$CACHE/sill/icons" -name "*-$1-*.png" 2>/dev/null | wc -l
}

step_ping_compositor() {
  nested wayland-info >"$OUT_DIR/wayland-info.log" 2>&1
}

step_start() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
  CONFIG=$(mktemp -d)
  CACHE=$(mktemp -d)
  style colour
  XDG_CONFIG_HOME="$CONFIG" XDG_CACHE_HOME="$CACHE" nested "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  sill ping >/dev/null 2>&1 || { echo "sill never answered ping" >&2; return 1; }
  local id
  for id in "${PINS[@]}"; do sill dock pin "$id" >>"$OUT_DIR/pin.log" 2>&1 || return 1; done
  for _ in $(seq 50); do DOCK=$(dock_name); [ -n "$DOCK" ] && break; sleep 0.1; done
  [ -n "${DOCK:-}" ] || { echo "no dock surface in 'sill debug stats'" >&2; return 1; }
  sleep 2
}

capture() {
  sill debug capture "$DOCK" "$SHOTS/dock-icons-$1.png" >"$OUT_DIR/capture-$1.log" 2>&1
  grep -q '"kind":"done"' "$OUT_DIR/capture-$1.log"
}

step_colour() {
  capture colour && [ "$(cached muted)" = 0 ] && [ "$(cached monochrome)" = 0 ]
}

# Switch to style $1 and wait for its rasters: the two theme icons, plus the two shipped ones
# under Monochrome.
switch_and_wait() {
  local want=$2 n=0
  style "$1"
  for _ in $(seq 100); do
    n=$(cached "$1")
    [ "$n" -ge "$want" ] && break
    sleep 0.1
  done
  echo "$1: $n re-tinted rasters in the cache" | tee -a "$OUT_DIR/cache.log" >&2
  # One more paint after the last raster lands.
  sleep 1
  capture "$1" && [ "$n" -ge "$want" ]
}

step_muted() { switch_and_wait muted 2; }
step_monochrome() { switch_and_wait monochrome 4; }

# The dock's cumulative frame count.
dock_frames() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"])' "$DOCK"
}

# Once the rasters have landed nothing re-tints or repaints: five idle seconds, zero frames.
step_idle() {
  local before after
  before=$(dock_frames)
  sleep 5
  after=$(dock_frames)
  echo "idle 5 s in Monochrome: frames $(( after - before ))" | tee "$OUT_DIR/idle.log" >&2
  [ "$before" = "$after" ]
}

status=0
for step in step_ping_compositor step_start step_colour step_muted step_monochrome step_idle; do
  if "$step"; then
    echo "PASS $step"
  else
    rc=$?
    echo "FAIL $step (exit $rc)"
    status=$(( rc > status ? rc : status ))
    [ "$step" = step_start ] || [ "$step" = step_ping_compositor ] && break
  fi
done
ls -l "$CACHE/sill/icons" >"$OUT_DIR/cache-files.log" 2>&1
echo "output dir: $OUT_DIR; shots: $SHOTS/dock-icons-{colour,muted,monochrome}.png" >&2
exit "$status"

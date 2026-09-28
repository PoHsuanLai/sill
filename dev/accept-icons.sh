#!/usr/bin/env bash
# The icon style on the dock, against the running nested cosmic-comp (`dev/nested.sh`; a
# running wayland-1 is pinged first, reused and never stopped). Starts `sill run` from this
# checkout's debug build with a scratch XDG_CONFIG_HOME, XDG_CACHE_HOME and XDG_RUNTIME_DIR (the
# nested socket as an absolute WAYLAND_DISPLAY) and a private session bus
# (dev/private-bus.conf), so neither quire's appearance file nor sill's pins nor the icon cache
# nor the real session bus are touched, and quits it at the end.
#
# sill ships no per-app artwork of its own any more (FINDINGS "App icons: shipped set removed"):
# the dock holds COSMIC Terminal, COSMIC Files, Google Chrome and btop, every one on the neutral
# plate with its own theme icon, tinted along with the plate under Muted and Monochrome (sill
# FINDINGS Q72). With `icons.style` set in the scratch `quire/appearance.toml` to colour, then
# muted, then monochrome (the running dock follows the file live), it checks that the Muted and
# Monochrome runs each write re-tinted rasters into the scratch cache, one per pinned app, and
# captures the dock surface each time: dev/out/dock-icons-{colour,muted,monochrome}.png. A last
# switch to `appearance.theme = "dark"` in Monochrome (no new raster: the dark scheme changes
# the plate's own stops, not the icon's) captures dev/out/dock-icons-monochrome-dark.png. The
# launcher (queried for "cosmic", Colour style) is captured too: dev/out/launcher-icons.png,
# COSMIC Terminal and Files as rows drawing their own hicolor icon, the same fallback path.
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
HOST_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
NESTED_SOCKET="$HOST_RUNTIME/$WAYLAND_DISPLAY"
export WAYLAND_DISPLAY

RUNTIME=$(mktemp -d)
chmod 700 "$RUNTIME"

sill() { XDG_RUNTIME_DIR="$RUNTIME" "$SILL_BIN" "$@"; }
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
  rm -rf "$RUNTIME"
}
trap cleanup EXIT

style() {
  mkdir -p "$CONFIG/quire"
  printf '[icons]\nstyle = "%s"\nmonochrome_tint = "space"\n' "$1" >"$CONFIG/quire/appearance.toml"
}

# Monochrome again, but `appearance.theme = "dark"`: no new raster (the dark scheme changes the
# plate alone, through quire's own stylesheet, not a re-tinted file), so no wait, only a shot.
style_monochrome_dark() {
  mkdir -p "$CONFIG/quire"
  printf '[appearance]\ntheme = "dark"\n\n[icons]\nstyle = "monochrome"\nmonochrome_tint = "space"\n' \
    >"$CONFIG/quire/appearance.toml"
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
  use_fake_ddc "$CONFIG" >/dev/null
  require_fake_ddc || return 1
  XDG_CONFIG_HOME="$CONFIG" XDG_CACHE_HOME="$CACHE" XDG_RUNTIME_DIR="$RUNTIME" \
    WAYLAND_DISPLAY="$NESTED_SOCKET" \
    dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- \
    sh -c "$PRIVATE_BOTH_BUSES" "$SILL_BIN" >"$OUT_DIR/daemon.log" 2>&1 &
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

launcher_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"].startswith("sill-launcher-"):
        print(row["surface"]); break'
}

# The launcher, queried for "cosmic" so COSMIC Terminal and Files show as rows: their own
# hicolor icon on the row's plate, the fallback every entry takes now (dev/out/launcher-icons.png).
step_launcher() {
  sill launcher open --query cosmic >>"$OUT_DIR/launcher.log" 2>&1
  local name="" rc=1
  for _ in $(seq 50); do name=$(launcher_name); [ -n "$name" ] && break; sleep 0.1; done
  if [ -n "$name" ]; then
    # The surface exists in `debug stats` before its first frame is presented; retry the
    # capture rather than racing it with one fixed sleep.
    for _ in $(seq 30); do
      sill debug capture "$name" "$SHOTS/launcher-icons.png" >"$OUT_DIR/capture-launcher.log" 2>&1
      grep -q '"kind":"done"' "$OUT_DIR/capture-launcher.log" && { rc=0; break; }
      sleep 0.2
    done
  else
    echo "no launcher surface in 'sill debug stats'" >&2
  fi
  sill launcher close >>"$OUT_DIR/launcher.log" 2>&1
  return "$rc"
}

# Switch to style $1 and wait for its rasters: one per pinned app (every app takes the theme
# path now, so Muted and Monochrome both re-tint all four).
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

step_muted() { switch_and_wait muted 4; }
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

# The plate under the dark scheme (sill FINDINGS Q72): the tinted plate's own dark stops, not
# the light ones the other shots show.
step_monochrome_dark() {
  style_monochrome_dark
  sleep 1
  capture monochrome-dark
}

status=0
for step in step_ping_compositor step_start step_colour step_launcher step_muted \
  step_monochrome step_idle step_monochrome_dark; do
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
echo "output dir: $OUT_DIR; shots: $SHOTS/dock-icons-{colour,muted,monochrome,monochrome-dark}.png, $SHOTS/launcher-icons.png" >&2
exit "$status"

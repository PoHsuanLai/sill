#!/usr/bin/env bash
# Wallpaper acceptance (design/20 §1.4), against the running nested cosmic-comp
# (`dev/nested.sh`; this script reuses `WAYLAND_DISPLAY` from shell-host's `.nested-env` and
# never stops the compositor) with a real `sill run`: `crates/sill/src/daemon.rs` now spawns a
# `Wallpaper` per output (`spawn_missing_wallpapers`/`spawn_wallpaper`, mirroring the bar's own,
# wired next to `spawn_missing_bars`) and sends the wallpaper service a `Prepare` at the
# output's buffer size, so this script starts its own `sill run`.
#
# The COSMIC background config and sill's own settings live under a scratch XDG_CONFIG_HOME (never
# the real ~/.config), so there is nothing to back up or restore. A private dbus-daemon
# (dev/private-bus.conf) is the daemon's session AND system bus, refused if it would be the real
# one.
#
# Checks: a wallpaper surface exists per output and presents at least one frame; its capture's
# centre pixel matches the marked source image's own centre pixel (cover-fit keeps the crop
# centred, `geometry.rs`); five idle seconds add zero frames. Screenshot:
# `dev/out/wallpaper-cosmic.png` (the nested screen, `shot.sh`); the pixel check itself reads
# `sill debug capture`'s own file, not the screenshot.
#
# Usage: dev/nested.sh start && dev/accept-wallpaper.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-wallpaper"
mkdir -p "$OUT_DIR" "$SILL_REPO/dev/out"

# shellcheck disable=SC1091
source "$SHELL_HOST_DEV/.nested-env"
export WAYLAND_DISPLAY

# A scratch XDG_CONFIG_HOME: the COSMIC background config and sill's own settings below are written
# under it, never under the real ~/.config. A private dbus-daemon (dev/private-bus.conf) stands in
# as both the daemon's session and system bus, refused if it would be the real one.
SCRATCH=$(mktemp -d)
export XDG_CONFIG_HOME="$SCRATCH/config"
mkdir -p "$XDG_CONFIG_HOME"
BUS=
BUS_PID=

sill() { DBUS_SESSION_BUS_ADDRESS="$BUS" DBUS_SYSTEM_BUS_ADDRESS="$BUS" "$SHELL_HOST_DEV/run.sh" "$SILL_BIN" "$@"; }

# --- A marked source image and a COSMIC background config pointing at it (all under the ------
# --- scratch XDG_CONFIG_HOME above, so nothing here is restored: it never touched the real one)

CONFIG_DIR="$XDG_CONFIG_HOME/cosmic/com.system76.CosmicBackground/v1"
CONFIG_FILE="$CONFIG_DIR/all"
SOURCE_PNG="$OUT_DIR/source.png"
SILL_PID=""
# The shell draws the background only when `spaces.wallpaper_drawer` is "shell" (cosmic-bg by
# default), so this script sets it in sill's (scratch) settings file for the run.
SETTINGS_FILE="$XDG_CONFIG_HOME/sill/settings.toml"

cleanup() {
  # Stop the daemon this script started (never the compositor), the private bus, and the scratch
  # config dir.
  if [ -n "$SILL_PID" ] && kill -0 "$SILL_PID" 2>/dev/null; then
    sill quit >/dev/null 2>&1 || true
    for _ in $(seq 20); do kill -0 "$SILL_PID" 2>/dev/null || break; sleep 0.1; done
    kill -0 "$SILL_PID" 2>/dev/null && kill "$SILL_PID" 2>/dev/null || true
  fi
  stop_private_bus
  rm -rf "$SCRATCH"
}
trap cleanup EXIT

mkdir -p "$CONFIG_DIR" "$(dirname "$SETTINGS_FILE")"
python3 - "$SETTINGS_FILE" <<'PY'
# Set spaces.wallpaper_drawer = "shell", keeping every other line of the file.
import re
import sys

path = sys.argv[1]
try:
    text = open(path).read()
except FileNotFoundError:
    text = ""
lines = [l for l in text.splitlines() if not re.match(r"\s*wallpaper_drawer\s*=", l)]
if "[spaces]" in (l.strip() for l in lines):
    i = next(i for i, l in enumerate(lines) if l.strip() == "[spaces]")
    lines.insert(i + 1, 'wallpaper_drawer = "shell"')
else:
    lines += ["", "[spaces]", 'wallpaper_drawer = "shell"']
open(path, "w").write("\n".join(lines) + "\n")
PY

python3 - "$SOURCE_PNG" <<'PY'
# A 1600 x 1000 source: a big solid-colour block centred in a contrasting field, well clear of
# every edge, so Lanczos3's own ringing near the resize never reaches the centre pixel this
# script checks (the same shape `resize.rs`'s own round-trip test uses, at a size close to a
# real output's).
import sys
from PIL import Image

path = sys.argv[1]
w, h = 1600, 1000
im = Image.new("RGB", (w, h), (10, 40, 140))
box_w, box_h = w // 3, h // 3
x0, y0 = (w - box_w) // 2, (h - box_h) // 2
for x in range(x0, x0 + box_w):
    for y in range(y0, y0 + box_h):
        im.putpixel((x, y), (220, 40, 20))
im.save(path)
PY

cat >"$CONFIG_FILE" <<RON
(
    output: "all",
    source: Path("$SOURCE_PNG"),
    filter_by_theme: false,
    rotation_frequency: 3600,
    filter_method: Lanczos,
    scaling_mode: Zoom,
    sampling_method: Alphanumeric,
)
RON

# --- Start the daemon ---------------------------------------------------------------------

(cd "$SILL_REPO" && cargo build -p sill --features debug >&2) || exit 1
start_private_bus || exit 1
use_fake_ddc "$SCRATCH" >/dev/null
require_fake_ddc || exit 1
DBUS_SESSION_BUS_ADDRESS="$BUS" DBUS_SYSTEM_BUS_ADDRESS="$BUS" \
  "$SHELL_HOST_DEV/run.sh" "$SILL_BIN" run >"$OUT_DIR/sill-run.log" 2>&1 &
SILL_PID=$!

step_ping() {
  for _ in $(seq 100); do
    sill ping >"$OUT_DIR/ping.log" 2>&1 && return 0
    kill -0 "$SILL_PID" 2>/dev/null || { echo "sill run exited early (see $OUT_DIR/sill-run.log)" >&2; return 1; }
    sleep 0.1
  done
  echo "sill never answered ping" >&2
  return 1
}

# Every wallpaper surface's frame/commit counters, as `name frames commits` lines.
wallpaper_counters() {
  sill debug stats | python3 -c '
import json, sys
reply = json.loads(sys.stdin.read())
for row in reply["v"]:
    if row["surface"].startswith("sill-wallpaper-"):
        print(row["surface"], row["frames"], row["commits"])'
}

step_surfaces_exist() {
  local rows=""
  for _ in $(seq 100); do
    rows=$(wallpaper_counters)
    [ -n "$rows" ] && break
    sleep 0.1
  done
  echo "$rows" | tee "$OUT_DIR/surfaces.log" >&2
  [ -n "$rows" ]
}

# The capture's centre pixel against the source's. The service resizes the source after the
# surface spawns (a debug build takes a second or two), and the surface shows `--paper` until
# the image lands and cross-fades in, so this polls for up to ten seconds; the idle step then
# measures a settled surface rather than the cross-fade.
step_centre_pixel() {
  local name cap
  name=$(wallpaper_counters | awk 'NR==1 {print $1}')
  [ -n "$name" ] || { echo "no wallpaper surface to capture" >&2; return 1; }
  cap="$OUT_DIR/wallpaper-surface.png"
  for _ in $(seq 20); do
    capture_matches "$name" "$cap" && return 0
    sleep 0.5
  done
  return 1
}

capture_matches() {
  local name=$1 cap=$2
  sill debug capture "$name" "$cap" >"$OUT_DIR/capture.log" 2>&1 || return 1
  grep -q '"kind":"done"' "$OUT_DIR/capture.log" || { echo "capture did not report done" >&2; return 1; }
  python3 - "$SOURCE_PNG" "$cap" <<'PY'
import sys
from PIL import Image

def centre(path):
    im = Image.open(path).convert("RGB")
    w, h = im.size
    return im.getpixel((w // 2, h // 2))

source, captured = centre(sys.argv[1]), centre(sys.argv[2])
print(f"source centre {source}, captured centre {captured}")
sys.exit(0 if source == captured else 1)
PY
}

step_idle() {
  local before after
  sleep 1
  before=$(wallpaper_counters) || return 1
  sleep 5
  after=$(wallpaper_counters) || return 1
  printf 'before:\n%s\nafter:\n%s\n' "$before" "$after" | tee "$OUT_DIR/idle.log" >&2
  "$SHELL_HOST_DEV/shot.sh" "$SILL_REPO/dev/out/wallpaper-cosmic.png" >&2 || true
  [ "$before" = "$after" ]
}

status=0
for step in step_ping step_surfaces_exist step_centre_pixel step_idle; do
  if "$step"; then
    echo "PASS $step"
  else
    rc=$?
    echo "FAIL $step (exit $rc)"
    status=$(( rc > status ? rc : status ))
  fi
done
echo "output dir: $OUT_DIR; screenshot: $SILL_REPO/dev/out/wallpaper-cosmic.png" >&2
exit "$status"

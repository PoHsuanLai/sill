#!/usr/bin/env bash
# M2 wallpaper acceptance (design/20 §1.4), against the running nested cosmic-comp
# (`dev/nested.sh`; this script reuses `WAYLAND_DISPLAY` from shell-host's `.nested-env` and
# never stops the compositor).
#
# `sill run`'s own wallpaper surface: `crates/sill/src/daemon.rs` only ever spawns a bar per
# output today (`spawn_missing_bars`, M1 daemon). Wiring `sill_services::wallpaper::spawn`'s
# output into the daemon's per-output surface set (`outputs.rs`'s `OutputSurfaces.wallpaper`
# field already anticipates it) is outside this wave's ownership — `daemon.rs` is frozen to
# everyone but the "daemon" wave (DESIGN.md "Wave ownership"). Step 1 below still runs against a
# live `sill run` when one is up and reports what `sill debug stats` shows, so the gap stays
# visible on every run rather than only in FINDINGS.md; it is recorded, not asserted, so it
# never fails this script by itself. The real acceptance (steps 2-4) runs through
# `examples/wallpaper_demo.rs`, this wave's own harness with a real `wallpaper::spawn` behind
# fake everything else — the same shape `dev/accept-bar.sh`'s menus step takes through
# `accept-bar-ui.sh cosmic` for a daemon/host gap of its own (FINDINGS G8).
#
# Checks: the wallpaper surface exists per output; a capture's centre pixel matches the marked
# source image's own centre pixel (cover-fit keeps the crop centred, `geometry.rs`); five idle
# seconds add zero frames. Screenshot: `dev/out/wallpaper-cosmic.png`.
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

sill() { "$SHELL_HOST_DEV/run.sh" "$SILL_BIN" "$@"; }
nested() { "$SHELL_HOST_DEV/run.sh" "$@"; }

# --- A marked source image and a COSMIC background config pointing at it, restored after -----

CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic/com.system76.CosmicBackground/v1"
CONFIG_FILE="$CONFIG_DIR/all"
BACKUP_FILE="$OUT_DIR/all.backup"
SOURCE_PNG="$OUT_DIR/source.png"
CREATED_MARKER="$OUT_DIR/all.accept-wallpaper-created"

restore_config() {
  if [ -f "$BACKUP_FILE" ]; then
    mkdir -p "$CONFIG_DIR"
    cp "$BACKUP_FILE" "$CONFIG_FILE"
  elif [ -f "$CREATED_MARKER" ]; then
    rm -f "$CONFIG_FILE"
  fi
}
trap restore_config EXIT

mkdir -p "$CONFIG_DIR"
rm -f "$BACKUP_FILE" "$CREATED_MARKER"
if [ -f "$CONFIG_FILE" ]; then
  cp "$CONFIG_FILE" "$BACKUP_FILE"
else
  : >"$CREATED_MARKER"
fi

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

# --- 1. What a live `sill run` shows today (recorded, not asserted; see the header) ----------

step_sill_run_today() {
  if ! sill ping >"$OUT_DIR/sill-ping.log" 2>&1; then
    echo "no running 'sill run' daemon (start one with dev/run.sh to see this step); skipped" \
      | tee "$OUT_DIR/sill-run-wallpaper.log" >&2
    return 0
  fi
  sill debug stats >"$OUT_DIR/sill-run-wallpaper.log" 2>&1
  if grep -q '"surface":"sill-wallpaper-' "$OUT_DIR/sill-run-wallpaper.log"; then
    echo "sill run: a wallpaper surface is present" >>"$OUT_DIR/sill-run-wallpaper.log"
  else
    echo "sill run: no wallpaper surface yet (daemon.rs gap, see this script's header)" \
      >>"$OUT_DIR/sill-run-wallpaper.log"
  fi
  cat "$OUT_DIR/sill-run-wallpaper.log" >&2
}
step_sill_run_today

# --- 2-4. The real pipeline, through wallpaper_demo -------------------------------------------

(cd "$SILL_REPO" && cargo build --release -p sill-surfaces --example wallpaper_demo >&2) || exit 1
export WALLPAPER_DEMO="$SILL_REPO/target/release/examples/wallpaper_demo"
export OUT_DIR SOURCE_PNG
export PNG_OUT="$SILL_REPO/dev/out/wallpaper-cosmic.png"

python3 - <<'PY'
import json, os, queue, subprocess, sys, threading, time

OUT = os.environ["OUT_DIR"]
SOURCE_PNG = os.environ["SOURCE_PNG"]
PNG_OUT = os.environ["PNG_OUT"]
LOG = open(f"{OUT}/accept.log", "w")

def log(*a):
    line = " ".join(str(x) for x in a)
    print(line, file=sys.stderr); LOG.write(line + "\n"); LOG.flush()

class Proc:
    def __init__(self, path):
        self.name, self.lines = os.path.basename(path), queue.Queue()
        self.p = subprocess.Popen([path], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE, text=True)
        threading.Thread(target=self._read, args=(self.p.stdout, True), daemon=True).start()
        threading.Thread(target=self._read, args=(self.p.stderr, False), daemon=True).start()
        log(f"spawn {self.name} pid={self.p.pid}")
    def _read(self, f, is_out):
        for line in f:
            LOG.write(f"  [{self.name}{'' if is_out else ' err'}] {line}"); LOG.flush()
            if is_out: self.lines.put(line)
    def send(self, s):
        log(f"> {s}"); self.p.stdin.write(s + "\n"); self.p.stdin.flush()
    def expect(self, pred, timeout=10.0):
        end = time.time() + timeout
        while time.time() < end:
            try: line = self.lines.get(timeout=max(0.01, end - time.time()))
            except queue.Empty: break
            try: v = json.loads(line)
            except ValueError: continue
            if pred(v): return v
        raise TimeoutError(f"{self.name}: expected line not seen")
    def stats(self):
        """Every surface's counters, by name: one host line, then one line per surface."""
        self.send("stats")
        self.expect(lambda v: "host" in v)
        rows = {}
        end = time.time() + 1.0
        while time.time() < end:
            try: v = self.expect(lambda v: "counters" in v, timeout=0.3)
            except TimeoutError: break
            rows[v["surface"]] = v
        return rows
    def quit(self):
        if self.p.poll() is None:
            try: self.send("quit")
            except BrokenPipeError: pass
            try: self.p.wait(3)
            except subprocess.TimeoutExpired: self.p.kill()

def wallpapers_ready(p, tries=200):
    for _ in range(tries):
        rows = p.stats()
        walls = {k: r for k, r in rows.items() if k.startswith("sill-wallpaper-")}
        if walls and all(r["counters"]["frames"] >= 1 for r in walls.values()):
            return walls
        time.sleep(0.05)
    raise TimeoutError("no wallpaper surface ever presented a frame")

def centre_pixel(path):
    from PIL import Image
    im = Image.open(path).convert("RGB")
    w, h = im.size
    return im.getpixel((w // 2, h // 2))

result = {}
demo = Proc(os.environ["WALLPAPER_DEMO"])
try:
    walls = wallpapers_ready(demo)
    result["surfaces"] = dict(names=sorted(walls.keys()), count=len(walls),
                              pass_=len(walls) >= 1)
    log("wallpaper surfaces:", result["surfaces"])

    name = next(iter(walls))
    cap = f"{OUT}/wallpaper-surface.png"
    demo.send(f"{name} capture {cap}")
    demo.expect(lambda v: "capture" in v)
    source_centre = centre_pixel(SOURCE_PNG)
    captured_centre = centre_pixel(cap)
    result["centre_pixel"] = dict(source=source_centre, captured=captured_centre,
                                  pass_=source_centre == captured_centre)
    log("centre pixel: source", source_centre, "captured", captured_centre)

    # Idle: five quiet seconds add no frames, commits or callbacks.
    time.sleep(0.5)
    before = demo.stats()[name]["counters"]
    time.sleep(5.0)
    after = demo.stats()[name]["counters"]
    idle = {k: after[k] - before[k] for k in ("frames", "commits", "callbacks")}
    result["idle"] = dict(delta=idle, pass_=idle["frames"] == 0)
    log("idle delta:", idle)

    import shutil
    shutil.copyfile(cap, PNG_OUT)
    result["screenshot"] = PNG_OUT
finally:
    demo.quit()

json.dump(result, open(f"{OUT}/result.json", "w"), indent=1)
print(json.dumps(result, indent=1))
checks = [v for k, v in result.items() if isinstance(v, dict) and "pass_" in v]
sys.exit(0 if len(checks) == 3 and all(v["pass_"] for v in checks) else 1)
PY
status=$?
echo "output dir: $OUT_DIR; screenshot: $SILL_REPO/dev/out/wallpaper-cosmic.png" >&2
exit "$status"

#!/usr/bin/env bash
# Milestone 1 bar-ui acceptance, on a nested headless KWin (`kwin_wayland --virtual` on its own
# D-Bus session, torn down afterwards; works while the real session is locked), against
# `examples/bar_demo.rs` (the real `Bar` on fake services) and shell-host's `probe_toplevel`:
#
#   1. exclusive zone: a maximised probe window is the output height minus the bar height;
#   2. idle: five seconds with nothing changing, 0 frames and 0 commits (`stats`);
#   3. menus: a click on the control center's item (the status item left of the clock, found
#      from the bar's capture) opens the menu popup; a click outside it dismisses it. The clock
#      opens no menu since M6: it toggles the notification center, which the demo does not
#      host (the daemon's `dev/accept-bar.sh` checks that);
#   4. the popup is transparent around its card: its corners, the middle of its top edge and a
#      point beside the card's top-right corner have alpha <= 16 in its capture (the control
#      center hangs 8 px under the popup's top edge, so the middle of that edge is in the reach
#      of its drop shadow, alpha about 10; an opaque ground would be 255);
#   5. a screenshot with the control center open, dev/out/bar.png (reviewed by eye).
#
# The fuller `dev/accept-bar.sh` (daemon-owned) runs the same steps through `sill` once the
# daemon lands. Logs and the result JSON: target/accept-bar-ui/. Exit 0 only when 1-4 pass.
#
# Usage: dev/accept-bar-ui.sh            (nested KWin: every step; screenshot dev/out/bar.png)
#        dev/accept-bar-ui.sh cosmic     (the running nested cosmic-comp, dev/nested.sh: steps 3-4
#                                         only, since `sill run` may already reserve a bar there;
#                                         screenshot dev/out/bar-cosmic-menu.png). `dev/accept-bar.sh`
#                                         calls this for its menus step: sill's debug IPC has no
#                                         click (FINDINGS "M1 bar-ui").
set -euo pipefail
mode="${1:-kwin}"
self="$(realpath "$0")"
repo="$(cd "$(dirname "$self")/.." && pwd)"
# shellcheck source=dev/common.sh
source "$repo/dev/common.sh"
host_repo="$(cd "$SHELL_HOST_DEV/../.." && pwd)"
out="$repo/target/accept-bar-ui"
mkdir -p "$out" "$repo/dev/out"

build() {
  (cd "$repo" && cargo build --release -p sill-surfaces --example bar_demo >&2)
  (cd "$host_repo" && cargo build --release -p shell-host --features debug-inject --example probe_toplevel >&2)
}
export BAR_DEMO="$repo/target/release/examples/bar_demo"
export PROBE="$host_repo/target/release/examples/probe_toplevel"
if [[ "$mode" == cosmic ]]; then
  build
  # shellcheck disable=SC1091
  source "$SHELL_HOST_DEV/.nested-env"
  export WAYLAND_DISPLAY ACCEPT_OUT="$out" ACCEPT_PNG="$repo/dev/out/bar-cosmic-menu.png"
  export ACCEPT_MODE=cosmic
elif [[ -z "${ACCEPT_NESTED:-}" ]]; then
  build
  rc=0
  ACCEPT_NESTED=1 dbus-run-session -- "$self" </dev/null || rc=$?
  # Daemons D-Bus-activated on the nested bus outlive dbus-run-session; stop them.
  sleep 1
  bus=$(cat "$out/.nested-bus" 2>/dev/null || true)
  if [ -n "$bus" ]; then
    for p in $(pgrep -u "$USER"); do
      if { tr '\0' '\n' <"/proc/$p/environ"; } 2>/dev/null | grep -qxF "DBUS_SESSION_BUS_ADDRESS=$bus"; then
        kill "$p" 2>/dev/null || true
      fi
    done
  fi
  exit "$rc"
else
echo "$DBUS_SESSION_BUS_ADDRESS" >"$out/.nested-bus"
sock="wayland-bar-ui-$$"
kwin_wayland --virtual --no-lockscreen --no-global-shortcuts --width 1920 --height 1080 \
  --socket "$sock" >"$out/kwin-virtual.log" 2>&1 &
kpid=$!
trap 'kill "$kpid" 2>/dev/null || true; wait "$kpid" 2>/dev/null || true' EXIT
for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$sock" ] && break; sleep 0.1; done
sleep 1.5
export WAYLAND_DISPLAY="$sock"
export ACCEPT_OUT="$out" ACCEPT_PNG="$repo/dev/out/bar.png" ACCEPT_MODE=kwin
fi
python3 - <<'PY'
import json, os, queue, re, subprocess, sys, threading, time

OUT, PNG, MODE = os.environ["ACCEPT_OUT"], os.environ["ACCEPT_PNG"], os.environ["ACCEPT_MODE"]
LOG = open(f"{OUT}/accept-{MODE}.log", "w")
PROCS = []

def log(*a):
    line = " ".join(str(x) for x in a)
    print(line, file=sys.stderr); LOG.write(line + "\n"); LOG.flush()

class Proc:
    def __init__(self, path, args=()):
        self.name, self.lines = os.path.basename(path), queue.Queue()
        self.p = subprocess.Popen([path, *args], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE, text=True)
        PROCS.append(self)
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

def output_info():
    txt = subprocess.run(["wayland-info"], capture_output=True, text=True).stdout
    lw, lh = map(int, re.search(r"logical_width: (\d+), logical_height: (\d+)", txt).groups())
    return dict(logical=[lw, lh])

def probe_maximized():
    p = Proc(os.environ["PROBE"])
    v = p.expect(lambda v: v.get("event") == "configure" and v["sized"] and (v["maximized"] or v["fullscreen"]))
    return p, v

def bar_ready(p):
    for _ in range(200):
        rows = p.stats()
        bars = {k: r for k, r in rows.items() if k.startswith("sill-bar-") and "/" not in k}
        if bars and all(r["counters"]["frames"] >= 1 for r in bars.values()):
            return next(iter(bars)), bars
        time.sleep(0.05)
    raise TimeoutError("the bar never presented")

def corner_alphas(path):
    """The popup capture's alpha at its four corners and at the middle of its top edge, above
    the card: all outside the card and the reach of its shadow."""
    from PIL import Image
    im = Image.open(path).convert("RGBA"); w, h = im.size
    points = ((0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1), (w // 2, 1), (w - 4, h // 4))
    return [im.getpixel((x, y))[3] for x, y in points]

def status_items(path):
    """The status items' centres, right to left, from a capture of the bar: runs of inked
    columns in its right half (the clock's is first), as dev/accept-real-input.sh reads them."""
    from PIL import Image
    img = Image.open(path).convert("RGB"); w, h = img.size
    ground = img.getpixel((w // 2, h // 2))
    def inked(x):
        return any(sum(abs(a - b) for a, b in zip(img.getpixel((x, y)), ground)) > 90
                   for y in range(4, h - 4))
    runs, run, gap = [], None, 0
    for x in range(w - 1, w // 2, -1):
        if inked(x):
            run = [x, x] if run is None else [x, run[1]]; gap = 0
        elif run is not None:
            gap += 1
            if gap > 7: runs.append(run); run = None
    return [(left + right) // 2 for left, right in runs]

def screenshot(path):
    if os.path.exists(path): os.remove(path)
    if MODE == "cosmic":
        with open(path, "wb") as f:
            subprocess.run(["grim", "-"], check=True, timeout=20, stdout=f)
    else:
        subprocess.run(["spectacle", "-b", "-n", "-f", "-o", path], check=True, timeout=20,
                       capture_output=True)
    for _ in range(100):
        if os.path.exists(path) and os.path.getsize(path) > 0: break
        time.sleep(0.05)

result = {}
try:
    if MODE == "kwin":
        info = output_info()
        base, v0 = probe_maximized(); time.sleep(0.3); base.quit()
    bar = Proc(os.environ["BAR_DEMO"])
    name, rows = bar_ready(bar)
    height = rows[name]["logical"]["height"]
    width = rows[name]["logical"]["width"]
    time.sleep(0.5)
    if MODE == "kwin":
        probe, v1 = probe_maximized(); time.sleep(1.0); probe.quit(); time.sleep(0.5)
        result["exclusive_zone"] = dict(output=info, baseline=v0["height"], with_bar=v1["height"],
                                        bar_height=height,
                                        pass_=v0["height"] - v1["height"] == height and v1["height"] == info["logical"][1] - height)
        # 2. Idle: let the probe's exit and the rect probes settle, then five quiet seconds.
        time.sleep(1.5)
        a = bar.stats()[name]["counters"]; time.sleep(5.0); b = bar.stats()[name]["counters"]
        idle = {k: b[k] - a[k] for k in ("frames", "commits", "callbacks")}
        result["idle"] = dict(delta=idle, pass_=idle["frames"] == 0 and idle["commits"] == 0)
    else:
        time.sleep(1.0)
    # 3. Menus: the control center's item, left of the clock; press and release on it.
    popup = f"{name}/popup"
    cap = f"{OUT}/bar-surface.png"
    bar.send(f"{name} capture {cap}"); bar.expect(lambda v: "capture" in v)
    items = status_items(cap)
    log(f"status items right to left: {items}")
    if len(items) < 2: raise RuntimeError(f"found {len(items)} status items")
    bar.send(f"{name} click {items[1]} {height // 2}")
    opened = False
    for _ in range(40):
        time.sleep(0.05)
        rows = bar.stats()
        if popup in rows and rows[popup]["counters"]["frames"] >= 1: opened = True; break
    popup_stats = rows.get(popup)
    time.sleep(0.8)
    screenshot(PNG)
    corners = None
    if opened:
        bar.send(f"{popup} capture {OUT}/popup-surface.png"); bar.expect(lambda v: "capture" in v)
        corners = corner_alphas(f"{OUT}/popup-surface.png")
    # An outside click: the empty middle of the bar (a compositor-level outside press cannot be
    # synthesized, shell-host F17; the bar treats a press on itself outside the titles the same).
    bar.send(f"{name} click {width // 2} {height // 2}")
    closed = False
    for _ in range(40):
        time.sleep(0.05)
        if popup not in bar.stats(): closed = True; break
    result["menus"] = dict(opened=opened, dismissed=closed, popup=popup_stats and dict(
        logical=popup_stats["logical"], frames=popup_stats["counters"]["frames"]),
        corner_alphas=corners, pass_=opened and closed)
    # The popup is transparent around the menu card and its shadow: its four corners.
    result["popup_transparent"] = dict(corner_alphas=corners,
                                       pass_=corners is not None and max(corners) <= 16)
    result["screenshot"] = PNG
    bar.quit()
finally:
    for p in PROCS: p.quit()
json.dump(result, open(f"{OUT}/result-{MODE}.json", "w"), indent=1)
print(json.dumps(result, indent=1))
checks = [v for v in result.values() if isinstance(v, dict)]
want = 4 if MODE == "kwin" else 2
sys.exit(0 if len(checks) >= want and all(v.get("pass_") for v in checks) else 1)
PY

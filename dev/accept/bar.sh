#!/usr/bin/env bash
# The bar's acceptance. Every daemon here runs from this checkout's debug build (`--features
# debug`) on a private bus that is both its session and its system bus (nothing activatable, so
# NetworkManager, BlueZ, UPower and the rest are simply absent), with a scratch HOME and XDG tree
# of its own, its own XDG_RUNTIME_DIR and the fake DDC; nothing reaches the user's session.
#
# Default: the daemon's basics and the bar's own, on the nested cosmic-comp (dev/nested.sh; a
# running one is reused and never stopped), then the bar demo's steps on a headless KWin:
#   Daemon (the nested compositor):
#     1. ping: the daemon answers.
#     2. single instance: a second `sill run` exits 0 fast without touching Wayland.
#     3. reload: `sill reload` answers Done.
#     4. IPC latency: a ping answers well inside the 2 s reply timeout.
#     5. exclusive zone: a maximised probe window is output_h - bar_h (less the dock's 68 when
#        the daemon runs one).
#     6. idle: 5 s with the pointer still, the bar's commit counter does not move; shoots
#        dev/out/bar-cosmic.png over the test wallpaper.
#     7. menus: a press on the control center's item opens its menu popup, an outside press
#        dismisses it (`sill debug click`); shoots dev/out/bar-cosmic-menu.png.
#     8. clock: a click toggles the notification center and opens no bar menu, a second click
#        closes it.
#     9. quit: `sill quit` ends the daemon and a `ping` after it finds nobody home. Runs last.
#   Bar demo (a headless KWin, `examples/bar_demo.rs`: the real `Bar` on fake services, with
#   shell-host's `probe_toplevel`; works while the real session is locked):
#     1. exclusive zone: a maximised probe window is the output height minus the bar height;
#     2. idle: five seconds with nothing changing, 0 frames and 0 commits (`stats`);
#     3. menus: a click on the control center's item (the status item left of the clock, found
#        from the bar's capture) opens the menu popup; a click outside it dismisses it. The clock
#        opens no menu: it toggles the notification center, which the demo does not host (the
#        daemon's clock step checks that);
#     4. the popup is transparent around its card: its corners, the middle of its top edge and a
#        point beside the card's top-right corner have alpha <= 16 in its capture (the control
#        center hangs 8 px under the popup's top edge, so the middle of that edge is in the reach
#        of its drop shadow, alpha about 10; an opaque ground would be 255);
#     5. a screenshot with the control center open, dev/out/bar.png (reviewed by eye).
#   The demo's logs and result JSON: target/accept-bar/. It passes only when 1-4 pass.
#
# --ui: only the bar demo's steps, on the headless KWin (no nested compositor needed).
# --ui-nested: the bar demo's steps 3-4 on the running nested cosmic-comp (`sill run` may already
#   reserve a bar there, so no exclusive-zone or idle step); screenshot dev/out/bar-cosmic-menu.png.
#
# --real-input [STEP...]: the bar's menus and the keyboard of the control center and the
# notification center under REAL input, on shell-host's private headless KWin (scripts/dev/
# kwin-virtual.sh) driven by shell-host's scripts/dev/fake-input.sh. Every menu and panel here is
# opened by a real pointer click on its bar item, never by `sill control-center toggle` or
# `sill notifications open`; every key is a real Wayland key event. `sill debug capture` and
# `sill debug notify` only look and seed. As on the user's session, an app window has the
# keyboard first: dev/key-sink.py, a plain GTK 4 window clicked into focus, which logs every key
# it is sent. A shell panel opened from the bar must take the keyboard from it; a key the sink
# logs went to the app instead. ONE fake-input client serves the whole run.
#   0. A real click on the sink gives it the keyboard; a real "k" reaches it.
#   1. A real click on bar item A (Wi-Fi), then one on item B (Sound): A's menu closes and B's
#      opens (B's pill on, A's off; the popup shows what B alone shows).
#   2. Hover switch: with B's menu open, the real pointer onto A switches the popup to A's menu;
#      a real click on A keeps it (the click the person meant), a second one closes it.
#   3. A real click on the control center's item opens it with the keyboard: real Tab moves the
#      focus ring (each Tab changes the panel, a Shift+Tab goes back), real Space toggles the
#      focused tile (Focus, first in `control_center.modules`: `notifications.dnd` turns on, and off
#      again on a second Space; the tile changes on its own, no `sill reload` needed), real Escape
#      closes the panel. The sink gets none of these keys.
#   4. Two notifications seeded; a real click on the clock opens the notification center with
#      the keyboard: real Down moves the focus ring onto a row, real Escape closes the center.
#      The sink gets none of these keys.
#   5. The notification center open, a real click on A closes it and opens A's menu (one
#      menu-bar panel at a time, as on macOS).
#   6. A's menu open, a real click on the clock closes the menu and opens the center, which has
#      the keyboard: real Down moves its ring, real Escape closes it; the sink gets no key.
#   Keyboard back. After a bar menu closes, does the app have the keyboard back (a real "j"
#      reaches the sink)? Recorded, not asserted: shell-host's side. Only on a full run.
# Captures: target/accept-bar/*.png. Then the input client, the sink, the daemon (`sill quit`) and
# the headless KWin are stopped. Steps 0-6 must pass; the keyboard-back line never changes the
# exit status.
#
# Only ONE headless KWin can exist at a time. SHELL_HOST_DEV may name a private copy of
# shell-host's scripts/dev (with tools/ beside its scripts/): kwin-virtual.sh keeps one .kwin-env
# per scripts/dev, so a copy runs beside another agent's headless KWin.
#
# Exit 0 only when every step passed, 1 on a failure.
#
# Usage: dev/nested.sh && dev/accept/bar.sh     (daemon steps, then the bar demo)
#        dev/accept/bar.sh --ui | --ui-nested
#        dev/accept/bar.sh --real-input [STEP...]   (default: every step, in order)
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_REPO/target/accept-bar"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
# The bar's height: `bar.height_px`, 24 unless the settings say otherwise.
BAR_H="${BAR_H:-24}"
BAR_DEMO="$SILL_REPO/target/release/examples/bar_demo"
SINK=
NAME=
STATUS=0

# The status items' points "x y", right to left (the clock's first), of the bar capture $1: runs
# of inked columns in its right half, y the middle of the bar.
inked_items() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
w, h = img.size
ground = img.getpixel((w // 2, h // 2))
def inked(x):
    return any(sum(abs(a - b) for a, b in zip(img.getpixel((x, y)), ground)) > 90
               for y in range(4, h - 4))
runs, run, gap = [], None, 0
for x in range(w - 1, w // 2, -1):
    if inked(x):
        run = [x, x] if run is None else [x, run[1]]
        gap = 0
    elif run is not None:
        gap += 1
        if gap > 7:
            runs.append(run); run = None
for left, right in runs:
    print((left + right) // 2, h // 2)
EOF
}

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

start_bar_daemon() {
  backdrop_seed "$CONFIG" || return 1
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  sleep 1
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
  if ! sill run "${FAKE_DDC_FLAGS[@]}" >"$OUT_DIR/single-instance.log" 2>&1; then
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

# 4. A request in the ordinary case answers well inside the 2 s reply timeout (`ipc::server`'s
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

# 5. A maximised foot window's height is output_h - bar_h (design/13 §13.3.1, design/29: Reserve(24)).
#    shell-host's `probe_toplevel` stands in for foot: it asks to be maximised and prints each
#    configure as JSON.
step_exclusive_zone() {
  local out_h height dock_reserve=0
  out_h=$(nested_output_size | awk '{print $2}')
  [ -n "$out_h" ] || { echo "no output height from wayland-info" >&2; return 1; }
  height=$(maximised_height) || return 1
  # The dock reserves its base + margin too when the daemon runs one on this output: gut 8
  # + tile 48 + 2 x the 6 px padding.
  if sill debug stats 2>/dev/null | grep -q '"sill-dock-'; then dock_reserve=68; fi
  echo "output ${out_h}, maximised window ${height}, bar ${BAR_H}, dock ${dock_reserve}" | tee -a "$OUT_DIR/exclusive-zone.log" >&2
  [ "$height" = "$(( out_h - BAR_H - dock_reserve ))" ]
}

# 6. Idle: 5 s with the pointer still, the bar's commit counter does not move.
#    Also shoots the nested screen to dev/out/bar-cosmic.png, over the test wallpaper
#    (dev/lib/backdrop.sh: the daemon's own when it draws one, else cosmic-bg's).
step_idle() {
  local before after
  sleep 1
  # The clock repaints on the minute; keep the 5 s window clear of a minute boundary.
  while [ "$(date +%S | sed 's/^0//')" -gt 50 ]; do sleep 1; done
  before=$(bar_counters) || return 1
  [ -n "$before" ] || { echo "no bar surface in 'sill debug stats'" >&2; return 1; }
  sleep 5
  after=$(bar_counters) || return 1
  printf 'before:\n%s\nafter:\n%s\n' "$before" "$after" | tee "$OUT_DIR/idle.log" >&2
  "$SHELL_HOST_DEV/shot.sh" "$SILL_REPO/dev/out/bar-cosmic.png" >&2 || true
  [ "$before" = "$after" ]
}

# The status items' centres, right to left, from a capture of bar $1 (the clock's is first).
bar_status_items() {
  sill debug capture "$1" "$OUT_DIR/bar.png" >/dev/null 2>&1 || return 1
  inked_items "$OUT_DIR/bar.png" | awk '{print $1}'
}

# 7. Menus: a press on a title opens its menu; an outside press dismisses it (design/13 §13.8 tests
#    1 and 3), on the daemon's own bar through `sill debug click`. The title is the control center's
#    item, the status item left of the clock (the clock opens no menu: it toggles the notification
#    center, step 8); the bar's empty middle stands in for an outside press (a compositor-level
#    outside press cannot be synthesized). Whether the popup is up is whether `sill debug capture
#    <bar>/popup` succeeds: `debug stats` lists bars only. Shoots the nested screen with the menu
#    open to dev/out/bar-cosmic-menu.png.
step_menus() {
  local log="$OUT_DIR/menus.log" bar width y popup opened=no closed=no items
  : >"$log"
  bar=$(bar_counters | awk 'NR==1 {print $1}')
  [ -n "$bar" ] || { echo "no bar surface in 'sill debug stats'" >&2; return 1; }
  width=$(nested_output_size | awk '{print $1}')
  [ -n "$width" ] || { echo "no output width from wayland-info" >&2; return 1; }
  y=$(( BAR_H / 2 ))
  popup="$bar/popup"
  popup_up() { sill debug capture "$popup" "$OUT_DIR/popup.png" >>"$log" 2>&1; }
  if popup_up; then
    echo "a popup was already open before the click" | tee -a "$log" >&2
    return 1
  fi
  mapfile -t items < <(bar_status_items "$bar")
  [ "${#items[@]}" -ge 2 ] || { echo "found ${#items[@]} status items" | tee -a "$log" >&2; return 1; }
  echo "click the control center's item at ${items[1]},$y on $bar" >>"$log"
  sill debug click "$bar" "${items[1]}" "$y" >>"$log" 2>&1 || return 1
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

# 8. The clock: a click toggles the notification center on its output
#    and opens no bar menu; a second click closes the center. Whether the center is up is
#    whether `sill debug capture sill-notification-center-<out>` succeeds.
step_clock_center() {
  local log="$OUT_DIR/clock.log" bar center y items opened=no closed=no menu=no
  : >"$log"
  bar=$(bar_counters | awk 'NR==1 {print $1}')
  [ -n "$bar" ] || { echo "no bar surface in 'sill debug stats'" >&2; return 1; }
  center="sill-notification-center-${bar#sill-bar-}"
  y=$(( BAR_H / 2 ))
  center_up() { sill debug capture "$center" "$OUT_DIR/center.png" >>"$log" 2>&1; }
  if center_up; then echo "the center was open before the click" | tee -a "$log" >&2; return 1; fi
  mapfile -t items < <(bar_status_items "$bar")
  [ "${#items[@]}" -ge 1 ] || { echo "no clock found on the bar" | tee -a "$log" >&2; return 1; }
  echo "click the clock at ${items[0]},$y on $bar" >>"$log"
  sill debug click "$bar" "${items[0]}" "$y" >>"$log" 2>&1 || return 1
  for _ in $(seq 40); do
    center_up && { opened=yes; break; }
    sleep 0.05
  done
  sill debug capture "$bar/popup" "$OUT_DIR/popup.png" >>"$log" 2>&1 && menu=yes
  sleep 0.5
  sill debug click "$bar" "${items[0]}" "$y" >>"$log" 2>&1 || return 1
  for _ in $(seq 40); do
    center_up || { closed=yes; break; }
    sleep 0.05
  done
  echo "the clock opened the center: $opened, a bar menu: $menu, a second click closed it: $closed" \
    | tee -a "$log" >&2
  [ "$opened" = yes ] && [ "$menu" = no ] && [ "$closed" = yes ]
}

# 9. Quit closes every surface and exits; a `ping` after it finds nobody home. Runs last: it
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


# The daemon steps, on the nested cosmic-comp. Sets STATUS.
run_daemon_steps() {
  nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; return 1; }
  use_nested_socket || return 1
  await_no_process 'target/[a-z]*/sill run' || return 1
  scratch_new
  start_private_bus || { scratch_drop; return 1; }
  start_bar_daemon || { stop_daemon; stop_private_bus; scratch_drop; return 1; }
  backdrop_up "$CONFIG" || echo "backdrop: none (the shots show the bare compositor)" >&2
  run_steps step_ping step_single_instance step_reload step_ipc_latency step_exclusive_zone \
    step_idle step_menus step_clock_center step_quit
  backdrop_down
  stop_daemon || STATUS=1
  stop_private_bus
  scratch_drop
}

# Runs the bar demo's checks in the world of the daemon (in_env: $WL, the private bus, the scratch
# tree): $1 is `kwin` or `cosmic` (which compositor takes the screenshot and which steps run), $2
# the screenshot's path. Exit 0 only when every check passed.
run_bar_demo() {
  in_env env ACCEPT_OUT="$OUT_DIR" ACCEPT_PNG="$2" ACCEPT_MODE="$1" BAR_DEMO="$BAR_DEMO" \
    PROBE="$PROBE" python3 - <<'PY'
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
    columns in its right half (the clock's is first), as the daemon steps read them."""
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
    # synthesized; the bar treats a press on itself outside the titles the same).
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
}

step_demo_kwin() { run_bar_demo kwin "$SHOTS/bar.png"; }
step_demo_nested() { run_bar_demo cosmic "$SHOTS/bar-cosmic-menu.png"; }

# The demo and shell-host's probe, built (a no-op when up to date).
build_demo() {
  build "-p sill-surfaces --example bar_demo --release" || return 1
  (cd "$HOST_REPO" && cargo build --release -p shell-host --features debug-inject \
    --example probe_toplevel) >>"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

# The bar demo on the headless KWin. Sets STATUS.
run_demo_kwin() {
  scratch_new
  start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; scratch_drop; return 1; }
  run_steps step_demo_kwin
  stop_kwin
  scratch_drop
}

# The bar demo on the nested cosmic-comp. Sets STATUS.
run_demo_nested() {
  nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; return 1; }
  use_nested_socket || return 1
  scratch_new
  start_private_bus || { scratch_drop; return 1; }
  run_steps step_demo_nested
  stop_private_bus
  scratch_drop
}

# ---- --real-input

# Real keys, one after another: each pressed, held 30 ms, released, then 250 ms to settle.
press_keys() { local k; for k in "$@"; do input "key $k down" "sleep 30" "key $k up" "sleep 250" || return 1; done; }
chord() {
  input "key $1 down" "sleep 30" "key $2 down" "sleep 30" "key $2 up" "sleep 30" "key $1 up" \
    "sleep 250"
}
move_to() { input "move $1 $2" "sleep 80"; }
click_at() { move_to "$1" "$2" && input "press left" "sleep 40" "release left" "sleep 300"; }


# The key sink: a GTK 4 window on KWin's socket and bus that logs the keys it gets.
start_sink() {
  in_env GDK_BACKEND=wayland python3 "$SILL_REPO/dev/key-sink.py" >"$OUT_DIR/sink.log" 2>&1 &
  SINK=$!
  for _ in $(seq 100); do grep -q ' ready$' "$OUT_DIR/sink.log" && return 0; sleep 0.1; done
  echo "the key sink never started" >&2
  return 1
}

stop_sink() {
  [ -n "$SINK" ] || return 0
  kill "$SINK" 2>/dev/null
  reap "$SINK" sink
  SINK=
}

sink_keys() { grep -c ' key ' "$OUT_DIR/sink.log"; }

# The sink's frame "x y w h" from KWin's scripting API.
sink_frame() {
  "$SHELL_HOST_DEV/kwin-windows.sh" 2>/dev/null | python3 -c '
import json, sys
for line in sys.stdin:
    w = json.loads(line)
    if w.get("caption") == "sill key sink":
        print(*w["frame"]); break'
}

# Starts the daemon with the settings the steps need and finds the bar, its popup and the center.
start_real_daemon() {
  mkdir -p "$CONFIG/sill"
  cat >"$CONFIG/sill/settings.toml" <<'EOF'
[control_center]
modules = ["focus", "appearance", "wifi", "bluetooth", "sound"]
menu_bar_wifi = "show"
menu_bar_sound = "show"
menu_bar_battery = "hide"

[notifications]
server = "own"
EOF
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  BAR="sill-bar-$NAME"
  POPUP="$BAR/popup"
  CENTER="sill-notification-center-$NAME"
  sleep 1.5
  echo "daemon on KWin's bus, runtime $RUNTIME, home $HOME_DIR, output $NAME" >&2
}

# Captures SURFACE $1 into $OUT_DIR/$2.png; fails while it is not mapped.
shoot() {
  sill debug capture "$1" "$OUT_DIR/$2.png" >>"$OUT_DIR/capture.log" 2>&1 && return 0
  echo "no capture of $1 for $2 (not mapped)" >&2
  return 1
}

# Capture SURFACE $1 into $2.png once two captures 150 ms apart agree (up to 3 s): a key's
# change may reach the screen a frame or two after the key, more under load.
settled() {
  local previous="$2.settle"
  shoot "$1" "$previous" || return 1
  for _ in $(seq 20); do
    sleep 0.15
    shoot "$1" "$2" || return 1
    [ "$(diff_box "$previous" "$2")" = none ] && return 0
    cp "$OUT_DIR/$2.png" "$OUT_DIR/$previous.png"
  done
  return 0
}
# The box two captures differ in, "x0 y0 x1 y1" or "none" ("size" when their sizes differ).
diff_box() {
  python3 - "$OUT_DIR/$1.png" "$OUT_DIR/$2.png" <<'EOF'
import sys
from PIL import Image, ImageChops
a, b = (Image.open(p).convert("RGBA") for p in sys.argv[1:3])
if a.size != b.size:
    print("size"); sys.exit(0)
box = ImageChops.difference(a, b).getbbox()
print(*box) if box else print("none")
EOF
}

# The status items' points "x y", right to left (the clock's first), from a capture of the bar.
status_points() {
  shoot "$BAR" bar || return 1
  inked_items "$OUT_DIR/bar.png"
}

# The bar's pixels under item "x y" (a 30 px square), as a hash: the item's pill on or off.
item_look() {
  shoot "$BAR" "$1" || return 1
  python3 - "$OUT_DIR/$1.png" "$2" "$3" <<'EOF'
import hashlib, sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
x, y = int(sys.argv[2]), int(sys.argv[3])
print(hashlib.sha1(img.crop((x - 15, 1, x + 15, img.size[1] - 1)).tobytes()).hexdigest()[:12])
EOF
}

# Close whatever the bar shows: real Escape (the popup has the keyboard) and a wait.
close_popup() {
  mapped "$POPUP" || return 0
  press_keys Escape
  wait_surface "$POPUP" 20 down && return 0
  echo "(real Escape left the popup up; a click on the sink closes it)" >&2
  click_at "$SINK_X" "$SINK_Y"
  wait_surface "$POPUP" 20 down
}

# 0.
step_sink_focus() {
  local frame x y w h before
  frame=$(sink_frame)
  [ -n "$frame" ] || { echo "the sink's window is not among KWin's" >&2; return 1; }
  read -r x y w h <<<"$frame"
  SINK_X=$(( x + w / 2 )); SINK_Y=$(( y + h / 2 ))
  click_at "$SINK_X" "$SINK_Y" || return 1
  sleep 0.3
  before=$(sink_keys)
  press_keys k || return 1
  sleep 0.3
  [ "$(sink_keys)" -gt "$before" ] || { echo "the sink got no key: $(tail -3 "$OUT_DIR/sink.log")" >&2; return 1; }
  echo "a real click on the sink ($SINK_X $SINK_Y) gave it the keyboard; a real 'k' reached it" >&2
}

# 1.
step_click_switch() {
  local items
  mapfile -t items < <(status_points)
  # Right to left: the clock, the control center, Sound, Wi-Fi.
  [ "${#items[@]}" -ge 4 ] || { echo "found ${#items[@]} status items, wanted 4" >&2; return 1; }
  CLOCK_AT=${items[0]}; CC_AT=${items[1]}; B_AT=${items[2]}; A_AT=${items[3]}
  # References: A's and B's menus each opened alone.
  local a_off b_off a_on b_on
  # shellcheck disable=SC2086
  a_off=$(item_look bar-a-off $A_AT); b_off=$(item_look bar-b-off $B_AT)
  # shellcheck disable=SC2086
  click_at $A_AT || return 1
  wait_surface "$POPUP" 20 up || { echo "a click on A ($A_AT) opened nothing" >&2; return 1; }
  sleep 0.5
  shoot "$POPUP" menu-a-alone || return 1
  # shellcheck disable=SC2086
  a_on=$(item_look bar-a-on $A_AT)
  close_popup || { echo "Escape left A's menu up" >&2; return 1; }
  # shellcheck disable=SC2086
  click_at $B_AT || return 1
  wait_surface "$POPUP" 20 up || { echo "a click on B ($B_AT) opened nothing" >&2; return 1; }
  sleep 0.5
  shoot "$POPUP" menu-b-alone || return 1
  # shellcheck disable=SC2086
  b_on=$(item_look bar-b-on $B_AT)
  close_popup || { echo "Escape left B's menu up" >&2; return 1; }
  [ "$(diff_box menu-a-alone menu-b-alone)" != none ] || { echo "A's and B's menus look alike" >&2; return 1; }
  [ "$a_on" != "$a_off" ] && [ "$b_on" != "$b_off" ] || { echo "no pill on an open item" >&2; return 1; }
  # A, then B.
  # shellcheck disable=SC2086
  click_at $A_AT || return 1
  wait_surface "$POPUP" 20 up || return 1
  sleep 0.5
  # shellcheck disable=SC2086
  click_at $B_AT || return 1
  sleep 0.6
  local popup a_now b_now shown
  popup=$(mapped "$POPUP" && echo up || echo down)
  shoot "$POPUP" menu-a-then-b
  # shellcheck disable=SC2086
  a_now=$(item_look bar-a-then-b $A_AT)
  # shellcheck disable=SC2086
  b_now=$(item_look bar-a-then-b $B_AT)
  shown=none
  [ "$popup" = up ] && [ "$(diff_box menu-a-then-b menu-b-alone)" = none ] && shown=B
  [ "$popup" = up ] && [ "$(diff_box menu-a-then-b menu-a-alone)" = none ] && shown=A
  echo "real click on A ($A_AT) then on B ($B_AT): popup $popup showing $shown; A's pill" \
    "$([ "$a_now" = "$a_on" ] && echo on || echo off), B's pill" \
    "$([ "$b_now" = "$b_on" ] && echo on || echo off)" >&2
  [ "$shown" = B ] && [ "$a_now" = "$a_off" ] && [ "$b_now" = "$b_on" ]
}

# 2.
step_hover_switch() {
  mapped "$POPUP" || {
    # shellcheck disable=SC2086
    click_at $B_AT || return 1
    wait_surface "$POPUP" 20 up || return 1
  }
  sleep 0.3
  # shellcheck disable=SC2086
  move_to $A_AT || return 1
  sleep 0.6
  local shown=other
  shoot "$POPUP" menu-hover-a || shown=down
  [ "$shown" = other ] && [ "$(diff_box menu-hover-a menu-a-alone)" = none ] && shown=A
  echo "B's menu open, the real pointer onto A: the popup shows $shown" >&2
  [ "$shown" = A ] || return 1
  # The click the person meant on the title the pointer switched to: A stays.
  # shellcheck disable=SC2086
  click_at $A_AT || return 1
  sleep 0.4
  mapped "$POPUP" || { echo "a click on A's title, switched to by hover, closed its menu" >&2; return 1; }
  # shellcheck disable=SC2086
  click_at $A_AT || return 1
  wait_surface "$POPUP" 20 down || { echo "a second click on A's title left its menu up" >&2; return 1; }
  echo "a real click on A's title keeps its menu; a second one closes it" >&2
}

# `notifications.dnd` in the scratch settings file once it reads $1 (up to 3 s), else what it
# reads then: the Focus tile's switch, stored by the daemon's focus task.
await_dnd() {
  local now=
  for _ in $(seq 30); do
    now=$(sed -n 's/^dnd *= *"\(.*\)"$/\1/p' "$CONFIG/sill/settings.toml")
    [ "$now" = "$1" ] && break
    sleep 0.1
  done
  echo "${now:-unset}"
}

# 3.
step_control_center_keys() {
  local sunk b1 b2 back space
  sunk=$(sink_keys)
  # shellcheck disable=SC2086
  click_at $CC_AT || return 1
  wait_surface "$POPUP" 20 up || { echo "a click on the control center ($CC_AT) opened nothing" >&2; return 1; }
  sleep 0.8
  settled "$POPUP" cc-0 || return 1
  press_keys Tab || return 1
  settled "$POPUP" cc-tab1 || return 1
  press_keys Tab || return 1
  settled "$POPUP" cc-tab2 || return 1
  chord Shift_L Tab || return 1
  settled "$POPUP" cc-back || return 1
  b1=$(diff_box cc-0 cc-tab1); b2=$(diff_box cc-tab1 cc-tab2); back=$(diff_box cc-tab1 cc-back)
  echo "control center opened by a real click on its item ($CC_AT): real Tab changed [$b1]," \
    "a second Tab [$b2], Shift+Tab back to the first Tab's look: [$back]" >&2
  # Space on the focused Focus tile stores `notifications.dnd`. The daemon's file watch republishes
  # the settings feed the tile reads on its own: no `sill reload` here; the key itself is all real,
  # and `settled` waits out the watch's debounce plus the redraw.
  press_keys space || return 1
  local dnd_on dnd_off
  dnd_on=$(await_dnd on)
  settled "$POPUP" cc-space || return 1
  space=$(diff_box cc-back cc-space)
  echo "real Space on the focused tile (Focus): notifications.dnd $dnd_on; the tile changed" \
    "[$space] with no 'sill reload'" >&2
  press_keys space || return 1
  dnd_off=$(await_dnd off)
  echo "real Space again: notifications.dnd $dnd_off" >&2
  press_keys Escape || return 1
  wait_surface "$POPUP" 20 down || { echo "real Escape left the control center up" >&2; return 1; }
  echo "real Escape closed it; keys the sink got meanwhile: $(( $(sink_keys) - sunk ))" >&2
  [ "$b1" != none ] && [ "$b2" != none ] && [ "$back" = none ] && [ "$space" != none ] \
    && [ "$dnd_on" = on ] && [ "$dnd_off" = off ] && [ "$(sink_keys)" = "$sunk" ]
}

# Two made-up notifications for the center's rows, once per run; both stay in the center after
# their banners leave.
SEEDED=no
seed_notes() {
  [ "$SEEDED" = no ] || return 0
  sill debug notify "Ada" --app chat --body "Hi" >>"$OUT_DIR/notify.log" 2>&1 || return 1
  sill debug notify "Grace" --app mail --body "The report" >>"$OUT_DIR/notify.log" 2>&1 || return 1
  sleep 8
  SEEDED=yes
}

# 4.
step_center_keys() {
  local sunk down tab esc
  seed_notes || return 1
  sunk=$(sink_keys)
  # shellcheck disable=SC2086
  click_at $CLOCK_AT || return 1
  wait_surface "$CENTER" 20 up || { echo "a click on the clock ($CLOCK_AT) opened nothing" >&2; return 1; }
  sleep 0.8
  settled "$CENTER" nc-0 || return 1
  press_keys Down || return 1
  settled "$CENTER" nc-down || return 1
  down=$(diff_box nc-0 nc-down)
  echo "notification center opened by a real click on the clock ($CLOCK_AT): real Down" \
    "changed [$down]" >&2
  # Tab (unhandled by the center's own key map) reaches Clear All, a quire `Button`;
  # the modality bridge (`sill_surfaces::host_modality::HostModalityBridge`) provides
  # `ds::HostModality` there too, so the ring should draw.
  press_keys Tab || return 1
  settled "$CENTER" nc-tab || return 1
  tab=$(diff_box nc-down nc-tab)
  echo "real Tab (after Down) reaches a quire control (Clear All): changed [$tab]" >&2
  press_keys Escape || return 1
  esc=down
  wait_surface "$CENTER" 20 down || esc=up
  echo "real Escape: the center is $esc; keys the sink got meanwhile: $(( $(sink_keys) - sunk ))" >&2
  [ "$esc" = up ] && { click_at "$SINK_X" "$SINK_Y"; wait_surface "$CENTER" 20 down; }
  [ "$down" != none ] && [ "$tab" != none ] && [ "$esc" = down ] && [ "$(sink_keys)" = "$sunk" ]
}

center_open() { mapped "$CENTER" && echo open || echo closed; }

# 5.
step_center_then_menu() {
  # shellcheck disable=SC2086
  click_at $CLOCK_AT || return 1
  wait_surface "$CENTER" 20 up || { echo "a click on the clock opened nothing" >&2; return 1; }
  sleep 0.6
  # shellcheck disable=SC2086
  click_at $A_AT || return 1
  sleep 0.8
  local popup center shown=other
  popup=$(mapped "$POPUP" && echo up || echo down)
  center=$(center_open)
  shoot "$POPUP" menu-after-center && [ "$(diff_box menu-after-center menu-a-alone)" = none ] && shown=A
  echo "notification center open, a real click on A: the center is $center, the popup" \
    "$popup showing $shown" >&2
  [ "$center" = closed ] && [ "$shown" = A ]
}

# 6.
step_menu_then_center() {
  seed_notes || return 1
  mapped "$POPUP" || {
    # shellcheck disable=SC2086
    click_at $A_AT || return 1
    wait_surface "$POPUP" 20 up || return 1
  }
  sleep 0.4
  local sunk down popup
  sunk=$(sink_keys)
  # shellcheck disable=SC2086
  click_at $CLOCK_AT || return 1
  wait_surface "$CENTER" 20 up || { echo "A's menu open, a click on the clock opened no center" >&2; return 1; }
  sleep 0.8
  popup=$(mapped "$POPUP" && echo up || echo down)
  settled "$CENTER" nc2-0 || return 1
  press_keys Down || return 1
  settled "$CENTER" nc2-down || return 1
  down=$(diff_box nc2-0 nc2-down)
  press_keys Escape || return 1
  wait_surface "$CENTER" 20 down || { echo "real Escape left the center up" >&2; click_at "$SINK_X" "$SINK_Y"; return 1; }
  echo "A's menu open, a real click on the clock: the popup is $popup, the center opened;" \
    "real Down changed [$down]; real Escape closed it; keys the sink got: $(( $(sink_keys) - sunk ))" >&2
  [ "$popup" = down ] && [ "$down" != none ] && [ "$(sink_keys)" = "$sunk" ]
}

# Whether a real "j" now reaches the sink: yes or no.
sink_takes_j() {
  local before
  before=$(sink_keys)
  press_keys j || return 1
  sleep 0.3
  [ "$(sink_keys)" -gt "$before" ] && echo yes || echo no
}

# Keyboard back. Recorded, not asserted (shell-host's side): the sink clicked back into
# focus, a bar menu opened by a real click and closed by real Escape, then a real "j": does the
# app have the keyboard back? Then the same around the notification center. OPEN while the
# menu leaves no window with the keyboard.
check_keyboard_back() {
  local after_menu after_center state=OPEN
  click_at "$SINK_X" "$SINK_Y" && sleep 0.3
  # shellcheck disable=SC2086
  click_at $A_AT && wait_surface "$POPUP" 20 up && sleep 0.3
  press_keys Escape && wait_surface "$POPUP" 20 down && sleep 0.3
  after_menu=$(sink_takes_j)
  click_at "$SINK_X" "$SINK_Y" && sleep 0.3
  # shellcheck disable=SC2086
  click_at $CLOCK_AT && wait_surface "$CENTER" 20 up && sleep 0.3
  press_keys Escape && wait_surface "$CENTER" 20 down && sleep 0.3
  after_center=$(sink_takes_j)
  [ "$after_menu" = yes ] && state=CLOSED
  echo "keyboard back $state: the app has the keyboard back after a bar menu closes: $after_menu; after" \
    "the notification center closes: $after_center" >&2
}

real_cleanup() {
  stop_input
  stop_sink
  stop_daemon
  stop_kwin
  rm -rf "$RUNTIME"
  scratch_drop
}

# Runs the steps "$@" (default: every one, in order), closing whatever the bar shows after each.
# Sets STATUS.
run_real_steps() {
  local steps=("$@") step worst=0
  [ "${#steps[@]}" -gt 0 ] || steps=(step_sink_focus step_click_switch step_hover_switch \
    step_control_center_keys step_center_keys step_center_then_menu step_menu_then_center)
  for step in "${steps[@]}"; do
    run_steps "$step"
    [ "$STATUS" = 0 ] || worst=1
    close_popup
  done
  STATUS=$worst
}

run_real_input() {
  build || return 1
  scratch_new
  # A short runtime folder of its own: the agent socket's path must fit in 108 bytes.
  rm -rf "$RUNTIME"
  RUNTIME=$(mktemp -d /tmp/sill-rt.XXXXXX)
  chmod 700 "$RUNTIME"
  # No key repeat: under load a key's release can reach the shell after its repeat delay, and a
  # repeated Tab or Space would walk or toggle past what the step looks at.
  mkdir -p "$SCRATCH/kwin-seed"
  printf '[Keyboard]\nKeyRepeat=nothing\nRepeatDelay=5000\n' >"$SCRATCH/kwin-seed/kcminputrc"
  export KWIN_VIRTUAL_CONFIG_SEED="$SCRATCH/kwin-seed"
  start_kwin || {
    echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2
    rm -rf "$RUNTIME"; scratch_drop; return 1; }
  trap real_cleanup EXIT
  : >"$OUT_DIR/capture.log"
  : >"$OUT_DIR/notify.log"
  start_real_daemon || { real_cleanup; return 1; }
  start_sink || { real_cleanup; return 1; }
  sleep 1
  start_input || { echo "no fake input client" >&2; real_cleanup; return 1; }
  # The seat's devices are announced with the input client's: the first events only make them known.
  input "move 300 900" "sleep 80" "move 310 900" "sleep 80" || { real_cleanup; return 1; }
  run_real_steps "$@"
  [ "$#" -gt 0 ] || check_keyboard_back || echo "the keyboard-back check could not be run" >&2
  echo "stopping the input client, the sink, the daemon and the headless KWin" >&2
  real_cleanup
  trap - EXIT
}

usage() { echo "usage: dev/accept/bar.sh [--ui | --ui-nested | --real-input [STEP...]]" >&2; exit 2; }

MODE=all
case "${1:-}" in
  "") ;;
  --ui) MODE=ui; shift ;;
  --ui-nested) MODE=ui-nested; shift ;;
  --real-input) MODE=real; shift ;;
  *) usage ;;
esac
[ "$MODE" = real ] || [ "$#" = 0 ] || usage

status=0
case "$MODE" in
  real)
    INPUT_TICKS=400 # 20 s for a fake-input answer under load
    run_real_input "$@" && status=$STATUS || status=1
    echo "logs and captures in $OUT_DIR"
    ;;
  ui)
    build_demo || exit 1
    run_demo_kwin && status=$STATUS || status=1
    ;;
  ui-nested)
    build_demo || exit 1
    run_demo_nested && status=$STATUS || status=1
    ;;
  all)
    build_demo || exit 1
    run_daemon_steps && status=$STATUS || status=1
    run_demo_kwin && [ "$STATUS" = 0 ] || status=1
    echo "logs in $OUT_DIR"
    ;;
esac
exit "$status"

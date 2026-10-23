#!/usr/bin/env bash
# A demo session: today's sill inside a headless casement, left running between commands.
#
#   dev/demo.sh start            build, seed a scratch desktop, leave it up
#   dev/demo.sh tour             play the tour and write a frame of each beat to dev/out/demo-*.png
#   dev/demo.sh video [out.mp4]  play the tour and record it (default dev/out/demo.mp4)
#   dev/demo.sh shot [out.png]   grim of the compositor's output (default dev/out/demo.png)
#   dev/demo.sh stop             quit the daemon and end everything `start` recorded
#
# `tour` and `video` start the demo first when it is not up.
#
# The daemon runs the way every dev script runs it (dev/lib): a private dbus-daemon as both its
# session and its system bus, a scratch HOME, XDG_CONFIG/DATA/STATE/CACHE_HOME and its own
# XDG_RUNTIME_DIR (so its IPC socket is never a real shell's), a fake DDC monitor, and a dry run
# for the power menu. A made-up NetworkManager, BlueZ and MPRIS player (sill-testkit's
# `mpris_player` example) are the only services on that bus. The terminal and the mail app it
# opens get the same scratch environment. ~/.config, ~/.cache, the real buses, wayland-0 and the
# host session are not touched: casement runs headless on its own socket and bus (the casement
# checkout's dev/up.sh, so this runs where that does: inside the jail).
#
# Every process `start` creates is recorded in target/demo/procs with its start time; `stop`
# (and a failed `start`) ends exactly those, by PID, and never anything matched by name, and
# then the casement `start` brought up (dev/down.sh, by the pids in its env file).
#
# DEMO_SIZE sets the output (default 1440x900, a 13-inch Mac's).
#
# Needs: the casement checkout's tools (dev/bootstrap.sh), grim, ffmpeg (video), python3 with PIL and dbus, dbus-daemon, fd (the
# launcher's file search). foot and mailo are opened when installed.
#
# Usage: dev/demo.sh start | tour | video [out.mp4] | shot [out.png] | stop
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/all.sh"

DEMO_DIR="$SILL_TARGET/demo"
OUT_DIR="$DEMO_DIR"
ENV_FILE="$DEMO_DIR/env"
PROCS="$DEMO_DIR/procs"
CLIENT="$DEMO_DIR/sill"
SHOTS="$SILL_REPO/dev/out"
PLAYER="$SILL_OUT/examples/mpris_player"
# Set by start (or read back from $ENV_FILE): the scratch tree's directory (scratch_use), the
# compositor's socket and bus, its kind and env file, the output's name.
NAME=

usage() {
  echo "usage: dev/demo.sh start | tour | video [out.mp4] | shot [out.png] | stop" >&2
  exit 2
}

die() { echo "demo.sh: $*" >&2; exit 1; }

# ---------------------------------------------------------------- the processes it started

# The start time of process $1 (field 22 of /proc/PID/stat, counted after the command name):
# with the pid it names one process for good, where a pid alone can be reused.
proc_token() { sed 's/.*) //' "/proc/$1/stat" 2>/dev/null | cut -d' ' -f20; }

# Records process $2 as $1 so `stop` can end it.
record_proc() {
  mkdir -p "$DEMO_DIR"
  echo "$1 $2 $(proc_token "$2")" >>"$PROCS"
}

# Whether the pid $1 is still the process (start time $2) that was recorded.
same_proc() { [ -n "$2" ] && [ "$(proc_token "$1")" = "$2" ]; }

# Records every process of this user whose environment names this demo's own XDG_RUNTIME_DIR (a
# fresh scratch directory nothing else has) and that is not recorded yet: what the terminal
# re-executed into, the shell in it, an app a dock click launched. An exact match on a unique
# value of the demo's environment, not a name pattern.
record_scratch_clients() {
  local pid
  [ -n "${RUNTIME:-}" ] || return 0
  for pid in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$' | sort -n); do
    grep -q " $pid " "$PROCS" 2>/dev/null && continue
    tr '\0' '\n' 2>/dev/null <"/proc/$pid/environ" | grep -qx "XDG_RUNTIME_DIR=$RUNTIME" || continue
    record_proc client "$pid"
  done
}

# Ends the recorded processes, newest first, each by its pid and only while it is still the one
# recorded: SIGTERM, then SIGKILL after 3 s.
end_recorded() {
  [ -f "$PROCS" ] || return 0
  local name pid token
  while read -r name pid token; do
    same_proc "$pid" "$token" || continue
    # HUP as well: an interactive shell (the terminal's) ignores TERM.
    kill -HUP "$pid" 2>/dev/null
    kill "$pid" 2>/dev/null
    for _ in $(seq 30); do same_proc "$pid" "$token" || break; sleep 0.1; done
    if same_proc "$pid" "$token"; then
      echo "demo.sh: $name ($pid) did not end; killing it" >&2
      kill -9 "$pid" 2>/dev/null
    fi
  done < <(tac "$PROCS")
  rm -f "$PROCS"
}

# The daemon's pid when it still runs (recorded as `daemon`), else nothing.
daemon_pid() {
  local name pid token
  [ -f "$PROCS" ] || return 0
  while read -r name pid token; do
    if [ "$name" = daemon ] && same_proc "$pid" "$token"; then echo "$pid"; return 0; fi
  done <"$PROCS"
}

# ---------------------------------------------------------------- the compositor

# Waits until the compositor answers a client.
await_compositor() {
  for _ in $(seq 25); do
    timeout 2 env WAYLAND_DISPLAY="$WL" XDG_RUNTIME_DIR="$(dirname "$WL")" wayland-info >/dev/null 2>&1 && return 0
    sleep 0.4
  done
  echo "the compositor is not answering clients" >&2
  return 1
}

# ---------------------------------------------------------------- the scratch desktop

write_scene() {
  mkdir -p "$CONFIG/sill"
  cat >"$CONFIG/sill/settings.toml" <<'EOF'
[spaces]

[launcher]
files_backend = "fd"
EOF
  cat >"$CONFIG/sill/layout.toml" <<'EOF'
version = 1

[[dock]]
kind = "quire.dock.app"
app = "foot"

[[dock]]
kind = "quire.dock.app"
app = "org.gnome.Nautilus"

[[dock]]
kind = "quire.dock.app"
app = "google-chrome"

[[dock]]
kind = "quire.dock.app"
app = "btop"
EOF
  backdrop_seed "$CONFIG"
}

# One made-up monitor on the output, so the Display slider has a level. The file is the only bus
# a debug build's DDC worker reads (`--fake-ddc`); the daemon re-reads it as it changes.
write_monitor() {
  printf '{"displays":[{"id":7,"connector":"%s","product":"Nested Display","brightness":{"value":64,"max":100}}]}\n' \
    "$NAME" >"$FAKE_DDC.tmp" && mv "$FAKE_DDC.tmp" "$FAKE_DDC"
}

# Starts the fake service "$1" (a python script of dev/, taking the bus address and a log file) on
# the private bus and waits for its "$2" line in its output.
start_fake() {
  local script="$1" ready="$2" log="$OUT_DIR/$1.out" pid
  : >"$DEMO_DIR/${script%.py}.log"
  in_env setsid python3 "$SILL_REPO/dev/$script" "$BUS" "$DEMO_DIR/${script%.py}.log" >"$log" 2>&1 &
  pid=$!
  record_proc "${script%.py}" "$pid"
  for _ in $(seq 100); do
    grep -q "$ready" "$log" && return 0
    sleep 0.05
  done
  echo "$script did not start (see $log)" >&2
  return 1
}

# sill-testkit's `mpris_player` on the private bus, so Now Playing has a track. It has to be on the
# bus before the daemon's first scan or arrive after: the service watches for players either way.
start_player() {
  in_env setsid "$PLAYER" demo --title "So What" --artist "Miles Davis" --album "Kind of Blue" \
    >"$DEMO_DIR/player.log" 2>&1 &
  record_proc player $!
  for _ in $(seq 100); do
    busctl --address "$BUS" --no-pager list 2>/dev/null | grep -q 'org.mpris.MediaPlayer2.demo' && return 0
    sleep 0.05
  done
  echo "the fake player did not claim its name (see $DEMO_DIR/player.log)" >&2
  return 1
}

# A terminal window in the demo session, in the scratch HOME.
start_terminal() {
  local bin
  bin=$(command -v foot || true)
  if [ -z "$bin" ]; then
    echo "demo.sh: no foot; the desktop will have no window" >&2
    return 0
  fi
  in_env setsid "$bin" >"$DEMO_DIR/term.log" 2>&1 &
  record_proc terminal $!
}

# mailo, the mail app, on a scratch mailbox (its data, config and runtime directories are the
# scratch tree's, so the real store is never opened and no running mailo is asked anything).
start_mail() {
  local bin thread
  bin=$(command -v mailo || true)
  if [ -z "$bin" ]; then
    echo "demo.sh: mailo is not on PATH; the desktop will not open the mail app" >&2
    return 0
  fi
  mkdir -p "$SCRATCH/seed"
  cat >"$SCRATCH/seed/inbox.mbox" <<'EOF'
From ada@example.com Mon Sep 28 09:12:00 2026
From: Ada <ada@example.com>
To: You <you@demo.local>
Subject: Lunch?
Date: Mon, 28 Sep 2026 09:12:00 +0000
Message-ID: <lunch-28@demo.local>
MIME-Version: 1.0
Content-Type: text/plain; charset=utf-8

Noon at the usual place. I saved a table by the window.

From studio@example.com Mon Sep 28 14:40:00 2026
From: Studio <studio@example.com>
To: You <you@demo.local>
Subject: The sketches
Date: Mon, 28 Sep 2026 14:40:00 +0000
Message-ID: <sketches-28@demo.local>
MIME-Version: 1.0
Content-Type: text/plain; charset=utf-8

Three directions for the cover. The second one is the one I would print.

From sam@example.com Tue Sep 29 08:05:00 2026
From: Sam <sam@example.com>
To: You <you@demo.local>
Subject: Thursday review
Date: Tue, 29 Sep 2026 08:05:00 +0000
Message-ID: <review-29@demo.local>
MIME-Version: 1.0
Content-Type: text/plain; charset=utf-8

Can we look at the draft before Thursday? I left notes in the margin.

EOF
  in_env "$bin" import "$SCRATCH/seed/inbox.mbox" >"$DEMO_DIR/mailo-import.log" 2>&1 \
    || echo "demo.sh: mailo import failed (see $DEMO_DIR/mailo-import.log)" >&2
  # The newest thread, so the window opens on a letter instead of "Nothing open".
  thread=$(in_env "$bin" list inbox 1 2>/dev/null | awk 'NF { print $NF; exit }')
  in_env setsid "$bin" ${thread:+open "$thread"} >"$DEMO_DIR/mailo.log" 2>&1 &
  record_proc mailo $!
}

# The client other terminals use to talk to this daemon: `target/demo/sill launcher open`.
write_client() {
  {
    echo '#!/usr/bin/env bash'
    printf 'exec env HOME=%q XDG_RUNTIME_DIR=%q WAYLAND_DISPLAY=%q DBUS_SESSION_BUS_ADDRESS=%q \\\n' \
      "$HOME_DIR" "$RUNTIME" "$WL" "$BUS"
    printf '  DBUS_SYSTEM_BUS_ADDRESS=%q XDG_CONFIG_HOME=%q XDG_DATA_HOME=%q XDG_STATE_HOME=%q \\\n' \
      "$BUS" "$CONFIG" "$DATA" "$STATE"
    printf '  XDG_CACHE_HOME=%q %q "$@"\n' "$CACHE" "$SILL_BIN"
  } >"$CLIENT"
  chmod +x "$CLIENT"
}

write_env() {
  {
    printf 'SCRATCH=%q\n' "$SCRATCH"
    printf 'BUS=%q\n' "$BUS"
    printf 'WL=%q\n' "$WL"
    printf 'NAME=%q\n' "$NAME"
    printf 'COMPOSITOR=%q\nCASEMENT_ENV=%q\nCASEMENT_BUS=%q\n' "$COMPOSITOR" "$CASEMENT_ENV" "$CASEMENT_BUS"
  } >"$ENV_FILE"
  write_client
}

# Reads back what `start` left: the scratch tree, the bus, the socket. Fails when no demo is up.
load_demo() {
  [ -f "$ENV_FILE" ] || return 1
  # shellcheck source=/dev/null
  source "$ENV_FILE"
  scratch_use "$SCRATCH"
  [ -n "$(daemon_pid)" ]
}

# Ends everything this demo started and removes its scratch tree. Safe to repeat.
teardown() {
  if [ -f "$ENV_FILE" ]; then
    # shellcheck source=/dev/null
    source "$ENV_FILE"
    scratch_use "$SCRATCH"
  fi
  # The daemon quits through its own IPC first, so its surfaces go before the compositor does.
  if [ -n "$(daemon_pid)" ] && [ -x "$SILL_BIN" ]; then
    in_env timeout 10 "$SILL_BIN" quit >/dev/null 2>&1
  fi
  record_scratch_clients
  end_recorded
  stop_compositor
  [ -z "${SCRATCH:-}" ] || scratch_drop
  rm -f "$ENV_FILE" "$CLIENT"
}

# ---------------------------------------------------------------- start

abort_start() {
  local rc=$?
  trap - EXIT
  [ "$rc" -eq 0 ] && return 0
  echo "demo.sh: start failed, tearing down what it created" >&2
  teardown
  exit "$rc"
}

cmd_start() {
  mkdir -p "$DEMO_DIR"
  if load_demo; then
    die "already running (daemon pid $(daemon_pid)); dev/demo.sh stop first."
  fi
  # A leftover env file of a demo that is gone: its tree and records are stale.
  [ ! -f "$ENV_FILE" ] || teardown
  rm -f "$PROCS"

  echo "demo.sh: building sill ($SILL_PROFILE)"
  build "-p sill-testkit --example mpris_player" || exit 1

  trap abort_start EXIT
  SIZE="${DEMO_SIZE:-1440x900}"
  start_compositor --default casement || exit 1
  await_compositor || exit 1

  scratch_new
  # From here a teardown has a tree to remove: the env file is what names it.
  write_env
  write_scene
  use_fake_ddc "$CONFIG" >/dev/null
  # A stand-in NetworkManager, so the Wi-Fi tile is a radio that starts off and joins "Home", and
  # a BlueZ, so Bluetooth reads "On" with a paired keyboard. BlueZ has to be on the bus before the
  # daemon connects: its task does not come back if the name is missing at startup.
  start_fake fake-nm.py "fake-nm: org.freedesktop.NetworkManager on" || exit 1
  start_fake fake-bluez.py "fake-bluez: org.bluez on" || exit 1
  start_player || exit 1

  # Setsid, so the daemon survives this script and a runner that kills its process group.
  LAUNCH_PREFIX=(setsid)
  : >"$SCRATCH/session.record"
  launch_daemon --session-dry-run "$SCRATCH/session.record" || { tail -n 40 "$OUT_DIR/daemon.log" >&2; exit 1; }
  record_proc daemon "$DAEMON"
  NAME=$(output_name) || exit 1
  write_monitor
  write_env

  # The wallpaper surface follows ping, once the image is prepared.
  for _ in $(seq 100); do
    [ -n "$(surface_named sill-wallpaper-)" ] && break
    sleep 0.1
  done

  start_terminal
  start_mail
  # Let the wallpaper and the windows paint, then post the two banners.
  sleep 1
  sill debug notify "Lunch?" --app Mail --body "Noon at the usual place." >"$DEMO_DIR/notify.log" 2>&1
  sill debug notify "Ping" --app Chat --body "Are you there?" >>"$DEMO_DIR/notify.log" 2>&1

  record_scratch_clients
  write_env
  trap - EXIT
  cat <<EOF

The demo is up, in a headless casement ($WL).

  1. Rest. Wallpaper, menu bar, dock, a terminal and the mail app.
  2. Move the pointer along the dock.
  3. Click the app name in the bar, then the next title.
  4. Launcher:  $CLIENT launcher open --query fi
  5. The control center opens, turns Wi-Fi on, and joins Home.
  6. The network list, a volume indicator, the app switcher, the power menu,
     and the desktop calendar.
  7. Click the clock, or  $CLIENT notifications open
     The center holds the two messages and the calendar.

Banners expire after a few seconds and stay in the notification center.

  dev/demo.sh tour              # play the tour, write dev/out/demo-*.png
  dev/demo.sh video [out.mp4]   # the same tour, recorded (default dev/out/demo.mp4)
  dev/demo.sh shot [out.png]    # default dev/out/demo.png
  dev/demo.sh stop

EOF
}

# ---------------------------------------------------------------- shot and video

# A screenshot of the output with grim, on the compositor's own socket: never the host
# (shell-host's shot.sh falls back to spectacle on the host when no compositor answers).
take_shot() {
  [ -S "$WL" ] || { echo "demo.sh: no compositor at $WL" >&2; return 1; }
  mkdir -p "$(dirname "$1")"
  WAYLAND_DISPLAY="$WL" grim "$1"
}

cmd_shot() {
  # The recorder is the screencopy client for the length of a video.
  [ -z "${DEMO_RECORD:-}" ] || return 0
  load_demo || die "no demo is running; dev/demo.sh start first."
  take_shot "$(realpath -m "${1:-$SHOTS/demo.png}")"
}

# Films the output with grim while a tour runs. One screencopy client at a time, so
# cmd_shot does not also call grim while this runs.
record_start() {
  RECORD_DIR=$(mktemp -d "$DEMO_DIR/frames.XXXXXX")
  RECORD_STOP="$DEMO_DIR/record.stop"
  rm -f "$RECORD_STOP"
  (
    export WAYLAND_DISPLAY="$WL"
    i=0
    prev=$(date +%s%N)
    while [ ! -f "$RECORD_STOP" ]; do
      if grim -t jpeg -q 75 "$(printf '%s/%05d.jpg' "$RECORD_DIR" "$i")" >/dev/null 2>&1; then
        now=$(date +%s%N)
        echo "$i $((now - prev))" >>"$RECORD_DIR/times"
        prev=$now
        i=$((i + 1))
      else
        sleep 0.05
      fi
    done
  ) &
  RECORD_PID=$!
  for _ in $(seq 50); do
    [ -s "$RECORD_DIR/00000.jpg" ] && break
    sleep 0.05
  done
  if [ ! -s "$RECORD_DIR/00000.jpg" ]; then
    echo "demo.sh: the recorder took no frames" >&2
    touch "$RECORD_STOP"
    wait "$RECORD_PID" 2>/dev/null
    RECORD_PID=
    return 1
  fi
  echo "demo.sh: recording the desktop" >&2
}

# Ends the recording and encodes it to $1 (a variable-frame-rate H.264 from the frames' own
# timestamps).
record_stop() {
  local out="$1"
  [ -n "${RECORD_PID:-}" ] || return 0
  touch "$RECORD_STOP"
  wait "$RECORD_PID" 2>/dev/null
  RECORD_PID=
  if [ ! -s "$RECORD_DIR/times" ]; then
    echo "demo.sh: no frames recorded" >&2
    rm -rf "$RECORD_DIR"
    return 1
  fi
  mkdir -p "$(dirname "$out")"
  python3 - "$RECORD_DIR" "$out" <<'PY'
import subprocess, sys
from pathlib import Path
frames, out = Path(sys.argv[1]), Path(sys.argv[2])
rows = []
for line in (frames / "times").read_text().splitlines():
    index, delta = line.split()
    rows.append((int(index), max(int(delta), 40_000_000) / 1e9))
list_path = frames / "list.txt"
with list_path.open("w") as fh:
    for index, duration in rows:
        fh.write(f"file '{frames / f'{index:05d}.jpg'}'\n")
        fh.write(f"duration {duration:.3f}\n")
    fh.write(f"file '{frames / f'{rows[-1][0]:05d}.jpg'}'\n")
# This ffmpeg build has no libx264; OpenH264 is the H.264 encoder it ships.
subprocess.run(
    ["ffmpeg", "-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", str(list_path),
     "-fps_mode", "vfr", "-c:v", "libopenh264", "-pix_fmt", "yuv420p",
     "-b:v", "8M", "-movflags", "+faststart", str(out)],
    check=True,
)
print(f"demo.sh: wrote {out} ({len(rows)} frames)")
PY
  rm -rf "$RECORD_DIR" "$RECORD_STOP"
}

cmd_video() {
  VIDEO_OUT="$(realpath -m "${1:-$SHOTS/demo.mp4}")"
  command -v ffmpeg >/dev/null || die "video needs ffmpeg"
  cmd_tour --video
}

# ---------------------------------------------------------------- the tour

# While recording, lingers on a surface that is already up: $1 seconds (default 1.8). The stills
# path does not wait.
show() { [ -z "${DEMO_RECORD:-}" ] || sleep "${1:-1.8}"; }

# Waits up to 5 s for surface $1 to be mapped, then $2 s (default 0.4) for its entrance to finish:
# a debug build on a busy host takes longer than a fixed sleep to show what was asked for.
await_up() {
  wait_surface "$1" 50 up || echo "demo.sh tour: $1 did not appear" >&2
  sleep "${2:-0.4}"
}

# The launcher's panel, for `sill debug key`.
launcher_panel() { echo "sill-launcher-$NAME"; }

# Types $1 into the open launcher's field, replacing what is there: Ctrl+A and BackSpace, then
# each character as a key of its own, so the panel stays the one panel the whole time.
type_query() {
  local panel char
  panel=$(launcher_panel)
  sill debug key "$panel" a+ctrl >/dev/null
  sill debug key "$panel" Backspace >/dev/null
  while IFS= read -r -n1 char; do
    [ -n "$char" ] || continue
    [ "$char" != " " ] || char=Space
    sill debug key "$panel" "$char" >/dev/null
    sleep 0.04
  done <<<"$1"
}

# One key on the launcher's panel, by the name launcher.sh's key_spec knows.
launcher_key() {
  local spec
  case "$1" in
    tab) spec=Tab ;;
    shift-tab) spec=Tab+shift ;;
    quick-look) spec=y+ctrl ;;
    *) spec="$1" ;;
  esac
  sill debug key "$(launcher_panel)" "$spec" >/dev/null
}

# What the bar (from its capture $1, $2 device px per logical px) has inked: "APP_X APP_Y NEXT_X
# NEXT_Y CLOCK_X CLOCK_Y" in logical px, the first title on the left, the status item right of the
# control center, and the clock.
bar_targets() {
  python3 - "$1" "$2" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
w, h = img.size
scale = float(sys.argv[2])
ground = img.getpixel((w // 2, h // 2))

def inked(x):
    return any(sum(abs(a - b) for a, b in zip(img.getpixel((x, y)), ground)) > 90
               for y in range(4, h - 4))

def runs(xs):
    found, run, gap = [], None, 0
    for x in xs:
        if inked(x):
            run = [x, x] if run is None else [min(x, run[0]), max(x, run[1])]
            gap = 0
        elif run is not None:
            gap += 1
            if gap > 8:
                found.append(run)
                run = None
    if run is not None:
        found.append(run)
    return found

left = [r for r in runs(range(0, w // 3)) if r[1] - r[0] > 30]
right = runs(range(w - 1, w // 2, -1))
if not left:
    sys.exit("no app title on the bar")
if len(right) < 2:
    sys.exit("no second menu on the bar")
# Right to left: the clock, the control center, then the next status menu.
second = right[2] if len(right) > 2 else right[1]

def center(run):
    return int(((run[0] + run[1]) / 2) / scale), int((h / 2) / scale)

print(*center(left[0]), *center(second), *center(right[0]))
PY
}

# The dock pill's span from its capture $1 ($2 logical px wide output): "LEFT RIGHT CENTRE_Y
# MIDDLE_PIN_X" in logical px. Five pins, the middle one is the third.
dock_geometry() {
  python3 - "$1" "$2" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
w, h = img.size
scale = w / int(sys.argv[2]) if int(sys.argv[2]) else 1
rows = [y for y in range(h) if sum(1 for x in range(0, w, 4) if img.getpixel((x, y))[3] > 80) > 6]
if not rows:
    sys.exit("dock pill not found")
cy = (rows[0] + rows[-1]) // 2
xs = [x for x in range(w) if img.getpixel((x, cy))[3] > 80]
if len(xs) < 4:
    sys.exit("dock pill has no width")
middle = xs[0] + (xs[-1] - xs[0]) * 2.5 / 5
print(int(xs[0] / scale), int(xs[-1] / scale), int(cy / scale), int(middle / scale))
PY
}

# The control center panel's light region in the popup capture $1: "LEFT TOP RIGHT BOTTOM" px.
panel_box() {
  python3 - "$1" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
w, h = img.size
xs, ys = [], []
for y in range(h):
    for x in range(w):
        r, g, b = img.getpixel((x, y))
        if min(r, g, b) > 220 and abs(r - b) < 25:
            xs.append(x)
            ys.append(y)
if not xs:
    sys.exit("no control center panel")
print(min(xs), min(ys), max(xs), max(ys))
PY
}

# The lowest white button of the polkit card's capture $1 (Cancel; Authenticate is the accent
# colour): "X Y" in the capture's px.
polkit_cancel() {
  python3 - "$1" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
w, h = img.size
white = []
for y in range(h):
    for x in range(0, w, 2):
        r, g, b = img.getpixel((x, y))
        if min(r, g, b) > 235 and abs(x - w / 2) < 280 and abs(y - h / 2) < 240:
            white.append((x, y))
if not white:
    sys.exit("no password card")
white.sort(key=lambda p: (p[1], p[0]))
bands = [[white[0]]]
for p in white[1:]:
    if p[1] - bands[-1][-1][1] > 8:
        bands.append([p])
    else:
        bands[-1].append(p)
cancel = bands[-1]
print(sum(p[0] for p in cancel) // len(cancel), sum(p[1] for p in cancel) // len(cancel))
PY
}

# Presses the control center's Wi-Fi tile at $1 $2 and waits up to 4 s for the stand-in
# NetworkManager to log "radio $3".
wifi_press() {
  local before
  before=$(grep -c "^radio $3" "$DEMO_DIR/fake-nm.log")
  sill debug click "$BAR/popup" "$1" "$2" >/dev/null
  for _ in $(seq 40); do
    [ "$(grep -c "^radio $3" "$DEMO_DIR/fake-nm.log")" -gt "$before" ] && return 0
    sleep 0.1
  done
  echo "demo.sh tour: the Wi-Fi tile did not log radio $3" >&2
  return 1
}

# Waits up to 4 s for the control center's popup to be its full height (its buffer is tall before
# the panel is on the output).
await_control_center() {
  for _ in $(seq 40); do
    sill debug capture "$BAR/popup" "$WORK/popup.png" >/dev/null 2>&1 \
      && python3 -c 'import sys; from PIL import Image; sys.exit(0 if Image.open(sys.argv[1]).height > 400 else 1)' \
        "$WORK/popup.png" && return 0
    sleep 0.1
  done
  return 1
}

# Closes whatever a previous look left open.
settle() {
  sill launcher close >/dev/null 2>&1
  sill notifications close >/dev/null 2>&1
  sill notifications dismiss-all >/dev/null 2>&1
  sill power-menu close >/dev/null 2>&1
  sill switcher cancel >/dev/null 2>&1
  # A click in the bar's empty middle dismisses a menu; it does not open one. The control center
  # does not dismiss on it, so toggle that one only while it is still up.
  if sill debug capture "$BAR/popup" "$WORK/probe.png" >/dev/null 2>&1; then
    sill debug click "$BAR" $((WIDTH / 2)) 12 >/dev/null 2>&1
    sleep 0.3
    if sill debug capture "$BAR/popup" "$WORK/probe.png" >/dev/null 2>&1; then
      sill control-center toggle >/dev/null 2>&1
      sleep 0.3
    fi
  fi
}

# The wallpaper surface exists before its first painted frame is on the output; the empty
# compositor is a flat light grey. Waits until a point of the screen has left that grey.
await_wallpaper() {
  echo "demo.sh tour: waiting for the wallpaper"
  for _ in $(seq 25); do
    take_shot "$WORK/ready.png"
    python3 - "$WORK/ready.png" <<'PY' && return 0
import sys
from PIL import Image
r, g, b = Image.open(sys.argv[1]).convert("RGB").getpixel((80, 200))
sys.exit(0 if abs(r - 233) + abs(g - 236) + abs(b - 230) > 40 else 1)
PY
    sleep 0.4
  done
}

beat_rest() {
  echo "demo.sh tour: 1 rest"
  cmd_shot "$SHOTS/demo-1-rest.png"
  show 1.4
}

beat_dock() {
  echo "demo.sh tour: 2 dock"
  local x0 x1 pill_y icon_x i x
  sill debug capture "$DOCK" "$WORK/dock.png" >/dev/null
  read -r x0 x1 pill_y _ <<<"$(dock_geometry "$WORK/dock.png" "$WIDTH")"
  # A short sweep, then a pause on the magnified icon.
  for i in $(seq 0 10); do
    x=$(( x0 + (x1 - x0) * i / 10 ))
    sill debug pointer "$DOCK" "$x" "$pill_y" >/dev/null
    if [ -n "${DEMO_RECORD:-}" ]; then sleep 0.02; else sleep 0.03; fi
  done
  cmd_shot "$SHOTS/demo-2-dock.png"
  show 1.4
  # The sweep leaves the pointer on the last icon, which magnifies the row and moves every icon.
  # Let that settle, then right-click the middle pin where it rests.
  sill debug leave "$DOCK" >/dev/null
  sleep 0.45
  sill debug capture "$DOCK" "$WORK/dock.png" >/dev/null
  read -r _ _ pill_y icon_x <<<"$(dock_geometry "$WORK/dock.png" "$WIDTH")"
  sill debug click "$DOCK" "$icon_x" "$pill_y" --button secondary >/dev/null
  # Long enough to read the menu in the recording; the stills path only waits for it to paint.
  await_up "$DOCK/popup" 0.3
  show 2.0
  cmd_shot "$SHOTS/demo-2-dock-menu.png"
  # Escape dismisses the popup through the panel's own key path. A click on the bar while this
  # popup is still the topmost is a protocol error and drops the shell. Leave as well, or the
  # magnification stays for the rest of the tour.
  sill debug key "$DOCK/popup" Escape >/dev/null 2>&1
  wait_surface "$DOCK/popup" 20 down || die "tour: the dock's menu did not close; stopping before a click on the bar drops the shell"
  sill debug leave "$DOCK" >/dev/null
  sleep 0.2
}

beat_menus() {
  echo "demo.sh tour: 3 menus"
  local scale app_x app_y next_x next_y
  sill debug capture "$BAR" "$WORK/bar.png" >/dev/null
  scale=$(python3 -c 'import sys; from PIL import Image; print(Image.open(sys.argv[1]).width / int(sys.argv[2]))' \
    "$WORK/bar.png" "$WIDTH")
  read -r app_x app_y next_x next_y CLOCK_X CLOCK_Y <<<"$(bar_targets "$WORK/bar.png" "$scale")"
  sill debug click "$BAR" "$app_x" "$app_y" >/dev/null
  await_up "$BAR/popup" 0.3
  cmd_shot "$SHOTS/demo-3-menu.png"
  show 1.8
  sill debug click "$BAR" "$next_x" "$next_y" >/dev/null
  sleep 0.7
  cmd_shot "$SHOTS/demo-3-switched.png"
  show 1.8
  sill debug click "$BAR" $((WIDTH / 2)) 12 >/dev/null
  wait_surface "$BAR/popup" 30 down
  sleep 0.15
}

# A one-page note in the scratch Documents, which Quick Look draws.
write_pdf() {
  mkdir -p "$HOME_DIR/Documents"
  python3 - "$HOME_DIR/Documents/Thursday-review.pdf" <<'PY'
import sys
from pathlib import Path
stream = b"""BT
/F1 22 Tf
36 470 Td
(Thursday review) Tj
0 -36 Td
/F1 12 Tf
(Can we look at the draft before Thursday?) Tj
0 -20 Td
(I left notes in the margin.) Tj
ET
"""
objs = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 420 540] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"endstream",
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
]
out = b"%PDF-1.4\n"
offs = []
for i, obj in enumerate(objs, 1):
    offs.append(len(out))
    out += f"{i} 0 obj\n".encode() + obj + b"\nendobj\n"
xref = len(out)
out += f"xref\n0 {len(objs)+1}\n0000000000 65535 f \n".encode()
out += b"".join(f"{o:010d} 00000 n \n".encode() for o in offs)
out += f"trailer\n<< /Size {len(objs)+1} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
Path(sys.argv[1]).write_bytes(out)
PY
}

beat_launcher() {
  echo "demo.sh tour: 4 launcher"
  # One opening. Later queries are typed into the field that is already up. Tab and Shift+Tab
  # pick one section, and typing stays inside that section, so a query that should search
  # everything is typed only after the keys that land back on every provider.
  sill launcher open --query "" >/dev/null
  await_up "$(launcher_panel)" 0.5
  cmd_shot "$SHOTS/demo-4-actions.png"
  show 1.6
  launcher_key tab
  sleep 0.7
  cmd_shot "$SHOTS/demo-4-apps.png"
  show 1.8
  # Applications is the section filter. One Shift+Tab returns to everything.
  launcher_key shift-tab
  sleep 0.12
  type_query '1024/16'
  sleep 0.5
  cmd_shot "$SHOTS/demo-4-calc.png"
  show 1.8
  type_query ""
  sleep 0.25
  launcher_key shift-tab
  sleep 0.2
  launcher_key shift-tab
  sleep 0.45
  cmd_shot "$SHOTS/demo-4-emoji.png"
  show 1.8
  # Emoji is a section too. Two Tabs pass Web and land on everything again.
  launcher_key tab
  sleep 0.12
  launcher_key tab
  sleep 0.12
  type_query heart
  sleep 0.5
  cmd_shot "$SHOTS/demo-4-launcher.png"
  show 1.4
  launcher_key quick-look
  sleep 0.35
  cmd_shot "$SHOTS/demo-4-preview.png"
  show 2.0
  write_pdf
  # The field still has the keyboard and the pane is already open, so this query replaces the
  # emoji with the PDF's first page. Another Quick Look would close the pane.
  type_query 'Thursday-review'
  sleep 0.85
  cmd_shot "$SHOTS/demo-4-pdf.png"
  show 2.2
  sill launcher close >/dev/null
  sleep 0.15
}

beat_control_center() {
  echo "demo.sh tour: 5 control center"
  local left top right bottom tile wx wy cx radios
  # A click in the bar's middle dismisses the menu from beat 3. The control center then opens by
  # its own command: guessing which status icon is which from ink runs opened Wi-Fi instead.
  sill debug click "$BAR" $((WIDTH / 2)) 12 >/dev/null 2>&1
  sleep 0.2
  sill control-center toggle >/dev/null
  await_control_center
  # While recording, grim belongs to the recorder, so just give the panel a moment to show.
  if [ -n "${DEMO_RECORD:-}" ]; then
    sleep 0.35
    show 1.8
  else
    for _ in $(seq 15); do
      cmd_shot "$SHOTS/demo-5-control-center.png"
      python3 - "$SHOTS/demo-5-control-center.png" <<'PY' && break
import sys
from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB")
r, g, b = im.getpixel((im.width - 80, 160))
sys.exit(0 if min(r, g, b) > 210 and abs(r - b) < 40 else 1)
PY
      sleep 0.2
    done
  fi
  # The theme control is the panel's last row: "System | Light | Dark", the last segment 68 px in
  # from the panel's right edge and 32 px up from its bottom. Dark, then back to System.
  sill debug capture "$BAR/popup" "$WORK/popup.png" >/dev/null
  read -r left top right bottom <<<"$(panel_box "$WORK/popup.png")"
  sill debug click "$BAR/popup" $(( right - 68 )) $(( bottom - 32 )) >/dev/null
  sleep 0.45
  cmd_shot "$SHOTS/demo-5-dark.png"
  show 1.6
  sill debug click "$BAR/popup" $(( left + 66 )) $(( bottom - 32 )) >/dev/null
  sleep 0.3
  # The stand-in NetworkManager logs every radio request ("radio on", "radio off") and starts with
  # the radio on: put it off first if it is not (a nothing logged yet means on), then press the
  # tile again and wait for the "radio on" it logs, which joins the saved network.
  sill debug capture "$BAR/popup" "$WORK/popup.png" >/dev/null
  read -r left top right bottom <<<"$(panel_box "$WORK/popup.png")"
  # Tiles are 56 px, padded 10, gapped 10, two columns. The body click stays left of the chevron;
  # the chevron is the tile's right edge.
  tile=$(( (right - left - 30) / 2 ))
  wx=$(( left + 48 ))
  wy=$(( top + 38 ))
  cx=$(( left + 10 + tile - 18 ))
  if [ "$(grep '^radio ' "$DEMO_DIR/fake-nm.log" | tail -1)" != "radio off" ]; then
    wifi_press "$wx" "$wy" off
  fi
  wifi_press "$wx" "$wy" on
  sleep 0.5
  cmd_shot "$SHOTS/demo-5-wifi.png"
  show 1.6
  # The chevron opens the network list: Home (just joined) and Cafe.
  sill debug click "$BAR/popup" "$cx" "$wy" >/dev/null
  sleep 0.7
  cmd_shot "$SHOTS/demo-5-wifi-list.png"
  show 2.2
  sill control-center toggle >/dev/null
  sleep 0.15

  echo "demo.sh tour: 5 bluetooth"
  sill control-center toggle >/dev/null
  await_control_center
  sleep 0.3
  sill debug capture "$BAR/popup" "$WORK/popup.png" >/dev/null
  read -r left top right bottom <<<"$(panel_box "$WORK/popup.png")"
  tile=$(( (right - left - 30) / 2 ))
  # Bluetooth is the top-right tile. Its chevron opens the device list.
  sill debug click "$BAR/popup" $(( left + 10 + tile + 10 + tile - 18 )) $(( top + 38 )) >/dev/null
  sleep 0.7
  cmd_shot "$SHOTS/demo-5-bluetooth.png"
  show 2.0
  sill control-center toggle >/dev/null
  sleep 0.15
}

beat_osd_switcher_power() {
  echo "demo.sh tour: 6 volume"
  # A made-up volume change: the indicator, not this machine's mixer. It stays about 1.5 s, so
  # the next indicator replaces it instead of a gap.
  sill debug osd volume 64 >/dev/null
  await_up "sill-osd-$NAME" 0.3
  cmd_shot "$SHOTS/demo-6-volume.png"
  show 1.15
  echo "demo.sh tour: 6 brightness"
  sill debug osd brightness 40 >/dev/null
  sleep 0.3
  cmd_shot "$SHOTS/demo-6-brightness.png"
  show 1.15
  # The last indicator has to finish before the switcher, or they overlap.
  sleep 0.35

  echo "demo.sh tour: 7 switcher"
  sill debug switcher foot org.gnome.Nautilus google-chrome >/dev/null
  await_up "sill-switcher-$NAME" 0.4
  cmd_shot "$SHOTS/demo-7-switcher.png"
  show 2.0
  sill switcher cancel >/dev/null 2>&1
  sleep 0.12

  echo "demo.sh tour: 8 power"
  sill power-menu open >/dev/null
  await_up "sill-power-menu-$NAME" 0.5
  cmd_shot "$SHOTS/demo-8-power.png"
  show 2.0
  sill power-menu close >/dev/null
  sleep 0.12
}

beat_screenshot_polkit() {
  echo "demo.sh tour: 10 screenshot"
  python3 - "$WORK/picture.png" <<'PY'
import sys
from PIL import Image, ImageDraw
im = Image.new("RGB", (960, 540), (232, 196, 150))
d = ImageDraw.Draw(im)
d.rectangle((0, 0, 960, 320), fill=(120, 176, 214))
d.ellipse((700, 48, 820, 168), fill=(245, 214, 120))
d.polygon([(0, 320), (280, 180), (520, 340), (960, 200), (960, 540), (0, 540)], fill=(92, 140, 96))
d.rectangle((70, 250, 250, 430), fill=(244, 240, 232))
d.polygon([(60, 250), (160, 170), (260, 250)], fill=(196, 122, 92))
im.save(sys.argv[1])
PY
  sill debug screenshot "$WORK/picture.png" >/dev/null
  # The thumbnail's surface captures before it is on screen; give it its slide-in.
  sleep 1.5
  cmd_shot "$SHOTS/demo-10-screenshot.png"
  # The thumbnail is the beat. It dismisses itself after about five seconds; stay with it, then
  # move on as it goes.
  show 3.2
  sleep 0.4

  echo "demo.sh tour: 11 password"
  local polkit px py
  sill debug polkit --message "Authentication is required to install updates" \
    --action org.freedesktop.packagekit.package-install >/dev/null
  await_up "sill-polkit-$NAME" 0.5
  cmd_shot "$SHOTS/demo-11-polkit.png"
  show 2.2
  polkit=$(surface_named sill-polkit-)
  if [ -n "$polkit" ] && sill debug capture "$polkit" "$WORK/polkit.png" >/dev/null 2>&1; then
    if read -r px py <<<"$(polkit_cancel "$WORK/polkit.png")" && [ -n "${py:-}" ]; then
      sill debug click "$polkit" "$px" "$py" >/dev/null 2>&1
      sleep 0.35
    fi
  fi
  sleep 0.15
}

beat_notifications() {
  echo "demo.sh tour: 12 notifications"
  sill notifications dismiss-all >/dev/null 2>&1
  sill debug notify "Lunch?" --app Mail --body "Noon at the usual place." >/dev/null
  sill debug notify "Ping" --app Chat --body "Are you there?" >/dev/null
  await_up "sill-banners-$NAME" 0.5
  cmd_shot "$SHOTS/demo-12-banners.png"
  show 2.0
  sill notifications close >/dev/null 2>&1
  # The clock opens the center.
  sill debug click "$BAR" "$CLOCK_X" "$CLOCK_Y" >/dev/null
  await_up "sill-notification-center-$NAME" 0.6
  cmd_shot "$SHOTS/demo-12-center.png"
  show 2.6
}

# Plays the beats on the running demo and writes one frame of each to dev/out. Pointer moves and
# clicks go through `sill debug`, the path the acceptance scripts use; nothing here is aimed at
# the host session. `tour --video` records the desktop for the length of the tour.
cmd_tour() {
  local video=0
  [ "${1:-}" != "--video" ] || video=1
  load_demo || cmd_start || exit 1
  load_demo || die "the demo did not come up"
  WORK="$DEMO_DIR/tour"
  mkdir -p "$SHOTS" "$WORK"
  WIDTH=$(nested_output_size | cut -d' ' -f1)
  [ -n "$WIDTH" ] || die "tour: no output"
  BAR=$(surface_named sill-bar-)
  DOCK=$(surface_named sill-dock-)
  [ -n "$BAR" ] && [ -n "$DOCK" ] || die "tour: the bar or the dock is missing"
  CLOCK_X=$((WIDTH - 60)) CLOCK_Y=12

  settle
  await_wallpaper
  if [ "$video" = 1 ]; then
    record_start || exit 1
    DEMO_RECORD=1
    trap 'record_stop "$VIDEO_OUT"' EXIT
  fi

  beat_rest
  beat_dock
  beat_menus
  beat_launcher
  beat_control_center
  beat_osd_switcher_power
  beat_screenshot_polkit
  beat_notifications

  if [ "$video" = 1 ]; then
    record_stop "$VIDEO_OUT"
    trap - EXIT
    DEMO_RECORD=
  fi
  echo
  echo "demo.sh tour: frames in $SHOTS"
  ls -1 "$SHOTS"/demo-*.png
}

# ---------------------------------------------------------------- stop

cmd_stop() {
  if [ ! -f "$ENV_FILE" ] && [ ! -f "$PROCS" ]; then
    echo "demo.sh: nothing to stop"
    return 0
  fi
  teardown
  echo "demo.sh: stopped"
}

case "${1:-}" in
  start) cmd_start ;;
  tour) cmd_tour ;;
  video)
    shift
    [ "$#" -le 1 ] || usage
    cmd_video "${1:-}"
    ;;
  stop) cmd_stop ;;
  shot)
    shift
    [ "$#" -le 1 ] || usage
    cmd_shot "${1:-}"
    ;;
  *) usage ;;
esac

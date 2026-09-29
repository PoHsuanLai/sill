#!/usr/bin/env bash
# sill's own lock screen acceptance (design/20 §1.9), on the nested cosmic-comp ONLY (dev/nested.sh;
# wayland-1 is pinged first, reused, and never stopped or restarted). Starts `sill run` from this
# checkout's debug build (`--features debug`) with a scratch XDG_CONFIG_HOME, XDG_STATE_HOME and
# XDG_CACHE_HOME, its own XDG_RUNTIME_DIR (the nested socket as an absolute WAYLAND_DISPLAY, so the
# lock marker and the single-instance lock are the script's), under a private session bus.
#
# SAFETY. The lock is only ever taken on the nested compositor: the script refuses to run when
# the nested socket is wayland-0 (the user's KDE session) and every `sill` call carries the
# nested socket. No password is typed and PAM never runs: every verdict is made up with
# `sill debug auth accept|refuse`, which only a debug daemon takes. Whatever fails, the script
# ends by accepting on the daemon when it is up, quitting it, and running the recovery
# lock-and-unlock (`session_locker 0`), so the nested session is always left unlocked.
#
#   1. `sill lock`: `sill lock status` says locked within 1 s, and a second lock client
#      (`session_locker`) is refused (exit 3) while sill holds the lock.
#   2. A capture of every output's lock surface (`sill-lock/<out>`; `debug stats` does not list
#      lock surfaces); the prompt's output shows the clock and the field
#      (dev/out/lock-cosmic.png), and its pixels differ from a clock-only surface's in the
#      prompt's band.
#   3. `sill debug auth refuse`: the prompt spins then shakes (captures in the prompt's band
#      differ from the idle one, and settle back to it), and the session stays locked.
#   4. Five more refusals (six wrongs): no lockout, so the prompt settles back to the
#      idle field every time (dev/out/lock-sixwrong-cosmic.png; the band matches the idle one, no
#      "Try again at" line), still locked.
#   5. 5 s idle while locked: 0 commits on the lock surfaces (counted from the daemon's
#      WAYLAND_DEBUG=client log, since `debug stats` has no lock rows).
#   6. `sill debug auth accept` right after the six wrongs (nothing to wait for): status unlocked within
#      the made-up check (250 ms) plus the accepted beat (settle(PersonaHop), 454 ms) plus 500 ms, the lock surface is gone,
#      the marker is removed, and the session is free (a stand-in locker locks and unlocks).
#   7. A crash while locked: `sill lock`, the marker is written, the daemon is killed (-9) and
#      started again: it locks at once from the marker, and `sill debug auth accept` unlocks.
#   8. A build without `debug` answers `sill debug auth accept` with "unsupported".
#
# Exit 0 when every step passed, 1 on a failure. Another sill daemon started from a
# `target/*/sill run` is waited for (up to 10 minutes).
#
# Usage: dev/nested.sh && dev/accept-lock.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-lock"
SHOTS="$SILL_REPO/dev/out"
LOCKER="$SILL_REPO/target/debug/examples/session_locker"
NODEBUG_BIN="$OUT_DIR/sill-nodebug"
mkdir -p "$OUT_DIR" "$SHOTS"

source "$SHELL_HOST_DEV/.nested-env"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
NESTED_SOCKET="$HOST_RUNTIME/$WAYLAND_DISPLAY"
case "$(basename "$NESTED_SOCKET")" in
  wayland-0) echo "the nested socket is wayland-0 (the real session): refusing" >&2; exit 1 ;;
esac
RUNTIME=$(mktemp -d)
chmod 700 "$RUNTIME"
CONFIG=$(mktemp -d)
STATE=$(mktemp -d)
CACHE=$(mktemp -d)
MARKER="$RUNTIME/sill/locked"
BIN="$SILL_BIN"
DAEMON=

# Every `sill` call carries the daemon's whole environment: a request that starts a daemon when
# none runs (anything but ping and quit) must never start one anywhere but the nested session.
sill() {
  XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$NESTED_SOCKET" XDG_CONFIG_HOME="$CONFIG" \
    XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" "$BIN" "$@"
}
nested_locker() { WAYLAND_DISPLAY="$NESTED_SOCKET" "$LOCKER" "$@"; }
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }

output_names() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):])'
}

# `sill lock status`'s phase word (locked, locking, unlocked).
lock_phase() { sill lock status 2>/dev/null | python3 -c '
import json, sys
r = json.loads(sys.stdin.read())["v"]
print(r["phase"])' 2>/dev/null; }

# ms since $1 until the status is "$2", or -1 within $3 ms.
until_phase() {
  local start=$1 want=$2 limit=$3 t
  while :; do
    t=$(( $(now_ms) - start ))
    [ "$t" -gt "$limit" ] && { echo -1; return; }
    [ "$(lock_phase)" = "$want" ] && { echo "$t"; return; }
    sleep 0.05
  done
}

# A capture of output $1's lock surface into $2, tried three times (a 2560 px capture on a busy
# machine can pass the client's 2 s wait).
capture() {
  local _
  for _ in 1 2 3; do
    sill debug capture "sill-lock/$1" "$2" >>"$OUT_DIR/capture.log" 2>&1 && return 0
    grep -q "no surface" <(tail -n1 "$OUT_DIR/capture.log") && return 1
  done
  return 1
}

# How many pixels differ (by more than 24 in any channel) between two PNGs in the band of the
# height from $3 to $4 percent.
band_diff() {
  python3 - "$1" "$2" "$3" "$4" <<'EOF'
import sys
from PIL import Image, ImageChops
a = Image.open(sys.argv[1]).convert("RGB")
b = Image.open(sys.argv[2]).convert("RGB")
if a.size != b.size:
    print(10**9); sys.exit()
w, h = a.size
box = (0, h * int(sys.argv[3]) // 100, w, h * int(sys.argv[4]) // 100)
d = ImageChops.difference(a.crop(box), b.crop(box)).convert("L").point(lambda v: 255 if v > 24 else 0)
print(sum(1 for v in d.tobytes() if v))
EOF
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill) >"$OUT_DIR/build.log" 2>&1 \
    && cp "$SILL_REPO/target/debug/sill" "$NODEBUG_BIN" \
    && (cd "$SILL_REPO" && cargo build -p sill --features debug \
      && cargo build -p sill-services --example session_locker) >>"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

start_daemon() {
  local log="$1"
  mkdir -p "$CONFIG/sill"
  : >"$CONFIG/sill/settings.toml"
  backdrop_seed "$CONFIG" || return 1
  use_fake_ddc "$CONFIG" >/dev/null
  require_fake_ddc || return 1
  WAYLAND_DEBUG="${WDEBUG:-}" XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" \
    XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$NESTED_SOCKET" \
    dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- \
    sh -c "$PRIVATE_BOTH_BUSES" "$BIN" "${FAKE_DDC_FLAGS[@]}" \
    >"$log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  NAMES=
  for _ in $(seq 50); do NAMES=$(output_names); [ -n "$NAMES" ] && break; sleep 0.1; done
  [ -n "$NAMES" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  FIRST=$(echo "$NAMES" | head -n1)
  # The lock screen's picture: the wallpaper service's blurred copy (resized and blurred once
  # per scratch cache, which a debug build takes a few seconds over).
  for _ in $(seq 300); do
    [ -n "$(find "$CACHE" -name '*-backdrop.png' 2>/dev/null)" ] && break
    sleep 0.1
  done
  sleep 1.5
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
}

# The one output whose lock surface draws the prompt: its capture's prompt band holds the most
# ink against a plain one. With one output, that output.
PROMPT_OUT=

# 1.
step_lock() {
  local log="$OUT_DIR/lock.log" start took refused
  start=$(now_ms)
  sill lock >"$log" 2>&1 || { cat "$log" >&2; return 1; }
  took=$(until_phase "$start" "locked" 1000)
  nested_locker 1000 >>"$log" 2>&1
  refused=$?
  echo "locked ${took} ms after 'sill lock'; a second lock client exited ${refused} (3 = refused)" \
    | tee -a "$log" >&2
  [ "$took" -ge 0 ] && [ "$refused" = 3 ]
}

# 2.
step_surfaces() {
  local log="$OUT_DIR/surfaces.log" name ok=0 best=-1 ink
  : >"$log"
  sleep 0.8
  for name in $NAMES; do
    if capture "$name" "$OUT_DIR/lock-$name.png"; then
      ink=$(python3 - "$OUT_DIR/lock-$name.png" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("L")
w, h = img.size
band = img.crop((w // 3, h * 70 // 100, 2 * w // 3, h * 97 // 100))
print(sum(1 for v in band.tobytes() if v > 200))
EOF
)
      echo "sill-lock/$name captured ($(python3 -c "from PIL import Image; print(Image.open('$OUT_DIR/lock-$name.png').size)")); bright pixels in the prompt band: $ink" | tee -a "$log" >&2
      [ "$ink" -gt "$best" ] && { best=$ink; PROMPT_OUT=$name; }
    else
      echo "sill-lock/$name: no capture" | tee -a "$log" >&2; ok=1
    fi
  done
  sill debug stats | grep -c '"sill-lock/' | xargs -I{} echo "debug stats rows for lock surfaces: {}" | tee -a "$log" >&2
  [ -n "$PROMPT_OUT" ] && cp "$OUT_DIR/lock-$PROMPT_OUT.png" "$SHOTS/lock-cosmic.png"
  cp "$OUT_DIR/lock-$PROMPT_OUT.png" "$OUT_DIR/idle.png" 2>/dev/null
  echo "the prompt is on $PROMPT_OUT: $SHOTS/lock-cosmic.png; backdrops written: $(find "$CACHE" -name '*-backdrop.png' | wc -l)" | tee -a "$log" >&2
  [ "$ok" = 0 ] && [ "$best" -gt 200 ]
}

# 3.
step_refuse_shakes() {
  local log="$OUT_DIR/refuse.log" i moved=0 diff last start
  : >"$log"
  start=$(now_ms)
  sill debug auth refuse >>"$log" 2>&1 || { cat "$log" >&2; return 1; }
  # The made-up check spins for 250 ms, then the field shakes for 420 ms and empties.
  for i in $(seq 12); do
    if sill debug capture "sill-lock/$PROMPT_OUT" "$OUT_DIR/refuse-$i.png" >>"$OUT_DIR/capture.log" 2>&1; then
      diff=$(band_diff "$OUT_DIR/idle.png" "$OUT_DIR/refuse-$i.png" 70 97)
      echo "$(( $(now_ms) - start )) ms: $diff pixels differ in the prompt band" >>"$log"
      [ "$diff" -gt 150 ] && moved=$(( moved + 1 ))
    else
      echo "$(( $(now_ms) - start )) ms: no capture" >>"$log"
    fi
  done
  sleep 0.6
  capture "$PROMPT_OUT" "$OUT_DIR/refuse-after.png"
  last=$(band_diff "$OUT_DIR/idle.png" "$OUT_DIR/refuse-after.png" 70 97)
  echo "refuse: $moved captures moved (spinner, shake); after it settled $last pixels differ; status $(lock_phase)" \
    | tee -a "$log" >&2
  [ "$moved" -ge 1 ] && [ "$last" -lt 150 ] && [ "$(lock_phase)" = "locked" ]
}

# 4.
step_no_lockout() {
  local log="$OUT_DIR/no-lockout.log" n diff
  : >"$log"
  for n in 2 3 4 5 6; do
    sill debug auth refuse >>"$log" 2>&1
    sleep 0.9
  done
  sleep 0.5
  capture "$PROMPT_OUT" "$SHOTS/lock-sixwrong-cosmic.png"
  diff=$(band_diff "$OUT_DIR/idle.png" "$SHOTS/lock-sixwrong-cosmic.png" 70 97)
  echo "six wrongs: $diff pixels differ from the idle prompt (no 'Try again at' line: no lockout); status $(lock_phase); $SHOTS/lock-sixwrong-cosmic.png" \
    | tee -a "$log" >&2
  [ "$diff" -lt 150 ] && [ "$(lock_phase)" = "locked" ]
}

# 5.
step_idle() {
  local log="$OUT_DIR/idle.log" wlog="$OUT_DIR/daemon-wayland-debug.log" from to commits start
  # A daemon logging its Wayland requests (WAYLAND_DEBUG=client), for this step only: the log
  # slows a capture past the client's 2 s wait, and `debug stats` has no lock rows.
  stop_daemon
  WDEBUG=client start_daemon "$wlog" || return 1
  start=$(now_ms)
  sill lock >"$log" 2>&1
  [ "$(until_phase "$start" "locked" 2000)" -ge 0 ] || { echo "not locked" | tee -a "$log" >&2; return 1; }
  # Keep the window inside one minute: the clock redraws once a minute.
  while [ "$(date +%S | sed 's/^0//')" -ge 50 ]; do sleep 1; done
  sleep 2
  from=$(wc -l <"$wlog")
  sleep 5
  to=$(wc -l <"$wlog")
  commits=$(python3 - "$wlog" "$from" "$to" <<'EOF'
import re, sys
path, start, end = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
lock_surfaces, count = set(), 0
with open(path, errors="replace") as log:
    for n, line in enumerate(log, 1):
        m = re.search(r"-> ext_session_lock_v1#\d+\.get_lock_surface\(new id \S+, wl_surface#(\d+)", line)
        if m and n <= start:
            lock_surfaces.add(m.group(1))
        m = re.search(r"-> wl_surface#(\d+)\.commit\(\)", line)
        if m and start < n <= end and m.group(1) in lock_surfaces:
            count += 1
        if n > end:
            break
print(len(lock_surfaces), count)
EOF
)
  echo "5 s locked and idle: lock surfaces seen, commits on them: $commits" | tee -a "$log" >&2
  start=$(now_ms)
  sill debug auth accept >>"$log" 2>&1
  until_phase "$start" "unlocked" 3000 >/dev/null
  stop_daemon
  start_daemon "$OUT_DIR/daemon.log" || return 1
  [ "$(echo "$commits" | cut -d' ' -f1)" -ge 1 ] && [ "$(echo "$commits" | cut -d' ' -f2)" = 0 ]
}

# 6.
step_accept() {
  local log="$OUT_DIR/accept.log" start took gone free
  : >"$log"
  start=$(now_ms)
  sill debug auth accept >>"$log" 2>&1 || { cat "$log" >&2; return 1; }
  took=$(until_phase "$start" "unlocked" 3000)
  if capture "$PROMPT_OUT" "$OUT_DIR/after-unlock.png"; then gone=no; else gone=yes; fi
  nested_locker 200 >>"$log" 2>&1
  free=$?
  echo "unlocked ${took} ms after 'sill debug auth accept' (limit 250 + 454 + 500: the check, the accepted beat, slack); lock surface gone: $gone; marker: $(test -e "$MARKER" && echo present || echo removed); a stand-in locker then locked and unlocked: exit $free" \
    | tee -a "$log" >&2
  [ "$took" -ge 0 ] && [ "$took" -le 1204 ] && [ "$gone" = yes ] && [ ! -e "$MARKER" ] && [ "$free" = 0 ]
}

# 7.
step_crash_relocks() {
  local log="$OUT_DIR/crash.log" start took pid again
  : >"$log"
  start=$(now_ms)
  sill lock >>"$log" 2>&1
  took=$(until_phase "$start" "locked" 1000)
  [ -e "$MARKER" ] || { echo "no marker at $MARKER while locked" | tee -a "$log" >&2; return 1; }
  pid=$(pgrep -P "$DAEMON" -x sill)
  echo "locked in ${took} ms, marker written; killing sill ($pid) with SIGKILL" | tee -a "$log" >&2
  kill -9 "$pid"
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  start_daemon "$OUT_DIR/daemon-restarted.log" || return 1
  start=$(now_ms)
  again=$(until_phase "$start" "locked" 3000)
  capture "$FIRST" "$OUT_DIR/relocked.png" && cp "$OUT_DIR/relocked.png" "$SHOTS/lock-relocked-cosmic.png"
  start=$(now_ms)
  sill debug auth accept >>"$log" 2>&1
  took=$(until_phase "$start" "unlocked" 3000)
  echo "restarted: locked from the marker (${again} ms after it answered); accept unlocked in ${took} ms; marker: $(test -e "$MARKER" && echo present || echo removed)" \
    | tee -a "$log" >&2
  [ "$again" -ge 0 ] && [ "$took" -ge 0 ] && [ ! -e "$MARKER" ]
}

# 8. NODEBUG_BIN is built without the `debug` feature (build(), above), so `sill run` takes no
# `--fake-ddc` (nor any other stand-in flag) there, and its DDC worker opens the real
# /dev/i2c-* on startup exactly as a release build would, a read-only probe this step cannot avoid
# without a stand-in i2c device, unlike every other daemon start in this suite.
step_release_refuses_fake() {
  local log="$OUT_DIR/nodebug.log" answer
  stop_daemon
  BIN="$NODEBUG_BIN"
  : >"$CONFIG/sill/settings.toml"
  XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" \
    XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$NESTED_SOCKET" \
    dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- \
    sh -c "$PRIVATE_BOTH_BUSES" "$BIN" \
    >"$log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  answer=$(sill debug auth accept 2>&1)
  echo "a build without debug answers 'sill debug auth accept': $answer" | tee -a "$log" >&2
  stop_daemon
  BIN="$SILL_BIN"
  echo "$answer" | grep -q "without the debug feature"
}

# Whatever happened: accept on a daemon that is up, quit it, and lock-and-unlock the nested
# session with the stand-in, so it is never left locked.
recover() {
  if [ -n "${DAEMON:-}" ]; then
    case "$(lock_phase)" in
      unlocked|"") ;;
      *) sill debug auth accept >/dev/null 2>&1; sleep 1.5 ;;
    esac
    stop_daemon
  fi
  if nested_locker 0 >"$OUT_DIR/recovery.log" 2>&1; then
    echo "recovery: the stand-in locked and unlocked the nested session (it is unlocked)" >&2
  else
    echo "recovery: the stand-in exited $? (see $OUT_DIR/recovery.log)" >&2
  fi
}

WAYLAND_DISPLAY="$NESTED_SOCKET" wayland-info >/dev/null 2>&1 || {
  echo "no nested compositor at $NESTED_SOCKET (dev/nested.sh first)" >&2; exit 1; }
build || exit 1
for _ in $(seq 600); do
  pgrep -f 'target/[a-z]*/sill run' >/dev/null || break
  [ -n "${WAITED:-}" ] || { echo "another sill daemon is running; waiting for it to quit" >&2; WAITED=1; }
  sleep 1
done
trap recover EXIT
status=0
if start_daemon "$OUT_DIR/daemon.log"; then
  for step in ${STEPS:-step_lock step_surfaces step_refuse_shakes step_no_lockout step_accept \
    step_idle step_crash_relocks step_release_refuses_fake}; do
    if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; fi
  done
else
  status=1
fi
recover
trap - EXIT
rm -rf "$RUNTIME" "$CONFIG" "$STATE" "$CACHE"
echo "logs in $OUT_DIR; screenshots $SHOTS/lock-cosmic.png, $SHOTS/lock-sixwrong-cosmic.png"
exit "$status"

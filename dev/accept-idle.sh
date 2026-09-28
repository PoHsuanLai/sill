#!/usr/bin/env bash
# sill's own idle acceptance (Q420 B; FINDINGS "sill idle (Q420 B)" and "sill idle fill"), on
# the nested cosmic-comp ONLY (dev/nested.sh; wayland-1 is pinged first, reused, and never
# stopped or restarted). Its "displays" are windows on the host, so powering them off darkens
# nothing real.
#
# SAFETY. Nothing here reaches logind, the session bus's org.freedesktop.ScreenSaver, PAM, a
# backlight or wayland-0:
# - one private dbus-daemon (dev/private-bus.conf) is both the daemon's system and session bus;
#   examples/logind_standin.rs owns org.freedesktop.login1 there and only RECORDS Suspend,
#   SetIdleHint and Inhibit (it sends PrepareForSleep and Lock on command);
# - the daemon gets its own XDG_RUNTIME_DIR, XDG_CONFIG_HOME, XDG_STATE_HOME and XDG_CACHE_HOME,
#   the nested socket as an absolute WAYLAND_DISPLAY (the script refuses wayland-0), and
#   XDG_CURRENT_DESKTOP=COSMIC so it resolves a sill session (Host::SillSession) and owns idle;
# - short idle times come from SILL_IDLE_TIMES, which only a debug build reads;
# - no password is typed: an asked lock is opened with `sill debug auth accept` (debug build);
# - no input is typed anywhere: a wake is `sill debug idle-wake` (debug build), a made-up
#   `resumed` (the nested cosmic-comp counts no synthetic input as activity);
# - whatever fails, the script ends by accepting on the daemon, quitting it, and running the
#   recovery lock-and-unlock (`session_locker 0`, F381), so the nested session is left unlocked.
#
# Times: screen off 6 s, dim 3 s before it at 50 %, lock grace 5 s, the lock screen's own
# screen-off 4 s; suspend never, except in step 7 (10 s).
#
#   1. Idle dims, then turns the displays off and locks with cause idle_screen_off. The dim
#      surface (`sill-idle-dim-<output>`) is unmapped before the dim (a capture is refused), and
#      captured twice while dimmed: early in the fade and once `--t-idle-dim` (2 s) has run.
#      Both must be the black scrim (every pixel RGB 0), the early one lighter than the late
#      one, and the late one at 50 % (alpha 128 +- 12). The late capture is kept as
#      dev/out/idle-dim.png, with the nested screen at that moment as dev/out/idle-dim-cosmic.png
#      (non-fatal). Once the displays are off the surface is unmapped again. G341 is looked for in the
#      WAYLAND_DEBUG=client log (set_mode(off) before ext_session_lock_v1.locked, and no
#      `failed`); the nested winit outputs refuse power control, so it cannot be shown here.
#   2. A wake within the grace: unlocked with no password (the daemon logs "grace NoPassword").
#   3. Idle again, then a wake after the grace (7 s): still locked ("grace Ask"); accept opens it.
#   4. `sill lock` (cause action), the lock screen goes dark after 4 s, a wake at once: still
#      locked, no grace.
#   5. A ScreenSaver.Inhibit from a test client holds idle for 9 s (> 6 s: no dim, no off, no
#      lock; `sill idle status` counts it); UnInhibit lets idle lock again; a second inhibit
#      ends when its client leaves the bus.
#   6. PrepareForSleep(true) from the stand-in locks with cause sleep; after the wake, a made-up wake:
#      still locked.
#   7. SILL_IDLE_TIMES=6,10: off and lock at 6 s, logind Suspend(false) recorded at about 10 s
#      from the last wake (G342: the lock's new watches do not restart the suspend watch), the
#      idle hint set true then false; after PrepareForSleep true/false and a wake: still locked.
#
# Exit 0 when every step passed, 1 on a failure. Another sill daemon started from a
# `target/*/sill run` is waited for (up to 10 minutes).
#
# Usage: dev/nested.sh && dev/accept-idle.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
# shellcheck source=dev/backdrop.sh
source "$(dirname "${BASH_SOURCE[0]}")/backdrop.sh"
OUT_DIR="$SILL_REPO/target/accept-idle"
SHOTS="$SILL_REPO/dev/out"
DIM_SURFACE=
mkdir -p "$OUT_DIR"
LOCKER="$SILL_REPO/target/debug/examples/session_locker"
LOGIND_BIN="$SILL_REPO/target/debug/examples/logind_standin"
CLIENT_BIN="$SILL_REPO/target/debug/examples/screensaver_client"

source "$SHELL_HOST_DEV/.nested-env"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
NESTED_SOCKET="$HOST_RUNTIME/$WAYLAND_DISPLAY"
case "$(basename "$NESTED_SOCKET")" in
  wayland-0) echo "the nested socket is wayland-0 (the real session): refusing" >&2; exit 1 ;;
esac
SCRATCH=$(mktemp -d)
RUNTIME="$SCRATCH/run"
CONFIG="$SCRATCH/config"
STATE="$SCRATCH/state"
CACHE="$SCRATCH/cache"
mkdir -p "$RUNTIME" "$CONFIG/sill" "$STATE" "$CACHE"
chmod 700 "$RUNTIME"
BUS=
BUS_PID=
LOGIND=
DAEMON=
TIMES="6,0"

in_env() {
  env XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$NESTED_SOCKET" XDG_CONFIG_HOME="$CONFIG" \
    XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" XDG_CURRENT_DESKTOP=COSMIC \
    DBUS_SESSION_BUS_ADDRESS="$BUS" DBUS_SYSTEM_BUS_ADDRESS="$BUS" "$@"
}
sill() { in_env "$SILL_BIN" "$@"; }
nested_locker() { WAYLAND_DISPLAY="$NESTED_SOCKET" "$LOCKER" "$@"; }
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }
# A wake: `sill debug idle-wake` stands in for the compositor's `resumed` (the nested
# cosmic-comp 1.8 counts no virtual-keyboard input as activity, and a key typed into the host's
# window of it would be real input on the person's own session; FINDINGS "sill idle fill").
# It also holds every idle notification afresh, so the idle timers count from here.
wake() { sill debug idle-wake >/dev/null; }
# A command to the logind stand-in.
logind() { echo "$*" >&3; }

# `sill lock status` as "phase cause" (cause null while unlocked).
lock_state() { sill lock status 2>/dev/null | python3 -c '
import json, sys
r = json.loads(sys.stdin.read())["v"]
print(r["phase"], r["cause"] or "none")' 2>/dev/null; }

# `sill idle status` as "mode phase dim inhibition inhibits".
idle_state() { sill idle status 2>/dev/null | python3 -c '
import json, sys
r = json.loads(sys.stdin.read())["v"]
m = r["mode"]["kind"] + ("" if r["mode"]["kind"] == "own" else ":" + r["mode"]["v"])
d = "clear" if r["dim"]["kind"] == "clear" else "dimmed:%d" % r["dim"]["v"]
print(m, r["phase"], d, r["inhibition"], r["inhibits"])' 2>/dev/null; }

# ms since $1 until `$2` prints something matching the regex $3, or -1 within $4 ms.
until_match() {
  local start=$1 probe=$2 want=$3 limit=$4 t
  while :; do
    t=$(( $(now_ms) - start ))
    [ "$t" -gt "$limit" ] && { echo -1; return; }
    "$probe" | grep -qE "$want" && { echo "$t"; return; }
    sleep 0.1
  done
}

# Wait up to ${2:-5} s for a line of the stand-in's log matching $1.
await_logind() {
  local tries=$(( ${2:-5} * 10 ))
  for _ in $(seq "$tries"); do
    grep -qE "$1" "$OUT_DIR/logind.log" && return 0
    sleep 0.1
  done
  return 1
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug &&
    cargo build -p sill-services --example session_locker --example logind_standin \
      --example screensaver_client) >"$OUT_DIR/build.log" 2>&1 \
    || { echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

start_bus() {
  local out
  out=$(dbus-daemon --config-file="$SILL_REPO/dev/private-bus.conf" --fork \
    --print-address=1 --print-pid=1) || return 1
  BUS=$(sed -n 1p <<<"$out")
  BUS_PID=$(sed -n 2p <<<"$out")
  case "$BUS" in unix:*) ;; *) echo "no private bus address: $out" >&2; return 1 ;; esac
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "not a private bus" >&2; return 1; }
  require_private_system_bus "$BUS" || return 1
  echo "private bus $BUS (pid $BUS_PID)" >&2
}

start_logind() {
  mkfifo "$SCRATCH/logind.in"
  "$LOGIND_BIN" "$BUS" <"$SCRATCH/logind.in" >"$OUT_DIR/logind.log" 2>&1 &
  LOGIND=$!
  exec 3>"$SCRATCH/logind.in"
  await_logind "^ready$" || { echo "the logind stand-in never started" >&2; return 1; }
}

start_daemon() {
  local log="$1"
  backdrop_seed "$CONFIG" || return 1
  printf '[session]\nlocker = "sill"\nlock_grace_s = 5\n\n[idle]\ndim_s = 3\ndim_level_pct = 50\nlocked_screen_off_s = 4\n' \
    >"$CONFIG/sill/settings.toml"
  use_fake_ddc "$SCRATCH" >/dev/null
  require_fake_ddc || return 1
  in_env WAYLAND_DEBUG="${WDEBUG:-}" SILL_IDLE_TIMES="$TIMES" "$SILL_BIN" run >"$log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  sill ping >/dev/null 2>&1 || { echo "the daemon never answered (see $log)" >&2; return 1; }
  [ "$(until_match "$(now_ms)" idle_state '^own ' 5000)" -ge 0 ] \
    || { echo "idle is not own: $(idle_state)" >&2; return 1; }
  sleep 2
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
}

# Accept whatever lock is up, and wait for it to open.
open_lock() {
  case "$(lock_state)" in
    "unlocked "*) return 0 ;;
  esac
  wake
  sill debug auth accept >/dev/null 2>&1
  [ "$(until_match "$(now_ms)" lock_state '^unlocked' 4000)" -ge 0 ]
}

# The nested output's connector name (the surfaces' namespace suffix).
output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-"):
        print(s[len("sill-bar-"):]); break'
}

# The PNG $1 as "width height rgb_max alpha_min alpha_max": a scrim is RGB 0 everywhere and one
# alpha across the whole surface.
scrim_of() {
  python3 - "$1" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
(r0, r1), (g0, g1), (b0, b1), (a0, a1) = img.getextrema()
print(img.width, img.height, max(r1, g1, b1), a0, a1)
EOF
}

# Capture the dim surface to $1; fails while it is unmapped.
capture_dim() { sill debug capture "$DIM_SURFACE" "$1" >/dev/null 2>&1; }

# 1.
step_dim_off_lock() {
  local log="$OUT_DIR/1-dim-off-lock.log" start dim off locked g341 name before early late after
  : >"$log"
  for _ in $(seq 50); do name=$(output_name); [ -n "$name" ] && break; sleep 0.1; done
  DIM_SURFACE="sill-idle-dim-$name"
  wake; start=$(now_ms)
  rm -f "$OUT_DIR"/dim-*.png
  if capture_dim "$OUT_DIR/dim-before.png"; then before=mapped; else before=unmapped; fi
  dim=$(until_match "$start" idle_state ' dimmed dimmed:50 ' 8000)
  local dimmed_at; dimmed_at=$(now_ms)
  capture_dim "$OUT_DIR/dim-early.png" && early=$(scrim_of "$OUT_DIR/dim-early.png")
  # `--t-idle-dim` is 2 s; the displays go off 3 s after the dim.
  sleep "$(python3 -c "print(max(0, 2.4 - ($(now_ms) - $dimmed_at) / 1000))")"
  capture_dim "$OUT_DIR/dim-late.png" && late=$(scrim_of "$OUT_DIR/dim-late.png")
  local late_at=$(( $(now_ms) - start ))
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/idle-dim-cosmic.png" >>"$log" 2>&1 \
    || echo "nested screenshot failed (non-fatal)" >&2
  cp "$OUT_DIR/dim-late.png" "$SHOTS/idle-dim.png" 2>/dev/null
  off=$(until_match "$start" idle_state ' displays_off ' 10000)
  locked=$(until_match "$start" lock_state '^locked idle_screen_off$' 12000)
  sleep 0.3
  if capture_dim "$OUT_DIR/dim-after.png"; then after=mapped; else after=unmapped; fi
  echo "dim surface $DIM_SURFACE: before the dim $before; early '${early:-no capture}'; at ${late_at} ms '${late:-no capture}' (width height rgb_max alpha_min alpha_max); after the displays went off $after" \
    | tee -a "$log" >&2
  local scrim_ok=1
  python3 - "${early:-}" "${late:-}" <<'EOF' && scrim_ok=0
import sys
early, late = sys.argv[1].split(), sys.argv[2].split()
if len(early) != 5 or len(late) != 5:
    sys.exit(1)
e, l = [int(v) for v in early], [int(v) for v in late]
black = e[2] == 0 and l[2] == 0
uniform = e[3] == e[4] and l[3] == l[4]
fading = 0 < e[4] < l[4]
at_level = abs(l[4] - 128) <= 12
sys.exit(0 if black and uniform and fading and at_level else 1)
EOF
  [ "$scrim_ok" = 0 ] && echo "scrim: black, uniform, fading in, at 50 %" | tee -a "$log" >&2
  g341=$(python3 - "$OUT_DIR/daemon-wdebug.log" <<'EOF'
import re, sys
off = lock = None
failed = 0
for line in open(sys.argv[1], errors="replace"):
    m = re.match(r"\[(\d+):(\d+):(\d+\.\d+)\]", line)
    if not m:
        continue
    t = int(m.group(1)) * 3600 + int(m.group(2)) * 60 + float(m.group(3))
    if re.search(r"zwlr_output_power_v1#\d+\.failed\(\)", line):
        failed += 1
    if off is None and re.search(r"zwlr_output_power_v1#\d+\.set_mode\(0\)", line):
        off = t
    if off is not None and lock is None and re.search(r"ext_session_lock_v1#\d+\.locked\(\)", line):
        lock = t
print("power-controls-failed %d off-requested-at %s locked-at %s" % (failed, off, lock))
if failed:
    print("NOT-ANSWERABLE-HERE (the nested outputs refuse zwlr_output_power_v1)")
elif off is not None and lock is not None and lock > off:
    print("GRANTED-WHILE-OFF")
else:
    print("NOT-SHOWN")
EOF
)
  echo "dimmed at ${dim} ms (want ~3000), displays off at ${off} ms (~6000), locked with cause idle_screen_off at ${locked} ms; G341: $(echo "$g341" | tr '\n' ' ')" \
    | tee -a "$log" >&2
  echo "idle: $(idle_state); lock: $(lock_state)" | tee -a "$log" >&2
  [ "$dim" -ge 2000 ] && [ "$off" -ge 5000 ] && [ "$locked" -ge 0 ] \
    && [ "$before" = unmapped ] && [ "$after" = unmapped ] && [ "$scrim_ok" = 0 ]
}

# 2.
step_wake_in_grace() {
  local log="$OUT_DIR/2-wake-in-grace.log" start opened
  : >"$log"
  sleep 1
  start=$(now_ms); wake
  opened=$(until_match "$start" lock_state '^unlocked' 3000)
  echo "a wake ~1 s after the idle lock: unlocked ${opened} ms later with no password; daemon: $(grep -o 'grace [A-Za-z]* (woke after [0-9]* ms)' "$DAEMON_LOG" | tail -n1); idle: $(idle_state)" \
    | tee -a "$log" >&2
  [ "$opened" -ge 0 ] && grep -q "grace NoPassword" "$DAEMON_LOG"
}

# 3.
step_wake_after_grace() {
  local log="$OUT_DIR/3-wake-after-grace.log" start locked still
  : >"$log"
  wake; start=$(now_ms)
  locked=$(until_match "$start" lock_state '^locked idle_screen_off$' 12000)
  sleep 7
  wake; sleep 1.5
  still=$(lock_state)
  echo "idle locked at ${locked} ms; a wake 7 s later: lock is '$still'; daemon: $(grep -o 'grace [A-Za-z]* (woke after [0-9]* ms)' "$DAEMON_LOG" | tail -n1)" \
    | tee -a "$log" >&2
  local ok=1
  [ "$locked" -ge 0 ] && [ "$still" = "locked idle_screen_off" ] \
    && grep 'grace ' "$DAEMON_LOG" | tail -n1 | grep -q "grace Ask" && ok=0
  open_lock || ok=1
  return $ok
}

# 4.
step_manual_lock_asks() {
  local log="$OUT_DIR/4-manual-lock.log" start locked off still graces
  : >"$log"
  wake; start=$(now_ms)
  sill lock >>"$log" 2>&1
  locked=$(until_match "$start" lock_state '^locked action$' 3000)
  off=$(until_match "$start" idle_state ' displays_off ' 8000)
  graces=$(grep -c 'grace ' "$DAEMON_LOG")
  wake; sleep 1.5
  still=$(lock_state)
  echo "sill lock: locked (action) at ${locked} ms; the lock screen went dark at ${off} ms (4 s); a wake at once: '$still'; grace lines before/after: $graces/$(grep -c 'grace ' "$DAEMON_LOG")" \
    | tee -a "$log" >&2
  local ok=1
  [ "$locked" -ge 0 ] && [ "$off" -ge 0 ] && [ "$still" = "locked action" ] \
    && [ "$graces" = "$(grep -c 'grace ' "$DAEMON_LOG")" ] && ok=0
  open_lock || ok=1
  return $ok
}

# 5.
step_inhibit_holds() {
  local log="$OUT_DIR/5-inhibit.log" held after locked gone ok=1
  : >"$log"
  mkfifo "$SCRATCH/client.in"
  "$CLIENT_BIN" "$BUS" <"$SCRATCH/client.in" >"$OUT_DIR/client.log" 2>&1 &
  local client=$!
  exec 4>"$SCRATCH/client.in"
  wake
  echo inhibit >&4; sleep 0.5
  held=$(idle_state)
  sleep 9
  after="$(idle_state) / $(lock_state)"
  echo uninhibit >&4; sleep 0.3
  local start; start=$(now_ms)
  locked=$(until_match "$start" lock_state '^locked idle_screen_off$' 12000)
  open_lock
  echo inhibit >&4; sleep 0.5
  local second; second=$(idle_state)
  exec 4>&-
  wait "$client" 2>/dev/null
  sleep 0.5
  gone=$(idle_state)
  echo "client: $(tr '\n' ' ' <"$OUT_DIR/client.log"); inhibited: '$held'; 9 s later: '$after'; after UnInhibit, idle locked at ${locked} ms; a second inhibit '$second', after its client left '$gone'" \
    | tee -a "$log" >&2
  case "$held" in "own active clear held 1") ;; *) return 1 ;; esac
  case "$after" in "own active clear held 1 / unlocked none") ;; *) return 1 ;; esac
  [ "$locked" -ge 0 ] || return 1
  case "$second" in *" held 1") ;; *) return 1 ;; esac
  case "$gone" in *" free 0") ok=0 ;; esac
  return $ok
}

# 6.
step_sleep_locks() {
  local log="$OUT_DIR/6-sleep.log" start locked still ok=1
  : >"$log"
  wake; start=$(now_ms)
  logind sleep
  locked=$(until_match "$start" lock_state '^locked sleep$' 4000)
  logind wake; sleep 0.5
  wake; sleep 1.5
  still=$(lock_state)
  echo "PrepareForSleep(true): locked (sleep) at ${locked} ms; after the wake and a made-up wake: '$still'; stand-in: $(grep -c '^inhibit sleep delay' "$OUT_DIR/logind.log") sleep delay inhibitor(s) taken" \
    | tee -a "$log" >&2
  [ "$locked" -ge 0 ] && [ "$still" = "locked sleep" ] && ok=0
  open_lock || ok=1
  return $ok
}

# 7.
step_idle_suspend() {
  local log="$OUT_DIR/7-suspend.log" start locked suspended still hints ok=1
  : >"$log"
  stop_daemon
  TIMES="6,10"
  DAEMON_LOG="$OUT_DIR/daemon-suspend.log"
  start_daemon "$DAEMON_LOG" || return 1
  : >"$OUT_DIR/logind.log.mark"
  local before; before=$(grep -c '^suspend ' "$OUT_DIR/logind.log")
  wake; start=$(now_ms)
  locked=$(until_match "$start" lock_state '^locked idle_screen_off$' 12000)
  suspended=-1
  while [ $(( $(now_ms) - start )) -lt 16000 ]; do
    [ "$(grep -c '^suspend ' "$OUT_DIR/logind.log")" -gt "$before" ] && { suspended=$(( $(now_ms) - start )); break; }
    sleep 0.1
  done
  logind sleep; sleep 0.3; logind wake; sleep 0.5
  wake; sleep 1.5
  still=$(lock_state)
  hints=$(grep '^idle-hint' "$OUT_DIR/logind.log" | tr '\n' ' ')
  echo "idle locked at ${locked} ms; Suspend recorded at ${suspended} ms (want ~10000, not ~16000: G342); $(grep '^suspend ' "$OUT_DIR/logind.log" | tail -n1); after sleep and wake and a made-up wake: '$still'; hints: $hints" \
    | tee -a "$log" >&2
  [ "$locked" -ge 0 ] && [ "$suspended" -ge 9000 ] && [ "$suspended" -le 12500 ] \
    && [ "$still" = "locked idle_screen_off" ] \
    && grep -q '^suspend interactive=false' "$OUT_DIR/logind.log" \
    && echo "$hints" | grep -q 'idle-hint true' && echo "$hints" | grep -q 'idle-hint false' && ok=0
  open_lock || ok=1
  return $ok
}

recover() {
  if [ -n "${DAEMON:-}" ]; then
    open_lock >/dev/null 2>&1
    stop_daemon
  fi
  if nested_locker 0 >"$OUT_DIR/recovery.log" 2>&1; then
    echo "recovery: the stand-in locked and unlocked the nested session (it is unlocked)" >&2
  else
    echo "recovery: the stand-in exited $? (see $OUT_DIR/recovery.log)" >&2
  fi
  exec 3>&- 2>/dev/null
  [ -n "$LOGIND" ] && kill "$LOGIND" 2>/dev/null
  [ -n "$BUS_PID" ] && kill "$BUS_PID" 2>/dev/null
  rm -rf "$SCRATCH"
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
DAEMON_LOG="$OUT_DIR/daemon-wdebug.log"
if start_bus && start_logind && WDEBUG=client start_daemon "$DAEMON_LOG"; then
  for step in ${STEPS:-step_dim_off_lock step_wake_in_grace step_wake_after_grace \
    step_manual_lock_asks step_inhibit_holds step_sleep_locks step_idle_suspend}; do
    if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; fi
  done
else
  status=1
fi
recover
trap - EXIT
echo "logs in $OUT_DIR"
exit "$status"

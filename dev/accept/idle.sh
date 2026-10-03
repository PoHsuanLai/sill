#!/usr/bin/env bash
# sill's own idle acceptance, on the nested cosmic-comp ONLY (dev/nested.sh; wayland-1 is pinged
# first, reused, and never stopped or restarted). Its "displays" are windows on the host, so
# powering them off darkens nothing real.
#
# SAFETY. Nothing here reaches logind, the session bus's org.freedesktop.ScreenSaver, PAM, a
# backlight or wayland-0:
# - one private dbus-daemon (dev/private-bus.conf) is both the daemon's system and session bus;
#   examples/logind_standin.rs owns org.freedesktop.login1 there and only RECORDS Suspend,
#   SetIdleHint and Inhibit (it sends PrepareForSleep and Lock on command);
# - the daemon gets its own XDG_RUNTIME_DIR, XDG_CONFIG_HOME, XDG_STATE_HOME and XDG_CACHE_HOME,
#   the nested socket as an absolute WAYLAND_DISPLAY (the script refuses wayland-0), and
#   XDG_CURRENT_DESKTOP=COSMIC so it resolves a sill session (Host::SillSession) and owns idle;
# - short idle times come from `sill run --idle-times`, which only a debug build takes;
# - no password is typed: an asked lock is opened with `sill debug auth accept` (debug build);
# - no input is typed anywhere: a wake is `sill debug idle-wake` (debug build), a made-up
#   `resumed` (the nested cosmic-comp counts no synthetic input as activity);
# - whatever fails, the script ends by accepting on the daemon, quitting it, and running the
#   recovery lock-and-unlock (`session_locker 0`), so the nested session is left unlocked.
#
# Waits: a functional check waits on the state it needs (`until_match`, `until_idle`, a capture
# polled until the scrim is at its level), never on a fixed sleep a loaded host stretches; a
# time against a budget goes through `budget_check` and is enforced only on a release build on a
# quiet host. A step that cannot show its condition because the host was too slow (a wake past
# the grace) skips instead of failing.
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
#      (non-fatal). Once the displays are off the surface is unmapped again. The power-off order
#      is looked for in the WAYLAND_DEBUG=client log (set_mode(off) before
#      ext_session_lock_v1.locked, and no `failed`); the nested winit outputs refuse power
#      control, so it cannot be shown here.
#   2. A wake within the grace: unlocked with no password (the daemon logs "grace NoPassword").
#   3. Idle again, then a wake after the grace (7 s): still locked ("grace Ask"); accept opens it.
#   4. `sill lock` (cause action), the lock screen goes dark after 4 s, a wake at once: still
#      locked, no grace.
#   5. A ScreenSaver.Inhibit from a test client holds idle for 9 s (> 6 s: no dim, no off, no
#      lock; `sill idle status` counts it); UnInhibit lets idle lock again; a second inhibit
#      ends when its client leaves the bus.
#   6. PrepareForSleep(true) from the stand-in locks with cause sleep; after the wake, a made-up wake:
#      still locked.
#   7. --idle-times 6,10: off and lock at 6 s, logind Suspend(false) recorded at about 10 s
#      from the last wake (the lock's new watches do not restart the suspend watch), the
#      idle hint set true then false; after PrepareForSleep true/false and a wake: still locked.
#
# Exit 0 when every step passed, 1 on a failure. Another sill daemon started from a
# `target/*/sill run` is waited for (up to 10 minutes).
#
# Usage: dev/nested.sh && dev/accept/idle.sh
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_TARGET/accept-idle"
SHOTS="$SILL_REPO/dev/out"
DIM_SURFACE=
mkdir -p "$OUT_DIR" "$SHOTS"
LOCKER="$SILL_OUT/examples/session_locker"
LOGIND_BIN="$SILL_OUT/examples/logind_standin"
CLIENT_BIN="$SILL_OUT/examples/screensaver_client"

use_nested_socket || exit 1
case "$(basename "$WL")" in
  wayland-0) echo "the nested socket is wayland-0 (the real session): refusing" >&2; exit 1 ;;
esac
scratch_new
mkdir -p "$CONFIG/sill"
LOGIND=
TIMES="6,0"
DAEMON_LOG="$OUT_DIR/daemon.log"
# The daemon resolves a sill session (Host::SillSession) and owns idle only as COSMIC.
COSMIC_ENV=(XDG_CURRENT_DESKTOP=COSMIC)
IN_ENV_EXTRA=("${COSMIC_ENV[@]}")

nested_locker() { WAYLAND_DISPLAY="$WL" "$LOCKER" "$@"; }
# A wake: `sill debug idle-wake` stands in for the compositor's `resumed` (the nested
# cosmic-comp 1.8 counts no virtual-keyboard input as activity, and a key typed into the host's
# window of it would be real input on the person's own session).
# It also holds every idle notification afresh, so the idle timers count from here.
wake() { sill debug idle-wake >/dev/null; }
# A command to the logind stand-in.
logind() { echo "$*" >&3; }

# `sill lock status` as "phase cause" (cause null while unlocked).
lock_state() { sill lock status 2>/dev/null | python3 -c '
import json, sys
r = json.loads(sys.stdin.read())["v"]["v"]
print(r["phase"], r["cause"] or "none")' 2>/dev/null; }

# `sill idle status` as "mode phase dim inhibition inhibits".
idle_state() { sill idle status 2>/dev/null | python3 -c '
import json, sys
r = json.loads(sys.stdin.read())["v"]["v"]
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

# ms until `sill idle status` matches the regex $1, or -1 within ${2:-5000} ms: a step waits on the
# state it needs, never on a sleep that a loaded host stretches.
until_idle() { until_match "$(now_ms)" idle_state "$1" "${2:-5000}"; }

# The scrim of the dim surface once it has faded in to its level: polls captures until one is
# RGB 0 everywhere at alpha 128 +- 12 (the 50 % level), for at most $2 ms; prints the last good
# capture's "width height rgb_max alpha_min alpha_max" (nothing when none was ever taken) and
# leaves that PNG at $1. The fade is 2 s; a slow capture only moves the poll, never the verdict.
capture_dim_at_level() {
  local png=$1 limit=$2 start seen="" amax
  start=$(now_ms)
  while [ $(( $(now_ms) - start )) -le "$limit" ]; do
    if capture_dim "$png.try.png"; then
      seen=$(scrim_of "$png.try.png")
      mv "$png.try.png" "$png"
      read -r _ _ _ _ amax <<<"$seen"
      [ "${amax:-0}" -ge 116 ] && [ "${amax:-0}" -le 140 ] && break
    fi
    sleep 0.05
  done
  echo "$seen"
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

start_logind() {
  mkfifo "$SCRATCH/logind.in"
  "$LOGIND_BIN" "$BUS" <"$SCRATCH/logind.in" >"$OUT_DIR/logind.log" 2>&1 &
  LOGIND=$!
  exec 3>"$SCRATCH/logind.in"
  await_logind "^ready$" || { echo "the logind stand-in never started" >&2; return 1; }
}

# Starts the daemon with the short idle times in TIMES; "$@" (NAME=VALUE) adds variables to its
# environment for its launch only.
start_idle_daemon() {
  backdrop_seed "$CONFIG" || return 1
  printf '[session]\nlock_grace_s = 5\n\n[idle]\ndim_s = 3\ndim_level_pct = 50\nlocked_screen_off_s = 4\n' \
    >"$CONFIG/sill/settings.toml"
  IN_ENV_EXTRA=("${COSMIC_ENV[@]}" "$@")
  launch_daemon --idle-times "$TIMES"
  local launched=$?
  IN_ENV_EXTRA=("${COSMIC_ENV[@]}")
  [ "$launched" = 0 ] || return 1
  [ "$(until_match "$(now_ms)" idle_state '^own ' 5000)" -ge 0 ] \
    || { echo "idle is not own: $(idle_state)" >&2; return 1; }
  sleep 2
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
  name=$(output_name) || return 1
  DIM_SURFACE="sill-idle-dim-$name"
  wake; start=$(now_ms)
  rm -f "$OUT_DIR"/dim-*.png
  if capture_dim "$OUT_DIR/dim-before.png"; then before=mapped; else before=unmapped; fi
  dim=$(until_match "$start" idle_state ' dimmed dimmed:50 ' 20000)
  # The early capture is the first one after the dim state appears; the late one waits for the
  # scrim to reach its level (a state), and the nested screen is kept at that moment.
  capture_dim "$OUT_DIR/dim-early.png" && early=$(scrim_of "$OUT_DIR/dim-early.png")
  late=$(capture_dim_at_level "$OUT_DIR/dim-late.png" 4000)
  local late_at=$(( $(now_ms) - start ))
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/idle-dim-cosmic.png" >>"$log" 2>&1 \
    || echo "nested screenshot failed (non-fatal)" >&2
  cp "$OUT_DIR/dim-late.png" "$SHOTS/idle-dim.png" 2>/dev/null
  off=$(until_match "$start" idle_state ' displays_off ' 25000)
  locked=$(until_match "$start" lock_state '^locked idle_screen_off$' 30000)
  # The surface is unmapped once the displays are off: wait for it, never a fixed sleep.
  after=mapped
  for _ in $(seq 50); do
    capture_dim "$OUT_DIR/dim-after.png" || { after=unmapped; break; }
    sleep 0.1
  done
  echo "dim surface $DIM_SURFACE: before the dim $before; early '${early:-no capture}'; at ${late_at} ms '${late:-no capture}' (width height rgb_max alpha_min alpha_max); after the displays went off $after" \
    | tee -a "$log" >&2
  # Functional: black, one alpha across the surface, the late capture at 50 %, the early one not
  # darker than it. Budget: the early capture caught the fade in progress (a slow capture on a
  # loaded host may see it already finished).
  local scrim_ok=1 fading=0
  fading=$(python3 - "${early:-}" "${late:-}" <<'EOF'
import sys
early, late = sys.argv[1].split(), sys.argv[2].split()
if len(early) != 5 or len(late) != 5:
    sys.exit(1)
e, l = [int(v) for v in early], [int(v) for v in late]
black = e[2] == 0 and l[2] == 0
uniform = e[3] == e[4] and l[3] == l[4]
not_darker = 0 < e[4] <= l[4]
at_level = abs(l[4] - 128) <= 12
if not (black and uniform and not_darker and at_level):
    sys.exit(1)
print(1 if e[4] < l[4] else 0)
EOF
  ) && scrim_ok=0
  if [ "$scrim_ok" = 0 ]; then
    echo "scrim: black, uniform, at 50 % once faded in" | tee -a "$log" >&2
    budget_check "the early capture caught the fade in progress (lighter than the late one)" "$fading" || scrim_ok=1
  fi
  g341=$(python3 - "$DAEMON_LOG" <<'EOF'
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
  echo "dimmed at ${dim} ms (want ~3000), displays off at ${off} ms (~6000), locked with cause idle_screen_off at ${locked} ms; power-off order: $(echo "$g341" | tr '\n' ' ')" \
    | tee -a "$log" >&2
  echo "idle: $(idle_state); lock: $(lock_state)" | tee -a "$log" >&2
  # Functional: idle never fires early, and does fire. The upper bounds are budgets.
  [ "$dim" -ge 2000 ] && [ "$off" -ge 5000 ] && [ "$locked" -ge 0 ] \
    && [ "$before" = unmapped ] && [ "$after" = unmapped ] && [ "$scrim_ok" = 0 ] \
    && budget_check "dimmed ${dim} ms, displays off ${off} ms after the wake against 4500 and 8000 ms" \
      "$(( dim <= 4500 && off <= 8000 ))"
}

# 2.
step_wake_in_grace() {
  local log="$OUT_DIR/2-wake-in-grace.log" start locked opened grace
  : >"$log"
  # A lock of its own, so the grace (5 s from the lock) is not spent on step 1's captures: idle
  # locks, and a wake is made the moment the state says it is locked.
  open_lock || return 1
  wake; start=$(now_ms)
  locked=$(until_match "$start" lock_state '^locked idle_screen_off$' 30000)
  [ "$locked" -ge 0 ] || { echo "idle never locked" | tee -a "$log" >&2; return 1; }
  start=$(now_ms); wake
  opened=$(until_match "$start" lock_state '^unlocked' 5000)
  grace=$(grep -o 'grace [A-Za-z]* (woke after [0-9]* ms)' "$DAEMON_LOG" | tail -n1)
  echo "a wake as soon as the idle lock was seen: unlocked ${opened} ms later with no password; daemon: $grace; idle: $(idle_state)" \
    | tee -a "$log" >&2
  # A host so loaded that the wake itself came after the 5 s grace cannot show the grace.
  case "$grace" in
    "grace Ask"*) open_lock; echo "the wake came after the grace window: the host is too slow to show it" >&2; return 2 ;;
  esac
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
  echo inhibit >&4
  # The inhibit is registered when idle says so, not after a fixed half second.
  [ "$(until_idle ' held 1$' 10000)" -ge 0 ] || echo "the inhibit was never registered" >&2
  held=$(idle_state)
  sleep 9
  after="$(idle_state) / $(lock_state)"
  echo uninhibit >&4
  [ "$(until_idle ' free 0$' 10000)" -ge 0 ] || echo "the inhibit was never released" >&2
  local start; start=$(now_ms)
  locked=$(until_match "$start" lock_state '^locked idle_screen_off$' 12000)
  open_lock
  echo inhibit >&4
  [ "$(until_idle ' held 1$' 10000)" -ge 0 ] || echo "the second inhibit was never registered" >&2
  local second; second=$(idle_state)
  exec 4>&-
  wait "$client" 2>/dev/null
  # Its client left the bus: the daemon notices on its own time.
  [ "$(until_idle ' free 0$' 10000)" -ge 0 ] || echo "the second inhibit was never dropped" >&2
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
  # Functional: locked within 15 s; the 4 s (inside logind's 5 s inhibitor delay) is a budget.
  locked=$(until_match "$start" lock_state '^locked sleep$' 15000)
  logind wake; sleep 0.5
  wake; sleep 1.5
  still=$(lock_state)
  echo "PrepareForSleep(true): locked (sleep) at ${locked} ms; after the wake and a made-up wake: '$still'; stand-in: $(grep -c '^inhibit sleep delay' "$OUT_DIR/logind.log") sleep delay inhibitor(s) taken" \
    | tee -a "$log" >&2
  [ "$locked" -ge 0 ] && budget_check "locked ${locked} ms after PrepareForSleep against 4000 ms" "$(( locked <= 4000 ))" \
    && [ "$still" = "locked sleep" ] && ok=0
  open_lock || ok=1
  return $ok
}

# 7.
step_idle_suspend() {
  local log="$OUT_DIR/7-suspend.log" start locked suspended still hints ok=1
  : >"$log"
  stop_daemon || return 1
  mv "$DAEMON_LOG" "$OUT_DIR/daemon-wdebug.log"
  TIMES="6,10"
  start_idle_daemon || return 1
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
  echo "idle locked at ${locked} ms; Suspend recorded at ${suspended} ms (want ~10000, not ~16000); $(grep '^suspend ' "$OUT_DIR/logind.log" | tail -n1); after sleep and wake and a made-up wake: '$still'; hints: $hints" \
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
  stop_private_bus
  scratch_drop
}

WAYLAND_DISPLAY="$WL" wayland-info >/dev/null 2>&1 || {
  echo "no nested compositor at $WL (dev/nested.sh first)" >&2; exit 1; }
build "-p sill-testkit --example session_locker --example logind_standin --example screensaver_client" || exit 1
await_no_process || exit 1
trap recover EXIT
status=0
if start_private_bus && start_logind && start_idle_daemon WAYLAND_DEBUG=client; then
  run_steps ${STEPS:-step_dim_off_lock step_wake_in_grace step_wake_after_grace \
    step_manual_lock_asks step_inhibit_holds step_sleep_locks step_idle_suspend}
  status=$STATUS
else
  status=1
fi
recover
trap - EXIT
echo "logs in $OUT_DIR"
exit "$status"

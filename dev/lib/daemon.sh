# Building, starting, asking and stopping the daemon under test. Expects OUT_DIR (the script's
# scratch output directory) and the variables scratch_new, start_kwin or the nested setup made:
# RUNTIME, WL, BUS, HOME_DIR, CONFIG, DATA, STATE, CACHE. IN_ENV_EXTRA (an array of NAME=VALUE) adds
# variables to every in_env call; IN_ENV_UNSET (an array of names) removes them.
IN_ENV_EXTRA=()
IN_ENV_UNSET=()
DAEMON=

# Runs "$@" with every ambient thing pointed at the script's own: its compositor, the private bus
# as BOTH the session and the system bus, and its scratch HOME and XDG directories (so its IPC
# socket is never the real one's and nothing reads the real ~/.config or ~/.cache).
in_env() {
  local unsets=() name
  for name in "${IN_ENV_UNSET[@]}"; do unsets+=(-u "$name"); done
  env "${unsets[@]}" HOME="${HOME_DIR:-$HOME}" XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$WL" \
    DBUS_SESSION_BUS_ADDRESS="$BUS" DBUS_SYSTEM_BUS_ADDRESS="$BUS" XDG_CONFIG_HOME="$CONFIG" \
    XDG_DATA_HOME="$DATA" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" \
    "${IN_ENV_EXTRA[@]}" "$@"
}

# The daemon's CLI, in the daemon's environment.
sill() { in_env "$SILL_BIN" "$@"; }

# Builds the debug daemon plus, as further arguments "$@", other cargo invocations' arguments
# (each one a quoted string such as "-p sill-testkit --example badge_emitter"), into
# $OUT_DIR/build.log.
build() {
  local extra
  {
    (cd "$SILL_REPO" && cargo build -p sill --features debug) || return 1
    for extra in "$@"; do
      # shellcheck disable=SC2086 # one word per cargo argument
      (cd "$SILL_REPO" && cargo build $extra) || return 1
    done
  } >"$OUT_DIR/build.log" 2>&1 || { echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

# Starts `sill run` in the background on the private buses with the fake DDC, its log in
# $OUT_DIR/daemon.log and the arguments "$@" after `run`; sets DAEMON and waits for `ping`.
launch_daemon() {
  use_fake_ddc "$CONFIG" >/dev/null
  require_private_system_bus "$BUS" || return 1
  require_fake_ddc || return 1
  in_env "$SILL_BIN" run "${FAKE_DDC_FLAGS[@]}" "$@" >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  wait_ping
}

# Waits up to 10 s for the daemon to answer `ping`.
wait_ping() {
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && return 0; sleep 0.1; done
  echo "the daemon never answered ping (see $OUT_DIR/daemon.log)" >&2
  return 1
}

# Quits the daemon through IPC and waits up to 10 s for it to end on its own; a daemon that does
# not is killed and the call fails (a daemon that stops ending on `quit` fails the script instead
# of being swept away). A no-op when none runs.
stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  local result=0
  in_env timeout 10 "$SILL_BIN" quit >/dev/null 2>&1
  for _ in $(seq 100); do kill -0 "$DAEMON" 2>/dev/null || break; sleep 0.1; done
  if kill -0 "$DAEMON" 2>/dev/null; then
    echo "the daemon ($DAEMON) did not end after quit" >&2
    kill "$DAEMON" 2>/dev/null
    result=1
  fi
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  return "$result"
}

# The first surface of `sill debug stats` named PREFIX... (popups, `NAME/menu`, never match).
surface_named() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith(sys.argv[1]) and "/" not in s:
        print(s); break' "$1"
}

# The output's connector name (the surfaces' namespace suffix), read off the bar; waits up to 5 s.
output_name() {
  local bar
  for _ in $(seq 50); do
    bar=$(surface_named sill-bar-)
    [ -n "$bar" ] && { echo "${bar#sill-bar-}"; return 0; }
    sleep 0.1
  done
  echo "no bar in 'sill debug stats'" >&2
  return 1
}

# A surface's cumulative frame counter, and its `frames callbacks` pair, from `sill debug stats`
# (which drains the timing samples; the counters stay cumulative).
frames_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"])' "$1"
}
counters_of() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"] == sys.argv[1]:
        print(row["frames"], row["callbacks"])' "$1"
}

# How many daemon-log lines match $1.
count_log() { grep -c -- "$1" "$OUT_DIR/daemon.log"; }

# Waits up to $2 tenths of a second for the daemon's log to hold $3 more lines matching $1 than
# it did when counted as $4.
wait_log() {
  local pattern="$1" tenths="$2" more="$3" before="$4"
  for _ in $(seq "$tenths"); do
    [ "$(count_log "$pattern")" -ge $(( before + more )) ] && return 0
    sleep 0.1
  done
  return 1
}

# The dry run is in force for surface $1's backend (`power_menu`): the daemon said so, every
# surface of it mounted on it, none on the live session. Sets DRY_RUN=proven or unproven; $RECORD
# names the file the dry run records into.
DRY_RUN=unproven
prove_dry_run() {
  local backend="$1" log="$OUT_DIR/dry-run.log"
  DRY_RUN=unproven
  {
    grep "sill: session: dry run, recording to $RECORD" "$OUT_DIR/daemon.log" || return 1
    grep "$backend.backend=dry-run" "$OUT_DIR/daemon.log" || return 1
    if grep "$backend.backend=live" "$OUT_DIR/daemon.log"; then return 1; fi
  } >"$log" 2>&1
  DRY_RUN=proven
  echo "dry run proven from the daemon's log (record file $RECORD)" | tee -a "$log" >&2
}

# Waits for process $1 (named $2) to end for 5 s, then ends it.
reap() {
  for _ in $(seq 50); do kill -0 "$1" 2>/dev/null || return 0; sleep 0.1; done
  echo "the $2 ($1) did not end; killing it" >&2
  kill "$1" 2>/dev/null
  sleep 0.5
  kill -9 "$1" 2>/dev/null
  return 0
}

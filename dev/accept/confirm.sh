#!/usr/bin/env bash
# The confirmation sheet and the seat at intentd, end to end, with the REAL intentd and `quire-do`
# from docket and the real sill daemon, on a private headless KWin started and stopped by the
# script (a private copy of shell-host's scripts/dev, so nothing of another session's
# `.kwin-env` is read or written). The chain under test is the one a login has:
#
#   quire-do  ->  intentd (the gate)  ->  org.quire.Confirm1 (sill's sheet)  ->  the person's answer
#                                    ->  org.quire.Shell IntentProvider1 (sill does the action)
#
# Nothing here reaches the person's session bus, logind, polkit, ~/.config or a real keyboard.
# One private bus is the daemon's session AND system bus (start_compositor refuses anything else);
# intentd, quire-do and sill all run in a scratch HOME and scratch XDG directories, with an
# XDG_DATA_DIRS that names nothing, so the only manifests are the two the script copies in
# (`dist/intents/org.quire.Shell.toml`, and docket's `org.quire.Memory.toml` for a hidden action).
# The person's answers are `sill debug key SURFACE KEY`: shell-host's Inject::Key, which enters
# the sheet where an xkb key does, whether or not the compositor gave it the keyboard.
#
# docket is built from $DOCKET_DIR (default ../docket beside this repository) into its own target
# directory, $OUT_DIR/docket-target (override with DOCKET_TARGET), never docket's own.
#
#   1. `quire-do apps` lists `shell`, and `quire-do shell --list` its actions.
#   2. A read (`shell launcher.open`) runs with no sheet: exit 0.
#   3. A write (`shell dock.pin`) asks: the sheet shows (captured into dev/out/confirm.png), then
#      Return allows it once it is armed: exit 0, the pin is in the dock's file, and `quire-do
#      undo --last` takes it back through the provider.
#   4. Escape denies: exit 4, nothing pinned.
#   5. "Allow from the terminal: this action, until logout" (Shift+Tab to it, Return): exit 0,
#      `sill intents status` lists the grant, and the next `dock.pin` asks nobody.
#   6. `sill intents revoke` takes the grant away: the next call asks again.
#   7. A destructive action (`shell screenshot`) asks for a HOLD: a tap of Space on the hold
#      button answers nothing, Space held for 1.5 s allows it: exit 0.
#   8. Three refused calls in a row (a hidden action, exit 3) pause the terminal: the next call
#      exits 7; `sill intents resume` lets it go on.
#   9. With intentd gone, `quire-do` exits 6, no sheet is left on screen, and sill is still alive.
#
# Exit 0 when every step passed, 1 on a failure, 2 when a step could not run.
#
# Usage: dev/accept/confirm.sh
set -uo pipefail
SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SELF/../.." && pwd)"
PRIVATE_DEV="$(mktemp -d)"
cp -r "${SHELL_HOST_DEV:-$REPO/../shell-host/scripts/dev}" "$PRIVATE_DEV/dev"
rm -f "$PRIVATE_DEV/dev/.kwin-env"
export SHELL_HOST_DEV="$PRIVATE_DEV/dev"
# shellcheck source=dev/lib/all.sh
source "$SELF/../lib/all.sh"
OUT_DIR="$SILL_OUT_ROOT/accept-confirm"
SHOTS="$SILL_SHOTS"
mkdir -p "$OUT_DIR" "$SHOTS"
DOCKET_DIR="${DOCKET_DIR:-$SILL_REPO/../docket}"
DOCKET_TARGET="${DOCKET_TARGET:-$OUT_DIR/docket-target}"
INTENTD="$DOCKET_TARGET/debug/accept-intentd"
QUIRE_DO="$DOCKET_TARGET/debug/quire-do"
INTENTD_PID=
ASKER=
SHEET=

scratch_new
mkdir -p "$CONFIG/sill" "$DATA/quire/intents"

# intentd names a caller that owns no bus name by its cgroup, and only a terminal's child
# (`vte-spawn-*.scope`) is the cli that `quire-do` must be; this script runs in whatever scope its
# caller has (a gate, an agent's shell, a service), so it presents `quire-do` as a terminal's
# child: it runs docket-accept's `accept-intentd` (the shipped intentd ignores INTENTD_PROC_ROOT),
# which reads its callers from a scratch proc root, where `as_terminal` writes the cgroup of the
# process it then execs.
PROC_ROOT="$OUT_DIR/proc"
as_terminal() {
  mkdir -p "$PROC_ROOT/$$"
  echo "0::/user.slice/user-1000.slice/user@1000.service/app.slice/vte-spawn-accept.scope" >"$PROC_ROOT/$$/cgroup"
  exec "$@"
}

# intentd and the daemon each know the other by its systemd unit: intentd gives the sheet host's
# role only to the main process of sill.service (the system's callers file, docket
# dist/intentd-callers.toml, handed over as INTENTD_SYSTEM_CALLERS), and the daemon's provider
# answers only intentd.service's main process. Each judges the other at its first call, at the
# start, so each is started through `as_unit UNIT`, which names its own process in the scratch proc
# root as docket's own harness does (world.rs `place`): its cgroup, and it as the unit's main
# process. It then execs the program, which keeps the pid. The daemon reads that root
# (`--proc-root`), intentd too (INTENTD_PROC_ROOT).
as_unit() {
  local unit="$1"
  shift
  mkdir -p "$PROC_ROOT/$$" "$PROC_ROOT/units"
  echo "0::/user.slice/user-1000.slice/user@1000.service/app.slice/$unit.service" >"$PROC_ROOT/$$/cgroup"
  echo "$$" >"$PROC_ROOT/units/$unit.service"
  exec "$@"
}
unit_prefix() { UNIT_PREFIX=(bash -c "$(declare -f as_unit); PROC_ROOT=\$0 as_unit \"\$@\"" "$PROC_ROOT" "$1"); }

# `quire-do ARGS` in the scratch environment, to completion.
quire() { in_env bash -c "$(declare -f as_terminal); PROC_ROOT=\$0 as_terminal \"\$@\"" "$PROC_ROOT" "$QUIRE_DO" "$@"; }

# `quire-do ARGS` in the background: its output and exit code go to $OUT_DIR/NAME.{out,code}.
quire_bg() {
  local name="$1"
  shift
  rm -f "$OUT_DIR/$name.code"
  ( quire "$@" >"$OUT_DIR/$name.out" 2>&1; echo $? >"$OUT_DIR/$name.code" ) &
  ASKER=$!
}

# The exit code of the background call NAME, waiting up to ${2:-10} s; -1 when it never ended.
code_of() {
  for _ in $(seq $(( ${2:-10} * 10 ))); do
    [ -f "$OUT_DIR/$1.code" ] && { cat "$OUT_DIR/$1.code"; return; }
    sleep 0.1
  done
  echo -1
}

# One field of `sill intents status`: link, showing, waiting, grants (space-separated), breaker,
# reach.
intents_field() {
  sill intents status 2>/dev/null | python3 -c '
import json, sys
r = json.loads(sys.stdin.read())["v"]["v"]
v = r[sys.argv[1]]
print(" ".join(v) if isinstance(v, list) else ("" if v is None else v))' "$1"
}

# Waits up to ${3:-10} s for `intents_field $1` to equal $2.
await_field() {
  local got=
  for _ in $(seq $(( ${3:-10} * 10 ))); do
    got=$(intents_field "$1")
    [ "$got" = "$2" ] && return 0
    sleep 0.1
  done
  echo "intents $1 is '$got', wanted '$2'" >&2
  return 1
}

# Waits up to ${1:-10} s for a sheet to be showing; prints its id.
await_sheet() {
  local id=
  for _ in $(seq $(( ${1:-10} * 10 ))); do
    id=$(intents_field showing)
    [ -n "$id" ] && { echo "$id"; return 0; }
    sleep 0.1
  done
  return 1
}

# Past the arming (500 ms) with margin, so a key counts.
armed() { sleep 1; }
key() { sill debug key "$SHEET" "$@" >>"$OUT_DIR/keys.log" 2>&1; }

# The sheet's capture into $1: mapped and drawn.
capture_sheet() {
  sill debug capture "$SHEET" "$1" >>"$OUT_DIR/capture.log" 2>&1 || return 1
  [ "$(opaque_pixels "$1")" -gt 1000 ]
}

# The sheet's capture into $1, waiting up to 20 s for it to be drawn (a loaded host takes a while).
await_captured() {
  for _ in $(seq 100); do capture_sheet "$1" && return 0; sleep 0.2; done
  return 1
}

# No question left over from a step that failed: whatever is on screen is denied, so one failure
# does not become every later one.
fresh() {
  for _ in 1 2 3 4 5; do
    [ -z "$(intents_field showing)" ] && return 0
    key Escape
    sleep 0.5
  done
  echo "a sheet will not go away" >&2
  return 1
}

start_intentd() {
  mkdir -p "$PROC_ROOT"
  cp "$DOCKET_DIR/dist/intentd-callers.toml" "$OUT_DIR/intentd-callers.toml" || return 1
  unit_prefix intentd
  in_env env INTENTD_PROC_ROOT="$PROC_ROOT" INTENTD_SYSTEM_CALLERS="$OUT_DIR/intentd-callers.toml" \
    "${UNIT_PREFIX[@]}" "$INTENTD" >"$OUT_DIR/intentd.log" 2>&1 &
  INTENTD_PID=$!
  # intentd owns org.quire.Intents1 once `quire-do apps` answers.
  for _ in $(seq 100); do
    quire apps >/dev/null 2>&1 && return 0
    kill -0 "$INTENTD_PID" 2>/dev/null || { echo "intentd died (see $OUT_DIR/intentd.log)" >&2; return 1; }
    sleep 0.1
  done
  echo "intentd never answered (see $OUT_DIR/intentd.log)" >&2
  return 1
}

# Whether the dock's pins file holds the app $1.
pinned() { grep -rqs -- "$1" "$CONFIG" "$DATA" "$STATE" 2>/dev/null; }

# 1.
step_manifest_listed() {
  local apps list
  apps=$(quire apps 2>&1) || { echo "quire-do apps failed: $apps" >&2; return 1; }
  list=$(quire shell --list 2>&1) || { echo "quire-do shell --list failed: $list" >&2; return 1; }
  echo "$apps" | tee "$OUT_DIR/1-apps.log" >&2
  echo "$list" | tee -a "$OUT_DIR/1-apps.log" >&2
  echo "$apps" | grep -qw shell || return 1
  for action in launcher.open workspace.switch dock.pin dock.unpin lock screenshot; do
    echo "$list" | grep -q "$action" || { echo "shell has no $action" >&2; return 1; }
  done
}

# 2.
step_a_read_asks_nobody() {
  fresh || return 1
  quire shell launcher.open --query mail >"$OUT_DIR/2.out" 2>&1
  local code=$?
  echo "launcher.open: exit $code, sheet '$(intents_field showing)'" | tee "$OUT_DIR/2.log" >&2
  [ "$code" = 0 ] && [ -z "$(intents_field showing)" ]
}

# 3.
step_a_write_asks_and_allow_pins() {
  fresh || return 1
  local log="$OUT_DIR/3-allow.log" id code
  : >"$log"
  quire_bg allow shell dock.pin --app e2e-allow.desktop
  id=$(await_sheet 10) || { echo "no sheet appeared" | tee -a "$log" >&2; return 1; }
  echo "the sheet for $id is up" | tee -a "$log" >&2
  # Past the arming (and the sheet's entrance): the picture is of the armed sheet, and Return
  # allows. (That Return is lost while it arms is the harness's check on its virtual clock: from
  # outside, the arming's 500 ms cannot be told from a slow poll.)
  armed
  await_captured "$SHOTS/confirm.png" || { echo "the sheet did not capture" | tee -a "$log" >&2; return 1; }
  for _ in 1 2 3; do
    key Enter
    [ -f "$OUT_DIR/allow.code" ] && break
    sleep 0.7
  done
  code=$(code_of allow)
  echo "dock.pin allowed: exit $code; pinned: $(pinned e2e-allow.desktop && echo yes || echo no); sheet '$(intents_field showing)'" | tee -a "$log" >&2
  [ "$code" = 0 ] && pinned e2e-allow.desktop && [ -z "$(intents_field showing)" ] || return 1
  # The provider takes it back: the pin leaves the file.
  quire undo --last >"$OUT_DIR/3-undo.out" 2>&1
  code=$?
  sleep 0.5
  echo "undo --last: exit $code; pinned: $(pinned e2e-allow.desktop && echo yes || echo no)" | tee -a "$log" >&2
  [ "$code" = 0 ] && ! pinned e2e-allow.desktop
}

# 4.
step_escape_denies() {
  fresh || return 1
  local log="$OUT_DIR/4-deny.log" code
  quire_bg deny shell dock.pin --app e2e-deny.desktop
  await_sheet 10 >/dev/null || { echo "no sheet appeared" | tee "$log" >&2; return 1; }
  key Escape
  code=$(code_of deny)
  echo "Escape: exit $code; pinned: $(pinned e2e-deny.desktop && echo yes || echo no)" | tee "$log" >&2
  [ "$code" = 4 ] && ! pinned e2e-deny.desktop
}

# 5.
step_allow_from_the_terminal() {
  fresh || return 1
  local log="$OUT_DIR/5-terminal.log" code
  quire_bg term shell dock.pin --app e2e-term.desktop
  await_sheet 10 >/dev/null || { echo "no sheet appeared" | tee "$log" >&2; return 1; }
  armed
  # The keyboard is on Allow; the terminal's offer is three buttons back (Don't Allow, Never,
  # then it: the offer sits above the footer's Never, Don't Allow and Allow).
  key Tab+shift
  key Tab+shift
  key Tab+shift
  key Enter
  code=$(code_of term)
  await_field grants "org.quire.Shell:shell.dock.pin" 10 || return 1
  echo "allow from the terminal: exit $code; grants: $(intents_field grants)" | tee "$log" >&2
  [ "$code" = 0 ] || return 1
  # The grant stands: the next call asks nobody.
  quire shell dock.pin --app e2e-term2.desktop >"$OUT_DIR/5-again.out" 2>&1
  code=$?
  echo "the next dock.pin: exit $code, sheet '$(intents_field showing)'" | tee -a "$log" >&2
  [ "$code" = 0 ] && [ -z "$(intents_field showing)" ] && pinned e2e-term2.desktop
}

# 6.
step_revoke_asks_again() {
  fresh || return 1
  local log="$OUT_DIR/6-revoke.log" code
  sill intents revoke org.quire.Shell:shell.dock.pin >>"$log" 2>&1 || return 1
  await_field grants "" 10 || return 1
  quire_bg again shell dock.pin --app e2e-term3.desktop
  await_sheet 10 >/dev/null || { echo "no sheet after the revoke" | tee -a "$log" >&2; return 1; }
  key Escape
  code=$(code_of again)
  echo "after the revoke the call asked again; Escape: exit $code" | tee -a "$log" >&2
  [ "$code" = 4 ]
}

# 7.
step_a_destructive_action_asks_for_a_hold() {
  fresh || return 1
  local log="$OUT_DIR/7-hold.log" code
  quire_bg hold shell screenshot --kind screen
  await_sheet 10 >/dev/null || { echo "no sheet appeared" | tee "$log" >&2; return 1; }
  armed
  await_captured "$SHOTS/confirm-hold.png" || echo "the hold sheet did not capture (non-fatal)" >&2
  # The keyboard is on Don't Allow; Tab reaches the hold button. A tap is not a hold.
  key Tab
  key Space
  sleep 0.3
  [ -n "$(intents_field showing)" ] || { echo "a tap answered a destructive action" | tee -a "$log" >&2; return 1; }
  # Held for a second, then let go.
  key Space --action press
  sleep 1.5
  key Space --action release
  code=$(code_of hold)
  echo "screenshot held through: exit $code; sheet '$(intents_field showing)'" | tee -a "$log" >&2
  [ "$code" = 0 ] && [ -z "$(intents_field showing)" ]
}

# 8.
step_the_breaker_pauses_and_resume_lets_go() {
  fresh || return 1
  local log="$OUT_DIR/8-breaker.log" code
  : >"$log"
  # Denials count toward the breaker too: start from a fresh one.
  sill intents resume >>"$log" 2>&1 || return 1
  sleep 0.5
  for key in f1 f2 f3; do
    quire memory forget "memory.fact:$key" >>"$log" 2>&1
    code=$?
    [ "$code" = 3 ] || { echo "a hidden action exited $code, not 3" | tee -a "$log" >&2; return 1; }
  done
  quire shell launcher.open >>"$log" 2>&1
  code=$?
  echo "after three refusals the next call exits $code" | tee -a "$log" >&2
  [ "$code" = 7 ] || return 1
  sill intents resume >>"$log" 2>&1 || return 1
  sleep 0.5
  quire shell launcher.open >>"$log" 2>&1
  code=$?
  echo "after Resume the next call exits $code" | tee -a "$log" >&2
  [ "$code" = 0 ]
}

# 9.
step_intentd_gone() {
  local log="$OUT_DIR/9-gone.log" code
  quire_bg orphan shell dock.pin --app e2e-orphan.desktop
  await_sheet 10 >/dev/null || { echo "no sheet appeared" | tee "$log" >&2; return 1; }
  kill "$INTENTD_PID" 2>/dev/null
  wait "$INTENTD_PID" 2>/dev/null
  INTENTD_PID=
  # Nobody waits for the answer: the sheet goes, sill stays.
  await_field showing "" 10 || return 1
  code=$(code_of orphan)
  quire shell launcher.open >>"$log" 2>&1
  local gone=$?
  sill ping >/dev/null 2>&1 || { echo "sill is gone" | tee -a "$log" >&2; return 1; }
  echo "the waiting call exited $code, a new one $gone (6: intentd is not there); sill answers ping" | tee -a "$log" >&2
  [ "$gone" = 6 ]
}

cleanup() {
  [ -n "$ASKER" ] && kill "$ASKER" 2>/dev/null
  stop_daemon
  [ -n "$INTENTD_PID" ] && kill "$INTENTD_PID" 2>/dev/null
  stop_compositor
  scratch_drop
  rm -rf "$PRIVATE_DEV"
}
trap cleanup EXIT

build_docket() {
  if [ -n "${SILL_PREBUILT:-}" ]; then
    # dev/accept.sh built docket outside the jail (DOCKET_TARGET); only check it is there.
    [ -x "$INTENTD" ] && [ -x "$QUIRE_DO" ] || { echo "$INTENTD or $QUIRE_DO is missing: run dev/accept.sh --build" >&2; return 1; }
    return 0
  fi
  {
    (cd "$DOCKET_DIR" && CARGO_TARGET_DIR="$DOCKET_TARGET" cargo build -p docket-accept --bin accept-intentd && CARGO_TARGET_DIR="$DOCKET_TARGET" cargo build -p docket-cli --bins)
  } >"$OUT_DIR/docket-build.log" 2>&1 || { echo "docket build failed (see $OUT_DIR/docket-build.log)" >&2; return 1; }
}

build || exit 1
build_docket || exit 1
start_compositor --default kwin || exit 1
IN_ENV_EXTRA=(XDG_DATA_DIRS="$DATA/none" XDG_CONFIG_DIRS="$CONFIG/none")
cp "$SILL_REPO/dist/intents/org.quire.Shell.toml" "$DATA/quire/intents/"
cp "$DOCKET_DIR/manifests/org.quire.Memory.toml" "$DATA/quire/intents/"
backdrop_seed "$CONFIG" || exit 1
: >"$CONFIG/sill/settings.toml"
status=0
# The stand-in intentd is the router for this daemon only (a debug build's `--dev-callers`).
dev_caller org.quire.Intents router "$INTENTD" || exit 1
unit_prefix sill
LAUNCH_PREFIX=("${UNIT_PREFIX[@]}")
if start_intentd && launch_daemon --proc-root "$PROC_ROOT"; then
  NAME=$(output_name) || exit 1
  SHEET="sill-confirm-$NAME"
  # The daemon names the seat: serving, and the surface exists.
  await_field link serving 10 || status=1
  run_steps step_manifest_listed step_a_read_asks_nobody step_a_write_asks_and_allow_pins \
    step_escape_denies step_allow_from_the_terminal step_revoke_asks_again \
    step_a_destructive_action_asks_for_a_hold step_the_breaker_pauses_and_resume_lets_go \
    step_intentd_gone
  [ "$status" = 0 ] && status=$STATUS
else
  status=1
fi
echo "logs in $OUT_DIR"
exit "$status"

#!/usr/bin/env bash
# The own polkit agent's acceptance, on the nested cosmic-comp (dev/nested.sh; wayland-1 is pinged
# first, reused, and never stopped).
#
# Nothing here reaches polkitd, the system bus, PAM or a real password. Every bus is one
# private dbus-daemon (dev/private-bus.conf): the daemon's session and system bus addresses both
# name it, and its polkit agent is pointed at it by `sill run --polkit-bus` (a debug build's flag,
# crates/sill/src/run_args.rs, with --polkit-helper and --polkit-session). There,
# examples/polkit_authority.rs plays polkitd's Authority (records the registration, sends
# BeginAuthentication and CancelAuthentication on command, prints each call's result), and
# examples/polkit_helper.rs plays polkit-agent-helper-1 (the helper protocol; accepts
# `sill-fake`, refuses anything else, runs no PAM). The daemon gets its own XDG_RUNTIME_DIR and
# a scratch XDG_CONFIG_HOME. Keys are typed with `wtype`
# on wayland-1's virtual keyboard, which reaches only the nested session.
#
#   1. The agent registered with the stand-in for ("unix-session", {session-id}) at sill's path,
#      `sill polkit status` says registered, and sill-polkit-<out> exists, hidden.
#   2. The stand-in's BeginAuthentication shows the prompt (a capture of the surface, and the
#      screenshot dev/out/polkit-cosmic.png); `sill polkit status` shows the action.
#   3. A wrong password: the stand-in helper refuses, the field shakes (frames while it plays)
#      and clears (the capture after it matches the one before typing), the request stays.
#   4. `sill-fake`: the stand-in Authority's call returns (authorized) and the prompt hides.
#   5. CancelAuthentication hides the prompt; the call fails Cancelled.
#   6. Escape cancels: the call fails Cancelled and the prompt hides.
#   7. `sill debug polkit` shows a made-up request with no bus call, and Escape dismisses it.
#   8. 5 s idle, hidden: 0 frames on the prompt surface.
#
# Usage: dev/nested.sh && dev/accept/polkit.sh
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_REPO/target/accept-polkit"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
AUTHORITY_BIN="$SILL_REPO/target/debug/examples/polkit_authority"
HELPER_BIN="$SILL_REPO/target/debug/examples/polkit_helper"
AGENT_PATH=/org/sill/PolicyKit1/AuthenticationAgent
SESSION_ID=accept-polkit
ACTION=org.sill.test.run

use_nested_socket || exit 1
scratch_new
mkdir -p "$CONFIG/sill"
AUTHORITY=

# Keys on the nested compositor's virtual keyboard (never wayland-0).
keys() { WAYLAND_DISPLAY="$WL" wtype "$@"; }
# A command to the stand-in Authority.
authority() { echo "$*" >&3; }

start_authority() {
  mkfifo "$SCRATCH/authority.in"
  "$AUTHORITY_BIN" "$BUS" <"$SCRATCH/authority.in" >"$OUT_DIR/authority.log" 2>&1 &
  AUTHORITY=$!
  exec 3>"$SCRATCH/authority.in"
  await_line "^ready$" || { echo "the stand-in Authority never started" >&2; return 1; }
}

# Wait up to ${2:-5} s for a line of the Authority's log matching $1.
await_line() {
  local tries=$(( ${2:-5} * 10 ))
  for _ in $(seq "$tries"); do
    grep -qE "$1" "$OUT_DIR/authority.log" && return 0
    sleep 0.1
  done
  return 1
}

start_polkit_daemon() {
  backdrop_seed "$CONFIG" || return 1
  : >"$CONFIG/sill/settings.toml"
  launch_daemon --polkit-bus "$BUS" --polkit-helper "$HELPER_BIN" --polkit-session "$SESSION_ID" \
    || return 1
  NAME=$(output_name) || return 1
  PROMPT="sill-polkit-$NAME"
  sleep 1
}

cleanup() {
  stop_daemon
  exec 3>&- 2>/dev/null
  [ -n "$AUTHORITY" ] && kill "$AUTHORITY" 2>/dev/null
  stop_private_bus
  scratch_drop
}
trap cleanup EXIT

# One field of `sill polkit status`: registration (its kind), showing, waiting.
status_field() {
  sill polkit status 2>/dev/null | python3 -c '
import json, sys
def find(v):
    if isinstance(v, dict):
        if "registration" in v and "waiting" in v:
            return v
        for x in v.values():
            r = find(x)
            if r: return r
    return None
r = find(json.loads(sys.stdin.read())) or {}
f = sys.argv[1]
if f == "registration": print(r.get("registration", {}).get("kind", ""))
else: print(r.get(f) if r.get(f) is not None else "")' "$1"
}

await_status() {
  local got=
  for _ in $(seq 50); do
    got=$(status_field "$1")
    [ "$got" = "$2" ] && return 0
    sleep 0.1
  done
  echo "status $1 is '$got', wanted '$2'" >&2
  return 1
}

# How many pixels differ between the PNGs $1 and $2 (all of them when the sizes differ).
pixels_apart() {
  python3 - "$1" "$2" <<'EOF'
import sys
from PIL import Image, ImageChops
a = Image.open(sys.argv[1]).convert("RGBA")
b = Image.open(sys.argv[2]).convert("RGBA")
if a.size != b.size:
    print(a.size[0] * a.size[1]); sys.exit()
diff = ImageChops.difference(a, b).convert("L")
print(sum(1 for p in diff.tobytes() if p > 24))
EOF
}

# The prompt captures (it is mapped) into $1 with its sheet drawn.
captured() {
  sill debug capture "$PROMPT" "$1" >>"$OUT_DIR/capture.log" 2>&1 || return 1
  [ "$(opaque_pixels "$1")" -gt 1000 ]
}

await_shown() {
  for _ in $(seq 50); do captured "$OUT_DIR/$1.png" && return 0; sleep 0.1; done
  echo "the prompt never showed" >&2
  return 1
}

await_hidden() {
  for _ in $(seq 30); do
    sill debug capture "$PROMPT" "$OUT_DIR/late.png" >>"$OUT_DIR/capture.log" 2>&1 || return 0
    sleep 0.1
  done
  echo "the prompt never hid" >&2
  return 1
}

# 1.
step_registered() {
  await_line "^registered unix-session session-id=$SESSION_ID path=$AGENT_PATH$" || {
    echo "the stand-in never saw the registration: $(cat "$OUT_DIR/authority.log")" >&2
    return 1
  }
  await_status registration registered || return 1
  sill debug stats | grep -q "\"$PROMPT\"" || { echo "$PROMPT is not in debug stats" >&2; return 1; }
  if sill debug capture "$PROMPT" "$OUT_DIR/hidden.png" >>"$OUT_DIR/capture.log" 2>&1; then
    echo "$PROMPT is mapped before any request" >&2; return 1
  fi
  echo "registered for ($SESSION_ID, $AGENT_PATH); $PROMPT exists, hidden" >&2
}

# 2.
step_begin_shows() {
  authority "begin k1 $ACTION"
  await_shown shown || return 1
  await_status showing "$ACTION" || return 1
  sleep 0.6
  captured "$OUT_DIR/empty.png" || return 1
  "$SHELL_HOST_DEV/shot.sh" "$SHOTS/polkit-cosmic.png" >>"$OUT_DIR/shot.log" 2>&1 ||
    echo "screenshot failed" >&2
  echo "BeginAuthentication k1: the prompt shows $ACTION" >&2
}

# 3.
step_wrong_shakes_and_clears() {
  local f0 f1 typed cleared
  keys "wrong-guess"
  sleep 0.3
  captured "$OUT_DIR/typed.png" || return 1
  typed=$(pixels_apart "$OUT_DIR/empty.png" "$OUT_DIR/typed.png")
  f0=$(frames_of "$PROMPT")
  keys -k Return
  sleep 1.5
  f1=$(frames_of "$PROMPT")
  captured "$OUT_DIR/cleared.png" || return 1
  cleared=$(pixels_apart "$OUT_DIR/empty.png" "$OUT_DIR/cleared.png")
  grep -q "polkit_helper: cookie k1 refused" "$OUT_DIR/daemon.log" || {
    echo "the stand-in helper never refused" >&2; return 1; }
  if grep -q "^result k1" "$OUT_DIR/authority.log"; then
    echo "k1 ended on one wrong try" >&2; return 1
  fi
  await_status showing "$ACTION" || return 1
  echo "wrong: helper refused; typing changed $typed px; $(( f1 - f0 )) frames while it shook; \
after it $cleared px differ from the empty field" >&2
  [ "$typed" -gt 50 ] && [ "$(( f1 - f0 ))" -gt 5 ] && [ "$cleared" -lt 50 ]
}

# 4.
step_right_completes() {
  keys "sill-fake"
  keys -k Return
  await_line "^result k1 authorized$" || {
    echo "the stand-in never saw k1 authorized: $(tail -3 "$OUT_DIR/authority.log")" >&2; return 1; }
  await_status showing "" && await_hidden || return 1
  echo "sill-fake: BeginAuthentication k1 returned; the prompt hid" >&2
}

# 5.
step_cancel_authentication() {
  authority "begin k2 $ACTION"
  await_shown k2 || return 1
  local start t
  start=$(now_ms)
  authority "cancel k2"
  await_line "^result k2 cancelled$" || { echo "k2 was not cancelled" >&2; return 1; }
  await_hidden || return 1
  t=$(( $(now_ms) - start ))
  echo "CancelAuthentication k2: the call failed Cancelled; hidden ${t} ms later" >&2
}

# 6.
step_escape_cancels() {
  authority "begin k3 $ACTION"
  await_shown k3 || return 1
  sleep 0.4
  keys -k Escape
  await_line "^result k3 cancelled$" || { echo "Escape did not cancel k3" >&2; return 1; }
  await_hidden || return 1
  echo "Escape: k3's call failed Cancelled; the prompt hid" >&2
}

# 7.
step_debug_polkit() {
  sill debug polkit --action org.sill.debug.made-up --message "A made-up request" \
    >>"$OUT_DIR/debug.log" 2>&1 || return 1
  await_shown made-up || return 1
  await_status showing org.sill.debug.made-up || return 1
  sleep 0.4
  keys -k Escape
  await_status showing "" && await_hidden || return 1
  if grep -q "made-up" "$OUT_DIR/authority.log"; then
    echo "the made-up request reached the bus" >&2; return 1
  fi
  echo "sill debug polkit: shown, dismissed by Escape, nothing on the bus" >&2
}

# 8.
step_idle() {
  local f0 f1
  sleep 1
  f0=$(frames_of "$PROMPT")
  sleep 5
  f1=$(frames_of "$PROMPT")
  echo "5 s idle, hidden: $PROMPT $(( f1 - f0 )) frames" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

WAYLAND_DISPLAY="$WL" wayland-info >/dev/null 2>&1 ||
  { echo "no nested compositor on $WL (dev/nested.sh first)" >&2; exit 1; }
command -v wtype >/dev/null || { echo "wtype is needed to type on wayland-1" >&2; exit 1; }
build "-p sill-testkit --example polkit_authority --example polkit_helper" || exit 1
await_no_process "sill run$" || exit 1
start_private_bus || exit 1
echo "private bus $BUS (pid $BUS_PID)" >&2
start_authority || exit 1
start_polkit_daemon || exit 1
run_steps step_registered step_begin_shows step_wrong_shakes_and_clears step_right_completes \
  step_cancel_authentication step_escape_cancels step_debug_polkit step_idle
status=$STATUS
stop_daemon || status=1
await_line "^unregistered unix-session session-id=$SESSION_ID path=$AGENT_PATH$" 3 &&
  echo "the agent unregistered as the daemon quit" >&2
echo "logs in $OUT_DIR; screenshot $SHOTS/polkit-cosmic.png"
exit "$status"

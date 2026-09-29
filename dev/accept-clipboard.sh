#!/usr/bin/env bash
# Clipboard history acceptance, on the nested
# cosmic-comp (dev/nested.sh; a running wayland-1 is pinged first, reused and never stopped).
# Starts `sill run` from this checkout's debug build (`--features debug`) with a scratch
# XDG_CONFIG_HOME and XDG_STATE_HOME, under a private session bus (dev/private-bus.conf), with
# its own XDG_RUNTIME_DIR and the nested socket as an absolute WAYLAND_DISPLAY. `wl-copy`,
# `wl-paste` and the `secret_source` example are pointed at the nested socket by absolute path;
# nothing here touches the session's own compositor (wayland-0) or its clipboard. Everything
# copied is made up, and no clipboard content is printed or written to a file: only counts,
# modes and protocols are logged. The nested selection is cleared before and after.
#
#   1. `sill clipboard status`: memory, ext (cosmic-comp offers ext_data_control_v1), 0 entries.
#   2. `wl-copy` of a made-up string: 1 entry.
#   3. `secret_source` (text marked with x-kde-passwordManagerHint) takes the selection: still 1.
#   4. `sill launcher open --query lantern`: one clipboard row (`sill debug launcher-rows`),
#      under Top Hit (a single match, never repeated below it) or Clipboard;
#      `launcher-key down` onto it (the emoji 🏮 can be the Top Hit for "lantern"),
#      `sill debug launcher-key enter`, then `wl-paste` answers the string.
#   5. `launcher.clipboard_history = "off"`, `sill reload`: off, not_connected, 0 entries.
#   6. Back on (the nested selection cleared first: turning history on keeps the selection
#      there is), two copies, `sill clipboard clear`: 0 entries.
#
# Exit 0 when every step passed, 1 on a failure.
#
# Usage: dev/nested.sh && dev/accept-clipboard.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-clipboard"
mkdir -p "$OUT_DIR"

source "$SHELL_HOST_DEV/.nested-env"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
NESTED_SOCKET="$HOST_RUNTIME/$WAYLAND_DISPLAY"
case "$NESTED_SOCKET" in
  */wayland-0) echo "refusing wayland-0, the session's own compositor" >&2; exit 1 ;;
esac
RUNTIME=$(mktemp -d)
chmod 700 "$RUNTIME"
WORD=lantern
MADE_UP="m9-accept-$WORD-5210 made-up words"

sill() { XDG_RUNTIME_DIR="$RUNTIME" "$SILL_BIN" "$@"; }
# `wl-*` on the nested compositor only; their stderr (never content) to the log.
nested_wl() { WAYLAND_DISPLAY="$NESTED_SOCKET" "$@" 2>>"$OUT_DIR/wl.log"; }

ping_nested() {
  [ -S "$NESTED_SOCKET" ] && WAYLAND_DISPLAY="$NESTED_SOCKET" wayland-info >/dev/null 2>&1
}

# "mode protocol entries" from `sill clipboard status` (counts only, by design).
status_line() {
  sill clipboard status 2>/dev/null | python3 -c '
import json, sys
try:
    doc = json.loads(sys.stdin.read())
except ValueError:
    sys.exit(0)
v = doc["v"]["v"] if isinstance(doc, dict) and "v" in doc else {}
history = v.get("status", {})
print(history.get("mode", "?"), history.get("protocol", "?"), v.get("entries", "?"))'
}

# Wait up to 5 s for the status to read WANT ("mode protocol entries").
wait_status() {
  local want="$1" got=""
  for _ in $(seq 50); do
    got=$(status_line)
    [ "$got" = "$want" ] && { echo "status: $got" >&2; return 0; }
    sleep 0.1
  done
  echo "status: $got (wanted $want)" >&2
  return 1
}

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug && cargo build -p sill-testkit --example secret_source) \
    >"$OUT_DIR/build.log" 2>&1 || { echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

write_settings() {
  mkdir -p "$CONFIG/sill"
  cat >"$CONFIG/sill/settings.toml" <<TOML
[launcher]
clipboard_history = "$1"
TOML
}

start_daemon() {
  CONFIG=$(mktemp -d)
  STATE=$(mktemp -d)
  write_settings memory
  use_fake_ddc "$CONFIG" >/dev/null
  require_fake_ddc || return 1
  XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_RUNTIME_DIR="$RUNTIME" \
    WAYLAND_DISPLAY="$NESTED_SOCKET" \
    dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- \
    sh -c "$PRIVATE_BOTH_BUSES" "$SILL_BIN" "${FAKE_DDC_FLAGS[@]}" \
    >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && return 0; sleep 0.1; done
  echo "the daemon never answered 'sill ping'" >&2
  return 1
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  rm -rf "${CONFIG:-}" "${STATE:-}"
}

# 1.
step_status() { wait_status "memory ext 0"; }

# 2.
step_copy() {
  nested_wl wl-copy -- "$MADE_UP" || return 1
  wait_status "memory ext 1"
}

# 3.
step_secret() {
  local secret out="$OUT_DIR/secret.log"
  "$SILL_REPO/target/debug/examples/secret_source" "$NESTED_SOCKET" 20 >"$out" 2>&1 &
  secret=$!
  for _ in $(seq 50); do grep -q ready "$out" && break; sleep 0.1; done
  grep -q ready "$out" || { echo "secret_source never set its selection" >&2; kill "$secret"; return 1; }
  sleep 0.5
  wait_status "memory ext 1"
  local code=$?
  kill "$secret" 2>/dev/null; wait "$secret" 2>/dev/null
  return "$code"
}

# The index (over every section's lines) of the one clipboard row in the `launcher-rows` file
# $1; fails unless there is exactly one, under "Top Hit" or "Clipboard". A single match is the
# Top Hit, never repeated below it. Clipboard titles read "<clipboard>" there,
# never the text.
clip_row() {
  python3 - "$1" <<'PY'
import json, sys
v = json.load(open(sys.argv[1])).get("v", {})
lines = [(s.get("title", ""), r) for s in v.get("sections", []) for r in s.get("rows", [])]
held = [(at, title) for at, (title, r) in enumerate(lines) if r == "<clipboard>"]
print(f"clipboard rows: {len(held)}, under {', '.join(t for _, t in held) or 'nothing'}",
      file=sys.stderr)
if len(held) != 1 or held[0][1] not in ("Top Hit", "Clipboard"):
    sys.exit(1)
print(held[0][0])
PY
}

# The launcher's selected line.
selected_row() {
  sill debug launcher-rows 2>/dev/null | python3 -c '
import json, sys
print(json.loads(sys.stdin.read())["v"]["selected"])' 2>/dev/null || echo -1
}

# 4.
step_launcher() {
  local rows="$OUT_DIR/launcher-rows.json"
  # The marked copy was killed above; put the made-up text back as the selection first, so a
  # paste proves the history's offer and not wl-copy's.
  nested_wl wl-copy -- "something else, made up" || return 1
  sill launcher open --query "$WORD" >/dev/null 2>&1 || { echo "launcher open failed" >&2; return 1; }
  sleep 0.5
  sill debug launcher-rows >"$rows" 2>&1
  local at
  at=$(clip_row "$rows") || return 1
  # Enter runs the selected row: another provider's row (the emoji 🏮 for "lantern") may be the
  # Top Hit, so move to the clipboard row first.
  for _ in $(seq 20); do
    [ "$(selected_row)" -lt "$at" ] || break
    sill debug launcher-key down >/dev/null 2>&1 || return 1
    sleep 0.2
  done
  [ "$(selected_row)" = "$at" ] || { echo "could not select the clipboard row ($at)" >&2; return 1; }
  sill debug launcher-key enter >/dev/null 2>&1 || return 1
  for _ in $(seq 30); do
    if [ "$(nested_wl wl-paste --no-newline --type text)" = "$MADE_UP" ]; then
      echo "wl-paste read the offered entry" >&2
      return 0
    fi
    sleep 0.1
  done
  echo "wl-paste did not read the offered entry" >&2
  return 1
}

# 5.
step_off() {
  write_settings off
  sill reload >/dev/null 2>&1 || return 1
  wait_status "off not_connected 0"
}

# 6.
step_clear() {
  # Turning history on reads the selection there is; start from none.
  nested_wl wl-copy --clear
  write_settings memory
  sill reload >/dev/null 2>&1 || return 1
  wait_status "memory ext 0" || return 1
  nested_wl wl-copy -- "m9-accept first made-up copy" || return 1
  wait_status "memory ext 1" || return 1
  nested_wl wl-copy -- "m9-accept second made-up copy" || return 1
  wait_status "memory ext 2" || return 1
  sill clipboard clear >/dev/null 2>&1 || return 1
  wait_status "memory ext 0"
}

ping_nested || { echo "no nested compositor at $NESTED_SOCKET (dev/nested.sh first)" >&2; exit 1; }
command -v wl-copy >/dev/null && command -v wl-paste >/dev/null \
  || { echo "wl-copy and wl-paste are missing (wl-clipboard); not installed by this script" >&2; exit 1; }
build || exit 1
for _ in $(seq 600); do
  pgrep -f 'target/[a-z]*/sill run' >/dev/null || break
  [ -n "${WAITED:-}" ] || { echo "another sill daemon is running; waiting for it to quit" >&2; WAITED=1; }
  sleep 1
done
: >"$OUT_DIR/wl.log"
nested_wl wl-copy --clear
start_daemon || { stop_daemon; exit 1; }
status=0
for step in step_status step_copy step_secret step_launcher step_off step_clear; do
  "$step"
  code=$?
  case "$code" in
    0) echo "PASS $step" ;;
    *) echo "FAIL $step"; status=1 ;;
  esac
done
stop_daemon
nested_wl wl-copy --clear
rm -rf "$RUNTIME"
echo "logs in $OUT_DIR (counts and fault kinds only)"
exit "$status"

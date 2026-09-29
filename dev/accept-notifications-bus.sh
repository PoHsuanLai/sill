#!/usr/bin/env bash
# The notifications server's acceptance, on the nested cosmic-comp (dev/nested.sh; a running
# wayland-1 is reused and never stopped).
#
# Everything D-Bus happens on a private bus (dev/private-bus.conf: no service directories, so
# nothing is ever activated): the daemon, notify-send, gdbus and the holder example get
# its address, and nothing here reads or writes the real session bus, whose
# org.freedesktop.Notifications belongs to the person's own daemon (plasmashell here). The daemon
# also gets its own XDG_RUNTIME_DIR (WAYLAND_DISPLAY an absolute socket path), so another sill
# running beside it keeps its own IPC socket, and a scratch XDG_CONFIG_HOME and XDG_STATE_HOME.
#
#   1. `sill notifications status` says holder sill, mode own.
#   2. GetServerInformation answers ('sill', 'sill', <version>, '1.2'); GetCapabilities the six
#      words.
#   3. `notify-send -a accept Hello`: status counts 1 banner and 1 kept.
#   4. CloseNotification <id>: `gdbus monitor` beside it records NotificationClosed (<id>, 3).
#   5. notifications.server = "passive" and `sill reload`: the name is released (holder nobody)
#      and nothing answers Notify.
#   6. With another holder started first (examples/notifications_holder.rs), mode own reports
#      holder other with that pid, and the holder keeps the name; when it leaves, sill takes it.
#   7. The history file under the scratch XDG_STATE_HOME survives a daemon restart (status kept
#      is the same).
#
# Usage: dev/nested.sh && dev/accept-notifications-bus.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-notifications-bus"
mkdir -p "$OUT_DIR"
HOLDER_BIN="$SILL_REPO/target/debug/examples/notifications_holder"
NAME=org.freedesktop.Notifications
OBJECT=/org/freedesktop/Notifications

# The scratch world the daemon lives in.
SCRATCH=$(mktemp -d)
RUNTIME="$SCRATCH/run"
CONFIG="$SCRATCH/config"
STATE="$SCRATCH/state"
mkdir -p "$RUNTIME" "$CONFIG/sill" "$STATE"
chmod 700 "$RUNTIME"
BUS=
BUS_PID=
DAEMON=
HOLDER=

# Every program run here sees only the private bus and the scratch directories.
in_env() {
  env XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$WL" DBUS_SESSION_BUS_ADDRESS="$BUS" \
    DBUS_SYSTEM_BUS_ADDRESS="$BUS" XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" "$@"
}
sill() { in_env "$SILL_BIN" "$@"; }
call() { in_env timeout 5 gdbus call --session --dest "$NAME" --object-path "$OBJECT" "$@"; }

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug &&
    cargo build -p sill-services --example notifications_holder) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
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

server_setting() {
  printf '[notifications]\nserver = "%s"\n' "$1" >"$CONFIG/sill/settings.toml"
}

start_daemon() {
  use_fake_ddc "$SCRATCH" >/dev/null
  require_fake_ddc || return 1
  in_env "$SILL_BIN" run >>"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && return 0; sleep 0.1; done
  echo "the daemon never answered ping (see $OUT_DIR/daemon.log)" >&2
  return 1
}

stop_daemon() {
  [ -n "$DAEMON" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
}

cleanup() {
  stop_daemon
  [ -n "$HOLDER" ] && kill "$HOLDER" 2>/dev/null
  [ -n "$BUS_PID" ] && kill "$BUS_PID" 2>/dev/null
  rm -rf "$SCRATCH"
}
trap cleanup EXIT

# One field of `sill notifications status`: mode, holder (its kind), pid, banners or kept.
status_field() {
  sill notifications status 2>/dev/null | python3 -c '
import json, sys
def find(v):
    if isinstance(v, dict):
        if "holder" in v and "kept" in v:
            return v
        for x in v.values():
            r = find(x)
            if r: return r
    return None
r = find(json.loads(sys.stdin.read())) or {}
f = sys.argv[1]
if f == "holder": print(r.get("holder", {}).get("kind", ""))
elif f == "pid": print((r.get("holder", {}).get("v") or {}).get("pid", ""))
else: print(r.get(f, ""))' "$1"
}

# Wait up to 5 s for status field $1 to read $2.
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

# The pid the bus says holds the name, empty when nobody does.
bus_holder_pid() {
  in_env timeout 5 gdbus call --session --dest org.freedesktop.DBus \
    --object-path /org/freedesktop/DBus \
    --method org.freedesktop.DBus.GetConnectionUnixProcessID "$NAME" 2>/dev/null |
    sed -E 's/^\(uint32 ([0-9]+),\)$/\1/'
}

# 1.
step_owns() {
  await_status mode own && await_status holder sill || return 1
  echo "status: mode own, holder sill" >&2
}

# 2.
step_information() {
  local info caps
  info=$(call --method "$NAME.GetServerInformation") || return 1
  caps=$(call --method "$NAME.GetCapabilities") || return 1
  echo "GetServerInformation $info; GetCapabilities $caps" | tee "$OUT_DIR/information.log" >&2
  [[ "$info" =~ ^\(\'sill\',\ \'sill\',\ \'[^\']+\',\ \'1\.2\'\)$ ]] || return 1
  [ "$caps" = "(['actions', 'body', 'body-hyperlinks', 'body-markup', 'icon-static', 'persistence'],)" ]
}

# 3.
step_notify_send() {
  ID=$(in_env timeout 5 notify-send -p -a accept "Hello") || return 1
  echo "notify-send answered id $ID" >&2
  await_status banners 1 && await_status kept 1
}

# 4.
step_close() {
  local monitor="$OUT_DIR/monitor.log"
  in_env gdbus monitor --session --dest "$NAME" >"$monitor" 2>&1 &
  local watcher=$!
  sleep 0.5
  call --method "$NAME.CloseNotification" "$ID" >/dev/null || { kill "$watcher"; return 1; }
  local heard=no
  for _ in $(seq 30); do
    grep -q "NotificationClosed (uint32 $ID, uint32 3)" "$monitor" && { heard=yes; break; }
    sleep 0.1
  done
  kill "$watcher" 2>/dev/null
  wait "$watcher" 2>/dev/null
  echo "gdbus monitor heard NotificationClosed ($ID, 3): $heard" >&2
  [ "$heard" = yes ] && await_status banners 0 && await_status kept 0
}

# 5.
step_passive() {
  server_setting passive
  sill reload >/dev/null || return 1
  await_status mode passive && await_status holder nobody || return 1
  if call --method "$NAME.Notify" -- accept 0 '' Nobody '' '@as []' '@a{sv} {}' -1 \
    >"$OUT_DIR/passive.log" 2>&1; then
    echo "something answered Notify while passive" >&2; return 1
  fi
  # Unanswered because nobody holds the name, not because the call was malformed.
  grep -qE "ServiceUnknown|NameHasNoOwner" "$OUT_DIR/passive.log" || {
    echo "Notify failed for another reason: $(cat "$OUT_DIR/passive.log")" >&2; return 1; }
  echo "passive: holder nobody, Notify unanswered ($(tr -d '\n' <"$OUT_DIR/passive.log"))" >&2
}

# 6.
step_other_holder() {
  stop_daemon
  "$HOLDER_BIN" "$BUS" >"$OUT_DIR/holder.log" 2>&1 &
  HOLDER=$!
  for _ in $(seq 50); do [ "$(bus_holder_pid)" = "$HOLDER" ] && break; sleep 0.1; done
  [ "$(bus_holder_pid)" = "$HOLDER" ] || { echo "the holder never took the name" >&2; return 1; }
  server_setting own
  start_daemon || return 1
  await_status mode own && await_status holder other && await_status pid "$HOLDER" || return 1
  sleep 1
  [ "$(bus_holder_pid)" = "$HOLDER" ] || { echo "sill replaced the holder" >&2; return 1; }
  echo "holder pid $HOLDER kept the name; status holder other, pid $HOLDER" >&2
  kill "$HOLDER"; wait "$HOLDER" 2>/dev/null; HOLDER=
  await_status holder sill || return 1
  echo "the holder left; sill took the name" >&2
}

# 7.
step_history_survives() {
  in_env timeout 5 notify-send -a accept "Kept" || return 1
  # The same Notify step 5 sent, answered now that sill holds the name.
  call --method "$NAME.Notify" -- other 0 '' 'Also kept' '' '@as []' '@a{sv} {}' -1 \
    >"$OUT_DIR/notify.log" 2>&1 || { cat "$OUT_DIR/notify.log" >&2; return 1; }
  await_status kept 2 || return 1
  local file="$STATE/sill/notifications.json"
  [ -s "$file" ] || { echo "no history file at $file" >&2; return 1; }
  stop_daemon
  start_daemon || return 1
  await_status kept 2 || return 1
  echo "kept 2 before and after a restart ($file)" >&2
}

source "$SHELL_HOST_DEV/.nested-env"
case "$WAYLAND_DISPLAY" in
  /*) WL="$WAYLAND_DISPLAY" ;;
  *) WL="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$WAYLAND_DISPLAY" ;;
esac
[ -S "$WL" ] || { echo "no nested compositor at $WL (dev/nested.sh first)" >&2; exit 1; }
build || exit 1
start_bus || exit 1
: >"$OUT_DIR/daemon.log"
server_setting own
start_daemon || exit 1
status=0
for step in step_owns step_information step_notify_send step_close step_passive \
  step_other_holder step_history_survives; do
  if "$step"; then echo "PASS $step"; else echo "FAIL $step"; status=1; fi
done
echo "logs in $OUT_DIR"
exit "$status"

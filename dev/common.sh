# Sourced by the dev/ wrappers: where the sibling shell-host checkout and this repo are.
# shell-host owns the nested-compositor harness (scripts/dev/); sill only wraps it.
SILL_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SHELL_HOST_DEV="${SHELL_HOST_DEV:-$SILL_REPO/../shell-host/scripts/dev}"
if [ ! -x "$SHELL_HOST_DEV/nested.sh" ]; then
  echo "dev: shell-host's harness not found at $SHELL_HOST_DEV (set SHELL_HOST_DEV)" >&2
  exit 1
fi
SILL_BIN="${SILL_BIN:-$SILL_REPO/target/debug/sill}"

# Refuses to let the daemon start when $1 (the address about to be used as its
# DBUS_SYSTEM_BUS_ADDRESS) would be the real system bus: unset, empty, or the stock
# unix:path=/run/dbus/system_bus_socket. A private bus (dev/private-bus.conf, or the private
# session bus a headless KWin or nested compositor set up for itself) must stand in instead, on
# both DBUS_SESSION_BUS_ADDRESS and DBUS_SYSTEM_BUS_ADDRESS, so NetworkManager, BlueZ, UPower,
# logind and the rest are simply absent rather than the person's own (FINDINGS F1068: a Wi-Fi
# menu capture showed real neighbour SSIDs because the daemon's system bus was never overridden
# and fell back to the real one). Call this immediately before backgrounding the daemon.
require_private_system_bus() {
  case "${1:-}" in
    ""|unix:path=/run/dbus/system_bus_socket)
      echo "refusing to start the daemon: its DBUS_SYSTEM_BUS_ADDRESS would be the real system bus (unset or /run/dbus/system_bus_socket); a private bus must stand in (FINDINGS F1068)" >&2
      return 1 ;;
  esac
}

# For the `dbus-run-session --config-file=dev/private-bus.conf -- ...` scripts: dbus-run-session
# sets DBUS_SESSION_BUS_ADDRESS for the command it runs but never touches DBUS_SYSTEM_BUS_ADDRESS,
# which is then whatever the script's own ambient environment had (often the real system bus,
# FINDINGS F1068). This shell fragment, run as the command dbus-run-session execs, copies its
# fresh private session bus onto DBUS_SYSTEM_BUS_ADDRESS too before starting the daemon (one
# private bus standing in for both), and refuses outright if dbus-run-session somehow left it
# real. Usage:
#   dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- \
#     sh -c "$PRIVATE_BOTH_BUSES" "$SILL_BIN"
read -r -d '' PRIVATE_BOTH_BUSES <<'SH' || true
case "$DBUS_SESSION_BUS_ADDRESS" in
  ""|unix:path=/run/dbus/system_bus_socket)
    echo "refusing to start the daemon: dbus-run-session gave no private bus (F1068)" >&2
    exit 1 ;;
esac
export DBUS_SYSTEM_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS"
exec "$0" run
SH

# For the `dev/nested.sh` scripts (dev/run.sh's `nested "$SILL_BIN" run`, which by itself passes
# the daemon nothing but WAYLAND_DISPLAY, so it fell back to the real session AND system bus,
# FINDINGS F1068): starts a private dbus-daemon (dev/private-bus.conf) and sets BUS to its
# address and BUS_PID to its pid. Put `DBUS_SESSION_BUS_ADDRESS="$BUS"
# DBUS_SYSTEM_BUS_ADDRESS="$BUS"` on the daemon's launch line (one private bus standing in for
# both) and call stop_private_bus when done.
start_private_bus() {
  local out
  out=$(dbus-daemon --config-file="$SILL_REPO/dev/private-bus.conf" --fork \
    --print-address=1 --print-pid=1) || return 1
  BUS=$(sed -n 1p <<<"$out")
  BUS_PID=$(sed -n 2p <<<"$out")
  case "$BUS" in unix:*) ;; *) echo "no private bus address: $out" >&2; return 1 ;; esac
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "not a private bus" >&2; return 1; }
  require_private_system_bus "$BUS" || return 1
}
stop_private_bus() {
  [ -n "${BUS_PID:-}" ] || return 0
  kill "$BUS_PID" 2>/dev/null
  BUS_PID=
}

# Writes a scratch DDC fixture naming no monitor (`{"displays":[]}`, the fixture format
# dev/accept-cc-real-input.sh's fake_monitor writes, only static) and exports SILL_FAKE_DDC to
# it, so a debug build's DDC worker (sill-services' `fake-ddc` feature, FINDINGS F1004) talks to
# it instead of opening the real /dev/i2c-* at startup. $1: the scratch directory to write
# ddc.json into (a fresh mktemp -d if omitted, not cleaned up here). Prints the fixture's path.
# Call once before start_daemon/step_start, before backgrounding the daemon (FINDINGS
# F1090-F1097, F1095, F1098: every `--features debug` script but accept-cc-real-input.sh let
# the DDC worker probe the user's real BenQ MA270U).
use_fake_ddc() {
  local dir="${1:-$(mktemp -d)}"
  SILL_FAKE_DDC="$dir/ddc.json"
  printf '{"displays":[]}\n' >"$SILL_FAKE_DDC"
  export SILL_FAKE_DDC
  echo "$SILL_FAKE_DDC"
}

# Refuses to let the daemon start unless SILL_FAKE_DDC is already set (by use_fake_ddc, or a
# script's own fixture as accept-cc-real-input.sh writes): the same pattern as
# require_private_system_bus, closing the DDC gap (FINDINGS F1095, F1098). A release
# (non-debug) build ignores SILL_FAKE_DDC entirely (the `fake-ddc` feature is debug-only) and
# always opens the real bus; this guard cannot see the build kind, so no dev script may start a
# release build against a compositor a person is not directly watching. Call immediately before
# backgrounding the daemon.
require_fake_ddc() {
  [ -n "${SILL_FAKE_DDC:-}" ] || {
    echo "refusing to start the daemon: SILL_FAKE_DDC is not set; a debug build's DDC worker" \
      "would open the real /dev/i2c-* (FINDINGS F1095, F1098)" >&2
    return 1
  }
}

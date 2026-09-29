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
# logind and the rest are simply absent rather than the person's own (a Wi-Fi menu capture once
# showed real neighbour SSIDs because the daemon's system bus was never overridden and fell back to
# the real one). Call this immediately before backgrounding the daemon.
require_private_system_bus() {
  case "${1:-}" in
    ""|unix:path=/run/dbus/system_bus_socket)
      echo "refusing to start the daemon: its DBUS_SYSTEM_BUS_ADDRESS would be the real system bus (unset or /run/dbus/system_bus_socket); a private bus must stand in" >&2
      return 1 ;;
  esac
}

# For the `dbus-run-session --config-file=dev/private-bus.conf -- ...` scripts: dbus-run-session
# sets DBUS_SESSION_BUS_ADDRESS for the command it runs but never touches DBUS_SYSTEM_BUS_ADDRESS,
# which is then whatever the script's own ambient environment had (often the real system bus). This
# shell fragment, run as the command dbus-run-session execs, copies its fresh private session bus
# onto DBUS_SYSTEM_BUS_ADDRESS too before starting the daemon (one private bus standing in for
# both), and refuses outright if dbus-run-session somehow left it real. Arguments after the
# binary go to `sill run` (the stand-in flags). Usage:
#   dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- \
#     sh -c "$PRIVATE_BOTH_BUSES" "$SILL_BIN" "${FAKE_DDC_FLAGS[@]}"
read -r -d '' PRIVATE_BOTH_BUSES <<'SH' || true
case "$DBUS_SESSION_BUS_ADDRESS" in
  ""|unix:path=/run/dbus/system_bus_socket)
    echo "refusing to start the daemon: dbus-run-session gave no private bus" >&2
    exit 1 ;;
esac
export DBUS_SYSTEM_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS"
exec "$0" run "$@"
SH

# For the `dev/nested.sh` scripts (dev/run.sh's `nested "$SILL_BIN" run`, which by itself passes
# the daemon nothing but WAYLAND_DISPLAY, so it fell back to the real session AND system bus):
# starts a private dbus-daemon (dev/private-bus.conf) and sets BUS to its address and BUS_PID to its
# pid. Put `DBUS_SESSION_BUS_ADDRESS="$BUS"
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
# dev/accept-cc-real-input.sh's fake_monitor writes, only static) and sets FAKE_DDC to it and
# FAKE_DDC_FLAGS to `--fake-ddc FILE`, the `sill run` option that makes a debug build's DDC
# worker talk to the file instead of opening the real /dev/i2c-*. Every daemon launch line passes
# "${FAKE_DDC_FLAGS[@]}" right after `run` (or after the binary, through PRIVATE_BOTH_BUSES).
# $1: the scratch directory to write ddc.json into (a fresh mktemp -d if omitted, not cleaned up
# here). Prints the fixture's path. Call before start_daemon/step_start, in the script's own
# shell (never in a command substitution, which would lose the variables).
FAKE_DDC_FLAGS=()
use_fake_ddc() {
  local dir="${1:-$(mktemp -d)}"
  FAKE_DDC="$dir/ddc.json"
  printf '{"displays":[]}\n' >"$FAKE_DDC"
  FAKE_DDC_FLAGS=(--fake-ddc "$FAKE_DDC")
  echo "$FAKE_DDC"
}

# Refuses to let the daemon start unless use_fake_ddc has made FAKE_DDC_FLAGS and its fixture
# exists: the same pattern as require_private_system_bus, closing the DDC gap. A release
# (non-debug) build has no `--fake-ddc` and always opens the real bus; this guard cannot see the
# build kind, so no dev script may start a release build against a compositor a person is not
# directly watching. Call immediately before backgrounding the daemon, whose launch line must
# pass "${FAKE_DDC_FLAGS[@]}".
require_fake_ddc() {
  [ "${FAKE_DDC_FLAGS[0]:-}" = --fake-ddc ] && [ -f "${FAKE_DDC_FLAGS[1]:-}" ] || {
    echo "refusing to start the daemon: no --fake-ddc fixture (use_fake_ddc); a debug build's" \
      "DDC worker would open the real /dev/i2c-*" >&2
    return 1
  }
}

# Private buses, scratch directories and fake hardware: everything a dev script needs so that
# the daemon it starts never reaches the real system or session bus, the real ~/.config or
# ~/.cache, or /dev/i2c-*. Sourced through dev/lib/all.sh; a script never redefines these.

# Refuses to let the daemon start when $1 (the address about to be used as its
# DBUS_SYSTEM_BUS_ADDRESS) would be the real system bus: unset, empty, or the stock
# unix:path=/run/dbus/system_bus_socket. A private bus (dev/private-bus.conf, or the private
# session bus a headless KWin sets up for itself) must stand in instead, on both
# DBUS_SESSION_BUS_ADDRESS and DBUS_SYSTEM_BUS_ADDRESS, so NetworkManager, BlueZ, UPower, logind
# and the rest are simply absent rather than the person's own (a Wi-Fi menu capture once showed
# real neighbour SSIDs because the daemon's system bus fell back to the real one). Call this
# immediately before backgrounding the daemon.
require_private_system_bus() {
  case "${1:-}" in
    ""|unix:path=/run/dbus/system_bus_socket)
      echo "refusing to start the daemon: its DBUS_SYSTEM_BUS_ADDRESS would be the real system bus (unset or /run/dbus/system_bus_socket); a private bus must stand in" >&2
      return 1 ;;
  esac
}

# For the `dbus-run-session --config-file=dev/private-bus.conf -- ...` scripts: dbus-run-session
# sets DBUS_SESSION_BUS_ADDRESS for the command it runs but never touches
# DBUS_SYSTEM_BUS_ADDRESS, which is then whatever the script's own ambient environment had (often
# the real system bus). This shell fragment, run as the command dbus-run-session execs, copies its
# fresh private session bus onto DBUS_SYSTEM_BUS_ADDRESS too before starting the daemon (one
# private bus standing in for both), and refuses outright if dbus-run-session left it real.
# Arguments after the binary go to `sill run`. Usage:
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

# Starts a private dbus-daemon and sets BUS to its address and BUS_PID to its pid: the one bus
# standing in for both the session and the system bus of a daemon on a nested compositor. $1: the
# bus config (default dev/private-bus.conf). Call stop_private_bus when done.
start_private_bus() {
  local out conf="${1:-$SILL_REPO/dev/private-bus.conf}"
  out=$(dbus-daemon --config-file="$conf" --fork --print-address=1 --print-pid=1) || return 1
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

# Writes a scratch DDC fixture naming no monitor (`{"displays":[]}`) and sets FAKE_DDC to it and
# FAKE_DDC_FLAGS to `--fake-ddc FILE`, the `sill run` option that makes a debug build's DDC worker
# talk to the file instead of opening the real /dev/i2c-*. Every daemon launch line passes
# "${FAKE_DDC_FLAGS[@]}" right after `run`. $1: the directory to write ddc.json into (a fresh
# mktemp -d if omitted). Call in the script's own shell (never in a command substitution, which
# would lose the variables).
FAKE_DDC_FLAGS=()
use_fake_ddc() {
  local dir="${1:-$(mktemp -d)}"
  FAKE_DDC="$dir/ddc.json"
  printf '{"displays":[]}\n' >"$FAKE_DDC"
  FAKE_DDC_FLAGS=(--fake-ddc "$FAKE_DDC")
  echo "$FAKE_DDC"
}

# Refuses to let the daemon start unless use_fake_ddc has made FAKE_DDC_FLAGS and its fixture
# exists. A release (non-debug) build has no `--fake-ddc` and always opens the real bus; this
# guard cannot see the build kind, so no dev script may start a release build against a
# compositor a person is not directly watching. Call immediately before backgrounding the daemon.
require_fake_ddc() {
  [ "${FAKE_DDC_FLAGS[0]:-}" = --fake-ddc ] && [ -f "${FAKE_DDC_FLAGS[1]:-}" ] || {
    echo "refusing to start the daemon: no --fake-ddc fixture (use_fake_ddc); a debug build's" \
      "DDC worker would open the real /dev/i2c-*" >&2
    return 1
  }
}

# Makes the scratch tree every daemon runs in: SCRATCH with its own HOME (HOME_DIR), XDG
# runtime (RUNTIME, mode 700), config (CONFIG), data (DATA), state (STATE) and cache (CACHE)
# directories, so nothing a script starts reads or writes the real ones. in_env (daemon.sh)
# hands them to every process it runs. Remove with scratch_drop.
scratch_new() {
  scratch_use "$(mktemp -d)"
}

# The same tree under the existing directory $1 (made by an earlier scratch_new, its path kept by
# a script whose steps run in separate processes: dev/demo.sh). Creates what is missing.
scratch_use() {
  SCRATCH="$1"
  HOME_DIR="$SCRATCH/home"
  RUNTIME="$SCRATCH/run"
  CONFIG="$SCRATCH/config"
  DATA="$SCRATCH/data"
  STATE="$SCRATCH/state"
  CACHE="$SCRATCH/cache"
  mkdir -p "$HOME_DIR" "$RUNTIME" "$CONFIG" "$DATA" "$STATE" "$CACHE"
  chmod 700 "$RUNTIME"
}

scratch_drop() {
  [ -z "${SCRATCH:-}" ] || rm -rf "$SCRATCH"
  SCRATCH=
}

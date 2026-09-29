# Real Wayland input into the headless KWin through shell-host's fake-input.sh: ONE client for
# the whole run, since each client is a new fake input device and KWin re-announces the seat's
# pointer for each, so a press would reach the bar once per pointer and a click toggled a popup
# open and shut. Needs OUT_DIR, SHELL_HOST_DEV and reap (daemon.sh).
INPUT=

start_input() {
  rm -f "$OUT_DIR/input.fifo"; mkfifo "$OUT_DIR/input.fifo" || return 1
  "$SHELL_HOST_DEV/fake-input.sh" <"$OUT_DIR/input.fifo" >"$OUT_DIR/input.log" 2>&1 &
  INPUT=$!
  exec 3>"$OUT_DIR/input.fifo"
}

stop_input() {
  [ -n "${INPUT:-}" ] || return 0
  exec 3>&-
  reap "$INPUT" input
  INPUT=
  rm -f "$OUT_DIR/input.fifo"
}

# The fake-input commands "$@", one per line; returns once each has been answered (`ok <line>`).
input() {
  local before
  before=$(grep -c '^ok ' "$OUT_DIR/input.log")
  printf '%s\n' "$@" >&3
  for _ in $(seq 100); do
    [ "$(grep -c '^ok ' "$OUT_DIR/input.log")" -ge $(( before + $# )) ] && return 0
    sleep 0.05
  done
  echo "fake input did not answer: $*" >&2
  return 1
}

# A real key press (`key NAME`) and a real left click at surface-independent output px (`click X Y`).
key() { input "key $1 down" "sleep 30" "key $1 up" "sleep 150"; }
click() { input "move $1 $2" "sleep 60" "press left" "sleep 40" "release left" "sleep 200"; }

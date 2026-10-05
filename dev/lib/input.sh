# Real Wayland input into the compositor: on the headless KWin through shell-host's fake-input.sh
# (ONE client for the whole run, since each client is a new fake input device and KWin
# re-announces the seat's pointer for each, so a press would reach the bar once per pointer and a
# click toggled a popup open and shut); on casement through `casementctl debug inject` reading the
# same lines from the same fifo (casement's inject grammar is a superset of fake-input.sh's, and
# it answers each line `ok ...`). Needs OUT_DIR, SHELL_HOST_DEV, reap (daemon.sh) and, on
# casement, the session start_compositor made.
INPUT=

start_input() {
  rm -f "$OUT_DIR/input.fifo"; mkfifo "$OUT_DIR/input.fifo" || return 1
  case "$COMPOSITOR" in
    casement) casementctl debug inject <"$OUT_DIR/input.fifo" >"$OUT_DIR/input.log" 2>&1 & ;;
    *) "$SHELL_HOST_DEV/fake-input.sh" <"$OUT_DIR/input.fifo" >"$OUT_DIR/input.log" 2>&1 & ;;
  esac
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

# The fake-input commands "$@", one per line; returns once each has been answered (`ok <line>`),
# waiting up to INPUT_TICKS (default 100) 50 ms ticks for them. Casement answers a line it refuses
# `err <line>: <why>`: that fails the call at once, with the reason.
input() {
  local before errors
  before=$(grep -c '^ok ' "$OUT_DIR/input.log")
  errors=$(grep -c '^err ' "$OUT_DIR/input.log")
  printf '%s\n' "$@" >&3
  for _ in $(seq "${INPUT_TICKS:-100}"); do
    [ "$(grep -c '^ok ' "$OUT_DIR/input.log")" -ge $(( before + $# )) ] && return 0
    if [ "$(grep -c '^err ' "$OUT_DIR/input.log")" -gt "$errors" ]; then
      echo "input refused: $(grep '^err ' "$OUT_DIR/input.log" | tail -n1)" >&2
      return 1
    fi
    sleep 0.05
  done
  echo "fake input did not answer: $*" >&2
  return 1
}

# A real key press (`key NAME`) and a real left click at surface-independent output px (`click X Y`).
key() { input "key $1 down" "sleep 30" "key $1 up" "sleep 150"; }
click() { input "move $1 $2" "sleep 60" "press left" "sleep 40" "release left" "sleep 200"; }

# A click on the surface $1 at its local point ($2, $3). On KWin and cosmic-comp `sill debug click`
# stands in for a pointer press (the daemon feeds it to the surface itself). Casement answers an
# `xdg_popup.grab` whose serial is no real press with `popup_done`, as the protocol says, so a
# popup opened by that stand-in is dismissed at once: on casement the click is a real pointer
# press through `casementctl debug inject`, at the global point (the surface's origin plus the
# point: callers name surfaces at the output's origin, such as the top bar).
surface_click() {
  case "$COMPOSITOR" in
    casement)
      local reply
      reply=$(casementctl debug inject "move $2 $3" "sleep 60" "press left" "sleep 40" "release left" "sleep 200") \
        && [ "$(grep -c '^ok ' <<<"$reply")" = 6 ] \
        || { echo "casementctl debug inject: $reply" >&2; return 1; }
      echo '{"kind":"done"}' ;;
    *) sill debug click "$1" "$2" "$3" ;;
  esac
}

# Sourced through dev/lib/all.sh: a wallpaper behind the shell, so the
# bar's and the dock's blur and tint have something to show. `backdrop_seed DIR` writes the
# test wallpaper (dev/backdrop.py) into COSMIC's background record under the config dir DIR
# (`cosmic/com.system76.CosmicBackground/v1/all`), which `sill run` reads when it draws the
# wallpaper itself (m2-wallpaper) and `cosmic-bg` reads otherwise. `backdrop_up DIR` starts
# `cosmic-bg` in the nested session on that config unless the running daemon already has a
# wallpaper surface; `backdrop_down` stops it.
BACKDROP_PNG="$SILL_TARGET/backdrop/bands.png"
BACKDROP_PID=

backdrop_seed() {
  local record="$1/cosmic/com.system76.CosmicBackground/v1"
  if [ ! -s "$BACKDROP_PNG" ]; then
    mkdir -p "$(dirname "$BACKDROP_PNG")"
    python3 "$SILL_REPO/dev/backdrop.py" "$BACKDROP_PNG" || return 1
  fi
  mkdir -p "$record"
  cat >"$record/all" <<RON
(
    output: "all",
    source: Path("$BACKDROP_PNG"),
    filter_by_theme: false,
    rotation_frequency: 3600,
    filter_method: Lanczos,
    scaling_mode: Zoom,
    sampling_method: Alphanumeric,
)
RON
  echo true >"$record/same-on-all"
}

# Whether a running daemon draws a wallpaper of its own. Asks only once `ping` has found one
# (never starts a daemon), and reads the whole reply before matching (an early `grep -q` exit
# would kill the caller with SIGPIPE under `pipefail`).
backdrop_by_sill() {
  local stats
  sill ping >/dev/null 2>&1 || return 1
  stats=$(sill debug stats 2>/dev/null) || return 1
  [[ "$stats" == *'"sill-wallpaper-'* ]]
}

backdrop_up() {
  backdrop_seed "$1" || return 1
  if backdrop_by_sill; then
    echo "backdrop: sill draws the wallpaper" >&2
    return 0
  fi
  XDG_CONFIG_HOME="$1" "$SHELL_HOST_DEV/run.sh" cosmic-bg >/dev/null 2>&1 &
  BACKDROP_PID=$!
  echo "backdrop: cosmic-bg (pid $BACKDROP_PID) draws the wallpaper" >&2
  sleep 1.5
}

backdrop_down() {
  [ -n "$BACKDROP_PID" ] || return 0
  kill "$BACKDROP_PID" 2>/dev/null
  pkill -P "$BACKDROP_PID" 2>/dev/null
  wait "$BACKDROP_PID" 2>/dev/null
  BACKDROP_PID=
}

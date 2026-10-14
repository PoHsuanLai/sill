# Sourced through dev/lib/all.sh: a wallpaper behind the shell, so the bar's and the dock's blur
# and tint have something to show. `backdrop_seed DIR` writes the test wallpaper
# (dev/backdrop.py) and names it as the wallpaper source under the config dir DIR
# (`sill/wallpaper-source`, one path), which `sill run` reads when it draws the wallpaper itself.
BACKDROP_PNG="$SILL_OUT_ROOT/backdrop/bands.png"

backdrop_seed() {
  if [ ! -s "$BACKDROP_PNG" ]; then
    mkdir -p "$(dirname "$BACKDROP_PNG")"
    python3 "$SILL_REPO/dev/backdrop.py" "$BACKDROP_PNG" || return 1
  fi
  mkdir -p "$1/sill"
  printf '%s\n' "$BACKDROP_PNG" >"$1/sill/wallpaper-source"
}

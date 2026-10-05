# Captures and what they show. Needs OUT_DIR and `sill` (daemon.sh); needs PIL for the readers.

# Whether SURFACE $1 is mapped (it captures), into $2 (default the scratch probe).
mapped() { sill debug capture "$1" "${2:-$OUT_DIR/probe.png}" >/dev/null 2>&1; }

# Waits up to $2 tenths of a second for SURFACE $1 to be up (or, with $3 = down, gone).
wait_surface() {
  local surface="$1" tenths="$2" want="${3:-up}"
  for _ in $(seq "$tenths"); do
    if mapped "$surface"; then [ "$want" = up ] && return 0
    else [ "$want" = down ] && return 0; fi
    sleep 0.1
  done
  return 1
}

# Captures SURFACE $1 into $2, appending the daemon's reply to $OUT_DIR/capture.log.
capture() { sill debug capture "$1" "$2" >>"$OUT_DIR/capture.log" 2>&1; }

# How many pixels of the PNG $1 are not fully transparent.
opaque_pixels() {
  python3 - "$1" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
print(sum(1 for a in img.getchannel("A").tobytes() if a > 0))
PY
}

# Captures SURFACE $1 and prints the RGBA at ($2, $3), surface-local logical px, where the
# surface is $4 logical px wide.
pixel() {
  local png="$OUT_DIR/pixel.png"
  sill debug capture "$1" "$png" >/dev/null 2>&1 || return 1
  python3 - "$png" "$2" "$3" "$4" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
scale = img.width / int(sys.argv[4])
x, y = int(float(sys.argv[2]) * scale), int(float(sys.argv[3]) * scale)
print(*img.getpixel((x, y)))
PY
}

# The fraction (0 to 1) of pixels of PNG $1 that differ from PNG $2 by more than a small step in
# some channel; 1 when the two are not the same size.
changed_fraction() {
  python3 - "$1" "$2" <<'PY'
import sys
from PIL import Image
a = Image.open(sys.argv[1]).convert("RGBA")
b = Image.open(sys.argv[2]).convert("RGBA")
if a.size != b.size:
    print(1.0)
else:
    pa, pb = a.tobytes(), b.tobytes()
    n = a.width * a.height
    differ = sum(
        1 for i in range(n)
        if max(abs(pa[4 * i + c] - pb[4 * i + c]) for c in range(4)) > 12
    )
    print(f"{differ / n:.4f}")
PY
}

# Whether the decimal $1 is at least $2.
at_least() { python3 -c 'import sys; sys.exit(0 if float(sys.argv[1]) >= float(sys.argv[2]) else 1)' "$1" "$2"; }

# The look checks of the scripts that change what every surface shows (user-style.sh, desktop's
# appearance-reaches-apps scenario). They read SURFACES (the surface names), OUT_DIR, DAEMON_PID,
# and CHANGED / SETTLED (the share of a surface's pixels that must differ, or may differ once it
# has reverted).

# Waits up to 20 s for surface $1 to differ from its $2 capture by $CHANGED (or, with $3 = same,
# to be within $SETTLED of it); captures into $OUT_DIR/$4-$1.png. Prints the fraction.
wait_look() {
  local surface=$1 reference=$2 want=$3 tag=$4 fraction=1
  for _ in $(seq 100); do
    # A capture that does not answer yet (a loaded host) is a poll that saw nothing, not a failure.
    capture "$surface" "$OUT_DIR/$tag-$surface.png" || { sleep 0.2; continue; }
    fraction=$(changed_fraction "$OUT_DIR/$tag-$surface.png" "$OUT_DIR/$reference-$surface.png")
    if [ "$want" = same ]; then at_least "$SETTLED" "$fraction" && { echo "$fraction"; return 0; }
    else at_least "$fraction" "$CHANGED" && { echo "$fraction"; return 0; }; fi
    sleep 0.2
  done
  echo "$fraction"
  return 1
}

# Every surface reaches the look $2 (different, or same) against the capture $1, captured as $3,
# and the same daemon ($DAEMON_PID) still answers.
expect_all() {
  local reference=$1 want=$2 tag=$3 surface fraction
  for surface in "${SURFACES[@]}"; do
    fraction=$(wait_look "$surface" "$reference" "$want" "$tag") || {
      echo "$surface: $tag is not $want as $reference (changed fraction $fraction)" >&2; return 1; }
    echo "$surface: $tag vs $reference changed fraction $fraction ($want)" | tee -a "$OUT_DIR/steps.log" >&2
  done
  kill -0 "$DAEMON_PID" 2>/dev/null && sill ping >/dev/null 2>&1 || {
    echo "the daemon ($DAEMON_PID) is gone or not answering" >&2; return 1; }
}

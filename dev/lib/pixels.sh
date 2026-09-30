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

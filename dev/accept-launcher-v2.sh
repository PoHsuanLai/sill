#!/usr/bin/env bash
# Launcher v2 acceptance (FINDINGS "M9 freeze", lane d "d-preview"), on the nested cosmic-comp
# (dev/nested.sh; a running wayland-1 is pinged first, reused and never stopped). Starts
# `sill run` from this checkout's debug build (`--features debug`) with a scratch HOME holding a
# PDF, a PNG, a Markdown file and six more `m9-*` documents, `launcher.files_backend = "fd"`, a
# scratch XDG_CONFIG_HOME, XDG_STATE_HOME and XDG_CACHE_HOME, its own XDG_RUNTIME_DIR (the
# nested socket as an absolute WAYLAND_DISPLAY) and a private session bus
# (dev/private-bus.conf). No key is typed anywhere: every key goes through
# `sill debug launcher-key`, every look through `sill debug launcher-rows` and
# `sill debug capture` of the nested panel. The daemon is quit at the end.
#
#   1. Open latency: 10 warm `sill launcher open` / `close`, the panel's
#      `launcher.open_to_frame_ms` p50/p95/max (FINDINGS F149 and the M4 figures are the
#      master numbers to hold against; asserted under the plan's 100 ms p95 budget only as a
#      report, since the host's load moves it, F170).
#   2. `--query m9` lists Top Hit, then Documents.
#   3. `down`, `space` (Top Hit is the PDF; `up` first so the selection is on it): the preview is
#      `pdf`, captured (dev/out/launcher-v2-preview-pdf.png).
#   4. `down` to the PNG: `image`; `space` hides it.
#   5. `quick-look` while typing shows it; `right` walks the caret to the end of the query (the
#      field keeps its caret where the opening's query left it: at the start, Q341) and one more
#      enters the pane; `escape` gives the keyboard back (the pane stays), `escape` hides the
#      pane, `escape` closes the launcher. On a row with actions: an `m9` document, else, while
#      the files provider (lane a) answers nothing, the Ghostty app row (it has desktop actions).
#   6. Documents over its cap reports `hidden > 0`; Enter on the "Show More" line reveals them.
#   7. `tab` to Emoji with nothing typed (Shift+Tab twice: Web, then Emoji) shows the emoji
#      (dev/out/launcher-v2-emoji.png; rows until quire's EmojiGrid, Q291).
#   8. 5 s idle after closing: 0 frames on the launcher's surfaces.
#
# A step that needs rows lanes a (files) and c (emoji) fill exits 2 (SKIP) while those
# providers answer nothing on this branch; `crates/sill-surfaces/tests/launcher_preview.rs`
# proves the same steps over fixtures (FINDINGS "M9 lane d"). Exit 0 only when every step
# passed.
#
# Usage: dev/nested.sh && dev/accept-launcher-v2.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-launcher-v2"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"
OPENINGS=10

source "$SHELL_HOST_DEV/.nested-env"
HOST_RUNTIME="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
NESTED_SOCKET="$HOST_RUNTIME/$WAYLAND_DISPLAY"
RUNTIME=$(mktemp -d)
chmod 700 "$RUNTIME"

sill() { XDG_RUNTIME_DIR="$RUNTIME" "$SILL_BIN" "$@"; }
key() { sill debug launcher-key "$1" >>"$OUT_DIR/keys.log" 2>&1; sleep 0.25; }
rows() { sill debug launcher-rows 2>>"$OUT_DIR/rows.log"; }

# One field of `sill debug launcher-rows`: preview, selected, headings, hidden:<title>,
# rows:<title>, index:<title>:<row> (the line's index over every section), query.
field() {
  rows | python3 -c '
import json, sys
text = sys.stdin.read().strip()
if not text:
    sys.exit(1)
v = json.loads(text)["v"]
want = sys.argv[1]
secs = v["sections"]
if want == "preview":
    p = v["preview"]
    print("hidden" if p["kind"] == "hidden" else p["v"])
elif want == "selected":
    print(v["selected"])
elif want == "query":
    print(v["query"])
elif want == "headings":
    print(",".join(s["title"] for s in secs))
elif want.startswith("hidden:"):
    print(next((s["hidden"] for s in secs if s["title"] == want[7:]), ""))
elif want.startswith("rows:"):
    print("|".join(next((s["rows"] for s in secs if s["title"] == want[5:]), [])))
elif want.startswith("index:"):
    _, title, row = want.split(":", 2)
    at = 0
    for s in secs:
        for r in s["rows"]:
            if s["title"] == title and r == row:
                print(at); sys.exit(0)
            at += 1
' "$1"
}

open_query() {
  sill launcher open --query "$1" >>"$OUT_DIR/keys.log" 2>&1
  for _ in $(seq 40); do
    [ -n "$(field headings 2>/dev/null)" ] && break
    sleep 0.1
  done
  sleep 0.4
}

close_launcher() { sill launcher close >>"$OUT_DIR/keys.log" 2>&1; sleep 0.3; }

is_open() { field query >/dev/null 2>&1; }

output_name() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

launcher_frames() {
  sill debug stats | python3 -c '
import json, sys
print(sum(r["frames"] for r in json.loads(sys.stdin.read())["v"] if r["surface"].startswith("sill-launcher-")))'
}

capture() { sill debug capture "sill-launcher-$NAME" "$1" >>"$OUT_DIR/capture.log" 2>&1; }

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2
    return 1
  }
}

# A scratch home: the documents the files provider's `fd` finds under it.
seed_home() {
  SCRATCH_HOME=$(mktemp -d)
  mkdir -p "$SCRATCH_HOME/Documents" "$SCRATCH_HOME/notes"
  python3 - "$SCRATCH_HOME" <<'EOF'
import sys, zlib, struct
home = sys.argv[1]
def pdf():
    rings = "".join(f"{20+(n*7)%160} {20+(n*13)%220} m {29+(n*7)%160} {20+(n*13)%220} {20+(n*7)%160} {29+(n*13)%220} {20+(n*7)%160} {20+(n*13)%220} c S\n" for n in range(3000))
    stream = "0.2 0.4 0.8 RG 0.3 w\n0.9 0.9 0.9 rg 20 20 160 220 re f\n" + rings
    objs = ["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 260] /Contents 4 0 R >>",
            f"<< /Length {len(stream)} >>\nstream\n{stream}endstream"]
    out, offs = b"%PDF-1.4\n", []
    for i, o in enumerate(objs):
        offs.append(len(out)); out += f"{i+1} 0 obj\n{o}\nendobj\n".encode()
    x = len(out)
    out += f"xref\n0 {len(objs)+1}\n0000000000 65535 f \n".encode()
    out += b"".join(f"{o:010} 00000 n \n".encode() for o in offs)
    out += f"trailer\n<< /Size {len(objs)+1} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n".encode()
    return out
def png(w=64, h=48):
    raw = b"".join(b"\0" + b"".join(bytes((40,120,220) if x < w//2 else (240,180,40)) for x in range(w)) for _ in range(h))
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")
open(f"{home}/Documents/m9-report.pdf", "wb").write(pdf())
open(f"{home}/Documents/m9-shot.png", "wb").write(png())
open(f"{home}/notes/m9-notes.md", "w").write("# M9 notes\n\nThe preview pane reads this head.\n")
for n in range(1, 7):
    open(f"{home}/Documents/m9-extra-{n}.pdf", "wb").write(pdf())
EOF
}

start_daemon() {
  CONFIG=$(mktemp -d)
  STATE=$(mktemp -d)
  CACHE=$(mktemp -d)
  mkdir -p "$CONFIG/sill"
  printf '[launcher]\nfiles_backend = "fd"\nweb_search = "on"\n' >"$CONFIG/sill/settings.toml"
  HOME="$SCRATCH_HOME" XDG_CONFIG_HOME="$CONFIG" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" \
    XDG_RUNTIME_DIR="$RUNTIME" WAYLAND_DISPLAY="$NESTED_SOCKET" \
    dbus-run-session --config-file="$SILL_REPO/dev/private-bus.conf" -- \
    "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "${NAME:-}" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  sleep 1
  echo "daemon on a private bus, runtime dir $RUNTIME, home $SCRATCH_HOME, output $NAME" >&2
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  rm -rf "$CONFIG" "$STATE" "$CACHE" "$SCRATCH_HOME"
}

percentiles() {
  python3 -c '
import sys
v = sorted(float(l) for l in sys.stdin if l.strip())
if not v: sys.exit(0)
pct = lambda p: v[min(len(v) - 1, int(round(p * (len(v) - 1))))]
print(f"{pct(.5):.1f} {pct(.95):.1f} {v[-1]:.1f}")'
}

# Whether the files provider answers: lane a fills it.
documents_answer() { [[ ",$(field headings)," == *",Documents,"* ]]; }

# 1.
step_latency() {
  local before n p50 p95 max
  open_query ""; close_launcher # a cold opening first
  before=$(grep -c 'launcher\.open_to_frame_ms=' "$OUT_DIR/daemon.log")
  for _ in $(seq "$OPENINGS"); do
    sill launcher open --query "" >>"$OUT_DIR/keys.log" 2>&1
    sleep 0.6
    close_launcher
  done
  n=$(( $(grep -c 'launcher\.open_to_frame_ms=' "$OUT_DIR/daemon.log") - before ))
  read -r p50 p95 max < <(grep -o 'launcher\.open_to_frame_ms=[0-9.]*' "$OUT_DIR/daemon.log" \
    | tail -n "$n" | cut -d= -f2 | percentiles)
  echo "$n warm openings: open_to_frame_ms p50 ${p50:-?} p95 ${p95:-?} max ${max:-?}" | tee "$OUT_DIR/latency.log" >&2
  [ "$n" -eq "$OPENINGS" ]
}

# 2.
step_sections() {
  open_query m9
  echo "m9 headings: $(field headings)" >&2
  documents_answer || { echo "SKIP: no Documents (lane a's files provider answers nothing here)" >&2; return 2; }
  [[ "$(field headings)" == "Top Hit,Documents"* ]]
}

# 3. and 4.
step_preview_pdf_and_image() {
  documents_answer || { echo "SKIP: no Documents (lane a)" >&2; return 2; }
  key up; key down; key up
  key space
  echo "after up/down/up, space: preview $(field preview)" >&2
  [ "$(field preview)" = pdf ] || return 1
  sleep 1.5
  capture "$SHOTS/launcher-v2-preview-pdf.png" || return 1
  local png
  png=$(field index:Documents:m9-shot.png)
  if [ -z "$png" ] && [ -n "$(field "index:Documents:Show More")" ]; then
    # Behind Show More (the index ranks names its own way): open the section first.
    local more
    more=$(field "index:Documents:Show More")
    while [ "$(field selected)" -lt "$more" ]; do key down; done
    key enter
    png=$(field index:Documents:m9-shot.png)
  fi
  [ -n "$png" ] || { echo "no m9-shot.png row: $(field rows:Documents)" >&2; return 1; }
  while [ "$(field selected)" -gt "$png" ]; do key up; done
  while [ "$(field selected)" -lt "$png" ]; do key down; done
  echo "on the PNG: preview $(field preview)" >&2
  [ "$(field preview)" = image ] || return 1
  capture "$SHOTS/launcher-v2-preview-image.png"
  key space
  [ "$(field preview)" = hidden ]
}

# 5.
step_pane_levels() {
  local query=m9
  documents_answer || query=ghostty
  close_launcher
  open_query "$query"
  echo "pane levels on --query $query ($(field headings))" >&2
  key quick-look
  [ "$(field preview)" != hidden ] || { echo "quick-look showed nothing" >&2; return 1; }
  for _ in $(seq ${#query}); do key right; done # the caret to the end
  key right # into the pane
  capture "$SHOTS/launcher-v2-pane-focus.png"
  key escape
  echo "escape 1: preview $(field preview), open $(is_open && echo yes || echo no)" >&2
  [ "$(field preview)" != hidden ] || { echo "the first Escape hid the pane: it never had the keyboard" >&2; return 1; }
  key escape
  echo "escape 2: preview $(field preview)" >&2
  [ "$(field preview)" = hidden ] || return 1
  is_open || return 1
  key escape
  ! is_open
}

# 6.
step_show_more() {
  close_launcher
  open_query m9
  documents_answer || { echo "SKIP: no Documents (lane a)" >&2; return 2; }
  local hidden more
  hidden=$(field hidden:Documents)
  echo "Documents hides $hidden" >&2
  [ "${hidden:-0}" -gt 0 ] || return 1
  more=$(field "index:Documents:Show More")
  while [ "$(field selected)" -lt "$more" ]; do key down; done
  key enter
  echo "after Show More: hidden $(field hidden:Documents), rows $(field rows:Documents)" >&2
  [ "$(field hidden:Documents)" = 0 ]
}

# 7.
step_emoji() {
  close_launcher
  open_query ""
  key shift-tab
  key shift-tab
  sleep 0.5
  echo "after two Shift+Tab: $(field headings) [$(field rows:Emoji)]" >&2
  [ -n "$(field rows:Emoji)" ] || { echo "SKIP: no Emoji rows (lane c's provider answers nothing here)" >&2; close_launcher; return 2; }
  capture "$SHOTS/launcher-v2-emoji.png"
  close_launcher
}

# 8.
step_idle() {
  close_launcher
  local before after
  before=$(launcher_frames)
  sleep 5
  after=$(launcher_frames)
  echo "closed: launcher frames $before, after 5 s idle $after" >&2
  [ "$before" = "$after" ]
}

# F. The same looks over the fakes' fixtures (`tests/launcher_support/m9.rs`: a PDF, a PNG, a
#    Markdown file, five more PDFs, a clipboard entry, emoji), on `examples/launcher_demo.rs` on
#    the nested compositor after the daemon has quit, while lanes a and c are unmerged:
#    dev/out/launcher-v2-{preview-pdf,preview-image,preview-text,show-more,pane-focus,emoji}-fixtures.png.
step_fixtures() {
  local demo_bin="$SILL_REPO/target/debug/examples/launcher_demo" log="$OUT_DIR/demo.log"
  local fifo="$OUT_DIR/demo.in" at
  rm -f "$fifo"; mkfifo "$fifo"
  "$SHELL_HOST_DEV/run.sh" "$demo_bin" <"$fifo" >"$log" 2>&1 &
  local demo=$!
  exec 7>"$fifo"
  sleep 2
  shoot() { echo "sill-launcher-$NAME capture $SHOTS/launcher-v2-$1-fixtures.png" >&7; sleep 0.8; }
  echo "open m9" >&7; sleep 2.5
  echo "press quick-look" >&7; sleep 2.5
  shoot preview-pdf
  echo "press down" >&7; sleep 1.2
  shoot preview-image
  echo "press down" >&7; sleep 1.2
  shoot preview-text
  echo "press quick-look" >&7; sleep 0.5
  # From the Markdown file (line 2) to the Show More line (6).
  for at in 1 2 3 4; do echo "press down" >&7; sleep 0.3; done
  echo "press enter" >&7; sleep 1.2
  shoot show-more
  echo "open m9" >&7; sleep 2
  echo "press quick-look" >&7; sleep 1.5
  for at in 1 2 3; do echo "press right" >&7; sleep 0.3; done
  sleep 1
  shoot pane-focus
  echo "open" >&7; sleep 2
  echo "press shift-tab" >&7; sleep 0.6
  echo "press shift-tab" >&7; sleep 1.5
  shoot emoji
  echo "close" >&7; sleep 0.5
  echo "quit" >&7
  exec 7>&-
  wait "$demo" 2>/dev/null
  rm -f "$fifo"
  local missing=0
  for at in preview-pdf preview-image preview-text show-more pane-focus emoji; do
    [ -s "$SHOTS/launcher-v2-$at-fixtures.png" ] || { echo "no $at capture" >&2; missing=1; }
  done
  [ "$missing" -eq 0 ]
}

"$SHELL_HOST_DEV/run.sh" wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
build || exit 1
(cd "$SILL_REPO" && cargo build -p sill-surfaces --example launcher_demo) >>"$OUT_DIR/build.log" 2>&1 || {
  echo "the demo did not build (see $OUT_DIR/build.log)" >&2
  exit 1
}
for _ in $(seq 600); do
  pgrep -f 'target/[a-z]*/sill run' >/dev/null || break
  [ -n "${WAITED:-}" ] || { echo "another sill daemon is running; waiting for it to quit" >&2; WAITED=1; }
  sleep 1
done
: >"$OUT_DIR/keys.log"
seed_home
start_daemon || { stop_daemon; exit 1; }
status=0
for step in step_latency step_sections step_preview_pdf_and_image step_pane_levels \
  step_show_more step_emoji step_idle; do
  "$step"
  rc=$?
  case "$rc" in
    0) echo "PASS $step" ;;
    2) echo "SKIP $step"; [ "$status" -eq 0 ] && status=2 ;;
    *) echo "FAIL $step (exit $rc)"; status=1 ;;
  esac
done
stop_daemon
if step_fixtures; then echo "PASS step_fixtures"; else echo "FAIL step_fixtures"; status=1; fi
rm -rf "$RUNTIME"
echo "logs in $OUT_DIR; captures in $SHOTS/launcher-v2-*.png"
exit "$status"

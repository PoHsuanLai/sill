#!/usr/bin/env bash
# Launcher v2 under real keyboard input (M9 lane e, FINDINGS "The M9 fill-in" (e) and "M9 lane
# e"), on shell-host's private headless KWin (scripts/dev/kwin-virtual.sh, FINDINGS F52) driven
# by shell-host's scripts/dev/fake-input.sh: every key is a real Wayland key event with a real
# serial and keyboard focus, reaching the palette's field and quire's key claim as a person's
# would. `sill debug launcher-rows` and `sill debug capture` only look; no key goes through
# `sill debug launcher-key`.
#
# The daemon runs from this checkout's debug build (`--features debug`) on the headless KWin's
# socket (an absolute WAYLAND_DISPLAY) and KWin's private D-Bus bus (shell-host's
# private-bus.conf: nothing activatable), with its own XDG_RUNTIME_DIR and a scratch HOME,
# XDG_CONFIG_HOME, XDG_DATA_HOME, XDG_STATE_HOME and XDG_CACHE_HOME; `launcher.files_backend =
# "fd"`. The bus goes to the daemon (and the stub file manager, as an argument) only, never to
# this shell (G52). ONE fake-input client serves the whole run (G50). Nothing here reaches the
# user's session (wayland-0), the nested compositor (wayland-1) or the real session bus;
# `wl-copy`/`wl-paste` get KWin's socket by absolute path and only made-up strings, and no
# clipboard content is printed.
#
#   1. Real typing of a file's name lists it (Top Hit); real Down then Space opens the preview
#      (and types no space), real Space closes it, real Ctrl+Y (⌘Y) opens it while typing.
#   2. Real → twice enters the pane (the pane shows after the first); real Tab walks its
#      controls (the focus ring moves inside the pane on each Tab and comes back round, the
#      provider filter is untouched); real Escape three times closes one level each: the ring
#      goes (the pane stays), the pane hides, the launcher closes.
#   3. Real Ctrl+R (⌘R) on a file row reveals it: the stub file manager
#      (examples/file_manager_stub.rs) records ShowItems with its file:// URI and a token.
#   4. Real Shift+Tab twice with nothing typed filters to Emoji (the grid); real Down, Down,
#      Right, Left, Up each move the selection by a row or a cell; real Enter copies the
#      selected glyph (`wl-paste` on KWin's socket reads it).
#   5. The clipboard: KWin offers ext_data_control_manager_v1 (G230), so `sill clipboard
#      status` says `memory ext`; a made-up copy made on KWin's socket, then another, shows as
#      one clipboard row for its word; real Enter restores it (`wl-paste` reads it back). Were
#      neither protocol offered, status would say `unsupported` and the step would be NOT
#      PROVEN.
#   Q340. Real Down past the card's visible rows (Documents filtered, 24 rows): is the selected
#      row scrolled into view? Real Up back to the first row. Recorded, not asserted (quire's
#      open gap): OPEN while the selected row is off the card.
#   Q341. `sill launcher open --query m9`, then a real "x": where the caret starts. Recorded, not
#      asserted: OPEN while the query becomes "xm9" (the caret at the start). Then real End and
#      →: the pane shows (the claim reads the caret at the end).
#
# Captures: target/accept-launcher-v2-real-input/*.png. Then the input client, the stub file
# manager, the daemon (`sill quit`) and the headless KWin are stopped.
#
# Exit 0 only when steps 1-5 passed; 1 when one failed; 2 when none failed but one was NOT
# PROVEN. The Q340/Q341 lines never change the exit status.
#
# Usage: dev/accept-launcher-v2-real-input.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-launcher-v2-real-input"
mkdir -p "$OUT_DIR"
KWIN_ENV="$SHELL_HOST_DEV/.kwin-env"
STUB_FM="$SILL_REPO/target/debug/examples/file_manager_stub"
SIZE=1920x1080

SCRATCH=$(mktemp -d)
# A short runtime folder of its own: the agent socket's path must fit in 108 bytes.
RUNTIME=$(mktemp -d /tmp/sill-rt.XXXXXX)
HOME_DIR="$SCRATCH/home"
CONFIG="$SCRATCH/config"
mkdir -p "$HOME_DIR/Documents" "$HOME_DIR/notes" "$CONFIG/sill" "$SCRATCH/data" \
  "$SCRATCH/state" "$SCRATCH/cache"
chmod 700 "$RUNTIME"
DAEMON=
INPUT=
FM=
WL=
BUS=
NAME=

in_env() {
  env HOME="$HOME_DIR" XDG_RUNTIME_DIR="$RUNTIME" XDG_CONFIG_HOME="$CONFIG" \
    XDG_DATA_HOME="$SCRATCH/data" XDG_STATE_HOME="$SCRATCH/state" \
    XDG_CACHE_HOME="$SCRATCH/cache" WAYLAND_DISPLAY="$WL" DBUS_SESSION_BUS_ADDRESS="$BUS" "$@"
}
sill() { in_env "$SILL_BIN" "$@"; }
# KWin's clipboard only: its socket by absolute path, stderr (never content) to the log.
kwin_wl() { env WAYLAND_DISPLAY="$WL" "$@" 2>>"$OUT_DIR/wl.log"; }

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug &&
    cargo build -p sill-services --example file_manager_stub) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

# The headless KWin, its socket (absolute) and bus read from its env file without sourcing it
# into this shell: `kwin-virtual.sh stop` must never find this script naming the bus (G52).
start_kwin() {
  # Never borrow (and later stop) a headless KWin someone else started.
  [ ! -e "$KWIN_ENV" ] || { echo "$KWIN_ENV exists: another headless KWin is running" >&2; return 1; }
  "$SHELL_HOST_DEV/kwin-virtual.sh" start "$SIZE" >"$OUT_DIR/kwin.log" 2>&1 || return 1
  local runtime socket
  runtime=$(sed -n 's/^export XDG_RUNTIME_DIR=//p' "$KWIN_ENV")
  socket=$(sed -n 's/^export WAYLAND_DISPLAY=//p' "$KWIN_ENV")
  BUS=$(sed -n "s/^export DBUS_SESSION_BUS_ADDRESS='\\(.*\\)'$/\\1/p" "$KWIN_ENV")
  case "$socket" in /*) WL="$socket" ;; *) WL="$runtime/$socket" ;; esac
  case "$WL" in
    *wayland-kwinvirt-*) ;;
    *) echo "refusing $WL: not the headless KWin's socket" >&2; stop_kwin; return 1 ;;
  esac
  [ -S "$WL" ] && [ -n "$BUS" ] || { echo "no socket or bus in $KWIN_ENV" >&2; stop_kwin; return 1; }
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "KWin's bus is the session's" >&2; stop_kwin; return 1; }
}

stop_kwin() { "$SHELL_HOST_DEV/kwin-virtual.sh" stop >>"$OUT_DIR/kwin.log" 2>&1; }

# Real input through ONE fake-input client for the whole run (G50).
start_input() {
  rm -f "$OUT_DIR/input.fifo"; mkfifo "$OUT_DIR/input.fifo" || return 1
  "$SHELL_HOST_DEV/fake-input.sh" <"$OUT_DIR/input.fifo" >"$OUT_DIR/input.log" 2>&1 &
  INPUT=$!
  exec 3>"$OUT_DIR/input.fifo"
}

# Wait up to 5 s for process $1 (named $2) to end, then end it.
reap() {
  for _ in $(seq 50); do kill -0 "$1" 2>/dev/null || return 0; sleep 0.1; done
  echo "the $2 ($1) did not end; killing it" >&2
  kill "$1" 2>/dev/null
  sleep 0.5
  kill -9 "$1" 2>/dev/null
  return 0
}

stop_input() {
  [ -n "$INPUT" ] || return 0
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
  for _ in $(seq 400); do
    [ "$(grep -c '^ok ' "$OUT_DIR/input.log")" -ge $(( before + $# )) ] && return 0
    sleep 0.05
  done
  echo "fake input did not answer: $*" >&2
  return 1
}
# Real keys, one after another: each pressed, held 30 ms, released, then 250 ms to settle.
key() { local k; for k in "$@"; do input "key $k down" "sleep 30" "key $k up" "sleep 250" || return 1; done; }
# A real chord: $1 held around $2.
chord() {
  input "key $1 down" "sleep 30" "key $2 down" "sleep 30" "key $2 up" "sleep 30" "key $1 up" \
    "sleep 250"
}
typed() { input "type \"$1\"" "sleep 250"; }

start_fm() {
  "$STUB_FM" "$BUS" >"$OUT_DIR/file-manager.log" 2>&1 &
  FM=$!
  for _ in $(seq 50); do grep -q '^ready' "$OUT_DIR/file-manager.log" && return 0; sleep 0.1; done
  echo "the stub file manager never started" >&2
  return 1
}

stop_fm() {
  [ -n "$FM" ] || return 0
  kill "$FM" 2>/dev/null
  wait "$FM" 2>/dev/null
  FM=
}

# A scratch home: what the files provider's `fd` finds under it. A PDF with a real page, a
# PNG, a Markdown note, and 24 `m9-list-NN.txt` for a list longer than the card (Q340).
seed_home() {
  python3 - "$HOME_DIR" <<'EOF'
import sys, zlib, struct
home = sys.argv[1]
def pdf():
    rings = "".join(f"{20+(n*7)%160} {20+(n*13)%220} m {29+(n*7)%160} {20+(n*13)%220} {20+(n*7)%160} {29+(n*13)%220} {20+(n*7)%160} {20+(n*13)%220} c S\n" for n in range(300))
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
for n in range(1, 25):
    open(f"{home}/Documents/m9-list-{n:02}.txt", "w").write(f"list note {n}\n")
EOF
}

start_daemon() {
  printf '[launcher]\nfiles_backend = "fd"\nweb_search = "on"\nclipboard_history = "memory"\n' \
    >"$CONFIG/sill/settings.toml"
  in_env "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && break; sleep 0.1; done
  for _ in $(seq 50); do NAME=$(output_name); [ -n "$NAME" ] && break; sleep 0.1; done
  [ -n "$NAME" ] || { echo "no bar in 'sill debug stats'" >&2; return 1; }
  sleep 1.5
  echo "daemon on KWin's bus, runtime $RUNTIME, home $HOME_DIR, output $NAME" >&2
}

# Ask the daemon to quit and wait up to 10 s; never kill it (FINDINGS G51).
stop_daemon() {
  [ -n "$DAEMON" ] || return 0
  in_env timeout 10 "$SILL_BIN" quit >/dev/null 2>&1
  local result=1
  for _ in $(seq 100); do kill -0 "$DAEMON" 2>/dev/null || { result=0; break; }; sleep 0.1; done
  [ "$result" = 0 ] || echo "the daemon ($DAEMON) did not end after quit" >&2
  DAEMON=
  return "$result"
}

output_name() {
  sill debug stats 2>/dev/null | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    s = row["surface"]
    if s.startswith("sill-bar-") and "/" not in s:
        print(s[len("sill-bar-"):]); break'
}

# One field of `sill debug launcher-rows` (clipboard titles are "<clipboard>" there): preview,
# selected, query, headings, count, hidden:<title>, rows:<title>, row:<index> (the line at that
# index over every section), where:<row> (the section titles holding that row). Empty and
# exit 1 while the launcher is closed.
field() {
  sill debug launcher-rows 2>/dev/null | python3 -c '
import json, sys
text = sys.stdin.read().strip()
try:
    doc = json.loads(text)
except ValueError:
    sys.exit(1)
if doc.get("kind") != "launcher_rows":
    sys.exit(1)
v = doc["v"]
want = sys.argv[1]
secs = v["sections"]
lines = [r for s in secs for r in s["rows"]]
if want == "preview":
    p = v["preview"]
    print("hidden" if p["kind"] == "hidden" else p["v"])
elif want == "selected":
    print(v["selected"])
elif want == "query":
    print(v["query"])
elif want == "headings":
    print(",".join(s["title"] for s in secs))
elif want == "count":
    print(len(lines))
elif want.startswith("hidden:"):
    print(next((s["hidden"] for s in secs if s["title"] == want[7:]), ""))
elif want.startswith("rows:"):
    print("|".join(next((s["rows"] for s in secs if s["title"] == want[5:]), [])))
elif want.startswith("row:"):
    at = int(want[4:])
    print(lines[at] if 0 <= at < len(lines) else "")
elif want.startswith("where:"):
    print(",".join(s["title"] for s in secs if want[6:] in s["rows"]))
' "$1"
}

is_open() { field query >/dev/null 2>&1; }

# Wait up to 3 s for field $1 to read $2.
await_field() {
  local got=
  for _ in $(seq 30); do
    got=$(field "$1")
    [ "$got" = "$2" ] && return 0
    sleep 0.1
  done
  echo "$1 is '$got', wanted '$2'" >&2
  return 1
}

# Open the launcher with nothing typed (the field has the keyboard), wait for its rows.
open_empty() {
  sill launcher close >/dev/null 2>&1
  sleep 0.3
  sill launcher open >/dev/null 2>&1
  for _ in $(seq 40); do is_open && break; sleep 0.1; done
  sleep 0.5
  is_open
}

close_launcher() { sill launcher close >/dev/null 2>&1; sleep 0.3; }

capture() { sleep 0.6; sill debug capture "sill-launcher-$NAME" "$OUT_DIR/$1.png" >>"$OUT_DIR/capture.log" 2>&1; }

# The box two captures differ in, "x0 y0 x1 y1 width" (the first capture's width), or "none".
diff_box() {
  python3 - "$OUT_DIR/$1.png" "$OUT_DIR/$2.png" <<'EOF'
import sys
from PIL import Image, ImageChops
a, b = (Image.open(p).convert("RGBA") for p in sys.argv[1:3])
if a.size != b.size:
    print(0, 0, a.size[0], a.size[1], a.size[0]); sys.exit(0)
box = ImageChops.difference(a, b).getbbox()
print(*box, a.size[0]) if box else print("none")
EOF
}

# How many pixels of a capture carry the selected row's tint (quire's light selection, about
# rgb(220, 229, 243)): a selected row on the card shows thousands.
selection_pixels() {
  python3 - "$OUT_DIR/$1.png" <<'EOF'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGB")
w, h = img.size
n = 0
for y in range(0, h, 2):
    for x in range(0, w, 2):
        r, g, b = img.getpixel((x, y))
        if 205 <= r <= 232 and 215 <= g <= 240 and b >= 236 and b - r >= 15:
            n += 1
print(n)
EOF
}

# 1.
step_type_and_preview() {
  open_empty || { echo "the launcher did not open" >&2; return 1; }
  typed "m9-report" || return 1
  await_field rows:"Top Hit" m9-report.pdf || return 1
  echo "real typing 'm9-report': Top Hit m9-report.pdf" >&2
  key BackSpace BackSpace BackSpace BackSpace BackSpace BackSpace BackSpace || return 1
  await_field query m9 || return 1
  sleep 0.5
  local second
  second=$(field row:1)
  key Down || return 1
  await_field selected 1 || return 1
  key space || return 1
  local shown
  shown=$(field preview)
  capture preview-space
  echo "backspaced to 'm9' ($(field headings)); real Down to '$second', real Space: preview" \
    "$shown, query '$(field query)'" >&2
  [ "$shown" != hidden ] && [ "$(field query)" = m9 ] || return 1
  key space || return 1
  await_field preview hidden || return 1
  [ "$(field query)" = m9 ] || { echo "Space typed into the field: '$(field query)'" >&2; return 1; }
  echo "real Space again: preview hidden, query still 'm9'" >&2
  typed "-rep" || return 1
  await_field query m9-rep || return 1
  chord Control_L y || return 1
  await_field preview pdf || return 1
  capture preview-ctrl-y
  echo "typing '-rep', real Ctrl+Y (⌘Y): preview pdf while typing" >&2
}

# 2.
step_pane_keys() {
  open_empty || return 1
  typed "m9-report" || return 1
  await_field rows:"Top Hit" m9-report.pdf || return 1
  [ "$(field preview)" = hidden ] || return 1
  key Right || return 1
  await_field preview pdf || { echo "the first → did not show the pane" >&2; return 1; }
  capture pane-shown
  key Right || return 1
  capture pane-tab0
  local headings b01 b12 b02 e1 x0 width shown_ring
  headings=$(field headings)
  key Tab || return 1
  capture pane-tab1
  key Tab || return 1
  capture pane-tab2
  b01=$(diff_box pane-tab0 pane-tab1)
  b12=$(diff_box pane-tab1 pane-tab2)
  b02=$(diff_box pane-tab0 pane-tab2)
  echo "real → twice; the ring before and after real Tab differs in [$b01], after a second" \
    "Tab in [$b12]; the first and third look the same: [$b02]; headings '$headings' ->" \
    "'$(field headings)'" >&2
  [ "$(field headings)" = "$headings" ] || { echo "Tab changed the provider filter" >&2; return 1; }
  [ "$b01" != none ] && [ "$b12" != none ] && [ "$b02" = none ] || return 1
  read -r x0 _ _ _ width <<<"$b01"
  [ "$x0" -gt $(( width * 6 / 10 )) ] || { echo "the Tab moved something left of the pane (x $x0)" >&2; return 1; }
  shown_ring=$(diff_box pane-shown pane-tab0)
  echo "the ring appeared on entering the pane in [$shown_ring]" >&2
  key Escape || return 1
  capture pane-escape1
  e1=$(diff_box pane-shown pane-escape1)
  echo "real Escape 1: preview $(field preview), the ring gone (against before entering: [$e1])" >&2
  [ "$(field preview)" = pdf ] && [ "$e1" = none ] || return 1
  key Escape || return 1
  await_field preview hidden || return 1
  is_open || { echo "the second Escape closed the launcher" >&2; return 1; }
  echo "real Escape 2: the pane hides, the launcher stays" >&2
  key Escape || return 1
  sleep 0.3
  is_open && { echo "the third Escape left the launcher open" >&2; return 1; }
  echo "real Escape 3: the launcher closes" >&2
}

# 3.
step_reveal() {
  start_fm || return 1
  open_empty || return 1
  typed "m9-report" || return 1
  await_field rows:"Top Hit" m9-report.pdf || return 1
  chord Control_L r || return 1
  local want="file://$HOME_DIR/Documents/m9-report.pdf" line=
  for _ in $(seq 30); do
    line=$(grep -P "^ShowItems\t" "$OUT_DIR/file-manager.log" | tail -1)
    [ -n "$line" ] && break
    sleep 0.1
  done
  stop_fm
  local uri token
  uri=$(cut -f2 <<<"$line"); token=$(cut -f3 <<<"$line")
  echo "real Ctrl+R (⌘R): the stub file manager got ShowItems ${uri:-(nothing)} token" \
    "'${token}'; launcher open afterwards: $(is_open && echo yes || echo no)" >&2
  [ "$uri" = "$want" ] && [ -n "$token" ]
}

# 4.
step_emoji() {
  open_empty || return 1
  chord Shift_L Tab || return 1
  chord Shift_L Tab || return 1
  await_field headings Emoji || return 1
  local cells at before moved=0 k want
  cells=$(field count)
  capture emoji-grid
  echo "real Shift+Tab twice, nothing typed: the Emoji grid, $cells cells" >&2
  at=$(field selected)
  for k in Down:8 Down:8 Right:1 Left:-1 Up:-8; do
    before=$at
    key "${k%%:*}" || return 1
    sleep 0.2
    at=$(field selected)
    want=$(( before + ${k#*:} ))
    if [ "$at" = "$want" ]; then
      echo "real ${k%%:*}: cell $before -> $at" >&2
    else
      echo "real ${k%%:*}: cell $before -> $at, wanted $want (preview $(field preview))" >&2
      moved=1
    fi
  done
  capture emoji-moved
  local glyph pasted
  glyph=$(field row:"$at")
  kwin_wl wl-copy --clear
  key Return || return 1
  for _ in $(seq 30); do
    pasted=$(kwin_wl timeout 2 wl-paste --no-newline 2>/dev/null)
    [ "$pasted" = "$glyph" ] && break
    sleep 0.1
  done
  echo "real Enter on cell $at: wl-paste on KWin's socket reads the selected glyph:" \
    "$([ "$pasted" = "$glyph" ] && echo yes || echo no); launcher open: $(is_open && echo yes || echo no)" >&2
  [ "$pasted" = "$glyph" ] && [ -n "$glyph" ] && [ "$moved" = 0 ]
}

# "mode protocol entries" from `sill clipboard status` (counts only).
clip_status() {
  sill clipboard status 2>/dev/null | python3 -c '
import json, sys
try:
    v = json.loads(sys.stdin.read())["v"]
except (ValueError, KeyError):
    sys.exit(0)
print(v.get("mode", "?"), v.get("protocol", "?"), v.get("entries", "?"))'
}

# 5. Returns 2 (NOT PROVEN) when KWin offers no data-control manager.
step_clipboard() {
  local status word=lighthouse made_up="m9-real-input lighthouse 7731 made-up words"
  status=$(clip_status)
  echo "sill clipboard status on KWin: $status" >&2
  case "$status" in
    "memory unsupported"*) echo "KWin offers no data-control manager (G230)" >&2; return 2 ;;
    "memory ext"* | "memory wlr"*) ;;
    *) return 1 ;;
  esac
  kwin_wl wl-copy -- "$made_up" || return 1
  sleep 0.5
  kwin_wl wl-copy -- "something else, also made up" || return 1
  sleep 0.5
  echo "two made-up copies on KWin's socket: $(clip_status)" >&2
  open_empty || return 1
  typed "$word" || return 1
  await_field where:"<clipboard>" "Top Hit" 2>/dev/null || await_field where:"<clipboard>" Clipboard \
    || { echo "no clipboard row for '$word': $(field headings)" >&2; return 1; }
  local rows
  rows=$(sill debug launcher-rows | grep -o '<clipboard>' | wc -l)
  echo "real typing '$word': $rows clipboard row, under $(field where:"<clipboard>")" >&2
  [ "$rows" = 1 ] || return 1
  [ "$(field row:"$(field selected)")" = "<clipboard>" ] || { echo "the clipboard row is not selected" >&2; return 1; }
  key Return || return 1
  local pasted=
  for _ in $(seq 30); do
    pasted=$(kwin_wl timeout 2 wl-paste --no-newline 2>/dev/null)
    [ "$pasted" = "$made_up" ] && break
    sleep 0.1
  done
  echo "real Enter: wl-paste on KWin's socket reads the restored entry:" \
    "$([ "$pasted" = "$made_up" ] && echo yes || echo no)" >&2
  [ "$pasted" = "$made_up" ]
}

# Q340. Recorded: whether real Down keeps the selected row on the card.
check_q340() {
  open_empty || return 1
  typed "m9-list" || return 1
  await_field rows:"Top Hit" m9-list-01.txt || return 1
  key Tab || return 1
  await_field headings Documents || return 1
  local rows first last deep back
  rows=$(field count)
  capture q340-top
  first=$(selection_pixels q340-top)
  key Down Down Down Down Down Down Down Down Down Down Down || return 1
  await_field selected 11 || return 1
  capture q340-down
  last=$(selection_pixels q340-down)
  key Up Up Up Up Up Up Up Up Up Up Up || return 1
  await_field selected 0 || return 1
  capture q340-up
  back=$(selection_pixels q340-up)
  deep=OPEN
  [ "$last" -gt $(( first / 2 )) ] && deep=CLOSED
  echo "Q340 $deep: Documents filtered by real Tab ($rows rows); the selected row's tint:" \
    "$first px on row 0, $last px after real Down x11 (row 11), $back px after real Up x11" \
    "(row 0); captures q340-{top,down,up}.png" >&2
  close_launcher
}

# Q341. Recorded: where the caret starts when the launcher opens with a query.
check_q341() {
  close_launcher
  sill launcher open --query m9 >/dev/null 2>&1
  await_field query m9 || return 1
  sleep 0.5
  capture q341-open
  typed "x" || return 1
  sleep 0.3
  local query caret=OPEN
  query=$(field query)
  [ "$query" = m9x ] && caret=CLOSED
  echo "Q341 $caret: after 'sill launcher open --query m9' a real 'x' makes the query" \
    "'$query' (m9x when the caret starts at the end)" >&2
  key BackSpace || return 1
  close_launcher
  sill launcher open --query m9 >/dev/null 2>&1
  await_field query m9 || return 1
  sleep 0.5
  key Right || return 1
  local first
  first=$(field preview)
  key End Right || return 1
  echo "real → after opening: preview $first; real End then →: preview $(field preview)," \
    "query '$(field query)'" >&2
  close_launcher
}

cleanup() {
  stop_input
  stop_fm
  close_launcher 2>/dev/null
  [ -n "$WL" ] && kwin_wl wl-copy --clear
  stop_daemon
  stop_kwin
  rm -rf "$SCRATCH" "$RUNTIME"
}

command -v wl-copy >/dev/null && command -v wl-paste >/dev/null \
  || { echo "wl-copy and wl-paste are missing (wl-clipboard); not installed by this script" >&2; exit 1; }
build || exit 1
seed_home
start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; rm -rf "$SCRATCH" "$RUNTIME"; exit 1; }
trap cleanup EXIT
: >"$OUT_DIR/wl.log"
: >"$OUT_DIR/capture.log"
echo "headless KWin at $WL; data-control: $(env WAYLAND_DISPLAY="$WL" wayland-info 2>/dev/null \
  | grep -o "'[a-z_]*data_control_manager_v1'" | tr '\n' ' ')" >&2
start_daemon || exit 1
start_input || { echo "no fake input client" >&2; exit 1; }
# The seat's devices are announced with the input client's: the first events only make them known.
input "move 300 900" "sleep 80" "move 310 900" "sleep 80" || exit 1
status=0
for step in step_type_and_preview step_pane_keys step_reveal step_emoji step_clipboard; do
  close_launcher
  "$step"
  rc=$?
  case "$rc" in
    0) echo "PASS $step" ;;
    2) echo "NOT PROVEN $step"; [ "$status" = 1 ] || status=2 ;;
    *) echo "FAIL $step"; status=1 ;;
  esac
done
check_q340 || echo "Q340 could not be checked" >&2
check_q341 || echo "Q341 could not be checked" >&2
echo "stopping the input client, the daemon and the headless KWin" >&2
cleanup
trap - EXIT
echo "logs and captures in $OUT_DIR"
exit "$status"

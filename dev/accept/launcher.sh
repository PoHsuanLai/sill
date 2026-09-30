#!/usr/bin/env bash
# Launcher acceptance, on a private headless KWin (shell-host scripts/dev/kwin-virtual.sh) that
# the script starts and stops. The default mode runs four groups of steps with every key sent by
# `sill debug key SURFACE KEY` (shell-host's `Inject::Key`: the key enters the panel where a real
# xkb key does, whether or not KWin gave the panel the keyboard); --real-input types real Wayland
# key events instead.
#
# Every group starts `sill run` from this checkout's debug build (`--features debug`) in its own
# scratch tree (scratch_new: HOME and every XDG directory, its own XDG_RUNTIME_DIR) on a private
# bus that is both its session and its system bus, with the fake DDC fixture, on the
# compositor's socket (an absolute WAYLAND_DISPLAY). Nothing reaches the person's files, index,
# settings, session bus or apps, and the daemon is quit at the end of its group. Each group
# keeps its logs in target/accept-launcher/<group>/.
#
# Default mode, groups in this order (`--only GROUP`, repeatable, runs just those):
#
#   toggle: open latency, panel and idle frames (p95 < 100 ms over 20 toggles)
#     1. 20 `sill launcher toggle` round trips, each timed on the daemon's own side
#        (`crates/sill/src/daemon.rs`'s `launcher.open_latency_ms` line). Asserts p95 < 100 ms;
#        prints p50/p95/max.
#     2. The panel exists after a toggle (`sill debug capture <launcher panel>`), and after the
#        closing toggle, 5 s idle adds 0 frames to every launcher surface.
#     3. `sill quit` ends the daemon, which then no longer answers `ping`.
#
#   ui: the panel and the catcher
#     1. Warm: both launcher surfaces exist from the start, hidden (no capture of either).
#     2. `sill launcher toggle` shows the panel: over 10 warm openings, a capture of
#        `sill-launcher-<output>` within 100 ms of the toggle has the panel's pixels (the first
#        opening after start is reported separately); the daemon's
#        `launcher.open_to_frame_ms` lines are printed.
#     3. `sill launcher toggle` again hides it (no capture), and a click on the catcher closes a
#        reopened one.
#     4. 5 s idle after the hide: 0 frames on the panel and the catcher.
#     5. Stress: 20 toggles 150 ms apart (each close lands mid entrance), then 5 s idle; the
#        daemon still answers `sill ping` and is still running.
#     6. On `examples/launcher_demo.rs` (fake results, driven through its stdin): `open`, then
#        the keys `f` and `i` (shell-host's `key` verb) show rows; Ctrl+K opens the selected
#        row's actions menu; Escape twice closes the menu, then the launcher. Captures of the
#        panel: target/accept-launcher/ui/demo-{open,actions,closed}.png.
#
#   palette: the palette, its preview pane, sections and emoji grid
#     A scratch HOME holds a PDF, a PNG, a Markdown file and six more `m9-*` documents,
#     `launcher.files_backend = "fd"`. Every key goes through `sill debug key`, every look
#     through `sill debug launcher-rows` and `sill debug capture` of the panel.
#     1. Open latency: 10 warm `sill launcher open` / `close`, the panel's
#        `launcher.open_to_frame_ms` p50/p95/max; only the count of openings is asserted, since
#        the host's load moves the numbers.
#     2. `--query m9` lists Top Hit, then Documents.
#     3. `down`, `space` (Top Hit is the PDF; `up` first so the selection is on it): the preview
#        is `pdf`, captured (dev/out/launcher-v2-preview-pdf.png).
#     4. `down` to the PNG: `image`; `space` hides it.
#     5. `quick-look` while typing shows it; `right` (→ with the caret at the
#        field's end, where `--query` leaves it) gives the pane's first control the keyboard; `escape` gives it back (the pane stays),
#        `escape` hides the pane, `escape` closes the launcher. On a row with actions: an `m9`
#        document, else, while the files provider answers nothing, the Ghostty app row.
#     6. Documents over its cap reports `hidden > 0`; Enter on the "Show More" line reveals them.
#     7. `tab` to Emoji with nothing typed (Shift+Tab twice: Web, then Emoji) shows quire's emoji
#        grid (dev/out/launcher-v2-emoji.png).
#     8. 5 s idle after closing: 0 frames on the launcher's surfaces.
#     C. Home then →, End then →: the key claim reads the caret under sill.
#     F. The same looks over the fakes' fixtures (`tests/launcher_support/m9.rs`: a PDF, a PNG, a
#        Markdown file, five more PDFs, a clipboard entry, emoji), on `examples/launcher_demo.rs`
#        (its keys are shell-host's `key` verb) after the daemon has quit, while the files and emoji providers answer nothing:
#        dev/out/launcher-v2-{preview-pdf,preview-image,preview-text,show-more,pane-focus,
#        emoji}-fixtures.png.
#     A step that needs rows from the files or emoji provider exits 2 (SKIP) while that provider
#     answers nothing on this branch; `crates/sill-launcher-ui/tests/launcher_preview.rs` proves the
#     same steps over fixtures.
#
#   files: the launcher's Documents
#     Everything runs in the scratch tree: Documents/m9-report.pdf and notes/m9-notes.md,
#     XDG_CURRENT_DESKTOP=X-Generic so xdg-open asks xdg-mime, which reads the scratch
#     mimeapps.list: every type opens with a stub .desktop whose Exec records its argument and
#     the activation token. The private bus (dev/private-bus.conf plus a scratch service folder)
#     has nothing activatable. examples/file_manager_stub.rs holds org.freedesktop.FileManager1
#     on it and records ShowItems. examples/files_probe.rs drives FilesProvider as the runner
#     does and carries out the first row's activation through the real launch service.
#     1. `files_backend = "fd"`: the provider lists m9-report.pdf under ~/Documents for
#        "m9-rep" and for "M9-REP" (names match without case); the daemon, after `sill launcher
#        open --query m9-rep`, lists it once in `sill debug launcher-rows`, under Top Hit (a
#        single match is the Top Hit, never repeated below it) or Documents.
#     2. Enter opens it: the stub opener records the file's path and the token, bare and in a
#        transient scope.
#     3. ⌘R reveals it: the running stub file manager records ShowItems with its file:// URI and
#        the token; with no file manager on the bus, xdg-open opens the folder instead; with the
#        stub only activatable (a service file on the private bus), the call starts it.
#     4. `files_backend = "off"`: the provider lists nothing; the daemon, after `sill reload`,
#        lists no file row in any section, Top Hit included, and no Documents section.
#     5. Tracker end to end in localsearch's own sandbox (a private bus and a scratch index).
#
# --real-input: shell-host's scripts/dev/fake-input.sh drives the headless KWin: every key is a
# real Wayland key event with a real serial and keyboard focus, reaching the palette's field and
# quire's key claim as a person's would. `sill debug launcher-rows` and `sill debug capture` only
# look; no key goes through `sill debug key`. The bus is KWin's private bus, given to
# the daemon (and the stub file manager, as an argument) only, never to this shell. ONE
# fake-input client serves the whole run. `wl-copy`/`wl-paste` get KWin's socket by absolute
# path and only made-up strings, and no clipboard content is printed. `launcher.files_backend =
# "fd"`.
#     1. Real typing of a file's name lists it (Top Hit); real Down then Space opens the preview
#        (and types no space), real Space closes it, real Ctrl+Y (⌘Y) opens it while typing.
#     2. Real → twice enters the pane (the pane shows after the first); real Tab walks its
#        controls (the focus ring moves inside the pane on each Tab and comes back round, the
#        provider filter is untouched); real Escape three times closes one level each: the ring
#        goes (the pane stays), the pane hides, the launcher closes.
#     3. Real Ctrl+R (⌘R) on a file row reveals it: the stub file manager records ShowItems with
#        its file:// URI and a token.
#     4. Real Shift+Tab twice with nothing typed filters to Emoji (the grid); real Down, Down,
#        Right, Left, Up each move the selection by a row or a cell; real Enter copies the
#        selected glyph (`wl-paste` on KWin's socket reads it).
#     5. The clipboard: KWin offers ext_data_control_manager_v1, so `sill clipboard status` says
#        `memory ext`; a made-up copy made on KWin's socket, then another, shows as one
#        clipboard row for its word; real Enter restores it (`wl-paste` reads it back). Were
#        neither protocol offered, status would say `unsupported` and the step would be
#        skipped (exit 2).
#     Scroll. Real Down past the card's visible rows (Documents filtered, 24 rows): is the
#        selected row scrolled into view? Real Up back to the first row. Recorded, not asserted
#        (quire's open gap): OPEN while the selected row is off the card.
#     Caret. `sill launcher open --query m9`, then a real "x": where the caret starts. Recorded,
#        not asserted: OPEN while the query becomes "xm9" (the caret at the start). Then real
#        End and →: the pane shows (the claim reads the caret at the end).
#     Captures: target/accept-launcher/real-input/*.png.
#
# Exit 0 only when every asserted step passed; 2 when none failed but a step could not run
# safely or was not proven (named); 1 on a failure. The scroll and caret lines never change the
# exit status.
#
# Usage: dev/accept/launcher.sh [--only toggle|ui|palette|files]...
#        dev/accept/launcher.sh --real-input
#        dev/accept/launcher.sh --self-test   (files step 4's check over a table, no compositor)
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"

usage() {
  echo "usage: dev/accept/launcher.sh [--only toggle|ui|palette|files]... | --real-input | --self-test" >&2
  exit 2
}
MODE=keys
ONLY=()
while [ $# -gt 0 ]; do
  case "$1" in
    --real-input) MODE=real ;;
    --self-test) MODE=self ;;
    --only)
      shift
      case "${1:-}" in toggle | ui | palette | files) ONLY+=("$1") ;; *) usage ;; esac ;;
    *) usage ;;
  esac
  shift
done
[ "$MODE" != real ] || [ "${#ONLY[@]}" = 0 ] || usage

OUT_ROOT="$SILL_REPO/target/accept-launcher"
SHOTS="$SILL_REPO/dev/out"
OUT_DIR="$OUT_ROOT"

# ---------------------------------------------------------------- shared helpers

# The worse of two verdicts: 1 (a failure) over 2 (skipped) over 0.
worst() {
  if [ "$1" = 1 ] || [ "$2" = 1 ]; then echo 1
  elif [ "$1" = 2 ] || [ "$2" = 2 ]; then echo 2
  else echo 0; fi
}

# Starts a group: its own log directory OUT_DIR and its scratch tree.
group_begin() {
  OUT_DIR="$OUT_ROOT/$1"
  mkdir -p "$OUT_DIR" "$SHOTS"
  scratch_new
}

# Ends a group: the daemon, the private bus and the scratch tree.
group_end() {
  local result=0
  stop_daemon || result=1
  stop_private_bus
  scratch_drop
  return "$result"
}

# The launcher's own surfaces, "name frames" lines, and the sum of their frames.
launcher_counters() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"].startswith("sill-launcher-"):
        print(row["surface"], row["frames"])'
}
launcher_frames() { launcher_counters | awk '{sum += $2} END {print sum + 0}'; }

# Every `launcher.open_latency_ms=<ms>` value the daemon's log holds, one per line.
open_latencies() {
  # The panel's request-to-first-frame line is the real open latency; the
  # daemon's `open_latency_ms` times the IPC dispatch alone and is the fallback.
  if grep -q 'launcher\.open_to_frame_ms=' "$OUT_DIR/daemon.log"; then
    grep -o 'launcher\.open_to_frame_ms=[0-9.]*' "$OUT_DIR/daemon.log" | cut -d= -f2
  else
    grep -o 'launcher\.open_latency_ms=[0-9.]*' "$OUT_DIR/daemon.log" | cut -d= -f2
  fi
}

# One field of `sill debug launcher-rows` (clipboard titles are "<clipboard>" there): preview,
# selected, query, headings, count, hidden:<title>, rows:<title>, row:<index> (the line at that
# index over every section), index:<title>:<row> (the row's index over every section),
# where:<row> (the section titles holding that row). Empty and exit 1 while the launcher is
# closed.
field() {
  sill debug launcher-rows 2>>"$OUT_DIR/rows.log" | python3 -c '
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
    print("hidden" if p is None else p)
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
elif want.startswith("index:"):
    _, title, row = want.split(":", 2)
    at = 0
    for s in secs:
        for r in s["rows"]:
            if s["title"] == title and r == row:
                print(at); sys.exit(0)
            at += 1
elif want.startswith("where:"):
    print(",".join(s["title"] for s in secs if want[6:] in s["rows"]))
' "$1"
}

is_open() { field query >/dev/null 2>&1; }

close_launcher() { sill launcher close >>"$OUT_DIR/keys.log" 2>&1; sleep 0.3; }

# Capture of the launcher panel into $1.
capture_launcher() { capture "sill-launcher-$NAME" "$1"; }

# Writes the documents the files provider's `fd` finds under home $1: a PDF (with $2 rings on
# its page), a PNG, a Markdown note, and, by $3, six more PDFs (`pdfs`) or 24 `m9-list-NN.txt`
# for a list longer than the card (`list`).
seed_documents() {
  mkdir -p "$1/Documents" "$1/notes"
  python3 - "$1" "$2" "$3" <<'EOF'
import sys, zlib, struct
home, ring_count, tail = sys.argv[1], int(sys.argv[2]), sys.argv[3]
def pdf():
    rings = "".join(f"{20+(n*7)%160} {20+(n*13)%220} m {29+(n*7)%160} {20+(n*13)%220} {20+(n*7)%160} {29+(n*13)%220} {20+(n*7)%160} {20+(n*13)%220} c S\n" for n in range(ring_count))
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
if tail == "pdfs":
    for n in range(1, 7):
        open(f"{home}/Documents/m9-extra-{n}.pdf", "wb").write(pdf())
else:
    for n in range(1, 25):
        open(f"{home}/Documents/m9-list-{n:02}.txt", "w").write(f"list note {n}\n")
EOF
}

# The stub file manager (examples/file_manager_stub.rs) on the private bus.
STUB_FM="$SILL_REPO/target/debug/examples/file_manager_stub"
FM=
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

# `sill debug key`'s spelling (a W3C key name, `+ctrl`, `+shift`) of a key by its usual word;
# any other word is already a key name (`f`, `Home`).
key_spec() {
  case "$1" in
    up) echo ArrowUp ;;
    down) echo ArrowDown ;;
    left) echo ArrowLeft ;;
    right) echo ArrowRight ;;
    enter) echo Enter ;;
    escape) echo Escape ;;
    space) echo Space ;;
    tab) echo Tab ;;
    shift-tab) echo Tab+shift ;;
    actions) echo k+ctrl ;;
    quick-look) echo y+ctrl ;;
    reveal) echo r+ctrl ;;
    *) echo "$1" ;;
  esac
}

# A key (`key_spec`'s words) on the demo's panel (its stdin is fd 7), through shell-host's `key` verb.
demo_key() { echo "sill-launcher-$NAME key $(key_spec "$1")" >&7; sleep 0.3; }

# ---------------------------------------------------------------- group toggle

TOGGLES=20
P95_BUDGET_MS=100

# toggle 1.
step_toggle_latency() {
  local log="$OUT_DIR/toggle-latency.log" before after n p50 p95 max
  : >"$log"
  before=$(open_latencies | wc -l)
  for _ in $(seq "$TOGGLES"); do
    # The daemon times and logs the round trip before it even looks at the request.
    sill launcher toggle >>"$log" 2>&1 || true
  done
  after=$(open_latencies | wc -l)
  n=$(( after - before ))
  echo "$TOGGLES toggles logged $n 'launcher.open_latency_ms' line(s)" | tee -a "$log" >&2
  if [ "$n" -ne "$TOGGLES" ]; then
    echo "expected exactly $TOGGLES (one per toggle); the logging plumbing is broken" >&2
    return 1
  fi
  read -r p50 p95 max < <(open_latencies | tail -n "$TOGGLES" | percentiles)
  echo "p50 ${p50} ms, p95 ${p95} ms, max ${max} ms" | tee -a "$log" >&2
  python3 -c "import sys; sys.exit(0 if float('$p95') < $P95_BUDGET_MS else 1)"
}

# toggle 2.
step_panel_and_idle() {
  local log="$OUT_DIR/panel.log" surface before after
  : >"$log"
  sill launcher toggle >>"$log" 2>&1 || true
  surface=$(launcher_counters | awk 'NR==1 {print $1}')
  if [ -z "$surface" ]; then
    echo "no sill-launcher-* surface in 'sill debug stats'" | tee -a "$log" >&2
    return 1
  fi
  sill debug capture "$surface" "$OUT_DIR/panel.png" >>"$log" 2>&1 || {
    echo "the launcher surface is listed but would not capture" >&2
    return 1
  }
  sill launcher toggle >>"$log" 2>&1 || true # close it
  # Counted once the close has landed: a frame of the opening still in flight at the capture
  # (a loaded host, a debug build) is the opening's, not idle's.
  sleep 0.5
  before=$(launcher_frames)
  sleep 5
  after=$(launcher_frames)
  echo "closed frames before $before, after 5s idle $after" | tee -a "$log" >&2
  [ "$before" = "$after" ]
}

# toggle 3.
step_quit() {
  sill quit >"$OUT_DIR/quit.log" 2>&1 || return 1
  grep -q '"kind":"done"' "$OUT_DIR/quit.log" || return 1
  sleep 0.2
  if sill ping >"$OUT_DIR/quit-ping.log" 2>&1; then
    echo "'sill ping' still succeeded after quit (see $OUT_DIR/quit-ping.log)" >&2
    return 1
  fi
}

group_toggle() {
  group_begin toggle
  build || { group_end; return 1; }
  start_private_bus || { group_end; return 1; }
  launch_daemon || { group_end; return 1; }
  run_steps step_toggle_latency step_panel_and_idle step_quit
  local result=$STATUS
  group_end || result=1
  return "$result"
}

# ---------------------------------------------------------------- group ui

DEMO="$SILL_REPO/target/debug/examples/launcher_demo"

# How many pixels differ between the PNGs $1 and $2.
differing_pixels() {
  python3 - "$1" "$2" <<'EOF'
import sys
from PIL import Image, ImageChops
a = Image.open(sys.argv[1]).convert("RGBA")
b = Image.open(sys.argv[2]).convert("RGBA")
print(sum(1 for p in ImageChops.difference(a, b).convert("L").getdata() if p > 16))
EOF
}

# ui 1.
step_ui_warm() {
  local log="$OUT_DIR/warm.log"
  sill debug stats >"$log" 2>&1
  grep -q "\"$PANEL\"" "$log" && grep -q "\"$CATCHER\"" "$log" || {
    echo "the launcher surfaces are not in 'sill debug stats'" >&2; return 1; }
  if sill debug capture "$PANEL" "$OUT_DIR/hidden.png" >>"$log" 2>&1; then
    echo "the panel is mapped before any toggle" >&2; return 1
  fi
  echo "both launcher surfaces spawned at start, hidden" | tee -a "$log" >&2
}

# ui 2.
# One toggle, timed to the first capture of the panel with its pixels (the capture's own time
# counted, the pixel count after it not); prints "ms".
toggle_to_capture() {
  local log="$1" start attempts=0 px
  start=$(date +%s%N)
  sill launcher toggle >>"$log" 2>&1 || return 1
  for _ in $(seq 200); do
    attempts=$(( attempts + 1 ))
    if sill debug capture "$PANEL" "$OUT_DIR/open.png" >>"$log" 2>&1; then
      local took=$(( ($(date +%s%N) - start) / 1000000 ))
      px=$(opaque_pixels "$OUT_DIR/open.png")
      [ "$px" -gt 1000 ] && { echo "$took"; return 0; }
    fi
  done
  return 1
}

step_ui_toggle_shows() {
  local log="$OUT_DIR/toggle.log" first warm=() ms
  : >"$log"
  first=$(toggle_to_capture "$log") || return 1
  sill launcher toggle >>"$log" 2>&1; sleep 0.4
  for _ in $(seq 10); do
    ms=$(toggle_to_capture "$log") || return 1
    warm+=("$ms")
    sill launcher toggle >>"$log" 2>&1; sleep 0.4
  done
  local frames
  frames=$(grep -o 'launcher.open_to_frame_ms=[0-9.]*' "$OUT_DIR/daemon.log" | cut -d= -f2 | tr '\n' ' ')
  python3 - "$first" "${warm[@]}" <<EOF | tee -a "$log" >&2
import sys
first, warm = int(sys.argv[1]), sorted(int(v) for v in sys.argv[2:])
print(f"toggle -> panel capture: first opening {first} ms; 10 warm openings "
      f"min {warm[0]} p50 {warm[len(warm)//2]} max {warm[-1]} ms")
print("daemon launcher.open_to_frame_ms (request to first frame): $frames")
EOF
  [ "$(printf '%s\n' "${warm[@]}" | sort -n | tail -1)" -le 100 ]
}

# ui 3.
step_ui_hide() {
  local log="$OUT_DIR/hide.log" hidden=no closed=no
  sill launcher toggle >"$log" 2>&1 || return 1
  sleep 0.5
  sill launcher toggle >>"$log" 2>&1 || return 1
  for _ in $(seq 40); do
    sill debug capture "$PANEL" "$OUT_DIR/after-hide.png" >>"$log" 2>&1 || { hidden=yes; break; }
    sleep 0.025
  done
  sill launcher toggle >>"$log" 2>&1
  sleep 0.5
  sill debug click "$CATCHER" 20 "$(( OUT_H - 20 ))" >>"$log" 2>&1
  for _ in $(seq 40); do
    sill debug capture "$PANEL" "$OUT_DIR/after-catch.png" >>"$log" 2>&1 || { closed=yes; break; }
    sleep 0.025
  done
  echo "toggle hides: $hidden; a catcher click closes: $closed" | tee -a "$log" >&2
  [ "$hidden" = yes ] && [ "$closed" = yes ]
}

# ui 4.
step_ui_idle() {
  local log="$OUT_DIR/idle.log" p0 c0 p1 c1
  sleep 1
  p0=$(frames_of "$PANEL"); c0=$(frames_of "$CATCHER")
  sleep 5
  p1=$(frames_of "$PANEL"); c1=$(frames_of "$CATCHER")
  echo "5 s idle after the hide: panel $(( p1 - p0 )) frames, catcher $(( c1 - c0 )) frames" \
    | tee "$log" >&2
  [ "$(( p1 - p0 ))" = 0 ] && [ "$(( c1 - c0 ))" = 0 ]
}

# ui 5.
step_ui_stress() {
  local log="$OUT_DIR/stress.log" start took
  : >"$log"
  start=$(date +%s%N)
  for _ in $(seq 20); do
    sill launcher toggle >>"$log" 2>&1 || { echo "a toggle failed" | tee -a "$log" >&2; return 1; }
    sleep 0.15
  done
  took=$(( ($(date +%s%N) - start) / 1000000 ))
  sleep 5
  local alive=no answers=no
  kill -0 "$DAEMON" 2>/dev/null && alive=yes
  sill ping >>"$log" 2>&1 && answers=yes
  local panics
  panics=$(grep -c 'panicked' "$OUT_DIR/daemon.log")
  echo "20 toggles 150 ms apart in ${took} ms, then 5 s idle: daemon running $alive, answers ping $answers, panics in its log $panics" \
    | tee -a "$log" >&2
  [ "$alive" = yes ] && [ "$answers" = yes ] && [ "$panics" = 0 ]
}

# ui 6.
step_ui_demo() {
  local log="$OUT_DIR/demo.log" fifo="$OUT_DIR/demo.in" ok=yes
  rm -f "$fifo"; mkfifo "$fifo"
  in_env "$DEMO" <"$fifo" >"$log" 2>&1 &
  local demo=$!
  exec 7>"$fifo"
  sleep 2
  # Generous waits: under load a debug demo's first frame has taken over a second, and a
  # `type` that lands before the opening's own reset is lost.
  echo "open" >&7; sleep 2.5
  demo_key f; demo_key i; sleep 2.0
  echo "$PANEL capture $OUT_DIR/demo-open.png" >&7; sleep 0.5
  demo_key actions; sleep 1.5
  echo "$PANEL capture $OUT_DIR/demo-actions.png" >&7; sleep 0.5
  demo_key escape; sleep 0.5
  demo_key escape; sleep 0.8
  echo "$PANEL capture $OUT_DIR/demo-closed.png" >&7; sleep 0.5
  echo "quit" >&7
  exec 7>&-
  wait "$demo" 2>/dev/null
  rm -f "$fifo"
  local open_px changed closed=no
  open_px=$(opaque_pixels "$OUT_DIR/demo-open.png" 2>/dev/null || echo 0)
  changed=$(differing_pixels "$OUT_DIR/demo-open.png" "$OUT_DIR/demo-actions.png" 2>/dev/null || echo 0)
  grep -q '"capture":"[^"]*demo-closed.png","err":"[^n]' "$log" && closed=yes
  echo "demo: open ${open_px} px; the actions menu changed ${changed} px; closed after two Escapes: $closed" \
    | tee -a "$log" >&2
  [ "$ok" = yes ] && [ "$open_px" -gt 1000 ] && [ "$changed" -gt 1000 ] && [ "$closed" = yes ]
}

group_ui() {
  group_begin ui
  build "-p sill-launcher-ui --example launcher_demo" || { group_end; return 1; }
  OUT_H="${SIZE#*x}"
  start_private_bus || { group_end; return 1; }
  backdrop_seed "$CONFIG" || { group_end; return 1; }
  launch_daemon || { group_end; return 1; }
  NAME=$(output_name) || { group_end; return 1; }
  PANEL="sill-launcher-$NAME"; CATCHER="sill-launcher-catcher-$NAME"
  sleep 1
  run_steps step_ui_warm step_ui_toggle_shows step_ui_hide step_ui_idle step_ui_stress
  local result=$STATUS
  stop_daemon || result=1
  run_steps step_ui_demo
  result=$(worst "$result" "$STATUS")
  group_end || result=1
  return "$result"
}

# ---------------------------------------------------------------- group palette

OPENINGS=10

# One key (`key_spec`'s words) on the launcher's panel, through `sill debug key`.
debug_key() { sill debug key "sill-launcher-$NAME" "$(key_spec "$1")" >>"$OUT_DIR/keys.log" 2>&1; sleep 0.25; }

open_query() {
  sill launcher open --query "$1" >>"$OUT_DIR/keys.log" 2>&1
  for _ in $(seq 40); do
    [ -n "$(field headings 2>/dev/null)" ] && break
    sleep 0.1
  done
  sleep 0.4
}

# Whether the files provider answers.
documents_answer() { [[ ",$(field headings)," == *",Documents,"* ]]; }

# palette 1.
step_palette_latency() {
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

# palette 2.
step_palette_sections() {
  open_query m9
  echo "m9 headings: $(field headings)" >&2
  documents_answer || { echo "SKIP: no Documents (the files provider answers nothing here)" >&2; return 2; }
  [[ "$(field headings)" == "Top Hit,Documents"* ]]
}

# palette 3. and 4.
step_palette_preview_pdf_and_image() {
  documents_answer || { echo "SKIP: no Documents (the files provider answers nothing)" >&2; return 2; }
  debug_key up; debug_key down; debug_key up
  debug_key space
  echo "after up/down/up, space: preview $(field preview)" >&2
  [ "$(field preview)" = pdf ] || return 1
  sleep 1.5
  capture_launcher "$SHOTS/launcher-v2-preview-pdf.png" || return 1
  local png
  png=$(field index:Documents:m9-shot.png)
  if [ -z "$png" ] && [ -n "$(field "index:Documents:Show More")" ]; then
    # Behind Show More (the index ranks names its own way): open the section first.
    local more
    more=$(field "index:Documents:Show More")
    while [ "$(field selected)" -lt "$more" ]; do debug_key down; done
    debug_key enter
    png=$(field index:Documents:m9-shot.png)
  fi
  [ -n "$png" ] || { echo "no m9-shot.png row: $(field rows:Documents)" >&2; return 1; }
  while [ "$(field selected)" -gt "$png" ]; do debug_key up; done
  while [ "$(field selected)" -lt "$png" ]; do debug_key down; done
  echo "on the PNG: preview $(field preview)" >&2
  [ "$(field preview)" = image ] || return 1
  capture_launcher "$SHOTS/launcher-v2-preview-image.png"
  debug_key space
  [ "$(field preview)" = hidden ]
}

# palette 5.
step_palette_pane_levels() {
  local query=m9
  documents_answer || query=ghostty
  close_launcher
  open_query "$query"
  echo "pane levels on --query $query ($(field headings))" >&2
  debug_key quick-look
  [ "$(field preview)" != hidden ] || { echo "quick-look showed nothing" >&2; return 1; }
  debug_key right # into the pane
  capture_launcher "$SHOTS/launcher-v2-pane-focus.png"
  debug_key escape
  echo "escape 1: preview $(field preview), open $(is_open && echo yes || echo no)" >&2
  [ "$(field preview)" != hidden ] || { echo "the first Escape hid the pane: it never had the keyboard" >&2; return 1; }
  debug_key escape
  echo "escape 2: preview $(field preview)" >&2
  [ "$(field preview)" = hidden ] || return 1
  is_open || return 1
  debug_key escape
  ! is_open
}

# palette 6.
step_palette_show_more() {
  close_launcher
  open_query m9
  documents_answer || { echo "SKIP: no Documents (the files provider answers nothing)" >&2; return 2; }
  local hidden more
  hidden=$(field hidden:Documents)
  echo "Documents hides $hidden" >&2
  [ "${hidden:-0}" -gt 0 ] || return 1
  more=$(field "index:Documents:Show More")
  while [ "$(field selected)" -lt "$more" ]; do debug_key down; done
  debug_key enter
  echo "after Show More: hidden $(field hidden:Documents), rows $(field rows:Documents)" >&2
  [ "$(field hidden:Documents)" = 0 ]
}

# palette 7.
step_palette_emoji() {
  close_launcher
  open_query ""
  debug_key shift-tab
  debug_key shift-tab
  sleep 0.5
  echo "after two Shift+Tab: $(field headings) [$(field rows:Emoji)]" >&2
  [ -n "$(field rows:Emoji)" ] || { echo "SKIP: no Emoji rows (the emoji provider answers nothing here)" >&2; close_launcher; return 2; }
  capture_launcher "$SHOTS/launcher-v2-emoji.png"
  close_launcher
}

# palette C. The caret under sill: After `--query m9` the caret is at the end, so Home puts it at
#    the start, where → moves it and the pane stays hidden; End, then →, finds the caret at the
#    end and opens the pane.
step_palette_caret() {
  close_launcher
  open_query m9
  documents_answer || { echo "SKIP: no Documents (the files provider answers nothing)" >&2; return 2; }
  debug_key Home; debug_key right; sleep 0.4
  local first
  first=$(field preview)
  debug_key End; debug_key right; sleep 0.6
  echo "Home, →: preview $first; End then →: preview $(field preview), query '$(field query)'" >&2
  [ "$first" = hidden ] && [ "$(field preview)" = pdf ] && [ "$(field query)" = m9 ]
}

# palette 8.
step_palette_idle() {
  close_launcher
  local before after
  before=$(launcher_frames)
  sleep 5
  after=$(launcher_frames)
  echo "closed: launcher frames $before, after 5 s idle $after" >&2
  [ "$before" = "$after" ]
}

# A capture of the demo (its stdin is fd 7) into launcher-v2-$1-fixtures.png.
fixtures_shot() { echo "sill-launcher-$NAME capture $SHOTS/launcher-v2-$1-fixtures.png" >&7; sleep 0.8; }

# palette F.
step_palette_fixtures() {
  local log="$OUT_DIR/demo.log" fifo="$OUT_DIR/demo.in" at
  rm -f "$fifo"; mkfifo "$fifo"
  in_env "$DEMO" <"$fifo" >"$log" 2>&1 &
  local demo=$!
  exec 7>"$fifo"
  sleep 2
  echo "open m9" >&7; sleep 2.5
  demo_key quick-look; sleep 2.5
  fixtures_shot preview-pdf
  demo_key down; sleep 1.2
  fixtures_shot preview-image
  demo_key down; sleep 1.2
  fixtures_shot preview-text
  demo_key quick-look; sleep 0.5
  # From the Markdown file (line 2) to the Show More line (6).
  for at in 1 2 3 4; do demo_key down; sleep 0.3; done
  demo_key enter; sleep 1.2
  fixtures_shot show-more
  echo "open m9" >&7; sleep 2
  demo_key quick-look; sleep 1.5
  demo_key right; sleep 1
  fixtures_shot pane-focus
  echo "open" >&7; sleep 2
  demo_key shift-tab; sleep 0.6
  demo_key shift-tab; sleep 1.5
  fixtures_shot emoji
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

group_palette() {
  group_begin palette
  build "-p sill-launcher-ui --example launcher_demo" || { group_end; return 1; }
  : >"$OUT_DIR/keys.log"
  seed_documents "$HOME_DIR" 3000 pdfs
  mkdir -p "$CONFIG/sill"
  printf '[launcher]\nfiles_backend = "fd"\nweb_search = "on"\n' >"$CONFIG/sill/settings.toml"
  start_private_bus || { group_end; return 1; }
  launch_daemon || { group_end; return 1; }
  NAME=$(output_name) || { group_end; return 1; }
  sleep 1
  echo "daemon on a private bus, runtime dir $RUNTIME, home $HOME_DIR, output $NAME" >&2
  run_steps step_palette_latency step_palette_sections step_palette_preview_pdf_and_image \
    step_palette_pane_levels step_palette_show_more step_palette_emoji \
    step_palette_caret step_palette_idle
  local result=$STATUS
  stop_daemon || result=1
  run_steps step_palette_fixtures
  result=$(worst "$result" "$STATUS")
  group_end || result=1
  echo "captures in $SHOTS/launcher-v2-*.png" >&2
  return "$result"
}

# ---------------------------------------------------------------- group files

FILES_PROBE="$SILL_REPO/target/debug/examples/files_probe"
TOKEN=sill-accept-files-token
STEP_FAILED=0
STEP_PENDING=0

files_probe() { in_env "$FILES_PROBE" --home "$HOME_DIR" "$@"; }

pass() { echo "step $1: PASS - $2"; }
fail() { echo "step $1: FAIL - $2"; STEP_FAILED=1; }
pending() { echo "step $1: PENDING - $2"; STEP_PENDING=1; }

# Runs the checks "$@": 1 when one failed, 2 when none failed but one is pending, else 0.
judge() {
  STEP_FAILED=0
  STEP_PENDING=0
  "$@"
  [ "$STEP_FAILED" = 0 ] || return 1
  [ "$STEP_PENDING" = 0 ] || return 2
}

seed_files() {
  mkdir -p "$HOME_DIR/Documents" "$HOME_DIR/notes" "$CONFIG/sill" "$DATA/applications"
  : >"$OPENED"
  printf '%%PDF-1.4\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n%%%%EOF\n' \
    >"$HOME_DIR/Documents/m9-report.pdf"
  printf '# m9 notes\n' >"$HOME_DIR/notes/m9-notes.md"
  cat >"$SCRATCH/record-open" <<EOF
#!/bin/sh
printf '%s\ttoken=%s\n' "\$1" "\${XDG_ACTIVATION_TOKEN:-}" >>"$OPENED"
grep -o 'app-sill-open[^/]*\.scope' /proc/\$\$/cgroup >>"$SCRATCH/scopes.log"
EOF
  chmod +x "$SCRATCH/record-open"
  cat >"$DATA/applications/sill-stub-opener.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Stub Opener
Exec=$SCRATCH/record-open %f
MimeType=application/pdf;text/markdown;text/plain;inode/directory;
NoDisplay=true
EOF
  {
    echo "[Default Applications]"
    for type in application/pdf text/markdown text/plain inode/directory; do
      echo "$type=sill-stub-opener.desktop"
    done
  } >"$CONFIG/mimeapps.list"
  files_settings fd
}

files_settings() {
  printf '[launcher]\nfiles_backend = "%s"\n' "$1" >"$CONFIG/sill/settings.toml"
}

# A private bus (dev/private-bus.conf plus one scratch service folder, empty until step 3 puts the
# stub file manager there to check a reveal through an activatable (not running) file manager):
# sets BUS and BUS_PID like start_private_bus.
start_bus_with_services() {
  local out
  mkdir -p "$SCRATCH/services"
  sed "s|</busconfig>|  <servicedir>$SCRATCH/services</servicedir>\n</busconfig>|" \
    "$SILL_REPO/dev/private-bus.conf" >"$SCRATCH/bus.conf"
  out=$(dbus-daemon --config-file="$SCRATCH/bus.conf" --fork \
    --print-address=1 --print-pid=1) || return 1
  BUS=$(sed -n 1p <<<"$out")
  BUS_PID=$(sed -n 2p <<<"$out")
  case "$BUS" in unix:*) ;; *) echo "no private bus address: $out" >&2; return 1 ;; esac
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "not a private bus" >&2; return 1; }
  require_private_system_bus "$BUS" || return 1
}

# `sill debug launcher-rows` after opening the launcher on $1.
daemon_rows() {
  sill launcher open --query "$1" >/dev/null 2>&1
  sleep 1
  local rows
  rows=$(sill debug launcher-rows 2>&1)
  sill launcher close >/dev/null 2>&1
  echo "$rows"
}

# The count of lines of $2 matching $1.
count_in() { grep -c -- "$1" "$2" 2>/dev/null || true; }

# The title of the one section of `sill debug launcher-rows` (on stdin) listing row $1, else
# nothing. A single match is the Top Hit, which is never repeated below it, so a
# file's row is under "Top Hit" or "Documents".
section_of() {
  python3 -c '
import json, sys
try:
    secs = json.loads(sys.stdin.read())["v"]["sections"]
except (ValueError, KeyError, TypeError):
    sys.exit(0)
held = [s["title"] for s in secs if sys.argv[1] in s["rows"]]
if len(held) == 1:
    print(held[0])' "$1"
}

files_listing() {
  files_probe --backend fd --query m9-rep >"$OUT_DIR/step1.txt" 2>&1
  if grep -qP '^row\tm9-report\.pdf\t~/Documents\tapplication/pdf$' "$OUT_DIR/step1.txt"; then
    pass 1 "the provider lists m9-report.pdf under ~/Documents"
  else
    fail 1 "the provider's rows: $(tr '\n' '|' <"$OUT_DIR/step1.txt")"
  fi
  files_probe --backend fd --query M9-REP >>"$OUT_DIR/step1.txt" 2>&1
  if [ "$(count_in $'^row\tm9-report\\.pdf\t' "$OUT_DIR/step1.txt")" = 2 ]; then
    pass 1 "names match without case: M9-REP lists m9-report.pdf too"
  else
    fail 1 "M9-REP: $(tr '\n' '|' <"$OUT_DIR/step1.txt")"
  fi
  local rows under
  rows=$(daemon_rows m9-rep)
  under=$(section_of m9-report.pdf <<<"$rows")
  case "$under" in
    "Top Hit" | Documents) pass 1 "the daemon lists m9-report.pdf under $under" ;;
    *) fail 1 "the daemon's rows: $rows" ;;
  esac
}

files_open() {
  files_probe --backend fd --query m9-rep --choose primary >"$OUT_DIR/step2.txt" 2>&1
  local want="$HOME_DIR/Documents/m9-report.pdf	token=$TOKEN"
  if grep -qxF "$want" "$OPENED"; then
    pass 2 "Enter opened the file with the stub opener, token passed (no user manager: bare)"
  else
    fail 2 "the stub opener recorded: $(tr '\n' '|' <"$OPENED") (probe: $(tr '\n' '|' <"$OUT_DIR/step2.txt"))"
  fi
  # The same open where the user's systemd manager is reachable: only systemd-run sees the real
  # runtime folder (for its socket); xdg-open and the stub still run in the scratch environment.
  : >"$OPENED"
  : >"$SCRATCH/scopes.log"
  local real_runtime="/run/user/$(id -u)"
  if [ ! -S "$real_runtime/systemd/private" ]; then
    pending 2 "no user systemd manager here: the scope path was not run"
    return
  fi
  in_env env XDG_RUNTIME_DIR="$real_runtime" "$FILES_PROBE" --home "$HOME_DIR" --backend fd \
    --query m9-rep --choose primary >>"$OUT_DIR/step2.txt" 2>&1
  if grep -qxF "$want" "$OPENED" && grep -q 'app-sill-open' "$SCRATCH/scopes.log"; then
    pass 2 "Enter opened it inside a transient scope ($(head -1 "$SCRATCH/scopes.log"))"
  else
    fail 2 "scoped open recorded: $(tr '\n' '|' <"$OPENED") scope: $(tr '\n' '|' <"$SCRATCH/scopes.log")"
  fi
}

files_reveal() {
  start_fm || { fail 3 "no stub file manager"; return; }
  files_probe --backend fd --query m9-rep --choose reveal >"$OUT_DIR/step3.txt" 2>&1
  local uri="file://$HOME_DIR/Documents/m9-report.pdf"
  if grep -qxF "ShowItems	$uri	$TOKEN" "$OUT_DIR/file-manager.log"; then
    pass 3 "⌘R revealed it through FileManager1.ShowItems with the token"
  else
    fail 3 "the stub file manager recorded: $(tr '\n' '|' <"$OUT_DIR/file-manager.log")"
  fi
  stop_fm
  : >"$OPENED"
  files_probe --backend fd --query m9-rep --choose reveal >>"$OUT_DIR/step3.txt" 2>&1
  if grep -qxF "$HOME_DIR/Documents	token=$TOKEN" "$OPENED"; then
    pass 3 "with no file manager on the bus, xdg-open opened the folder"
  else
    fail 3 "the fallback recorded: $(tr '\n' '|' <"$OPENED")"
  fi
  # An activatable file manager: the stub as a bus service, started by the ShowItems call.
  cat >"$SCRATCH/services/org.freedesktop.FileManager1.service" <<EOF
[D-BUS Service]
Name=org.freedesktop.FileManager1
Exec=/bin/sh -c 'exec $STUB_FM "\$DBUS_STARTER_ADDRESS" >>$OUT_DIR/file-manager-activated.log 2>&1'
EOF
  : >"$OUT_DIR/file-manager-activated.log"
  bus_call org.freedesktop.DBus.ReloadConfig >/dev/null
  : >"$OPENED"
  files_probe --backend fd --query m9-rep --choose reveal >>"$OUT_DIR/step3.txt" 2>&1
  if grep -qxF "ShowItems	$uri	$TOKEN" "$OUT_DIR/file-manager-activated.log" && [ ! -s "$OPENED" ]; then
    pass 3 "an activatable file manager was started and revealed it through ShowItems"
  else
    fail 3 "activated stub recorded: $(tr '\n' '|' <"$OUT_DIR/file-manager-activated.log") opener: $(tr '\n' '|' <"$OPENED")"
  fi
  stop_activated_fm
}

# A method of the private bus's own interface, with $2… as dbus-send arguments.
bus_call() {
  dbus-send --bus="$BUS" --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus "$@"
}

# Stop the stub the bus started (its pid from the bus), and make the name unactivatable again.
stop_activated_fm() {
  local pid
  pid=$(bus_call org.freedesktop.DBus.GetConnectionUnixProcessID \
    string:org.freedesktop.FileManager1 2>/dev/null | awk '/uint32/ {print $2}')
  [ -n "$pid" ] && kill "$pid" 2>/dev/null
  rm -f "$SCRATCH/services/org.freedesktop.FileManager1.service"
  bus_call org.freedesktop.DBus.ReloadConfig >/dev/null 2>&1
}

files_off() {
  files_probe --backend off --query m9-rep >"$OUT_DIR/step4.txt" 2>&1
  if [ "$(count_in $'^row\t' "$OUT_DIR/step4.txt")" = 0 ] && grep -qx 'rows 0' "$OUT_DIR/step4.txt"; then
    pass 4 "files_backend off: the provider lists nothing"
  else
    fail 4 "off still listed: $(tr '\n' '|' <"$OUT_DIR/step4.txt")"
  fi
  files_settings off
  sill reload >/dev/null 2>&1
  local rows held
  rows=$(daemon_rows m9-rep)
  held=$(file_rows <<<"$rows")
  case "$held" in
    none) pass 4 "after reload no file row anywhere (Top Hit included), no Documents section" ;;
    unread) fail 4 "the daemon's rows did not read as JSON: $rows" ;;
    *) fail 4 "files_backend off, yet the daemon lists: $held" ;;
  esac
}

# Where `sill debug launcher-rows` (on stdin) shows a file: "none" when no section is Documents
# and no row in any section (Top Hit included) is a seeded file's name, "unread" when the
# rows are not JSON, else each offending section and row.
file_rows() {
  python3 -c '
import json, sys
FILES = {"m9-report.pdf", "m9-notes.md"}
try:
    secs = json.loads(sys.stdin.read())["v"]["sections"]
except (ValueError, KeyError, TypeError):
    print("unread")
    sys.exit(0)
held = [s["title"] + ": " + r for s in secs for r in s["rows"] if r in FILES]
held += [s["title"] for s in secs if s["title"] == "Documents"]
print("; ".join(held) if held else "none")'
}

# `--self-test`: files step 4's check over a table of rows, no compositor needed. The first case
# is a file as the Top Hit, which a "no Documents title" check alone passes.
self_test() {
  local ok=0 rows want got
  while IFS='|' read -r rows want; do
    got=$(file_rows <<<"$rows")
    case "$got" in none) got=pass ;; *) got=fail ;; esac
    [ "$got" = "$want" ] || { echo "self-test: $rows: $got, want $want"; ok=1; }
  done <<'EOF'
{"v":{"sections":[{"title":"Top Hit","rows":["m9-report.pdf"]},{"title":"Web","rows":["w"]}]}}|fail
{"v":{"sections":[{"title":"Recent","rows":["m9-notes.md"]}]}}|fail
{"v":{"sections":[{"title":"Documents","rows":["x"]}]}}|fail
{"v":{"sections":[{"title":"Top Hit","rows":["Search the web for m9-rep"]}]}}|pass
not json|fail
EOF
  [ "$ok" = 0 ] && echo "self-test: PASS"
  return "$ok"
}

files_tracker() {
  if ! command -v localsearch >/dev/null; then
    pending 5 "no localsearch here: Tracker's sandbox test was not run"
    return
  fi
  (cd "$SILL_REPO" && cargo test -p sill-search --test files_tracker_live -- --ignored \
    --nocapture) >"$OUT_DIR/step5-tracker.txt" 2>&1
  if grep -q 'test result: ok. 1 passed' "$OUT_DIR/step5-tracker.txt"; then
    pass 5 "Tracker found m9-report.pdf and 'm9 dir' in localsearch's sandbox"
  else
    fail 5 "Tracker's sandbox test (see $OUT_DIR/step5-tracker.txt)"
  fi
}

step_files_listing() { judge files_listing; }
step_files_open() { judge files_open; }
step_files_reveal() { judge files_reveal; }
step_files_off() { judge files_off; }
step_files_tracker() { judge files_tracker; }

group_files_end() {
  local result=0
  stop_fm
  [ -z "$BUS" ] || stop_activated_fm
  group_end || result=1
  IN_ENV_EXTRA=()
  IN_ENV_UNSET=()
  return "$result"
}

group_files() {
  group_begin files
  # xdg-open must ask xdg-mime, which reads the scratch mimeapps.list.
  IN_ENV_UNSET=(DISPLAY KDE_FULL_SESSION KDE_SESSION_VERSION DESKTOP_SESSION XDG_SESSION_DESKTOP
    GNOME_DESKTOP_SESSION_ID BROWSER)
  IN_ENV_EXTRA=(XDG_CURRENT_DESKTOP=X-Generic)
  OPENED="$SCRATCH/opened.log"
  build "-p sill-testkit --example files_probe --example file_manager_stub" \
    || { group_files_end; return 1; }
  start_bus_with_services || { group_files_end; return 1; }
  seed_files
  launch_daemon || { group_files_end; return 1; }
  run_steps step_files_listing step_files_open step_files_reveal step_files_off
  local result=$STATUS
  stop_daemon || result=1
  run_steps step_files_tracker
  result=$(worst "$result" "$STATUS")
  group_files_end || result=1
  return "$result"
}

# ---------------------------------------------------------------- real input

# A real key, several in a row: each pressed, held 30 ms, released, then 250 ms to settle.
press() { local k; for k in "$@"; do input "key $k down" "sleep 30" "key $k up" "sleep 250" || return 1; done; }
# A real chord: $1 held around $2.
chord() {
  input "key $1 down" "sleep 30" "key $2 down" "sleep 30" "key $2 up" "sleep 30" "key $1 up" \
    "sleep 250"
}
typed() { input "type \"$1\"" "sleep 250"; }

# KWin's clipboard only: its socket by absolute path, stderr (never content) to the log.
kwin_wl() { in_env "$@" 2>>"$OUT_DIR/wl.log"; }

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

# A capture of the panel into $OUT_DIR/$1.png, once it has settled.
shot() { sleep 0.6; capture_launcher "$OUT_DIR/$1.png"; }

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

# real 1.
step_real_type_and_preview() {
  close_launcher
  open_empty || { echo "the launcher did not open" >&2; return 1; }
  typed "m9-report" || return 1
  await_field rows:"Top Hit" m9-report.pdf || return 1
  echo "real typing 'm9-report': Top Hit m9-report.pdf" >&2
  press BackSpace BackSpace BackSpace BackSpace BackSpace BackSpace BackSpace || return 1
  await_field query m9 || return 1
  sleep 0.5
  local second
  second=$(field row:1)
  press Down || return 1
  await_field selected 1 || return 1
  press space || return 1
  local shown
  shown=$(field preview)
  shot preview-space
  echo "backspaced to 'm9' ($(field headings)); real Down to '$second', real Space: preview" \
    "$shown, query '$(field query)'" >&2
  [ "$shown" != hidden ] && [ "$(field query)" = m9 ] || return 1
  press space || return 1
  await_field preview hidden || return 1
  [ "$(field query)" = m9 ] || { echo "Space typed into the field: '$(field query)'" >&2; return 1; }
  echo "real Space again: preview hidden, query still 'm9'" >&2
  typed "-rep" || return 1
  await_field query m9-rep || return 1
  chord Control_L y || return 1
  await_field preview pdf || return 1
  shot preview-ctrl-y
  echo "typing '-rep', real Ctrl+Y (⌘Y): preview pdf while typing" >&2
}

# real 2.
step_real_pane_keys() {
  close_launcher
  open_empty || return 1
  typed "m9-report" || return 1
  await_field rows:"Top Hit" m9-report.pdf || return 1
  [ "$(field preview)" = hidden ] || return 1
  press Right || return 1
  await_field preview pdf || { echo "the first → did not show the pane" >&2; return 1; }
  shot pane-shown
  press Right || return 1
  shot pane-tab0
  local headings b01 b12 b02 e1 x0 width shown_ring
  headings=$(field headings)
  press Tab || return 1
  shot pane-tab1
  press Tab || return 1
  shot pane-tab2
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
  press Escape || return 1
  shot pane-escape1
  e1=$(diff_box pane-shown pane-escape1)
  echo "real Escape 1: preview $(field preview), the ring gone (against before entering: [$e1])" >&2
  [ "$(field preview)" = pdf ] && [ "$e1" = none ] || return 1
  press Escape || return 1
  await_field preview hidden || return 1
  is_open || { echo "the second Escape closed the launcher" >&2; return 1; }
  echo "real Escape 2: the pane hides, the launcher stays" >&2
  press Escape || return 1
  sleep 0.3
  is_open && { echo "the third Escape left the launcher open" >&2; return 1; }
  echo "real Escape 3: the launcher closes" >&2
}

# real 3.
step_real_reveal() {
  close_launcher
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

# real 4.
step_real_emoji() {
  close_launcher
  open_empty || return 1
  chord Shift_L Tab || return 1
  chord Shift_L Tab || return 1
  await_field headings Emoji || return 1
  local cells at before moved=0 k want
  cells=$(field count)
  shot emoji-grid
  echo "real Shift+Tab twice, nothing typed: the Emoji grid, $cells cells" >&2
  at=$(field selected)
  for k in Down:8 Down:8 Right:1 Left:-1 Up:-8; do
    before=$at
    press "${k%%:*}" || return 1
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
  shot emoji-moved
  local glyph pasted
  glyph=$(field row:"$at")
  kwin_wl wl-copy --clear
  press Return || return 1
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
    v = json.loads(sys.stdin.read())["v"]["v"]
except (ValueError, KeyError):
    sys.exit(0)
history = v.get("status", {})
print(history.get("mode", "?"), history.get("protocol", "?"), v.get("entries", "?"))'
}

# real 5. Returns 2 (not proven) when KWin offers no data-control manager.
step_real_clipboard() {
  close_launcher
  local status word=lighthouse made_up="m9-real-input lighthouse 7731 made-up words"
  status=$(clip_status)
  echo "sill clipboard status on KWin: $status" >&2
  case "$status" in
    "memory unsupported"*) echo "KWin offers no data-control manager" >&2; return 2 ;;
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
  press Return || return 1
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

# Scroll. Recorded: whether real Down keeps the selected row on the card.
record_scroll() {
  open_empty || return 1
  typed "m9-list" || return 1
  await_field rows:"Top Hit" m9-list-01.txt || return 1
  press Tab || return 1
  await_field headings Documents || return 1
  local rows first last deep back
  rows=$(field count)
  shot q340-top
  first=$(selection_pixels q340-top)
  press Down Down Down Down Down Down Down Down Down Down Down || return 1
  await_field selected 11 || return 1
  shot q340-down
  last=$(selection_pixels q340-down)
  press Up Up Up Up Up Up Up Up Up Up Up || return 1
  await_field selected 0 || return 1
  shot q340-up
  back=$(selection_pixels q340-up)
  deep=OPEN
  [ "$last" -gt $(( first / 2 )) ] && deep=CLOSED
  echo "scroll $deep: Documents filtered by real Tab ($rows rows); the selected row's tint:" \
    "$first px on row 0, $last px after real Down x11 (row 11), $back px after real Up x11" \
    "(row 0); captures q340-{top,down,up}.png" >&2
  close_launcher
}

# Caret. Recorded: where the caret starts when the launcher opens with a query.
record_caret() {
  close_launcher
  sill launcher open --query m9 >/dev/null 2>&1
  await_field query m9 || return 1
  sleep 0.5
  shot q341-open
  typed "x" || return 1
  sleep 0.3
  local query caret=OPEN
  query=$(field query)
  [ "$query" = m9x ] && caret=CLOSED
  echo "caret $caret: after 'sill launcher open --query m9' a real 'x' makes the query" \
    "'$query' (m9x when the caret starts at the end)" >&2
  press BackSpace || return 1
  close_launcher
  sill launcher open --query m9 >/dev/null 2>&1
  await_field query m9 || return 1
  sleep 0.5
  press Right || return 1
  local first
  first=$(field preview)
  press End Right || return 1
  echo "real → after opening: preview $first; real End then →: preview $(field preview)," \
    "query '$(field query)'" >&2
  close_launcher
}

cleanup_real() {
  stop_input
  stop_fm
  close_launcher 2>/dev/null
  [ -z "$WL" ] || kwin_wl wl-copy --clear
  stop_daemon
  stop_kwin
  scratch_drop
}

main_real() {
  OUT_DIR="$OUT_ROOT/real-input"
  mkdir -p "$OUT_DIR"
  command -v wl-copy >/dev/null && command -v wl-paste >/dev/null \
    || { echo "wl-copy and wl-paste are missing (wl-clipboard); not installed by this script" >&2; exit 1; }
  scratch_new
  build "-p sill-testkit --example file_manager_stub" || { scratch_drop; exit 1; }
  seed_documents "$HOME_DIR" 300 list
  start_kwin || {
    echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2
    scratch_drop
    exit 1
  }
  trap cleanup_real EXIT
  : >"$OUT_DIR/wl.log"
  : >"$OUT_DIR/capture.log"
  : >"$OUT_DIR/keys.log"
  echo "headless KWin at $WL; data-control: $(env WAYLAND_DISPLAY="$WL" wayland-info 2>/dev/null \
    | grep -o "'[a-z_]*data_control_manager_v1'" | tr '\n' ' ')" >&2
  mkdir -p "$CONFIG/sill"
  printf '[launcher]\nfiles_backend = "fd"\nweb_search = "on"\nclipboard_history = "memory"\n' \
    >"$CONFIG/sill/settings.toml"
  launch_daemon || exit 1
  NAME=$(output_name) || exit 1
  sleep 1.5
  echo "daemon on KWin's bus, runtime $RUNTIME, home $HOME_DIR, output $NAME" >&2
  start_input || { echo "no fake input client" >&2; exit 1; }
  INPUT_TICKS=400
  # The seat's devices are announced with the input client's: the first events only make them known.
  input "move 300 900" "sleep 80" "move 310 900" "sleep 80" || exit 1
  run_steps step_real_type_and_preview step_real_pane_keys step_real_reveal step_real_emoji \
    step_real_clipboard
  local status=$STATUS
  record_scroll || echo "the scroll check could not be run" >&2
  record_caret || echo "the caret check could not be run" >&2
  echo "stopping the input client, the daemon and the headless KWin" >&2
  cleanup_real
  trap - EXIT
  echo "logs and captures in $OUT_DIR"
  exit "$status"
}

# ---------------------------------------------------------------- debug keys

cleanup_keys() {
  stop_fm
  stop_daemon
  stop_private_bus
  stop_kwin
  scratch_drop
}

main_keys() {
  local total=0 group
  OUT_DIR="$OUT_ROOT"
  mkdir -p "$OUT_ROOT" "$SHOTS"
  start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; exit 1; }
  trap cleanup_keys EXIT
  echo "headless KWin at $WL" >&2
  for group in toggle ui palette files; do
    [ "${#ONLY[@]}" = 0 ] || [[ " ${ONLY[*]} " == *" $group "* ]] || continue
    "group_$group"
    total=$(worst "$total" "$?")
  done
  cleanup_keys
  trap - EXIT
  echo "logs in $OUT_ROOT; captures in $SHOTS"
  exit "$total"
}

case "$MODE" in
  self) self_test; exit ;;
  real) main_real ;;
  *) main_keys ;;
esac

#!/usr/bin/env bash
# Launcher open latency of ONE daemon binary, on the harness's private headless KWin (never the
# person's session): request (`sill launcher open`) to the first frame the compositor showed of the
# panel, the daemon's own `launcher.open_to_frame_ms` line. Run it once per build and compare.
#
#   dev/measure-launcher-open.sh --bin target/release/sill --label media [--samples 30] [--cold 8]
#
# The binary must be built with `--features debug` (that is what gives `sill run` its `--fake-ddc`,
# so a release build never opens the real /dev/i2c-*), e.g.
#   cargo build --release -p sill --features debug                          # with libav
#   cargo build --release -p sill --features debug --no-default-features    # without it
#
# Everything runs in a scratch tree (HOME and every XDG directory), on a private dbus-daemon that
# is both session and system bus, with the fake DDC fixture. Phases:
#   cold:   --cold fresh daemons, each timed on its FIRST open (the process has just started;
#           libav's shared libraries, if linked, are loaded by now, but nothing is warm)
#   warm:   one daemon, one discarded open, then --samples opens with an empty query, each closed
#   media:  the same daemon, --samples opens with a query that lists the media files, so the
#           launcher has the recordings' rows (the preview pane's peek of a recording is a
#           separate, later step: `examples/peek_latency.rs` times it)
# Prints p50/p95/max for each, the daemon's RSS (closed, then open) and the binary's size.
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/all.sh"

BIN= LABEL= SAMPLES=30 COLD=8
FIXTURES="${FIXTURES:-$SILL_REPO/../anyview/crates/anyview-media/tests/fixtures}"
while [ $# -gt 0 ]; do
  case "$1" in
    --bin) BIN="$2"; shift ;;
    --label) LABEL="$2"; shift ;;
    --samples) SAMPLES="$2"; shift ;;
    --cold) COLD="$2"; shift ;;
    *) echo "usage: $0 --bin PATH --label NAME [--samples N] [--cold N]" >&2; exit 2 ;;
  esac
  shift
done
[ -x "$BIN" ] && [ -n "$LABEL" ] || { echo "need --bin (an executable) and --label" >&2; exit 2; }
SILL_BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"
OUT_DIR="$SILL_REPO/target/measure-launcher/$LABEL"
mkdir -p "$OUT_DIR"

cleanup() {
  stop_daemon
  stop_private_bus
  stop_kwin
  scratch_drop
}

# Every `launcher.open_to_frame_ms` value in the daemon's log, one per line.
frames() { grep -o 'launcher\.open_to_frame_ms=[0-9.]*' "$OUT_DIR/daemon.log" | cut -d= -f2; }

# One opening with query $1, waited for until its line is logged (up to 10 s); closes it.
open_once() {
  local before after
  before=$(frames | wc -l)
  sill launcher open --query "$1" >>"$OUT_DIR/keys.log" 2>&1
  for _ in $(seq 200); do
    after=$(frames | wc -l)
    [ "$after" -gt "$before" ] && break
    sleep 0.05
  done
  [ "$after" -gt "$before" ] || { echo "no open_to_frame_ms line for query '$1'" >&2; return 1; }
  sleep 0.3
  sill launcher close >>"$OUT_DIR/keys.log" 2>&1
  sleep 0.3
}

rss_kb() { awk '/^VmRSS/ {print $2}' "/proc/$DAEMON/status"; }

summary() { # name, values on stdin
  local p50 p95 max n
  mapfile -t v
  n=${#v[@]}
  read -r p50 p95 max < <(printf '%s\n' "${v[@]}" | percentiles)
  printf '%-6s n=%-3s p50 %8s ms  p95 %8s ms  max %8s ms\n' "$1" "$n" "$p50" "$p95" "$max" | tee -a "$OUT_DIR/summary.txt"
}

start_scratch_daemon() {
  : >"$OUT_DIR/daemon.log"
  launch_daemon || return 1
  sleep 1.5
}

: >"$OUT_DIR/summary.txt" >"$OUT_DIR/keys.log"
start_kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; exit 1; }
trap cleanup EXIT
scratch_new
mkdir -p "$HOME_DIR/Documents" "$CONFIG/sill"
cp "$FIXTURES/clip.mkv" "$HOME_DIR/Documents/m9media-clip.mkv"
cp "$FIXTURES/cover.mp3" "$HOME_DIR/Documents/m9media-cover.mp3"
cp "$FIXTURES/tone.flac" "$HOME_DIR/Documents/m9media-tone.flac"
printf '[launcher]\nfiles_backend = "fd"\nweb_search = "on"\n' >"$CONFIG/sill/settings.toml"
start_private_bus || exit 1
{
  echo "label $LABEL, binary $SILL_BIN"
  echo "size $(stat -c %s "$SILL_BIN") bytes, libav libs linked: $(ldd "$SILL_BIN" | grep -c 'libav\|libsw')"
  echo "load average at start: $(cut -d' ' -f1-3 /proc/loadavg)"
} | tee -a "$OUT_DIR/summary.txt"

# cold: a fresh daemon per sample, its first opening.
cold=()
for _ in $(seq "$COLD"); do
  start_scratch_daemon || exit 1
  open_once "" || exit 1
  cold+=("$(frames | head -1)")
  stop_daemon || exit 1
done
printf '%s\n' "${cold[@]}" | summary cold

# warm and media, on one daemon.
start_scratch_daemon || exit 1
NAME=$(output_name) || exit 1
open_once "" || exit 1 # discarded
for _ in $(seq "$SAMPLES"); do open_once "" || exit 1; done
frames | tail -n "$SAMPLES" | summary warm
echo "daemon RSS closed: $(rss_kb) kB" | tee -a "$OUT_DIR/summary.txt"
for _ in $(seq "$SAMPLES"); do open_once "m9media" || exit 1; done
frames | tail -n "$SAMPLES" | summary media
sill launcher open --query "m9media" >>"$OUT_DIR/keys.log" 2>&1
sleep 1.5
echo "daemon RSS open with the media rows: $(rss_kb) kB" | tee -a "$OUT_DIR/summary.txt"
echo "rows: $(sill debug launcher-rows 2>/dev/null | head -c 300)" >>"$OUT_DIR/summary.txt"
sill launcher close >>"$OUT_DIR/keys.log" 2>&1
echo "raw samples in $OUT_DIR/daemon.log"

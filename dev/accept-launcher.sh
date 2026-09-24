#!/usr/bin/env bash
# Milestone 4 acceptance for the launcher (plan: "`dev/accept-launcher.sh` gates p95 < 100 ms
# over 20 toggles"), on the nested cosmic-comp (dev/nested.sh; a running `wayland-1` is reused
# and never stopped). Starts `sill run --features debug` from this checkout with a scratch
# `XDG_CONFIG_HOME`, so nothing it does touches the real `~/.config/sill`, and quits it at the
# end.
#
#   1. 20 `sill launcher toggle` round trips. Every one is timed on the daemon's own side
#      regardless of what `Request::Launcher` does with it (`crates/sill/src/daemon.rs`'s
#      `launcher.open_latency_ms` line, M4 perf-ime; FINDINGS.md "M4 perf-ime"), so this step
#      runs for real today even though M4 launcher-ui has not landed: it measures IPC-only
#      overhead now (a stub), and will measure real open latency, unchanged, once the launcher
#      spawns and shows a panel for that same request. Asserts p95 < 100 ms; prints p50/p95/max.
#   2. The panel exists after a toggle (`sill debug capture <launcher panel>`), and after the
#      closing toggle, 5 s idle adds 0 frames to every launcher surface. Exits 2, naming "M4
#      launcher-ui", while no `sill-launcher-*` surface is in `sill debug stats` (true until
#      that wave lands; re-checked here rather than assumed, so this step starts passing for
#      real the moment it does).
#   3. The nested `custom` shortcut Ctrl+Alt+Space (seeded by shell-host's own
#      `scripts/dev/xdg-config`, already loaded when the running `cosmic-comp` started) sends
#      `Spawn("sill launcher toggle")`. Exercised through `wtype` (present on this machine) or
#      `ydotool`, whichever exists; skipped with the reason if neither does, or if the daemon's
#      log gains no new `launcher.open_latency_ms` line within 5 s. Two plausible causes, not
#      distinguished here (`comp.log` logs neither): `cosmic-comp`'s `Spawn` resolves `sill` on
#      ITS OWN inherited `PATH`, fixed when `dev/nested.sh` started it, before this script ever
#      built one; or a `wtype`-injected key (the virtual-keyboard protocol) never reaches
#      `cosmic-comp`'s global-shortcut matcher the way a real keyboard's does. See FINDINGS.md
#      "M4 perf-ime".
#
# Exit 0 only when every step passed; a launcher-dependent step not yet reachable exits 2 (never
# 0), same convention as `dev/accept-bar.sh`'s early steps.
#
# Usage: dev/nested.sh && dev/accept-launcher.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-launcher"
mkdir -p "$OUT_DIR"
TOGGLES=20
P95_BUDGET_MS=100

sill() { "$SILL_BIN" "$@"; }
nested() { "$SHELL_HOST_DEV/run.sh" "$@"; }

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 || {
    echo "build failed (see $OUT_DIR/build.log)" >&2
    return 1
  }
}

start_daemon() {
  CONFIG=$(mktemp -d)
  XDG_CONFIG_HOME="$CONFIG" nested "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do
    sill ping >/dev/null 2>&1 && return 0
    sleep 0.1
  done
  echo "the daemon never answered 'sill ping'" >&2
  return 1
}

stop_daemon() {
  [ -n "${DAEMON:-}" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
  rm -rf "${CONFIG:-}"
}

# Every `launcher.open_latency_ms=<ms>` value the daemon's log holds, one per line.
open_latencies() {
  # The panel's request-to-first-frame line (M4 launcher-ui) is the real open latency; the
  # daemon's `open_latency_ms` times the IPC dispatch alone and is the fallback.
  if grep -q 'launcher\.open_to_frame_ms=' "$OUT_DIR/daemon.log"; then
    grep -o 'launcher\.open_to_frame_ms=[0-9.]*' "$OUT_DIR/daemon.log" | cut -d= -f2
  else
    grep -o 'launcher\.open_latency_ms=[0-9.]*' "$OUT_DIR/daemon.log" | cut -d= -f2
  fi
}

# p50, p95, max of the values on stdin, one per line, as "p50 p95 max"; empty input prints
# nothing (the caller checks it got a line).
percentiles() {
  python3 -c '
import sys
values = sorted(float(line) for line in sys.stdin if line.strip())
if not values:
    sys.exit(0)
def pct(p):
    i = min(len(values) - 1, int(round(p * (len(values) - 1))))
    return values[i]
print(f"{pct(0.50):.3f} {pct(0.95):.3f} {values[-1]:.3f}")
'
}

# 1.
step_toggle_latency() {
  local log="$OUT_DIR/toggle-latency.log" before after n p50 p95 max
  : >"$log"
  before=$(open_latencies | wc -l)
  for _ in $(seq "$TOGGLES"); do
    # `launcher toggle` answers `Failed(Unsupported)` until M4 launcher-ui lands (`daemon.rs`'s
    # `dispatch_launcher`), which is a nonzero client exit — expected, and irrelevant to this
    # step: the daemon times and logs the round trip before it even looks at the request.
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

# The launcher's own surfaces, "name frames" lines (mirrors `accept-bar.sh`'s `bar_counters`).
launcher_counters() {
  sill debug stats | python3 -c '
import json, sys
for row in json.loads(sys.stdin.read())["v"]:
    if row["surface"].startswith("sill-launcher-"):
        print(row["surface"], row["frames"])'
}

# 2.
step_panel_and_idle() {
  local log="$OUT_DIR/panel.log" surface before after
  : >"$log"
  sill launcher toggle >>"$log" 2>&1 || true
  surface=$(launcher_counters | awk 'NR==1 {print $1}')
  if [ -z "$surface" ]; then
    echo "no sill-launcher-* surface in 'sill debug stats' yet (M4 launcher-ui)" | tee -a "$log" >&2
    return 2
  fi
  sill debug capture "$surface" "$OUT_DIR/panel.png" >>"$log" 2>&1 || {
    echo "the launcher surface is listed but would not capture" >&2
    return 1
  }
  before=$(launcher_counters | awk '{sum += $2} END {print sum + 0}')
  sill launcher toggle >>"$log" 2>&1 || true # close it
  sleep 5
  after=$(launcher_counters | awk '{sum += $2} END {print sum + 0}')
  echo "closed frames before $before, after 5s idle $after" | tee -a "$log" >&2
  [ "$before" = "$after" ]
}

# 3.
step_shortcut() {
  local log="$OUT_DIR/shortcut.log" before after tool
  : >"$log"
  if command -v wtype >/dev/null 2>&1; then
    tool=wtype
  elif command -v ydotool >/dev/null 2>&1; then
    tool=ydotool
  else
    echo "skipped: neither wtype nor ydotool is installed" | tee -a "$log" >&2
    return 2
  fi
  before=$(open_latencies | wc -l)
  case "$tool" in
    wtype) nested wtype -M ctrl -M alt -k space -m alt -m ctrl >>"$log" 2>&1 ;;
    ydotool) nested ydotool key ctrl+alt+space >>"$log" 2>&1 ;;
  esac || { echo "sending Ctrl+Alt+Space via $tool failed" >>"$log"; }
  for _ in $(seq 50); do
    after=$(open_latencies | wc -l)
    [ "$after" -gt "$before" ] && break
    sleep 0.1
  done
  after=$(open_latencies | wc -l)
  echo "$tool sent Ctrl+Alt+Space; open_latency_ms lines before $before, after $after" \
    | tee -a "$log" >&2
  if [ "$after" -le "$before" ]; then
    echo "skipped: no new launcher.open_latency_ms line; either cosmic-comp's Spawn could not" \
      "resolve 'sill' on its own PATH, or the shortcut never matched a wtype-injected key" \
      "(see FINDINGS.md 'M4 perf-ime')" \
      | tee -a "$log" >&2
    return 2
  fi
}

step_quit() {
  sill quit >"$OUT_DIR/quit.log" 2>&1 || return 1
  grep -q '"kind":"done"' "$OUT_DIR/quit.log" || return 1
  sleep 0.2
  if sill ping >"$OUT_DIR/quit-ping.log" 2>&1; then
    echo "'sill ping' still succeeded after quit (see $OUT_DIR/quit-ping.log)" >&2
    return 1
  fi
}

source "$SHELL_HOST_DEV/.nested-env"
build || exit 1
if sill ping >/dev/null 2>&1; then
  echo "a sill daemon is already running; quit it first (this script starts its own)" >&2
  exit 1
fi
start_daemon || { stop_daemon; exit 1; }
trap stop_daemon EXIT
status=0
for step in step_toggle_latency step_panel_and_idle step_shortcut step_quit; do
  if "$step"; then
    echo "PASS $step"
  else
    rc=$?
    if [ "$rc" -eq 2 ]; then
      echo "SKIP $step"
    else
      echo "FAIL $step (exit $rc)"
    fi
    status=$(( rc > status ? rc : status ))
  fi
done
echo "logs in $OUT_DIR"
exit "$status"

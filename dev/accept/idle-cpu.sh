#!/usr/bin/env bash
# sill's idle-cost acceptance, on the nested cosmic-comp ONLY (dev/nested.sh; wayland-N is reused
# and never stopped). A release+debug build is started on a private bus (both session and system
# bus, nothing else on it) with scratch HOME/XDG and the fake DDC, left alone for WARMUP seconds,
# then sampled over DURATION seconds from /proc/<pid>:
#
#   1. CPU: utime+stime over the window, as a percentage of one core, must stay under
#      MAX_CPU_PCT (default 1.0). A shell with nothing to do and nothing on the screen moving
#      must not spin: before the fix it burned about 48 %.
#   2. Threads: the highest thread count seen must stay under MAX_THREADS (default 120: the
#      tokio workers, one per core, and a few dozen others). The blocking pool is capped
#      (`max_blocking_threads`), so a loop of `spawn_blocking` calls cannot grow it without
#      bound. The nested daemon has no GPU adapter and renders on Mesa's llvmpipe, which starts
#      one thread pool per core per device: LP_NUM_THREADS=2 keeps those out of the count.
#   3. RSS: reported only (MAX_RSS_MB, when set, makes it a limit).
#
# SAFETY: the daemon gets its own XDG_RUNTIME_DIR/HOME/XDG_*, the private bus as both buses, the
# fake DDC; it only touches processes it started itself (kill by its own pid).
#
# Usage: dev/nested.sh && dev/accept/idle-cpu.sh
#   DURATION=60 WARMUP=20 MAX_CPU_PCT=1.0 MAX_THREADS=120 SILL_PROFILE=release|debug (default release)
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_REPO/target/accept-idle-cpu"
mkdir -p "$OUT_DIR"
DURATION="${DURATION:-60}"
WARMUP="${WARMUP:-20}"
INTERVAL="${INTERVAL:-5}"
MAX_CPU_PCT="${MAX_CPU_PCT:-1.0}"
MAX_THREADS="${MAX_THREADS:-120}"
MAX_RSS_MB="${MAX_RSS_MB:-}"
PROFILE="${SILL_PROFILE:-release}"
SILL_BIN="$SILL_REPO/target/$PROFILE/sill"
CSV="$OUT_DIR/idle.csv"

# utime+stime in clock ticks of every thread of $1 (the process's own stat line).
ticks_of() {
  awk '{ rest = $0; sub(/^.*\) /, "", rest); split(rest, f, " "); print f[12] + f[13] }' "/proc/$1/stat"
}
threads_of() { awk '/^Threads:/ { print $2 }' "/proc/$1/status"; }
rss_mb_of() { awk '/^VmRSS:/ { printf "%d\n", $2 / 1024 }' "/proc/$1/status"; }

step_idle_cost() {
  clear_of_the_minute
  sleep "$WARMUP"
  kill -0 "$DAEMON" 2>/dev/null || { echo "the daemon died during warm-up (see $OUT_DIR/daemon.log)" >&2; return 1; }
  local hz t0 t1 ticks0 ticks max_threads=0 threads rss max_rss=0 elapsed=0 pct
  hz=$(getconf CLK_TCK)
  ticks0=$(ticks_of "$DAEMON")
  t0=$(date +%s.%N)
  echo "elapsed_s,ticks,threads,rss_mb" >"$CSV"
  while [ "$elapsed" -lt "$DURATION" ]; do
    sleep "$INTERVAL"
    elapsed=$((elapsed + INTERVAL))
    kill -0 "$DAEMON" 2>/dev/null || { echo "the daemon died while sampled" >&2; return 1; }
    ticks=$(ticks_of "$DAEMON"); threads=$(threads_of "$DAEMON"); rss=$(rss_mb_of "$DAEMON")
    echo "$elapsed,$ticks,$threads,$rss" >>"$CSV"
    [ "$threads" -le "$max_threads" ] || max_threads=$threads
    [ "$rss" -le "$max_rss" ] || max_rss=$rss
  done
  t1=$(date +%s.%N)
  pct=$(python3 -c 'import sys
hz, t0, t1, a, b = map(float, sys.argv[1:])
print(f"{100.0 * (b - a) / hz / (t1 - t0):.2f}")' "$hz" "$t0" "$t1" "$ticks0" "$ticks")
  echo "idle over ${DURATION}s: cpu ${pct}% of a core (max ${MAX_CPU_PCT}), threads max ${max_threads} (max ${MAX_THREADS}), rss ${max_rss} MB; samples in $CSV"
  local result=0
  python3 -c 'import sys; sys.exit(0 if float(sys.argv[1]) <= float(sys.argv[2]) else 1)' "$pct" "$MAX_CPU_PCT" \
    || { echo "FAIL cpu ${pct}% > ${MAX_CPU_PCT}%" >&2; result=1; }
  [ "$max_threads" -le "$MAX_THREADS" ] || { echo "FAIL threads ${max_threads} > ${MAX_THREADS}" >&2; result=1; }
  [ -z "$MAX_RSS_MB" ] || [ "$max_rss" -le "$MAX_RSS_MB" ] || { echo "FAIL rss ${max_rss} MB > ${MAX_RSS_MB}" >&2; result=1; }
  return "$result"
}

cleanup() {
  local result=0
  stop_daemon || result=1
  stop_private_bus
  scratch_drop
  return "$result"
}

nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
use_nested_socket || exit 1
if [ "$PROFILE" = release ]; then
  (cd "$SILL_REPO" && cargo build --release -p sill --features debug) >"$OUT_DIR/build.log" 2>&1 \
    || { echo "build failed (see $OUT_DIR/build.log)" >&2; exit 1; }
else
  build || exit 1
fi
IN_ENV_EXTRA+=(LP_NUM_THREADS=2)
scratch_new
start_private_bus || { scratch_drop; exit 1; }
launch_daemon || { cleanup; exit 1; }
run_steps step_idle_cost
status=$STATUS
cleanup || status=1
echo "logs in $OUT_DIR"
exit "$status"

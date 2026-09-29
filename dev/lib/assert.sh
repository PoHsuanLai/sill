# Steps and their verdicts, timing and statistics.

# Milliseconds since the epoch.
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }

# "p50 p95 max" of the numbers on stdin, three decimals.
percentiles() {
  python3 -c '
import sys
values = sorted(float(line) for line in sys.stdin if line.strip())
if not values:
    sys.exit(0)
def pct(p):
    return values[min(len(values) - 1, int(round(p * (len(values) - 1))))]
print(f"{pct(0.50):.3f} {pct(0.95):.3f} {values[-1]:.3f}")'
}

# Runs each function named in "$@" as a step and prints PASS, SKIP or FAIL for it: a step
# returns 0 when it passed, 2 when it could not run safely, anything else on a failure. Sets
# STATUS to 0 when every step passed, 2 when a step skipped and none failed, 1 otherwise.
run_steps() {
  local step result
  STATUS=0
  for step in "$@"; do
    "$step"; result=$?
    case "$result" in
      0) echo "PASS $step" ;;
      2) echo "SKIP $step"; [ "$STATUS" = 1 ] || STATUS=2 ;;
      *) echo "FAIL $step"; STATUS=1 ;;
    esac
  done
}

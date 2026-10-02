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

# Sleeps past the minute's turn when it is within 10 s: the clock redraws the bar and the widgets
# once a minute, which a frame count over a few idle seconds must not include.
clear_of_the_minute() {
  if [ "$((10#$(date +%S)))" -ge 50 ]; then sleep $((62 - 10#$(date +%S))); fi
}

# A latency budget's verdict. $1: what was measured against what; $2: 1 when it is inside the
# budget, 0 when not. The budgets are release-on-an-idle-host budgets (budgets_apply,
# compositor.sh): on a debug build, which runs several times slower, or on an oversubscribed
# host the line is printed as "not enforced" and the check passes, so such a run fails only on
# functional faults.
budget_check() {
  if budgets_apply; then
    echo "budget: $1" >&2
    [ "$2" = 1 ]
  else
    echo "budget (not enforced: $(budgets_why)): $1; inside: $([ "$2" = 1 ] && echo yes || echo no)" >&2
    return 0
  fi
}

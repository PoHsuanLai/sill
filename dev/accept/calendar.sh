#!/usr/bin/env bash
# Calendar acceptance (design/20 §1.12 and §1.14), on the nested cosmic-comp (dev/nested.sh; its
# socket is reused and never stopped).
# Starts `sill run` from this checkout's debug build (`--features debug`) on a private bus (the
# daemon's session AND system bus), with a scratch HOME and XDG directories, its own
# XDG_RUNTIME_DIR (the nested socket as an absolute WAYLAND_DISPLAY) and a fake DDC.
# `calendar.sources` names three temp `.ics` files written here (never a real
# calendar: `auto` is not used), each its own calendar so Up Next's colour bars tell them apart:
# work.ics "Design review" today in an hour, team.ics a weekly "Yoga" (a week ago and today, in
# three hours), personal.ics "Dentist" tomorrow at 09:00. `widgets.center = ["calendar",
# "up_next"]`. The daemon is quit at the end.
#
#   1. `sill notifications open`: a capture of `sill-notification-center-<out>` shows the month
#      with today on the accent disc (dev/out/calendar-cosmic.png; the output with the center
#      open, dev/out/calendar-widget-cosmic.png).
#   2. The Large month lists today's events still to come under it, from the agenda Up Next reads,
#      then Up Next lists the fixture's next events in order, as many as the Medium tile's body
#      holds without clipping (two, at the default cell and `up_next_max`). The colour bars, top to
#      bottom, are the month's work and team, then Up Next's work and team (quire's label hues
#      `--c-{red,amber,green,blue,violet}`, picked by the calendar's name as
#      `widgets::up_next::calendar_hue` picks it); personal's (tomorrow's Dentist) is not today's
#      and is Up Next's third-soonest, so it draws nowhere.
#   3. work.ics rewritten with the review cancelled: within 2 s the bars are the month's team,
#      then Up Next's team and personal (now the two soonest).
#   4. `calendar.week_numbers = "show"` and `sill reload`: the week column appears (the month,
#      centred in its tile, moves today's disc right by half a 24 px column).
#   5. 5 s idle while open, clear of the minute's turn: 0 frames on the center.
#
# Exit 0 when every step passed, 2 when the notification center draws no widgets (the widget
# slot is not in this build), 1 on a failure. Another sill daemon started from a
# `target/*/sill run` is waited for (up to 10 minutes).
#
# Usage: dev/nested.sh && dev/accept/calendar.sh
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_REPO/target/accept-calendar"
SHOTS="$SILL_REPO/dev/out"
mkdir -p "$OUT_DIR" "$SHOTS"

scratch_new
trap scratch_drop EXIT
CAL="$SCRATCH/calendars"
mkdir -p "$CAL"

# A screenshot of the nested output only (shot.sh falls back to the host's session when no
# nested compositor answers, so it is never called without one).
shot() {
  "$SHELL_HOST_DEV/run.sh" wayland-info >/dev/null 2>&1 || { echo "no nested compositor" >&2; return 1; }
  "$SHELL_HOST_DEV/shot.sh" "$1" >/dev/null 2>&1
}

# The calendar files: one event each, in local (floating) time, from now.
write_fixture() {
  python3 - "$CAL" "${1:-CONFIRMED}" <<'EOF'
import sys, datetime as dt
cal, review = sys.argv[1], sys.argv[2]
now = dt.datetime.now().replace(second=0, microsecond=0)
fmt = lambda t: t.strftime("%Y%m%dT%H%M%S")
def write(name, body):
    text = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//sill//accept-calendar//EN\r\n" \
        + body + "END:VCALENDAR\r\n"
    open(f"{cal}/{name}.part", "w").write(text)
    import os; os.replace(f"{cal}/{name}.part", f"{cal}/{name}")
def event(uid, title, start, end, extra=""):
    return (f"BEGIN:VEVENT\r\nUID:{uid}\r\nSUMMARY:{title}\r\nDTSTART:{fmt(start)}\r\n"
            f"DTEND:{fmt(end)}\r\n{extra}END:VEVENT\r\n")
review_at = now + dt.timedelta(hours=1)
write("work.ics", event("review", "Design review", review_at, review_at + dt.timedelta(minutes=30),
                        f"STATUS:{review}\r\n"))
yoga = now + dt.timedelta(hours=3) - dt.timedelta(days=7)
write("team.ics", event("yoga", "Yoga", yoga, yoga + dt.timedelta(hours=1),
                        "RRULE:FREQ=WEEKLY;COUNT=2\r\n"))
dentist = (now + dt.timedelta(days=1)).replace(hour=9, minute=0)
write("personal.ics", event("dentist", "Dentist", dentist, dentist + dt.timedelta(hours=1)))
EOF
}

# The label hue (1 red, 2 amber, 3 green, 4 blue, 5 violet) a calendar name takes (FNV-1a, as
# `widgets::up_next::calendar_hue`).
hue_of() {
  python3 -c '
import sys
h = 2166136261
for b in sys.argv[1].encode():
    h = ((h ^ b) * 16777619) & 0xFFFFFFFF
print(h % 5 + 1)' "$1"
}

# What a capture of the center shows: "BARS <i> <i> ... ; DISC <x> <y> <w> <h>", the events'
# colour bars top to bottom (thin upright runs of a label hue, light or dark: the month's list,
# then Up Next's) and the largest round blob of one tinted colour (today's accent disc), or
# "DISC none".
probe() {
  python3 - "$1" <<'EOF'
import sys
import numpy as np
from PIL import Image
img = np.asarray(Image.open(sys.argv[1]).convert("RGBA")).astype(int)
ys, xs = np.nonzero(img[..., 3] > 200)
img = img[ys.min():ys.max() + 1, xs.min():xs.max() + 1]
rgb, alpha = img[..., :3], img[..., 3]
# quire's label hues (`LabelHue::family`), light then dark, in the order red, amber, green,
# blue, violet.
swatches = np.array([[0xE8, 0x48, 0x3C], [0xF0, 0xA8, 0x1E], [0x28, 0xB2, 0x4A], [0x2B, 0x7C, 0xFF],
                     [0x8B, 0x5C, 0xF0], [0xFF, 0x6B, 0x60], [0xF5, 0xBC, 0x4E], [0x4F, 0xD0, 0x6A],
                     [0x4C, 0x9B, 0xFF], [0xA9, 0x8B, 0xFF]])
dist = np.abs(rgb[:, :, None, :] - swatches[None, None, :, :]).sum(axis=3)
nearest, best = dist.argmin(axis=2), dist.min(axis=2)
on = (best < 36) & (alpha > 200)

def components(mask):
    seen = np.zeros(mask.shape, bool)
    ys, xs = np.nonzero(mask)
    for y0, x0 in zip(ys, xs):
        if seen[y0, x0]:
            continue
        stack, pts = [(y0, x0)], []
        seen[y0, x0] = True
        while stack:
            y, x = stack.pop()
            pts.append((y, x))
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                v, u = y + dy, x + dx
                if 0 <= v < mask.shape[0] and 0 <= u < mask.shape[1] and mask[v, u] and not seen[v, u]:
                    seen[v, u] = True
                    stack.append((v, u))
        yield np.array(pts)

bars = []
for pts in components(on):
    h = pts[:, 0].max() - pts[:, 0].min() + 1
    w = pts[:, 1].max() - pts[:, 1].min() + 1
    if h >= 14 and w <= 6:
        idx = np.bincount(nearest[pts[:, 0], pts[:, 1]], minlength=10).argmax() % 5 + 1
        bars.append((pts[:, 0].min(), idx))
bars.sort()

mx, mn = rgb.max(axis=2), rgb.min(axis=2)
sat = (mx - mn) / np.maximum(mx, 1)
# The accent is a pastel fill since quire accent B (#94c0fe light: saturation 0.42), so the bar
# is 0.3; the label hues are masked out above.
vivid = (sat > 0.3) & (mx > 90) & (alpha > 200) & ~on
disc = None
for pts in components(vivid):
    h = pts[:, 0].max() - pts[:, 0].min() + 1
    w = pts[:, 1].max() - pts[:, 1].min() + 1
    fill = len(pts) / (h * w)
    # Today's busy dot may touch the disc's foot: the width is the disc's, the height a little more.
    if 16 <= w <= 40 and w - 3 <= h <= w + 12 and fill > 0.45 and (disc is None or len(pts) > disc[4]):
        disc = (pts[:, 1].min(), pts[:, 0].min(), w, h, len(pts))
line = "BARS " + " ".join(str(i) for _, i in bars) + " ; DISC "
line += " ".join(str(v) for v in disc[:4]) if disc else "none"
print(line)
EOF
}

bars_of() { sed -e 's/^BARS *//' -e 's/ *;.*$//' <<<"$1"; }
disc_of() { sed -e 's/^.*DISC //' <<<"$1"; }

write_settings() {
  mkdir -p "$CONFIG/sill"
  cat >"$CONFIG/sill/settings.toml" <<TOML
[calendar]
sources = "$CAL/work.ics;$CAL/team.ics;$CAL/personal.ics"
week_numbers = "$1"

[widgets]
center = ["calendar", "up_next"]
TOML
}

start_daemon() {
  backdrop_seed "$CONFIG" || return 1
  write_fixture
  write_settings hide
  launch_daemon || return 1
  NAME=$(output_name) || return 1
  CENTER="sill-notification-center-$NAME"
  sleep 1.5
  backdrop_up "$CONFIG" || echo "backdrop: none (the shots show the bare compositor)" >&2
  echo "daemon on a private bus and runtime dir $RUNTIME; calendars in $CAL; $CENTER" >&2
}

cleanup() {
  local result=0
  backdrop_down
  stop_daemon || result=1
  stop_private_bus
  scratch_drop
  return "$result"
}

WORK=$(hue_of work)
TEAM=$(hue_of team)
PERSONAL=$(hue_of personal)

# 1. and 2.
step_month_and_up_next() {
  local log="$OUT_DIR/open.log" seen
  : >"$log"
  sill notifications open >>"$log" 2>&1 || return 1
  wait_surface "$CENTER" 20 up || { echo "the center never mapped" >&2; return 1; }
  sleep 1
  sill debug capture "$CENTER" "$SHOTS/calendar-cosmic.png" >>"$log" 2>&1 || return 1
  shot "$SHOTS/calendar-widget-cosmic.png" || echo "screenshot failed" >&2
  seen=$(probe "$SHOTS/calendar-cosmic.png")
  echo "capture: $seen (work $WORK, team $TEAM, personal $PERSONAL)" | tee -a "$log" >&2
  if [ "$(bars_of "$seen")" = "" ] && [ "$(disc_of "$seen")" = none ]; then
    echo "the center draws no widgets: the widget slot (CenterWidgets) is not in this build" >&2
    return 2
  fi
  DISC_BEFORE=$(disc_of "$seen")
  [ "$DISC_BEFORE" != none ] || { echo "no accent disc on today" >&2; return 1; }
  [ "$(bars_of "$seen")" = "$WORK $TEAM $WORK $TEAM" ] \
    || { echo "the month's events and Up Next's are not review, yoga (a Medium tile holds two)" >&2; return 1; }
}

# 3.
step_edit() {
  local log="$OUT_DIR/edit.log" start seen elapsed
  : >"$log"
  start=$(date +%s%N)
  write_fixture CANCELLED
  for _ in $(seq 40); do
    sill debug capture "$CENTER" "$OUT_DIR/edit.png" >>"$log" 2>&1 || return 1
    seen=$(probe "$OUT_DIR/edit.png")
    [ "$(bars_of "$seen")" = "$TEAM $TEAM $PERSONAL" ] && break
    sleep 0.05
  done
  elapsed=$(( ($(date +%s%N) - start) / 1000000 ))
  echo "after the edit: $seen, shown after ${elapsed} ms (the probe's own time included)" \
    | tee -a "$log" >&2
  [ "$(bars_of "$seen")" = "$TEAM $TEAM $PERSONAL" ] && [ "$elapsed" -le 2000 ]
}

# 4.
step_week_numbers() {
  local log="$OUT_DIR/weeks.log" seen before after
  : >"$log"
  write_settings show
  sill reload >>"$log" 2>&1 || return 1
  sleep 1
  sill debug capture "$CENTER" "$OUT_DIR/weeks.png" >>"$log" 2>&1 || return 1
  shot "$OUT_DIR/weeks-output.png" || true
  seen=$(probe "$OUT_DIR/weeks.png")
  before=$(cut -d' ' -f1 <<<"$DISC_BEFORE")
  after=$(disc_of "$seen" | cut -d' ' -f1)
  echo "week numbers shown: $seen; today's disc from x $before to x $after" | tee -a "$log" >&2
  [ "$after" != none ] && [ $(( after - before )) -ge 9 ] && [ $(( after - before )) -le 15 ]
}

# 5.
step_idle() {
  local log="$OUT_DIR/idle.log" f0 f1
  : >"$log"
  sleep 2
  clear_of_the_minute
  f0=$(frames_of "$CENTER")
  sleep 5
  f1=$(frames_of "$CENTER")
  echo "5 s idle open: $(( f1 - f0 )) frames on $CENTER" | tee -a "$log" >&2
  sill notifications close >>"$log" 2>&1
  [ "$(( f1 - f0 ))" = 0 ]
}

use_nested_socket || exit 1
nested wayland-info >/dev/null 2>&1 || { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
build || exit 1
await_no_process 'target/[a-z]*/sill run' || exit 1
start_private_bus || exit 1
start_daemon || { cleanup; exit 1; }
status=0
for step in step_month_and_up_next step_edit step_week_numbers step_idle; do
  "$step"
  code=$?
  case "$code" in
    0) echo "PASS $step" ;;
    2) echo "SKIP $step (see above)"; status=2; break ;;
    *) echo "FAIL $step"; status=1 ;;
  esac
done
cleanup || status=1
echo "logs in $OUT_DIR; screenshots $SHOTS/calendar-cosmic.png, $SHOTS/calendar-widget-cosmic.png"
exit "$status"

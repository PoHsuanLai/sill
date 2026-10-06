#!/usr/bin/env bash
# The accounts sheet's acceptance, on a private headless compositor (casement under
# dev/accept.sh --compositor casement, else KWin, started and stopped by the script), against a
# stand-in accountd: examples/accountd_standin.rs owns `org.quire.Accounts1` on the private bus,
# drives sill's `org.quire.AccountsSheet1` (Open, Update, Close) with SheetView JSON from this
# directory's heredocs, and prints each `Input` the sheet sends back. Keys are `sill debug key
# SURFACE KEY` (shell-host's Inject::Key, which enters the sheet where an xkb key does).
#
# Nothing here reaches the person's accountd, session bus, secret store or ~/.config: the private
# bus is the daemon's session AND system bus, HOME and the XDG directories are scratch.
#
#   1. The sheet's surface exists, hidden, and a stand-in that does not own org.quire.Accounts1
#      cannot open one (refused, still hidden).
#   2. The provider list opens as a centred window (capture, dev/out/accounts-providers.png);
#      Escape sends {"kind":"dismiss"}.
#   3. The password flow: the sign-in form opens, "ada", Tab, "sill-fake", Return sends a Submit
#      carrying both; the secret is in no daemon log line and not in the surface's markup.
#   4. Working, Failed (Return is Try Again: a Retry), the browser wait and the device code each
#      show, and Escape dismisses each.
#   5. The consent question: Return is Allow Once (an Answer with account and scope), Escape a
#      dismissal; Close takes the sheet down.
#   6. 5 s idle, hidden: 0 frames on the sheet's surface.
#
# Usage: dev/accept/accounts.sh
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
OUT_DIR="$SILL_OUT_ROOT/accept-accounts"
SHOTS="$SILL_SHOTS"
mkdir -p "$OUT_DIR" "$SHOTS"
STANDIN_BIN="$SILL_OUT/examples/accountd_standin"
SECRET=sill-fake

scratch_new
mkdir -p "$CONFIG/sill" "$OUT_DIR/views"
STANDIN=
SHEET=

# The views accountd sends, as files the stand-in reads.
write_views() {
  local dir="$OUT_DIR/views"
  cat >"$dir/providers.json" <<'JSON'
{"kind":"providers","v":[{"id":"nextcloud","label":"Nextcloud","mark":"nextcloud"},
 {"id":"generic-imap","label":"Email (IMAP)","mark":"mail"}]}
JSON
  cat >"$dir/form.json" <<'JSON'
{"kind":"sign_in","v":{"provider":"nextcloud","fields":[
 {"kind":"address","entry":"plain","presence":"required","prefill":null},
 {"kind":"password","entry":"secret","presence":"required","prefill":null}],"problem":null}}
JSON
  cat >"$dir/working.json" <<'JSON'
{"kind":"working","v":"nextcloud"}
JSON
  cat >"$dir/failed.json" <<'JSON'
{"kind":"failed","v":{"provider":"nextcloud","fault":"unreachable"}}
JSON
  cat >"$dir/browser.json" <<'JSON'
{"kind":"browser_wait","v":{"provider":"google","url":"https://accounts.example.org/auth?c=1"}}
JSON
  cat >"$dir/code.json" <<'JSON'
{"kind":"show_code","v":{"provider":"microsoft","user_code":"ABCD-EFGH","url":"https://example.org/device"}}
JSON
  cat >"$dir/consent.json" <<'JSON'
{"kind":"consent","v":{"app":{"name":"org.quire.Photos","isolation":"flatpak"},
 "kind":"storage","class":"app_own","usage":"interactive","accounts":[
 {"account":"home","label":"ada@home.example","provider":"nextcloud"},
 {"account":"work","label":"ada@work.example","provider":"microsoft"}]}}
JSON
}

# A command to the stand-in.
standin() { echo "$*" >&3; }

# Wait up to ${2:-5} s for a line of the stand-in's log matching $1.
await_line() {
  for _ in $(seq $(( ${2:-5} * 10 ))); do
    grep -qE "$1" "$OUT_DIR/standin.log" && return 0
    sleep 0.1
  done
  return 1
}

start_standin() {
  mkfifo "$SCRATCH/standin.in"
  "$STANDIN_BIN" "$BUS" <"$SCRATCH/standin.in" >"$OUT_DIR/standin.log" 2>&1 &
  STANDIN=$!
  exec 3>"$SCRATCH/standin.in"
  await_line "^ready$" || { echo "the stand-in accountd never started" >&2; return 1; }
}

press() { sill debug key "$SHEET" "$1" >>"$OUT_DIR/keys.log" 2>&1; }
type_text() {
  local text="$1" i
  for (( i = 0; i < ${#text}; i++ )); do press "${text:i:1}" || return 1; done
}

captured() {
  sill debug capture "$SHEET" "$1" >>"$OUT_DIR/capture.log" 2>&1 || return 1
  [ "$(opaque_pixels "$1")" -gt 1000 ]
}
await_shown() {
  for _ in $(seq 50); do captured "$OUT_DIR/$1.png" && return 0; sleep 0.1; done
  echo "the sheet never showed ($1)" >&2
  return 1
}
await_hidden() {
  for _ in $(seq 30); do
    sill debug capture "$SHEET" "$OUT_DIR/late.png" >>"$OUT_DIR/capture.log" 2>&1 || return 0
    sleep 0.1
  done
  echo "the sheet never hid" >&2
  return 1
}

# Open view $2 as handle $1 and wait for it to show.
open_view() {
  standin "open $1 $OUT_DIR/views/$2.json"
  await_line "^opened $1$" || { echo "Open $1 was refused: $(tail -2 "$OUT_DIR/standin.log")" >&2; return 1; }
  await_shown "$1"
}

# Close handle $1 and wait for the sheet to go.
close_view() {
  standin "close $1"
  await_line "^closed $1$" || return 1
  await_hidden
}

# How many "input $1 ..." lines the stand-in has heard.
heard() { grep -c "^input $1 " "$OUT_DIR/standin.log"; }

# Wait for an "input $1" line matching $2.
await_input() {
  for _ in $(seq 50); do
    grep -E "^input $1 .*$2" "$OUT_DIR/standin.log" >/dev/null && return 0
    sleep 0.1
  done
  echo "no input for $1 matching $2: $(grep "^input $1" "$OUT_DIR/standin.log" | tail -2)" >&2
  return 1
}

cleanup() {
  stop_daemon
  exec 3>&- 2>/dev/null
  [ -n "$STANDIN" ] && kill "$STANDIN" 2>/dev/null
  stop_compositor
  scratch_drop
}
trap cleanup EXIT

# 1.
step_hidden_and_forged_open_refused() {
  sill debug stats | grep -q "\"$SHEET\"" || { echo "$SHEET is not in debug stats" >&2; return 1; }
  if sill debug capture "$SHEET" "$OUT_DIR/hidden.png" >>"$OUT_DIR/capture.log" 2>&1; then
    echo "$SHEET is mapped before any sheet" >&2; return 1
  fi
  standin "forge-open evil $OUT_DIR/views/providers.json"
  await_line "^forged evil refused$" || { echo "a forged Open was not refused: $(tail -2 "$OUT_DIR/standin.log")" >&2; return 1; }
  sleep 0.5
  if sill debug capture "$SHEET" "$OUT_DIR/forged.png" >>"$OUT_DIR/capture.log" 2>&1; then
    echo "a forged Open drew" >&2; return 1
  fi
  echo "$SHEET exists, hidden; a forged Open was refused and drew nothing" >&2
}

# 2.
step_providers_window_and_escape() {
  open_view p1 providers || return 1
  sleep 0.6
  captured "$OUT_DIR/providers.png" || return 1
  capture "$SHEET" "$SHOTS/accounts-providers.png" || echo "capture failed" >&2
  press Escape || return 1
  await_input p1 '"kind":"dismiss"' || return 1
  close_view p1 || return 1
  echo "the provider list showed as a window; Escape sent a dismissal" >&2
}

# 3.
step_password_flow() {
  open_view s1 form || return 1
  sleep 0.6
  type_text ada || return 1
  press Tab || return 1
  type_text "$SECRET" || return 1
  sleep 0.3
  captured "$OUT_DIR/typed.png" || return 1
  capture "$SHEET" "$SHOTS/accounts-sign-in.png" || echo "capture failed" >&2
  press Enter || return 1
  await_input s1 '"kind":"submit"' || return 1
  grep -E "^input s1 .*ada" "$OUT_DIR/standin.log" | grep -q "$SECRET" || {
    echo "the submit does not carry both answers" >&2; return 1; }
  if grep -rqs -- "$SECRET" "$OUT_DIR/daemon.log" "$OUT_DIR/keys.log" 2>/dev/null; then
    echo "the secret is in a log" >&2; return 1
  fi
  close_view s1 || return 1
  echo "typed an address and a password, Return sent one Submit; the secret is in no daemon log" >&2
}

# 4.
step_other_steps_and_cancel() {
  open_view w1 working || return 1
  press Escape || return 1
  await_input w1 '"kind":"dismiss"' || return 1
  close_view w1 || return 1
  open_view f1 failed || return 1
  sleep 0.5
  capture "$SHEET" "$SHOTS/accounts-failed.png" || echo "capture failed" >&2
  press Enter || return 1
  await_input f1 '"kind":"retry"' || return 1
  press Escape || return 1
  await_input f1 '"kind":"dismiss"' || return 1
  close_view f1 || return 1
  open_view b1 browser || return 1
  capture "$SHEET" "$SHOTS/accounts-browser.png" || echo "capture failed" >&2
  press Enter || return 1
  await_input b1 '"kind":"open_again"' || return 1
  press Escape || return 1
  await_input b1 '"kind":"dismiss"' || return 1
  close_view b1 || return 1
  open_view c1 code || return 1
  capture "$SHEET" "$SHOTS/accounts-code.png" || echo "capture failed" >&2
  press Escape || return 1
  await_input c1 '"kind":"dismiss"' || return 1
  close_view c1 || return 1
  echo "working, failed (Try Again), browser (Return sent Open Again) and code each showed and dismissed" >&2
}

# 5.
step_consent() {
  open_view k1 consent || return 1
  sleep 0.6
  capture "$SHEET" "$SHOTS/accounts-consent.png" || echo "capture failed" >&2
  press Enter || return 1
  await_input k1 '"kind":"answer".*"scope":"once"' || return 1
  press Escape || return 1
  await_input k1 '"kind":"dismiss"' || return 1
  close_view k1 || return 1
  echo "consent: Return answered Allow Once, Escape dismissed" >&2
}

# 6.
step_idle() {
  local f0 f1
  sleep 1
  f0=$(frames_of "$SHEET")
  sleep 5
  f1=$(frames_of "$SHEET")
  echo "5 s idle, hidden: $SHEET $(( f1 - f0 )) frames" >&2
  [ "$(( f1 - f0 ))" = 0 ]
}

build "-p sill-testkit --example accountd_standin" || exit 1
start_compositor --default kwin || { echo "the headless KWin did not start (see $OUT_DIR/kwin.log)" >&2; exit 1; }
echo "headless KWin at $WL, private bus $BUS" >&2
write_views
backdrop_seed "$CONFIG" || exit 1
: >"$CONFIG/sill/settings.toml"
start_standin || exit 1
launch_daemon || exit 1
NAME=$(output_name) || exit 1
SHEET="sill-accounts-sheet-$NAME"
sleep 1
run_steps step_hidden_and_forged_open_refused step_providers_window_and_escape step_password_flow \
  step_other_steps_and_cancel step_consent step_idle
status=$STATUS
echo "logs in $OUT_DIR; captures $SHOTS/accounts-*.png"
exit "$status"

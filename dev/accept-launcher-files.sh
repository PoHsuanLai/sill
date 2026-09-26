#!/usr/bin/env bash
# The launcher's Documents (M9 lane a, FINDINGS "The M9 fill-in"), on the nested cosmic-comp
# (dev/nested.sh; wayland-1 is pinged first, reused, and never stopped).
#
# Nothing here reads the person's files, index or settings, or opens anything in their apps.
# Everything runs in one scratch environment: a scratch HOME holding Documents/m9-report.pdf
# and notes/m9-notes.md, its own XDG_RUNTIME_DIR, XDG_CONFIG_HOME, XDG_DATA_HOME, XDG_STATE_HOME
# and XDG_CACHE_HOME, a private session bus (dev/private-bus.conf: nothing activatable), and
# XDG_CURRENT_DESKTOP=X-Generic so xdg-open asks xdg-mime, which reads the scratch
# mimeapps.list: every type opens with a stub .desktop whose Exec records its argument and the
# activation token. examples/file_manager_stub.rs holds org.freedesktop.FileManager1 on the
# private bus and records ShowItems. examples/files_probe.rs drives FilesProvider as the runner
# does and carries out the first row's activation through the real launch service.
#
#   1. `files_backend = "fd"`: the provider lists m9-report.pdf under ~/Documents for
#      "m9-rep" and for "M9-REP" (names match without case); the daemon, after `sill launcher
#      open --query m9-rep`, lists it under Documents in `sill debug launcher-rows` (PENDING
#      until lane d answers rows).
#   2. Enter opens it: the stub opener records the file's path and the token, bare and in a
#      transient scope.
#   3. ⌘R reveals it: the running stub file manager records ShowItems with its file:// URI and
#      the token; with no file manager on the bus, xdg-open opens the folder instead; with the
#      stub only activatable (a service file on the private bus), the call starts it.
#   4. `files_backend = "off"`: the provider lists nothing; the daemon, after `sill reload`,
#      has no Documents section (PENDING until lane d).
#   5. Detection on this machine (the real session bus: whether names exist, nothing more), and
#      Tracker end to end in localsearch's own sandbox (a private bus and a scratch index).
#
# Exit 0 when every step passed, 1 when one failed, 2 when none failed but a step waits on
# another lane.
#
# Usage: dev/nested.sh && dev/accept-launcher-files.sh
set -uo pipefail
# shellcheck source=dev/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
OUT_DIR="$SILL_REPO/target/accept-launcher-files"
mkdir -p "$OUT_DIR"
PROBE="$SILL_REPO/target/debug/examples/files_probe"
STUB_FM="$SILL_REPO/target/debug/examples/file_manager_stub"
TOKEN=sill-accept-files-token

SCRATCH=$(mktemp -d)
HOME_DIR="$SCRATCH/home"
RUNTIME="$SCRATCH/run"
CONFIG="$SCRATCH/config"
DATA="$SCRATCH/data"
OPENED="$SCRATCH/opened.log"
mkdir -p "$HOME_DIR/Documents" "$HOME_DIR/notes" "$RUNTIME" "$CONFIG/sill" "$DATA/applications" \
  "$SCRATCH/state" "$SCRATCH/cache"
chmod 700 "$RUNTIME"
: >"$OPENED"
BUS=
BUS_PID=
FM=
DAEMON=
FAILED=0
PENDING=0

# shellcheck disable=SC1091
source "$SHELL_HOST_DEV/.nested-env"
case "$WAYLAND_DISPLAY" in
  /*) WL="$WAYLAND_DISPLAY" ;;
  *) WL="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$WAYLAND_DISPLAY" ;;
esac

in_env() {
  env -u DISPLAY -u KDE_FULL_SESSION -u KDE_SESSION_VERSION -u DESKTOP_SESSION \
    -u XDG_SESSION_DESKTOP -u GNOME_DESKTOP_SESSION_ID -u BROWSER \
    HOME="$HOME_DIR" XDG_RUNTIME_DIR="$RUNTIME" XDG_CONFIG_HOME="$CONFIG" \
    XDG_DATA_HOME="$DATA" XDG_STATE_HOME="$SCRATCH/state" XDG_CACHE_HOME="$SCRATCH/cache" \
    XDG_CURRENT_DESKTOP=X-Generic WAYLAND_DISPLAY="$WL" \
    DBUS_SESSION_BUS_ADDRESS="$BUS" "$@"
}
sill() { in_env "$SILL_BIN" "$@"; }
probe() { in_env "$PROBE" --home "$HOME_DIR" "$@"; }

pass() { echo "step $1: PASS - $2"; }
fail() { echo "step $1: FAIL - $2"; FAILED=1; }
pending() { echo "step $1: PENDING - $2"; PENDING=1; }

build() {
  (cd "$SILL_REPO" && cargo build -p sill --features debug &&
    cargo build -p sill-services --example files_probe --example file_manager_stub) \
    >"$OUT_DIR/build.log" 2>&1 || { echo "build failed (see $OUT_DIR/build.log)" >&2; return 1; }
}

seed() {
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
  settings fd
}

settings() {
  printf '[launcher]\nfiles_backend = "%s"\n' "$1" >"$CONFIG/sill/settings.toml"
}

start_bus() {
  local out
  # dev/private-bus.conf plus one scratch service folder, empty until step 3 puts the stub file
  # manager there to check a reveal through an activatable (not running) file manager.
  mkdir -p "$SCRATCH/services"
  sed "s|</busconfig>|  <servicedir>$SCRATCH/services</servicedir>\n</busconfig>|" \
    "$SILL_REPO/dev/private-bus.conf" >"$SCRATCH/bus.conf"
  out=$(dbus-daemon --config-file="$SCRATCH/bus.conf" --fork \
    --print-address=1 --print-pid=1) || return 1
  BUS=$(sed -n 1p <<<"$out")
  BUS_PID=$(sed -n 2p <<<"$out")
  case "$BUS" in unix:*) ;; *) echo "no private bus address: $out" >&2; return 1 ;; esac
  [ "$BUS" != "${DBUS_SESSION_BUS_ADDRESS:-}" ] || { echo "not a private bus" >&2; return 1; }
}

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

start_daemon() {
  in_env "$SILL_BIN" run >"$OUT_DIR/daemon.log" 2>&1 &
  DAEMON=$!
  for _ in $(seq 100); do sill ping >/dev/null 2>&1 && return 0; sleep 0.1; done
  echo "the daemon never answered (see daemon.log)" >&2
  return 1
}

stop_daemon() {
  [ -n "$DAEMON" ] || return 0
  sill quit >/dev/null 2>&1
  wait "$DAEMON" 2>/dev/null
  DAEMON=
}

cleanup() {
  stop_daemon
  stop_fm
  [ -n "$BUS" ] && stop_activated_fm
  [ -n "$BUS_PID" ] && kill "$BUS_PID" 2>/dev/null
  rm -rf "$SCRATCH"
}
trap cleanup EXIT

# `sill debug launcher-rows` after opening the launcher on $1: "pending" while lane d has not
# filled it, else the rows.
daemon_rows() {
  sill launcher open --query "$1" >/dev/null 2>&1
  sleep 1
  local rows
  rows=$(sill debug launcher-rows 2>&1)
  sill launcher close >/dev/null 2>&1
  case "$rows" in *"not filled yet"*) echo pending ;; *) echo "$rows" ;; esac
}

# The count of lines of $2 matching $1.
count() { grep -c -- "$1" "$2" 2>/dev/null || true; }

step1() {
  probe --backend fd --query m9-rep >"$OUT_DIR/step1.txt" 2>&1
  if grep -qP '^row\tm9-report\.pdf\t~/Documents\tapplication/pdf$' "$OUT_DIR/step1.txt"; then
    pass 1 "the provider lists m9-report.pdf under ~/Documents"
  else
    fail 1 "the provider's rows: $(tr '\n' '|' <"$OUT_DIR/step1.txt")"
  fi
  probe --backend fd --query M9-REP >>"$OUT_DIR/step1.txt" 2>&1
  if [ "$(count $'^row\tm9-report\\.pdf\t' "$OUT_DIR/step1.txt")" = 2 ]; then
    pass 1 "names match without case: M9-REP lists m9-report.pdf too"
  else
    fail 1 "M9-REP: $(tr '\n' '|' <"$OUT_DIR/step1.txt")"
  fi
  local rows
  rows=$(daemon_rows m9-rep)
  case "$rows" in
    pending) pending 1 "the daemon's launcher-rows is not filled yet (M9 lane d)" ;;
    *Documents*m9-report.pdf*) pass 1 "the daemon lists m9-report.pdf under Documents" ;;
    *) fail 1 "the daemon's rows: $rows" ;;
  esac
}

step2() {
  probe --backend fd --query m9-rep --choose primary >"$OUT_DIR/step2.txt" 2>&1
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
  in_env env XDG_RUNTIME_DIR="$real_runtime" "$PROBE" --home "$HOME_DIR" --backend fd \
    --query m9-rep --choose primary >>"$OUT_DIR/step2.txt" 2>&1
  if grep -qxF "$want" "$OPENED" && grep -q 'app-sill-open' "$SCRATCH/scopes.log"; then
    pass 2 "Enter opened it inside a transient scope ($(head -1 "$SCRATCH/scopes.log"))"
  else
    fail 2 "scoped open recorded: $(tr '\n' '|' <"$OPENED") scope: $(tr '\n' '|' <"$SCRATCH/scopes.log")"
  fi
}

step3() {
  start_fm || { fail 3 "no stub file manager"; return; }
  probe --backend fd --query m9-rep --choose reveal >"$OUT_DIR/step3.txt" 2>&1
  local uri="file://$HOME_DIR/Documents/m9-report.pdf"
  if grep -qxF "ShowItems	$uri	$TOKEN" "$OUT_DIR/file-manager.log"; then
    pass 3 "⌘R revealed it through FileManager1.ShowItems with the token"
  else
    fail 3 "the stub file manager recorded: $(tr '\n' '|' <"$OUT_DIR/file-manager.log")"
  fi
  stop_fm
  : >"$OPENED"
  probe --backend fd --query m9-rep --choose reveal >>"$OUT_DIR/step3.txt" 2>&1
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
  probe --backend fd --query m9-rep --choose reveal >>"$OUT_DIR/step3.txt" 2>&1
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

step4() {
  probe --backend off --query m9-rep >"$OUT_DIR/step4.txt" 2>&1
  if [ "$(count $'^row\t' "$OUT_DIR/step4.txt")" = 0 ] && grep -qx 'rows 0' "$OUT_DIR/step4.txt"; then
    pass 4 "files_backend off: the provider lists nothing"
  else
    fail 4 "off still listed: $(tr '\n' '|' <"$OUT_DIR/step4.txt")"
  fi
  settings off
  sill reload >/dev/null 2>&1
  local rows
  rows=$(daemon_rows m9-rep)
  case "$rows" in
    pending) pending 4 "the daemon's launcher-rows is not filled yet (M9 lane d)" ;;
    *Documents*) fail 4 "the daemon still has a Documents section: $rows" ;;
    *) pass 4 "after reload the daemon has no Documents section" ;;
  esac
}

step5() {
  (cd "$SILL_REPO" && cargo test -p sill-services --test files_walk_live -- --ignored \
    --nocapture this_machine_has_tracker) >"$OUT_DIR/step5-detect.txt" 2>&1
  local detected
  detected=$(grep -o 'sill: files detect: .*' "$OUT_DIR/step5-detect.txt")
  if grep -q 'test result: ok. 1 passed' "$OUT_DIR/step5-detect.txt"; then
    pass 5 "${detected:-detect ran}"
  else
    fail 5 "detect: ${detected:-see $OUT_DIR/step5-detect.txt}"
  fi
  if ! command -v localsearch >/dev/null; then
    pending 5 "no localsearch here: Tracker's sandbox test was not run"
    return
  fi
  (cd "$SILL_REPO" && cargo test -p sill-services --test files_tracker_live -- --ignored \
    --nocapture) >"$OUT_DIR/step5-tracker.txt" 2>&1
  if grep -q 'test result: ok. 1 passed' "$OUT_DIR/step5-tracker.txt"; then
    pass 5 "Tracker found m9-report.pdf and 'm9 dir' in localsearch's sandbox"
  else
    fail 5 "Tracker's sandbox test (see $OUT_DIR/step5-tracker.txt)"
  fi
}

"$SHELL_HOST_DEV/run.sh" wayland-info >/dev/null 2>&1 ||
  { echo "no nested compositor (dev/nested.sh first)" >&2; exit 1; }
build || exit 1
start_bus || exit 1
seed
start_daemon || exit 1
step1
step2
step3
step4
stop_daemon
step5

if [ "$FAILED" = 1 ]; then exit 1; fi
if [ "$PENDING" = 1 ]; then exit 2; fi
exit 0

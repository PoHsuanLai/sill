#!/usr/bin/env bash
# The real dist/sill-session, run on a nested cosmic-comp (a window on the host session) with
# SILL_SESSION_NESTED=1, then checked: the environment the components get, the config it seeds,
# session.env, and `sill workspace` over the daemon's IPC. It never starts a login, never
# installs anything and never reaches the host session.
#
# SAFETY.
# - SILL_SESSION_NESTED=1 makes the script skip everything that reaches the host's systemd,
#   logind or wallet (the systemd import, sill-session.target, the power-key inhibitor, ksecretd).
# - One private dbus-daemon is both the session and the system bus of everything started;
#   HOME, XDG_RUNTIME_DIR, XDG_CONFIG_HOME, XDG_DATA_HOME, XDG_STATE_HOME and XDG_CACHE_HOME are
#   scratch; the process runs under `env -i`, so no host variable leaks in (the host's Wayland
#   socket is the only host thing it is given, as an absolute path).
# - `sill` is a stub on PATH that records its environment and runs the debug build with
#   --fake-ddc and no idle times, and points `session detect-gpu` at a made-up sysfs (a hybrid
#   desktop: the monitor on the second of two GPUs); the seed files come from this checkout
#   (SILL_SESSION_DATA).
# Two runs: the detected GPU, then a COSMIC_RENDER_DEVICE the person set in session.env.
# - Only the processes this script started are killed (the compositor's process group).
#
# Needs: target/debug/sill built with `--features debug`, cosmic-comp, a running Wayland session.
# Exit 0 when every check passed, 1 on a failure.
#
# Usage: dev/session-smoke.sh
set -uo pipefail
# shellcheck source=dev/lib/all.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/all.sh"

FAILED=0
OUT_DIR=
COMP_PID=
check() { # check NAME COMMAND...
  local name="$1"; shift
  if "$@"; then echo "PASS $name"; else echo "FAIL $name"; FAILED=1; fi
}
check_not() { ! "$@"; }
no_update_error() { ! grep -q "could not update" "$OUT_DIR/session.log"; }
in_probe() { grep -qxF -- "$1" "$PROBE"; }
not_in_probe() { ! grep -q -- "^$1=" "$PROBE"; }

HOST_WL="${WAYLAND_DISPLAY:-wayland-0}"
case "$HOST_WL" in /*) ;; *) HOST_WL="${XDG_RUNTIME_DIR:?needs a host session}/$HOST_WL" ;; esac
[ -S "$HOST_WL" ] || { echo "no host Wayland socket at $HOST_WL" >&2; exit 1; }
REAL_SILL="${SILL_BIN:-$SILL_OUT/sill}"
[ -x "$REAL_SILL" ] || { echo "build first: cargo build -p sill --features debug ($REAL_SILL)" >&2; exit 1; }


# A made-up sysfs: card1 on 0000:10:00.0 with a connected output, card2 on 0000:01:00.0 without.
make_sysfs() {
  local root="$1" card addr render
  for row in "card1 0000:10:00.0 renderD128" "card2 0000:01:00.0 renderD129"; do
    read -r card addr render <<<"$row"
    mkdir -p "$root/devices/$addr/drm/$render" "$root/devices/$addr/drm/$card" "$root/class/drm/$card"
    ln -s "$root/devices/$addr" "$root/class/drm/$card/device"
  done
  mkdir -p "$root/class/drm/card1-HDMI-A-1" "$root/class/drm/card2-DP-1"
  echo connected >"$root/class/drm/card1-HDMI-A-1/status"
  echo disconnected >"$root/class/drm/card2-DP-1/status"
}

# scenario NAME EXPECTED_RENDER_DEVICE [SESSION_ENV_LINE]: one session start and its checks.
scenario() {
  local name="$1" want_device="$2" session_env="${3:-}"
  echo "== $name"
  scratch_new
  start_private_bus || exit 1
  OUT_DIR="$SCRATCH/out"; mkdir -p "$OUT_DIR"
  PROBE="$SCRATCH/sill-env.txt"
  COMP_PID=
  local bin="$SCRATCH/bin" sysfs="$SCRATCH/sysfs"
  mkdir -p "$bin"
  make_sysfs "$sysfs"
  use_fake_ddc "$CONFIG" >/dev/null
  cat >"$bin/sill" <<STUB
#!/bin/sh
case "\${1:-} \${2:-}" in
  "session detect-gpu") exec "$REAL_SILL" session detect-gpu --sysfs "$sysfs" ;;
  "run "*)
    shift
    env >"$PROBE"
    exec "$REAL_SILL" run --fake-ddc "$FAKE_DDC" --idle-times 0,0 "\$@" ;;
esac
exec "$REAL_SILL" "\$@"
STUB
  chmod +x "$bin/sill"

  # What a greeter might leave behind, and a user's own seed: the first must go, the second stay.
  local seeded="$CONFIG/cosmic/com.system76.CosmicSettings.Shortcuts/v1/custom"
  mkdir -p "$(dirname "$seeded")" "$CONFIG/sill"
  echo '{ /* mine */ }' >"$seeded"
  printf '%s\n' 'COSMIC_DISABLE_SYNCOBJ=1' "$session_env" >"$CONFIG/sill/session.env"

  env -i PATH="$bin:/usr/local/bin:/usr/bin:/bin" HOME="$HOME_DIR" XDG_RUNTIME_DIR="$RUNTIME" \
    XDG_CONFIG_HOME="$CONFIG" XDG_DATA_HOME="$DATA" XDG_STATE_HOME="$STATE" XDG_CACHE_HOME="$CACHE" \
    DBUS_SESSION_BUS_ADDRESS="$BUS" DBUS_SYSTEM_BUS_ADDRESS="$BUS" WAYLAND_DISPLAY="$HOST_WL" \
    COSMIC_BACKEND=winit GTK_IM_MODULE=ibus QT_IM_MODULE=ibus \
    __EGL_VENDOR_LIBRARY_FILENAMES=/usr/share/glvnd/egl_vendor.d/50_mesa.json DRI_PRIME=1 \
    __GL_SYNC_TO_VBLANK=0 vblank_mode=0 \
    SILL_SESSION_NESTED=1 SILL_SESSION_DATA="$SILL_REPO/dist" \
    setsid "$SILL_REPO/dist/sill-session" >"$OUT_DIR/session.log" 2>&1 &
  COMP_PID=$!

  # The daemon answers ping once the session script has got as far as `sill run`.
  WL=
  for _ in $(seq 150); do
    [ -z "$WL" ] && for s in "$RUNTIME"/wayland-*; do [ -S "$s" ] && WL="$s"; done
    [ -n "$WL" ] && in_env "$REAL_SILL" ping >/dev/null 2>&1 && break
    sleep 0.2
  done
  if ! in_env "$REAL_SILL" ping >/dev/null 2>&1; then
    echo "the session never brought sill up; log:" >&2; cat "$OUT_DIR/session.log" >&2
    FAILED=1; end_scenario; return
  fi

  check "env: desktop names" in_probe XDG_CURRENT_DESKTOP=COSMIC
  check "env: session desktop" in_probe XDG_SESSION_DESKTOP=COSMIC
  check "env: session type" in_probe XDG_SESSION_TYPE=wayland
  if command -v fcitx5 >/dev/null; then
    check "env: XMODIFIERS for fcitx5" in_probe 'XMODIFIERS=@im=fcitx'
    check "env: GTK_IM_MODULE unset" not_in_probe GTK_IM_MODULE
    check "env: QT_IM_MODULE unset" not_in_probe QT_IM_MODULE
  fi
  check "env: render device ($want_device)" in_probe "COSMIC_RENDER_DEVICE=$want_device"
  check "env: session.env applies" in_probe COSMIC_DISABLE_SYNCOBJ=1
  check "env: the compositor gave WAYLAND_DISPLAY" grep -q '^WAYLAND_DISPLAY=' "$PROBE"
  echo "info: DISPLAY (Xwayland) = $(sed -n 's/^DISPLAY=//p' "$PROBE")"
  check "seed: key repeat 400/10" grep -q 'repeat_delay: 400, repeat_rate: 10' \
    "$CONFIG/cosmic/com.system76.CosmicComp/v1/xkb_config"
  check "seed: shortcuts file copied" test -s \
    "$CONFIG/cosmic/com.system76.CosmicSettings.Shortcuts/v1/system_actions"
  check "seed: an existing file is kept" grep -q mine "$seeded"
  check "activation environment imported" no_update_error
  check "journal: the GPU choice is logged" grep -q 'render device' "$OUT_DIR/session.log"
  check "workspace: overview is unsupported" check_not in_env "$REAL_SILL" workspace overview
  check "workspace: next answers" in_env "$REAL_SILL" workspace next
  check "workspace: prev answers" in_env "$REAL_SILL" workspace prev
  end_scenario
}

end_scenario() {
  [ -z "$COMP_PID" ] || kill -- "-$COMP_PID" 2>/dev/null
  sleep 1
  [ -z "$COMP_PID" ] || kill -9 -- "-$COMP_PID" 2>/dev/null
  if [ "$FAILED" != 0 ] && [ -d "$OUT_DIR" ]; then
    mkdir -p "$SILL_TARGET" && rm -rf "$SILL_TARGET/session-smoke-out" &&
      cp -r "$OUT_DIR" "$SILL_TARGET/session-smoke-out"
  fi
  COMP_PID=
  stop_private_bus
  scratch_drop
}
trap end_scenario EXIT

scenario "the GPU the monitor is on is detected" "pci-0000:10:00.0" ""
scenario "a COSMIC_RENDER_DEVICE in session.env wins" "renderD129" "COSMIC_RENDER_DEVICE=renderD129"

if [ "$FAILED" = 0 ]; then
  echo "SESSION SMOKE GREEN"
else
  echo "SESSION SMOKE FAILED (logs in target/session-smoke-out)"
fi
exit "$FAILED"

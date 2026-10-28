#!/usr/bin/env bash
# Crate boundaries, mechanically enforced.
#
# `cargo tree -i <dep>` exits 101 when the dependency is absent, which is precisely the state
# we want. Checking the exit status would therefore fail whenever the boundary holds, so we
# check for OUTPUT instead: any line naming the dependency is a leak.
#
# Each rule is "<crate>: <forbidden deps...>". Forbidden names are exact package names; a
# family such as blitz-* is spelled out member by member.
set -uo pipefail
cd "$(dirname "$0")/.."

# ARCHITECTURE.md section 1 is the crate map; this file is its mechanical form, in two parts.
#
# RULES: what a crate reaches through ANY path (`cargo tree -i`, transitive).
# sill-launcher, sill-model and sill-ipc are pure: no UI, no D-Bus, no Wayland, no runtime, no
# audio, no design system. sill-settings is data and UI-free, but it names ds-settings (its dioxus
# feature off), whose file I/O brings zbus and tokio, so only the UI is forbidden to it.
# sill-services and sill-search are effectful but UI-free: their state reaches surfaces only
# through watch channels read by sill-ui-kit's hooks, so neither links a renderer. sill-services
# names ds-settings (its dioxus feature off) for Night Shift's SettingsWriter edits.
# sill-testkit is std only and names no sill crate.
# sill-shell-kit (the shell-only parts over quire's `ds`) names no other sill crate and no service,
# bus, runtime or compositor client: it is what the surfaces and sill-ui-kit draw with.
# sill-ui-kit is what every surface shares, so it reaches no surface crate and not the binary.
# `sill` (the whole process) takes the viewer's light tier, its history, its vocabulary and its
# D-Bus requests, and never its media player: it reaches neither libmpv binding.
# The surface crates never depend on one another and none depends on `sill`; the daemon is the one
# crate that names them all.
PURE="dioxus dioxus-core zbus wayland-client tokio libpulse-binding shell-host ds ds-shell sill-shell-kit ds-blitz ds-harness blitz-dom"
UI_FREE="dioxus dioxus-core blitz-dom shell-host ds ds-shell sill-shell-kit ds-blitz ds-harness"
SURFACES=(sill-overlays sill-notify-ui sill-dock sill-launcher-ui sill-bar)
# The other surfaces, and the binary, for one surface crate.
others() {
  local self="$1" c
  for c in "${SURFACES[@]}" sill; do [ "$c" = "$self" ] || printf '%s ' "$c"; done
}
RULES=(
  "sill-launcher: $PURE"
  "sill-model: $PURE"
  "sill-settings: $UI_FREE"
  "sill-ipc: $PURE"
  "sill-services: $UI_FREE"
  "sill-search: $UI_FREE"
  "sill-shell-kit: sill-launcher sill-model sill-settings sill-ipc sill-services sill-search sill-testkit sill-ui-kit ${SURFACES[*]} sill zbus tokio wayland-client libpulse-binding shell-host ds-blitz blitz-dom"
  "sill-testkit: sill-launcher sill-model sill-settings sill-ipc sill-services sill-search sill-shell-kit sill-ui-kit ${SURFACES[*]} sill"
  "sill-ui-kit: ${SURFACES[*]} sill"
  "sill: mpv-wgpu-player rsmpv"
)
for surface in "${SURFACES[@]}"; do
  RULES+=("$surface: $(others "$surface")")
done
fail=0

for rule in "${RULES[@]}"; do
  crate="${rule%%:*}"
  read -r -a forbidden <<<"${rule#*:}"
  # A crate that cargo cannot find would make every check below pass vacuously.
  if ! cargo tree -p "$crate" --depth 0 >/dev/null 2>&1; then
    echo "ERROR: cargo tree cannot resolve $crate; the boundary was not checked"
    fail=1
    continue
  fi
  leaked=0
  for dep in "${forbidden[@]}"; do
    if cargo tree -p "$crate" -i "$dep" -e normal,build 2>/dev/null | grep -q .; then
      echo "LEAK: $crate depends on $dep"
      cargo tree -p "$crate" -i "$dep" -e normal,build 2>/dev/null | head -20
      leaked=1
      fail=1
    fi
  done
  if [ "$leaked" -eq 0 ]; then
    echo "boundary holds: $crate reaches none of ${forbidden[*]}"
  fi
done

# The allowed edges between our own crates and quire and shell-host (ARCHITECTURE.md section 1):
# each crate's DIRECT normal and build dependencies that are path dependencies, and nothing else.
# A dependency on a crate not listed is a leak; so is one the crate no longer has, so the table
# stays exact. Dev dependencies are outside it (a test may reach further than the crate does).
EDGES=(
  "sill-launcher: almanac-core docket-core docket-vocab ds-core prov"
  "sill-testkit:"
  "sill-shell-kit: ds ds-core ds-shell"
  "sill-model: almanac-core casement-protocols companion-model companion-wire cua-action cua-bus docket-core docket-manifest docket-permissions-core docket-vocab docket-wire ds-core ds-helpers keycap-protocol porter-capability porter-core porter-ids porter-infer porter-sheet prov sill-launcher voice-wire"
  "sill-settings: ds-core ds-settings keycap-protocol sill-launcher sill-model"
  "sill-ipc: ds-core sill-launcher sill-model"
  "sill-services: almanac-core almanac-dbus anyview-core anyview-platform casement-protocols companion-model companion-wire cua-action cua-bus cua-dbus desktop-client docket-client docket-core docket-dbus docket-manifest docket-permissions-core docket-provider docket-vocab docket-wire ds-core ds-helpers ds-settings keycap-protocol porter-client porter-core porter-dbus porter-ids porter-infer porter-sheet porter-tailnet porter-tailscale porter-wire prov sill-launcher sill-model voice-dbus voice-wire"
  "sill-search: almanac-core anyview-core anyview-store companion-model companion-wire cua-action docket-core docket-manifest docket-permissions-core docket-vocab ds-core ds-settings prov sill-launcher sill-model sill-services"
  "sill-ui-kit: companion-wire cua-action docket-core docket-manifest docket-permissions-core docket-vocab ds ds-behaviour ds-blitz ds-core ds-harness ds-settings keycap-client porter-capability porter-core prov shell-host sill-launcher sill-model sill-search sill-services sill-settings sill-shell-kit"
  "sill-bar: ds ds-blitz ds-settings shell-host sill-launcher sill-model sill-services sill-settings sill-shell-kit sill-ui-kit"
  "sill-dock: ds ds-blitz shell-host sill-launcher sill-model sill-services sill-settings sill-shell-kit sill-ui-kit"
  "sill-launcher-ui: almanac-core anyview-core anyview-fs anyview-image anyview-peek anyview-platform companion-model companion-wire docket-core docket-permissions-core docket-vocab ds ds-behaviour ds-blitz prov shell-host sill-launcher sill-model sill-search sill-services sill-settings sill-shell-kit sill-ui-kit"
  "sill-notify-ui: ds ds-blitz ds-settings shell-host sill-model sill-services sill-settings sill-shell-kit sill-ui-kit"
  "sill-overlays: ds ds-behaviour ds-settings ds-shell porter-capability porter-core porter-ids porter-sheet shell-host sill-launcher sill-model sill-services sill-settings sill-shell-kit sill-ui-kit"
  "sill: ds ds-blitz ds-core ds-extensions ds-lint ds-settings prov shell-host shell-verbs sill-bar sill-dock sill-ipc sill-launcher sill-launcher-ui sill-model sill-notify-ui sill-overlays sill-search sill-services sill-settings sill-shell-kit sill-ui-kit"
)
for edge in "${EDGES[@]}"; do
  crate="${edge%%:*}"
  read -r -a allowed <<<"${edge#*:}"
  found=$(cargo tree -p "$crate" --depth 1 -e normal,build --prefix none --all-features 2>/dev/null \
    | grep '(/' | awk '{print $1}' | grep -vx "$crate" | sort -u | tr '\n' ' ')
  want=$(printf '%s\n' "${allowed[@]}" | grep . | sort -u | tr '\n' ' ')
  if [ "$found" != "$want" ]; then
    echo "EDGE: $crate depends on [${found% }], the table allows [${want% }]"
    fail=1
  else
    echo "edges hold: $crate depends on [${found% }]"
  fi
done

# Every workspace member has a row above, so a new crate cannot slip in unchecked.
for member in $(cargo metadata --no-deps --format-version 1 2>/dev/null | grep -o '"name":"sill[a-z-]*"' | cut -d'"' -f4 | sort -u); do
  printf '%s\n' "${EDGES[@]}" | grep -q "^$member:" || { echo "ERROR: $member has no row in EDGES"; fail=1; }
done

exit "$fail"

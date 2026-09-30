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
# through watch channels read by sill-ui-kit's hooks, so neither links a renderer.
# sill-testkit is std only and names no sill crate.
# sill-ui-kit is what every surface shares, so it reaches no surface crate and not the binary.
# The surface crates never depend on one another and none depends on `sill`; the daemon is the one
# crate that names them all.
PURE="dioxus dioxus-core zbus wayland-client tokio libpulse-binding shell-host ds ds-shell ds-blitz ds-harness blitz-dom"
UI_FREE="dioxus dioxus-core blitz-dom shell-host ds ds-shell ds-blitz ds-harness"
SURFACES=(sill-overlays sill-notify-ui sill-widgets-ui sill-dock sill-launcher-ui sill-bar)
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
  "sill-services: $UI_FREE ds-settings"
  "sill-search: $UI_FREE"
  "sill-testkit: sill-launcher sill-model sill-settings sill-ipc sill-services sill-search sill-ui-kit ${SURFACES[*]} sill"
  "sill-ui-kit: ${SURFACES[*]} sill"
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
  "sill-launcher:"
  "sill-testkit:"
  "sill-model: sill-launcher"
  "sill-settings: ds-core ds-settings sill-model"
  "sill-ipc: sill-launcher sill-model"
  "sill-services: sill-launcher sill-model"
  "sill-search: ds-settings sill-launcher sill-model sill-services"
  "sill-ui-kit: ds ds-blitz ds-harness ds-settings ds-shell shell-host sill-launcher sill-model sill-search sill-services sill-settings"
  "sill-bar: ds ds-blitz ds-settings ds-shell shell-host sill-launcher sill-model sill-services sill-settings sill-ui-kit"
  "sill-dock: ds ds-blitz ds-shell shell-host sill-launcher sill-model sill-services sill-settings sill-ui-kit"
  "sill-launcher-ui: ds ds-blitz shell-host sill-launcher sill-model sill-search sill-services sill-settings sill-ui-kit"
  "sill-notify-ui: ds ds-blitz ds-settings ds-shell shell-host sill-model sill-services sill-settings sill-ui-kit"
  "sill-widgets-ui: ds ds-shell shell-host sill-settings sill-ui-kit"
  "sill-overlays: ds ds-settings ds-shell shell-host sill-launcher sill-model sill-services sill-settings sill-ui-kit"
  "sill: ds-lint ds-blitz ds-settings ds-shell shell-host sill-bar sill-dock sill-ipc sill-launcher sill-launcher-ui sill-model sill-notify-ui sill-overlays sill-search sill-services sill-settings sill-ui-kit sill-widgets-ui"
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

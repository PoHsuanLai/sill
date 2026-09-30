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

# sill-ipc, sill-launcher and sill-model are pure: no UI, no D-Bus, no Wayland, no runtime, no
# audio, no design system.
# sill-services and sill-search are effectful but UI-free: their state reaches surfaces only
# through watch channels read by sill-ui-kit's hooks (DESIGN.md "Crates"), so neither links a
# renderer. sill-settings is data and reaches neither ds nor dioxus: it names ds-core for `Word`
# and ds-settings (its dioxus feature off) for the schema; sill-search reads schema files through
# ds-settings the same way.
RULES=(
  "sill-ipc: dioxus dioxus-core zbus wayland-client tokio libpulse-binding"
  "sill-launcher: dioxus dioxus-core zbus wayland-client tokio libpulse-binding"
  "sill-model: dioxus dioxus-core zbus wayland-client tokio libpulse-binding shell-host ds blitz-dom"
  "sill-settings: dioxus dioxus-core blitz-dom shell-host ds ds-shell"
  "sill-services: dioxus dioxus-core blitz-dom shell-host ds ds-settings"
  "sill-search: dioxus dioxus-core blitz-dom shell-host ds ds-shell"
)
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

exit "$fail"

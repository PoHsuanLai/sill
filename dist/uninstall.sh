#!/usr/bin/env bash
# Remove what dist/install.sh put in place: the binary, the PAM service, the intents manifest, the
# skills and the settings schema. Files an older install put in place (the session script, the
# login-screen entry, the user target and the seed files) are removed too. Plasma, Toshy, keycap's
# system files and your own config are not touched. Run as yourself; sudo asks once.
#
#   dist/uninstall.sh --dry-run        print every step, change nothing
#   dist/uninstall.sh                  remove the system files
#   dist/uninstall.sh --forget-seeds   also remove the keycap layer in ~/.config/keycap and the
#                                      seeded files an older install copied into your config, but
#                                      only those still identical to what was installed (files
#                                      you never changed)
set -euo pipefail
# shellcheck source=install-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/install-lib.sh"

usage() { sed -n '2,12p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
export EXTRA_FLAGS="--forget-seeds"
FLAG_forget_seeds=0
parse_flags "$@"
[ "$DRY" = 0 ] || echo "dry run: nothing below is executed"

# Legacy paths of earlier installs (the old COSMIC (sill) login), cleaned up so they do not linger.
LEGACY_FILES=(
  /usr/local/bin/sill-session
  /usr/share/wayland-sessions/sill.desktop
  /etc/systemd/user/sill-session.target
)
LEGACY_SEED_DIR=/usr/local/share/sill
LEGACY_USER_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic"

step "1/3 checks"
refuse_root
need_sudo

step "2/3 system files"
for path in "$PAM_DEST" "$INTENTS_DEST" "$SCHEMA_DEST" "$BIN_DEST" "${LEGACY_FILES[@]}"; do
  if [ -e "$path" ]; then run sudo rm -f "$path"; else note "already gone: $path"; fi
done
if [ "${FLAG_forget_seeds:-0}" = 1 ]; then
  step "2b the seeded user files that still match"
  if [ -d "$LEGACY_SEED_DIR/cosmic" ]; then
    while IFS= read -r file; do
      mine="$LEGACY_USER_DIR/$file"
      if [ -f "$mine" ] && cmp -s "$mine" "$LEGACY_SEED_DIR/cosmic/$file"; then
        run rm -f "$mine"
      elif [ -f "$mine" ]; then
        note "kept (you changed it): $mine"
      fi
    done < <(cd "$LEGACY_SEED_DIR/cosmic" && find . -type f | sed 's|^\./||' | sort)
  fi
  if [ -f "$KEYCAP_LAYER_DEST" ] && cmp -s "$KEYCAP_LAYER_DEST" "$KEYCAP_LAYER_SRC"; then
    run rm -f "$KEYCAP_LAYER_DEST"
  elif [ -f "$KEYCAP_LAYER_DEST" ]; then
    note "kept (you changed it): $KEYCAP_LAYER_DEST"
  fi
fi
while IFS= read -r file; do
  if [ -e "$SKILLS_DEST/$file" ]; then run sudo rm -f "$SKILLS_DEST/$file"; else note "already gone: $SKILLS_DEST/$file"; fi
done < <(skills)
for id in $(cd "$DIST/skills" && ls); do
  [ ! -d "$SKILLS_DEST/$id" ] || run sudo rmdir --ignore-fail-on-non-empty "$SKILLS_DEST/$id"
done
if [ -d "$LEGACY_SEED_DIR" ]; then run sudo rm -rf "$LEGACY_SEED_DIR"; else note "already gone: $LEGACY_SEED_DIR"; fi

step "3/3 the user manager"
run systemctl --user daemon-reload
note "done. Log out; pick Plasma at the login screen."

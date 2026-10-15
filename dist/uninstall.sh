#!/usr/bin/env bash
# Remove what dist/install.sh put in place: the binary, the PAM service, the intents manifest, the
# skills and the settings schema. Files an older install put in place (the session script, the
# login-screen entry, the user target and the seed directory) are removed too. Plasma, Toshy, keycap's
# system files and your own config are not touched. Run as yourself; the root step asks once (root
# is $SUDO, else sudo, doas or pkexec; with none the commands are printed for you to run).
#
#   dist/uninstall.sh --dry-run        print every step, change nothing
#   dist/uninstall.sh                  remove the system files
#   dist/uninstall.sh --forget-seeds   also remove the keycap layer in ~/.config/keycap, but only
#                                      while it is still identical to the seed (a file you never
#                                      changed)
#   --prefix DIR, --destdir DIR        the same as install.sh's
set -euo pipefail
# shellcheck source=install-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/install-lib.sh"

usage() { awk 'NR > 1 { if (!/^#/) exit; sub(/^# ?/, ""); print }' "${BASH_SOURCE[0]}"; }
export EXTRA_FLAGS="--forget-seeds"
FLAG_forget_seeds=0
parse_flags "$@"
[ "$DRY" = 0 ] || echo "dry run: nothing below is executed"

# Paths of earlier installs, cleaned up so they do not linger.
LEGACY_FILES=(
  /usr/local/bin/sill-session
  /usr/share/wayland-sessions/sill.desktop
  /etc/systemd/user/sill-session.target
)
LEGACY_SEED_DIR=/usr/local/share/sill

step "1/3 checks"
refuse_root
need_root_helper

step "2/3 system files"
for path in "$PAM_DEST" "$INTENTS_DEST" "$SCHEMA_DEST" "$BIN_DEST" "${LEGACY_FILES[@]}"; do
  if [ -e "$DESTDIR$path" ]; then as_root rm -f "$DESTDIR$path"; else note "already gone: $DESTDIR$path"; fi
done
# The DDC/CI rule and module load, only while they are still the copies install.sh made.
removed_rule=0
for pair in "$DDC_RULE_SRC $DDC_RULE_DEST" "$DDC_MODULES_SRC $DDC_MODULES_DEST"; do
  read -r src dest <<<"$pair"
  if [ -f "$DESTDIR$dest" ] && cmp -s "$src" "$DESTDIR$dest"; then
    as_root rm -f "$DESTDIR$dest"
    [ "$dest" != "$DDC_RULE_DEST" ] || removed_rule=1
  elif [ -e "$DESTDIR$dest" ]; then
    note "kept (not ours or changed): $DESTDIR$dest"
  fi
done
[ "$removed_rule" = 0 ] || [ -n "$DESTDIR" ] || as_root udevadm control --reload-rules
if [ "${FLAG_forget_seeds:-0}" = 1 ]; then
  step "2b the keycap layer, if it still matches"
  if [ -f "$KEYCAP_LAYER_DEST" ] && cmp -s "$KEYCAP_LAYER_DEST" "$KEYCAP_LAYER_SRC"; then
    run rm -f "$KEYCAP_LAYER_DEST"
  elif [ -f "$KEYCAP_LAYER_DEST" ]; then
    note "kept (you changed it): $KEYCAP_LAYER_DEST"
  fi
fi
while IFS= read -r file; do
  if [ -e "$DESTDIR$SKILLS_DEST/$file" ]; then as_root rm -f "$DESTDIR$SKILLS_DEST/$file"; else note "already gone: $DESTDIR$SKILLS_DEST/$file"; fi
done < <(skills)
for id in $(cd "$DIST/skills" && ls); do
  [ ! -d "$DESTDIR$SKILLS_DEST/$id" ] || as_root rmdir --ignore-fail-on-non-empty "$DESTDIR$SKILLS_DEST/$id"
done
if [ -d "$DESTDIR$LEGACY_SEED_DIR" ]; then as_root rm -rf "$DESTDIR$LEGACY_SEED_DIR"; else note "already gone: $DESTDIR$LEGACY_SEED_DIR"; fi

step "3/3 the user manager"
if [ -n "$DESTDIR" ]; then note "skipped: a staging root"; else run systemctl --user daemon-reload; fi
print_pending
note "done. Log out; pick Plasma at the login screen."

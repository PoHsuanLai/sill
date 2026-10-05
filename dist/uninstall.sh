#!/usr/bin/env bash
# Remove what dist/install.sh put in place: the binary, the session script, the login-screen
# entry, the user target, the PAM service, the intents manifest and the seed files. Plasma, Toshy,
# keycap's system files and your own config are not touched. Run as yourself; sudo asks once.
#
#   dist/uninstall.sh --dry-run        print every step, change nothing
#   dist/uninstall.sh                  remove the system files
#   dist/uninstall.sh --forget-seeds   also remove the files sill-session copied into
#                                      ~/.config/cosmic and the keycap layer in
#                                      ~/.config/keycap, but only those still identical to the
#                                      seeds (files you never changed)
set -euo pipefail
# shellcheck source=install-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/install-lib.sh"

usage() { sed -n '2,11p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
export EXTRA_FLAGS="--forget-seeds"
FLAG_forget_seeds=0
parse_flags "$@"
[ "$DRY" = 0 ] || echo "dry run: nothing below is executed"

step "1/3 checks"
refuse_root
need_sudo

step "2/3 system files"
for path in "$DESKTOP_DEST" "$SESSION_DEST" "$TARGET_DEST" "$PAM_DEST" "$INTENTS_DEST" "$BIN_DEST"; do
  if [ -e "$path" ]; then run sudo rm -f "$path"; else note "already gone: $path"; fi
done
if [ "${FLAG_forget_seeds:-0}" = 1 ]; then
  step "2b the seeded user files that still match"
  while IFS= read -r file; do
    mine="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic/$file"
    if [ -f "$mine" ] && cmp -s "$mine" "$DIST/cosmic/$file"; then
      run rm -f "$mine"
    elif [ -f "$mine" ]; then
      note "kept (you changed it): $mine"
    fi
  done < <(seeds)
  if [ -f "$KEYCAP_LAYER_DEST" ] && cmp -s "$KEYCAP_LAYER_DEST" "$KEYCAP_LAYER_SRC"; then
    run rm -f "$KEYCAP_LAYER_DEST"
  elif [ -f "$KEYCAP_LAYER_DEST" ]; then
    note "kept (you changed it): $KEYCAP_LAYER_DEST"
  fi
fi
if [ -d "$SEED_DEST" ]; then run sudo rm -rf "$SEED_DEST"; else note "already gone: $SEED_DEST"; fi

step "3/3 the user manager"
run systemctl --user daemon-reload
note "done. Log out; pick Plasma at the login screen."

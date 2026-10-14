#!/usr/bin/env bash
# Install sill from this checkout: build `sill` in release, then put the binary, the PAM service,
# the intents manifest (`org.quire.Shell`, for intentd and `quire-do`), the companion skills
# (dist/skills) and the settings schema where a login reads them, and copy the keycap layer for
# the sill session to ~/.config/keycap/keycap.toml when that file is missing (never over yours).
# The login itself, "sill (casement)", is the desktop repo's session/ and install/. Run as
# yourself; sudo asks once. Safe to run again.
#
#   dist/install.sh --dry-run    print every step, change nothing (no build; the lock check still runs)
#   dist/install.sh              do it
#
# Not installed, on purpose: dist/udev and dist/modules-load.d (DDC/CI already works on this
# machine through the i2c ACL), dist/sill.service (the desktop session starts sill itself), and
# anything of Plasma's or Toshy's, and keycap's machine file /etc/keycap/keycap.toml.
# dist/uninstall.sh undoes it. docs/first-login.md is next.
set -euo pipefail
# shellcheck source=install-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/install-lib.sh"

usage() { sed -n '2,15p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
parse_flags "$@"
[ "$DRY" = 0 ] || echo "dry run: nothing below is executed"

step "1/7 checks"
refuse_root
command -v cargo >/dev/null || { echo "cargo not found" >&2; exit 1; }
if [ -e "$HOME/.cargo/bin/sill" ]; then
  note "WARNING: $HOME/.cargo/bin/sill would come first on a login shell's PATH and shadow $BIN_DEST;"
  note "         remove it (rm $HOME/.cargo/bin/sill) or the session runs the old build"
fi
need_sudo

step "2/7 the lock file against the sibling checkouts"
check_lock

step "3/7 build sill (release, from $ROOT)"
run cargo build --release --locked --manifest-path "$ROOT/Cargo.toml" -p sill
BUILT="${CARGO_TARGET_DIR:-$ROOT/target}/release/sill"
[ "$DRY" = 1 ] || [ -x "$BUILT" ] || { echo "no binary at $BUILT" >&2; exit 1; }

step "4/7 the binary"
put 755 "$BUILT" "$BIN_DEST"

step "5/7 the PAM service, the intents manifest, the skills and the settings schema"
put 644 "$DIST/pam.d/sill" "$PAM_DEST"
# intentd (docket) lists it, and `quire-do shell --list` shows it: sill serves it as `org.quire.Shell`.
put 644 "$DIST/intents/org.quire.Shell.toml" "$INTENTS_DEST"
# The skills the companion loads (docket-eval --check-skills validates them); they grant nothing.
while IFS= read -r file; do
  put 644 "$DIST/skills/$file" "$SKILLS_DEST/$file"
done < <(skills)
# The settings schema, written by the binary just built, so it always matches it.
schema_dir=$(mktemp -d)
trap 'rm -rf "$schema_dir"' EXIT
if [ "$DRY" = 1 ]; then
  note "would write the settings schema with $BUILT --write-schema and install it at $SCHEMA_DEST"
else
  "$BUILT" --write-schema "$schema_dir" >/dev/null
  put 644 "$schema_dir/sill.settings.toml" "$SCHEMA_DEST"
fi

step "6/7 the keycap layer for the sill session (copied when missing; never overwritten)"
if [ -f "$KEYCAP_LAYER_DEST" ] && cmp -s "$KEYCAP_LAYER_SRC" "$KEYCAP_LAYER_DEST"; then
  note "unchanged: $KEYCAP_LAYER_DEST"
elif [ -e "$KEYCAP_LAYER_DEST" ]; then
  note "kept (yours): $KEYCAP_LAYER_DEST; compare with $KEYCAP_LAYER_SRC"
else
  run install -D -m 644 "$KEYCAP_LAYER_SRC" "$KEYCAP_LAYER_DEST"
fi

step "7/7 SELinux labels and the user manager"
if command -v restorecon >/dev/null; then
  run sudo restorecon -F "$BIN_DEST" "$PAM_DEST" "$INTENTS_DEST" "$SCHEMA_DEST"
else
  note "no restorecon: skipped"
fi
run systemctl --user daemon-reload

step "done"
note "Next: install the \"sill (casement)\" login from the desktop repo, then docs/first-login.md. Plasma is untouched and stays in the session list."

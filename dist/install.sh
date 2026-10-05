#!/usr/bin/env bash
# Install the sill session from this checkout: build `sill` in release, then put the binary, the
# session script, the login-screen entry, the user target, the PAM service, the intents manifest
# (`org.quire.Shell`, for intentd and `quire-do`) and the COSMIC seed files where a login reads
# them, and copy the keycap layer for the sill session to ~/.config/keycap/keycap.toml when that
# file is missing (never over yours). Run as yourself; sudo asks once. Safe to run again.
#
#   dist/install.sh --dry-run    print every step, change nothing (no build; the lock check still runs)
#   dist/install.sh              do it
#
# Not installed, on purpose: dist/udev and dist/modules-load.d (DDC/CI already works on this
# machine through the i2c ACL), dist/sill.service (the session script starts sill itself), and
# anything of Plasma's or Toshy's, and keycap's machine file /etc/keycap/keycap.toml.
# dist/uninstall.sh undoes it. docs/first-login.md is next.
set -euo pipefail
# shellcheck source=install-lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/install-lib.sh"

usage() { sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
parse_flags "$@"
[ "$DRY" = 0 ] || echo "dry run: nothing below is executed"

step "1/8 checks"
refuse_root
command -v cargo >/dev/null || { echo "cargo not found" >&2; exit 1; }
command -v cosmic-comp >/dev/null || note "WARNING: cosmic-comp is not on PATH; the session cannot start without it"
[ -d "${DESKTOP_DEST%/*}" ] || { echo "no ${DESKTOP_DEST%/*}: is a display manager installed?" >&2; exit 1; }
if [ -e "$HOME/.cargo/bin/sill" ]; then
  note "WARNING: $HOME/.cargo/bin/sill would come first on a login shell's PATH and shadow $BIN_DEST;"
  note "         remove it (rm $HOME/.cargo/bin/sill) or the session runs the old build"
fi
need_sudo

step "2/8 the lock file against the sibling checkouts"
check_lock

step "3/8 build sill (release, from $ROOT)"
run cargo build --release --locked --manifest-path "$ROOT/Cargo.toml" -p sill
BUILT="${CARGO_TARGET_DIR:-$ROOT/target}/release/sill"
[ "$DRY" = 1 ] || [ -x "$BUILT" ] || { echo "no binary at $BUILT" >&2; exit 1; }

step "4/8 the binary and the session script"
put 755 "$BUILT" "$BIN_DEST"
put 755 "$DIST/sill-session" "$SESSION_DEST"

step "5/8 the login-screen entry, the user target, the PAM service and the intents manifest"
put 644 "$DIST/sill.desktop" "$DESKTOP_DEST"
put 644 "$DIST/sill-session.target" "$TARGET_DEST"
put 644 "$DIST/pam.d/sill" "$PAM_DEST"
# intentd (docket) lists it, and `quire-do shell --list` shows it: sill serves it as `org.quire.Shell`.
put 644 "$DIST/intents/org.quire.Shell.toml" "$INTENTS_DEST"

step "6/8 the COSMIC seed files (copied into ~/.config/cosmic at login when missing; never overwritten)"
while IFS= read -r file; do
  put 644 "$DIST/cosmic/$file" "$SEED_DEST/cosmic/$file"
done < <(seeds)

step "7/8 the keycap layer for the sill session (copied when missing; never overwritten)"
if [ -f "$KEYCAP_LAYER_DEST" ] && cmp -s "$KEYCAP_LAYER_SRC" "$KEYCAP_LAYER_DEST"; then
  note "unchanged: $KEYCAP_LAYER_DEST"
elif [ -e "$KEYCAP_LAYER_DEST" ]; then
  note "kept (yours): $KEYCAP_LAYER_DEST; compare with $KEYCAP_LAYER_SRC"
else
  run install -D -m 644 "$KEYCAP_LAYER_SRC" "$KEYCAP_LAYER_DEST"
fi

step "8/8 SELinux labels and the user manager"
if command -v restorecon >/dev/null; then
  run sudo restorecon -F "$BIN_DEST" "$SESSION_DEST" "$DESKTOP_DEST" "$TARGET_DEST" "$PAM_DEST" "$INTENTS_DEST"
  run sudo restorecon -RF "$SEED_DEST"
else
  note "no restorecon: skipped"
fi
run systemctl --user daemon-reload

step "done"
note "Next: docs/first-login.md. Plasma is untouched and stays in the session list."

# shellcheck shell=bash disable=SC2034
# Shared by dist/install.sh and dist/uninstall.sh (sourced, never run). Where each piece of the
# session goes, and the helpers that print every step and honour --dry-run.
#
# Nothing here touches Plasma's files, Toshy's units or keycap's system files: the session is the
# files below, the binary, and one user file for keycap (the sill layer), and nothing else.

DIST="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIST/.." && pwd)"

BIN_DEST=/usr/local/bin/sill
SESSION_DEST=/usr/local/bin/sill-session
DESKTOP_DEST=/usr/share/wayland-sessions/sill.desktop
TARGET_DEST=/etc/systemd/user/sill-session.target
PAM_DEST=/etc/pam.d/sill
# What the shell can be asked to do (`org.quire.Shell`): intentd reads every manifest under
# $XDG_DATA_DIRS/quire/intents/, and /usr/local/share is in the default XDG_DATA_DIRS.
INTENTS_DEST=/usr/local/share/quire/intents/org.quire.Shell.toml
# The seed files sill-session copies into ~/.config/cosmic when they are missing there: the
# shortcuts (dist/cosmic) and the keyboard repeat. System-wide COSMIC defaults under
# /usr/share/cosmic belong to cosmic-settings-daemon's package, so nothing is written there.
SEED_DEST=/usr/local/share/sill
# The sill session's keycap layer. sill forwards this user file to keycap, which lays it over
# /etc/keycap/keycap.toml (the Plasma overlay, which Plasma never overrides); the file is copied
# when missing and never overwritten.
KEYCAP_LAYER_SRC="$DIST/keycap/keycap.toml"
KEYCAP_LAYER_DEST="${XDG_CONFIG_HOME:-$HOME/.config}/keycap/keycap.toml"

DRY=0

# parse_flags "$@": --dry-run, and the extra flags a script allows ($EXTRA_FLAGS, space-separated).
parse_flags() {
  local arg name
  for arg in "$@"; do
    case "$arg" in
      --dry-run) DRY=1 ;;
      -h|--help) usage; exit 0 ;;
      *) if [[ " ${EXTRA_FLAGS:-} " == *" $arg "* ]]; then name="${arg#--}"; declare -g "FLAG_${name//-/_}=1"; else
           echo "unknown argument: $arg" >&2; usage >&2; exit 2; fi ;;
    esac
  done
}

step() { printf '\n==> %s\n' "$*"; }
note() { printf '    %s\n' "$*"; }

# run CMD...: print the command; run it unless --dry-run.
run() {
  printf '    $ %s\n' "$*"
  [ "$DRY" = 1 ] || "$@"
}

# put MODE SRC DEST: install SRC at DEST as root, unless DEST already is a copy of it.
put() {
  local mode="$1" src="$2" dest="$3"
  if [ -f "$dest" ] && cmp -s "$src" "$dest"; then
    note "unchanged: $dest"
    return 0
  fi
  run sudo install -D -m "$mode" "$src" "$dest"
}

# seeds: the seed files' paths relative to dist/cosmic, one per line.
seeds() { (cd "$DIST/cosmic" && find . -type f | sed 's|^\./||' | sort); }

# sibling_checkouts: the sibling repos the workspace names by path (`../quire`, ...), one per line.
sibling_checkouts() {
  grep -rhoE --include=Cargo.toml '"\.\./[A-Za-z0-9_-]+/' "$ROOT/Cargo.toml" "$ROOT/crates" 2>/dev/null |
    tr -d '"' | cut -d/ -f2 | sort -u
}

# behind_siblings: "name N" for each sibling checkout that is N commits behind its upstream.
behind_siblings() {
  local name behind
  while IFS= read -r name; do
    behind="$(git -C "$ROOT/../$name" rev-list --count 'HEAD..@{u}' 2>/dev/null)" || continue
    [ "${behind:-0}" -eq 0 ] || echo "$name $behind"
  done < <(sibling_checkouts)
}

# check_lock: the build uses --locked, so Cargo.lock must already fit the sibling checkouts. When it
# does not, say which siblings are behind (cargo's own message does not) and stop.
check_lock() {
  if cargo metadata --locked --format-version 1 --manifest-path "$ROOT/Cargo.toml" >/dev/null 2>&1; then
    note "Cargo.lock fits the sibling checkouts"
    return 0
  fi
  echo "cargo metadata --locked fails: Cargo.lock does not fit the sibling checkouts next to $ROOT" >&2
  local behind
  behind="$(behind_siblings)"
  if [ -n "$behind" ]; then
    echo "these siblings are behind their upstream (last fetched state):" >&2
    while read -r name count; do
      echo "  $name: $count commits behind; git -C $ROOT/../$name pull --ff-only" >&2
    done <<<"$behind"
  else
    echo "no sibling is behind its fetched upstream: git fetch them, or sill and a sibling are mid-change (try again later)" >&2
  fi
  echo "then run this again; the exact cargo error: cargo metadata --locked --manifest-path $ROOT/Cargo.toml" >&2
  return 1
}

refuse_root() {
  [ "$(id -u)" -ne 0 ] || {
    echo "run this as yourself, not root: it builds as you and uses sudo only to install" >&2
    exit 1
  }
}

need_sudo() {
  [ "$DRY" = 1 ] && return 0
  command -v sudo >/dev/null || { echo "sudo is needed" >&2; exit 1; }
  note "sudo will ask for your password once"
  sudo -v
}

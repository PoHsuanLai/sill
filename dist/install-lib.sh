# shellcheck shell=bash disable=SC2034
# Shared by dist/install.sh and dist/uninstall.sh (sourced, never run). Where each piece of the
# session goes, and the helpers that print every step and honour --dry-run.
#
# Nothing here touches Plasma's files, Toshy's units or keycap: the session is the files below and
# the binary, and nothing else.

DIST="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$DIST/.." && pwd)"

BIN_DEST=/usr/local/bin/sill
SESSION_DEST=/usr/local/bin/sill-session
DESKTOP_DEST=/usr/share/wayland-sessions/sill.desktop
TARGET_DEST=/etc/systemd/user/sill-session.target
PAM_DEST=/etc/pam.d/sill
# The seed files sill-session copies into ~/.config/cosmic when they are missing there: the
# shortcuts (dist/cosmic) and the keyboard repeat. System-wide COSMIC defaults under
# /usr/share/cosmic belong to cosmic-settings-daemon's package, so nothing is written there.
SEED_DEST=/usr/local/share/sill

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

# The one line every dev script starts with: source "$(dirname "${BASH_SOURCE[0]}")/../lib/all.sh"
# Sourcing it defines SILL_REPO, SHELL_HOST_DEV, SILL_BIN and the helpers of the other lib files.
LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
for lib in bus compositor daemon input pixels assert backdrop; do
  # shellcheck source=/dev/null
  source "$LIB_DIR/$lib.sh"
done
unset lib

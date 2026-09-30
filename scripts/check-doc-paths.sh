#!/usr/bin/env bash
# Every backticked path in the repo's markdown files must name a file or directory that exists.
# A token counts as a path when it ends in .rs/.css/.md/.sh or starts with `crates/`, `design/`,
# `docs/`, `scripts/` or `examples/`. It resolves when it exists relative to the repo root or to
# the doc's own directory, or when it is the tail of a tracked file's path (`root/ds.rs`,
# `ds-style/tokens/colour.rs` for `crates/ds-style/src/tokens/colour.rs`), in this repo or a
# sibling repo checked out beside it. Tokens with a glob, a placeholder, a space or a leading
# `~` (files outside the repo) are skipped, as is anything between `<!-- paths: skip -->` and
# `<!-- paths: end -->` (a table of old locations that no longer exist). Prints one `doc: token` line per miss; exits 1 on any.
# Run by hand after moving or renaming files; it is not part of the gate.
set -uo pipefail
cd "$(dirname "$0")/.."
root=$(pwd)

index=$(mktemp)
trap 'rm -f "$index"' EXIT
git ls-files >"$index"
for sibling in ../*/; do
  [ -d "$sibling/.git" ] || [ -f "$sibling/.git" ] || continue
  [ "$(cd "$sibling" && pwd)" = "$root" ] && continue
  git -C "$sibling" ls-files 2>/dev/null >>"$index"
done

status=0
while IFS= read -r doc; do
  dir=$(dirname "$doc")
  awk '/<!-- paths: skip -->/{skip=1} !skip{print} /<!-- paths: end -->/{skip=0}' "$doc" | grep -o '`[^` ]*`' | tr -d '`' | sort -u | while IFS= read -r token; do
    case "$token" in
      *'*'*|*'<'*|*'{'*|*'$'*|*'('*|*'|'*|*'='*|*','*|http*|*'::'*|'~'*|.*) continue ;;
    esac
    path=${token%%#*}
    path=${path%:[0-9]*}
    case "$path" in
      *.rs|*.css|*.md|*.sh|crates/*|docs/*|scripts/*|examples/*) ;;
      *) continue ;;
    esac
    [ -z "$path" ] && continue
    trimmed=${path%/}
    [ -e "$trimmed" ] && continue
    [ -e "$dir/$trimmed" ] && continue
    stripped=${trimmed#./}
    stripped=${stripped#../}
    crate=${stripped%%/*}
    if [ "$crate" != "$stripped" ]; then
      grep -qxF -- "crates/$crate/src/${stripped#*/}" "$index" && continue
    fi
    grep -qF -- "$stripped" "$index" && grep -q -- "\(^\|/\)$(printf '%s' "$stripped" | sed 's/[.[\*^$]/\\&/g')\(/\|$\)" "$index" && continue
    echo "$doc: $token"
  done
done < <(git ls-files '*.md' | grep -v '^examples/.*/target/') | tee "${TMPDIR:-/tmp}/check-doc-paths.$$" 
n=$(wc -l <"${TMPDIR:-/tmp}/check-doc-paths.$$"); rm -f "${TMPDIR:-/tmp}/check-doc-paths.$$"
[ "$n" -eq 0 ] || { echo "$n missing path(s)" >&2; exit 1; }

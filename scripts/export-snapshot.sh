#!/usr/bin/env bash
# Writes a consumer's snapshot of the toolkit (D239, roadmap D16) from a git revision.
#   scripts/export-snapshot.sh <dest> [--rev <rev>]
# <rev> defaults to HEAD. The snapshot is `git archive` of an allow list, never the working tree,
# so a dirty tree changes nothing and SNAPSHOT's sha is true:
#   Cargo.toml, rust-toolchain.toml, LICENSE-*, THIRD_PARTY_NOTICES.md, licenses/,
#   crates/*/Cargo.toml, crates/*/src, crates/*/assets
# No apps/, tools/, docs/, crates/*/tests (they need the gallery's fonts), no Cargo.lock (the
# consumer's lock rules), nothing from assets/lunegit (no open licence). The root manifest's
# `members` becomes ["crates/*"]. <dest> may be absent, empty, or hold an earlier SNAPSHOT (then
# it is replaced whole, once the new tree has passed every check); any other non-empty <dest> is
# refused untouched.
# Exit 0 done, 1 a check failed (dest untouched), 2 usage or setup. bash 3.2 / BSD safe.
set -euo pipefail
export LC_ALL=C

usage() {
  echo "usage: export-snapshot.sh <dest> [--rev <rev>]" >&2
  exit 2
}

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest=""
rev="HEAD"
while [ $# -gt 0 ]; do
  case "$1" in
    --rev)
      [ $# -ge 2 ] || usage
      rev="$2"
      shift 2
      ;;
    -*) usage ;;
    *)
      [ -z "$dest" ] || usage
      dest="$1"
      shift
      ;;
  esac
done
[ -n "$dest" ] || usage

git() { command git -C "$ROOT" "$@"; }

sha="$(git rev-parse --verify "$rev^{commit}" 2>/dev/null)" || {
  echo "export-snapshot: cannot resolve revision '$rev'" >&2
  exit 2
}

# An absolute <dest> without a trailing slash; its parent must exist.
case "$dest" in
  /*) ;;
  *) dest="$PWD/$dest" ;;
esac
while [ "${dest%/}" != "$dest" ] && [ "$dest" != "/" ]; do dest="${dest%/}"; done
parent="$(dirname "$dest")"
[ -d "$parent" ] || {
  echo "export-snapshot: $parent is not a directory" >&2
  exit 2
}
if [ -e "$dest" ]; then
  [ -d "$dest" ] || {
    echo "export-snapshot: $dest exists and is not a directory" >&2
    exit 2
  }
  if [ -n "$(ls -A "$dest")" ] && [ ! -f "$dest/SNAPSHOT" ]; then
    echo "export-snapshot: $dest is not empty and holds no SNAPSHOT; left untouched" >&2
    exit 2
  fi
fi

stage="$(mktemp -d "$parent/.$(basename "$dest").stage.XXXXXX")"
old=""
cleanup() {
  rm -rf "$stage"
  [ -z "$old" ] || [ ! -d "$old" ] || {
    # A swap that did not finish: put the old tree back.
    [ -e "$dest" ] || mv "$old" "$dest"
  }
}
trap cleanup EXIT

fail() {
  echo "export-snapshot: FAILED: $1" >&2
  exit 1
}

# ---- the allow list, from the tree at <rev> ----------------------------------------------------
paths=(Cargo.toml rust-toolchain.toml THIRD_PARTY_NOTICES.md licenses)
for f in $(git ls-tree --name-only "$sha" | grep '^LICENSE-' || true); do
  paths+=("$f")
done
for c in $(git ls-tree --name-only "$sha" crates/ | sed 's|^crates/||'); do
  for p in Cargo.toml src assets; do
    if git cat-file -e "$sha:crates/$c/$p" 2>/dev/null; then
      paths+=("crates/$c/$p")
    fi
  done
done
for p in Cargo.toml rust-toolchain.toml; do
  git cat-file -e "$sha:$p" 2>/dev/null || fail "$p is not in $rev"
done

git archive "$sha" -- "${paths[@]}" | tar -x -C "$stage"

# ---- the root manifest's members ---------------------------------------------------------------
n="$(grep -c '^members = ' "$stage/Cargo.toml" || true)"
[ "$n" = "1" ] || fail "the root manifest has $n 'members = ' lines, not exactly one"
sed 's|^members = .*|members = ["crates/*"]|' "$stage/Cargo.toml" > "$stage/Cargo.toml.new"
mv "$stage/Cargo.toml.new" "$stage/Cargo.toml"
grep -q '^members = \["crates/\*"\]$' "$stage/Cargo.toml" || fail "members was not rewritten"

# ---- fail closed -------------------------------------------------------------------------------
# A path that must not ship.
forbidden() {
  case "$1" in
    apps/* | tools/* | docs/* | crates/*/tests/* | */assets/lunegit/* | assets/lunegit/*) return 0 ;;
  esac
  return 1
}
bad=""
while IFS= read -r f; do
  if forbidden "$f"; then bad="$bad $f"; fi
done < <(cd "$stage" && find . -type f | sed 's|^\./||')
[ -z "$bad" ] || fail "staged path(s) that must not ship:$bad"
lg="$(cd "$stage" && find . -type d -path '*lunegit*' | head -3)"
[ -z "$lg" ] || fail "a directory named for lunegit is staged: $lg"

# A copy of a bundled fixture under any name: compare blob ids with the fixtures at <rev>.
fixtures="$(git ls-tree -r "$sha" -- apps/gallery/assets/lunegit | awk '{print $3}' | sort -u)"
if [ -n "$fixtures" ]; then
  ours="$(find "$stage" -type f -print0 | xargs -0 git -C "$ROOT" hash-object --no-filters -- | sort -u)"
  dup="$(printf '%s\n%s\n' "$fixtures" "$ours" | sort | uniq -d | head -3)"
  [ -z "$dup" ] || fail "an exported file is a copy of a Lunegit fixture (blob $(echo "$dup" | head -1))"
fi

printf '%s\n' "$sha" > "$stage/SNAPSHOT"

# ---- swap in -----------------------------------------------------------------------------------
if [ -e "$dest" ]; then
  old="$(mktemp -d "$parent/.$(basename "$dest").old.XXXXXX")"
  rmdir "$old"
  mv "$dest" "$old"
fi
mv "$stage" "$dest"
[ -z "$old" ] || rm -rf "$old"
old=""
echo "export-snapshot: $dest at $sha"

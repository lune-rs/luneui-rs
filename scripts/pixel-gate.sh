#!/usr/bin/env bash
# Pixel gate for a drawing change: dumps the gallery's reference scenes for a base revision and
# for the working tree, then compares them. Zero delta unless a scene is named with --allow.
#
#   scripts/pixel-gate.sh [--base <rev>] [--allow <scene-id>]...
#
#   --base   revision to compare against (default HEAD). Built from `git archive <rev>` in
#            target/pixel-gate/base-src with its own target dir, so the two builds do not share
#            binaries. A base without apps/gallery is an error: nothing to compare is not a pass.
#   --allow  a scene id (the part of a `gallery --list` name before `@`, for example `chrome`)
#            whose frames may differ: its files move aside and are compared report-only.
#
# The candidate is the working tree (committed or not).
# Exit 0: zero delta on every common non-allowed scene and no scene missing from the candidate.
# Exit 1: a pixel delta or a scene missing from the candidate.
# Exit 2: usage or setup error (bad rev, no gallery in the base, unknown --allow id, a build or
# dump failure, a dump dir with no PNGs, no compare summary).
# Writes only under target/pixel-gate/ (plus cargo's own caches); never prints the environment.
# Dumps are offscreen but need a logged-in macOS GUI session (hidden winit window). bash 3.2 safe.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
REV="HEAD"
ALLOW=()

die() {
  echo "pixel-gate: $*" >&2
  exit 2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --base) [ "$#" -ge 2 ] || die "--base needs a value"; REV="$2"; shift 2 ;;
    --allow)
      [ "$#" -ge 2 ] || die "--allow needs a value"
      case "$2" in '' | *[!A-Za-z0-9-]*) die "bad scene id: $2" ;; esac
      ALLOW+=("$2")
      shift 2
      ;;
    -h | --help) sed -n '2,19p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
done

S="$ROOT/target/pixel-gate"
BASE_SRC="$S/base-src"
BASE_TARGET="$S/base-target"
OUT_BASE="$S/base"
OUT_CAND="$S/cand"
ALLOWED="$S/allowed"

# --- base revision ------------------------------------------------------------------------------
git -C "$ROOT" rev-parse --verify "$REV^{commit}" > /dev/null 2>&1 || die "not a commit: $REV"
mkdir -p "$S"
rm -rf "$BASE_SRC"
mkdir -p "$BASE_SRC"
git -C "$ROOT" archive "$REV" | tar -x -C "$BASE_SRC" || die "git archive of $REV failed"
[ -f "$BASE_SRC/apps/gallery/Cargo.toml" ] || die "base $REV has no apps/gallery (nothing to compare)"

# --- candidate scene list, --allow validation ---------------------------------------------------
LIST="$(cd "$ROOT" && cargo run -q --locked -p gallery -- --list)" || die "candidate gallery --list failed"
[ -n "$LIST" ] || die "candidate gallery --list printed nothing"
for id in ${ALLOW[@]+"${ALLOW[@]}"}; do
  printf '%s\n' "$LIST" | grep -q -x -E -- "$id@[0-9]+x" || die "--allow $id: not a scene of the candidate's gallery --list"
done

# --- dumps --------------------------------------------------------------------------------------
# count_pngs <dir>: number of PNG files directly in the dir.
count_pngs() {
  find "$1" -maxdepth 1 -type f -name '*.png' | wc -l | tr -d ' '
}

rm -rf "$OUT_BASE" "$OUT_CAND" "$ALLOWED"
mkdir -p "$OUT_BASE" "$OUT_CAND"
echo "pixel-gate: dumping base $REV"
(cd "$BASE_SRC" && cargo run -q --locked --target-dir "$BASE_TARGET" -p gallery -- --dump "$OUT_BASE") \
  || die "base build or dump failed"
echo "pixel-gate: dumping the working tree"
(cd "$ROOT" && cargo run -q --locked -p gallery -- --dump "$OUT_CAND") || die "candidate build or dump failed"
[ "$(count_pngs "$OUT_BASE")" -gt 0 ] || die "base dump holds no PNGs"
[ "$(count_pngs "$OUT_CAND")" -gt 0 ] || die "candidate dump holds no PNGs"

# compare <base dir> <candidate dir>: runs the working tree's dump-compare with --allow-new;
# prints its output and sets CMP_RC (0 ok, 1 delta or missing). Exit 2 when there is no summary.
compare() {
  local out rc=0
  out="$(cd "$ROOT" && cargo run -q --locked -p dump-compare -- "$1" "$2" --allow-new 2>&1)" || rc=$?
  printf '%s\n' "$out"
  printf '%s\n' "$out" | tail -n 1 | grep -q -E '^[0-9]+ files, ' || die "dump-compare printed no summary (exit $rc)"
  CMP_RC=$rc
}

# --- allowed scenes: moved aside, compared report-only ------------------------------------------
if [ "${#ALLOW[@]}" -gt 0 ]; then
  mkdir -p "$ALLOWED/base" "$ALLOWED/cand"
  for id in "${ALLOW[@]}"; do
    for f in "$OUT_BASE/$id@"*.png; do [ -e "$f" ] && mv "$f" "$ALLOWED/base/"; done
    for f in "$OUT_CAND/$id@"*.png; do [ -e "$f" ] && mv "$f" "$ALLOWED/cand/"; done
  done
  echo "pixel-gate: allowed scenes (report only): ${ALLOW[*]}"
  CMP_RC=0
  compare "$ALLOWED/base" "$ALLOWED/cand"
fi

# --- main compare -------------------------------------------------------------------------------
echo "pixel-gate: comparing base $REV with the working tree"
CMP_RC=0
compare "$OUT_BASE" "$OUT_CAND"
case "$CMP_RC" in
  0) echo "pixel-gate: ok" ;;
  *) echo "pixel-gate: FAILED: pixel delta or missing scene" >&2; exit 1 ;;
esac

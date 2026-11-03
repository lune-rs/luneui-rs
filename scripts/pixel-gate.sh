#!/usr/bin/env bash
# Pixel gate for a drawing change: dumps the gallery's reference scenes for a base revision and
# for the working tree, then compares them. Zero delta unless a scene is named with --allow.
#
#   scripts/pixel-gate.sh [--base <rev>] [--allow <scene-id>]...
#
#   --base   revision to compare against (default HEAD), resolved once to a commit sha. Built
#            from `git archive <sha>` in a base source dir outside the repo,
#            ${TMPDIR:-/tmp}/luneui-pixel-gate-<cksum of the repo root>/base-src (fixed per checkout),
#            with its own target dir, so the two builds do not share binaries; the base's local crates
#            always rebuild from exactly <rev>'s sources (a stale binary from an earlier base run is
#            never reused). Guard: if any parent dir of the base source holds a .cargo/config.toml or
#            .cargo/config other than ${CARGO_HOME:-$HOME/.cargo}'s, exit 2 naming it (no cargo
#            config from elsewhere reaches the base build). A base without apps/gallery is an error:
#            nothing to compare is not a pass.
#   --allow  a scene id (the part of a `gallery --list` name before `@`, for example `chrome`)
#            whose frames may differ: its files move aside and are compared report-only.
#
# The candidate is the working tree (committed or not).
# Exit 0: zero delta on every common non-allowed scene and no scene missing from the candidate.
# Exit 1: a pixel delta or a scene missing from the candidate.
# Exit 2: usage or setup error (bad rev, no gallery in the base, a cargo config above the base
# source, unknown --allow id, a build or dump failure, a dump dir with no PNGs, no compare summary).
# Writes only under target/pixel-gate/ and the base source dir above (plus cargo's own caches);
# never prints the environment.
# The working tree dumps on a headless device (no window); a base from before M2 step 4 (52b11d6 and
# older) opens a hidden winit window, so such a base needs a logged-in macOS GUI session. bash 3.2 safe.
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
    -h | --help) sed -n '2,/^set -euo pipefail$/p' "${BASH_SOURCE[0]}" | sed '$d'; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
done

S="$ROOT/target/pixel-gate"
BASE_TMP="${TMPDIR:-/tmp}"
BASE_TMP="${BASE_TMP%/}/luneui-pixel-gate-$(printf '%s' "$ROOT" | cksum | cut -d ' ' -f 1)"
BASE_SRC="$BASE_TMP/base-src"
BASE_TARGET="$S/base-target"
OUT_BASE="$S/base"
OUT_CAND="$S/cand"
ALLOWED="$S/allowed"

# --- base revision ------------------------------------------------------------------------------
SHA="$(git -C "$ROOT" rev-parse --verify "$REV^{commit}" 2> /dev/null)" || die "not a commit: $REV"
mkdir -p "$S"
rm -rf "$BASE_SRC"
mkdir -p "$BASE_SRC"
# tar -m: files get the extraction time, not the commit time of $REV. Cargo's freshness check for
# path packages is mtime-based and the package path and target dir are the same every run, so with
# commit-time mtimes a base older than the last base build would reuse that build's binary and the
# gate would compare against the wrong revision. With -m every local crate rebuilds from exactly
# $REV's sources (registry dependencies stay cached).
git -C "$ROOT" archive "$SHA" | tar -x -m -C "$BASE_SRC" || die "git archive of $REV failed"
[ -f "$BASE_SRC/apps/gallery/Cargo.toml" ] || die "base $REV has no apps/gallery (nothing to compare)"
echo "pixel-gate: base source in $BASE_SRC"

# --- guard: no cargo config above the base source, except CARGO_HOME's ----------------------------
CARGO_HOME_DIR="${CARGO_HOME:-$HOME/.cargo}"
CARGO_HOME_DIR="$(cd "$CARGO_HOME_DIR" 2> /dev/null && pwd -P || printf '%s' "$CARGO_HOME_DIR")"
d="$(cd "$BASE_SRC/.." && pwd -P)"
while :; do
  if [ -d "$d/.cargo" ] && [ "$(cd "$d/.cargo" && pwd -P)" != "$CARGO_HOME_DIR" ]; then
    for f in "$d/.cargo/config.toml" "$d/.cargo/config"; do
      [ -e "$f" ] && die "cargo config above the base source would reach the base build: $f"
    done
  fi
  [ "$d" = "/" ] && break
  d="$(dirname "$d")"
done

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
echo "pixel-gate: dumping base $REV ($(printf '%s' "$SHA" | cut -c 1-7))"
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

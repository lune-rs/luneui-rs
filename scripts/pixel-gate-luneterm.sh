#!/usr/bin/env bash
# Cross-repo pixel gate: proves this repo's crates/lune-ui draws the same pixels as the Luneterm
# crates/lune-ui it was extracted from, using Luneterm's 18-scene spike dump (M1 authority).
#
#   scripts/pixel-gate-luneterm.sh [--luneterm <dir>] [--rev <sha>] [--lock <file>] [--parity-only]
#
#   --luneterm   Luneterm checkout to clone from (default: `Luneterm` beside this repo's main
#                checkout). Read only; never modified.
#   --rev        Luneterm commit the crate was extracted from (default: the M1 rev).
#   --lock       Cargo.lock to check (default: this repo's). Used with --parity-only in tests.
#   --parity-only  Only phase 1.
#
# Phases:
#   1 parity   every registry package (name version) in --lock exists in Luneterm's Cargo.lock at
#              --rev, so both repos build the same dependency versions.
#   2 clone    scratch clone of Luneterm at target/pixel-gate-luneterm/luneterm, detached at --rev.
#   3 run A    Luneterm's own crates/lune-ui builds the spike; dump into target/pixel-gate-luneterm/a.
#   4 run B    the clone's root Cargo.toml is rewritten so `lune-ui` is THIS repo's crates/lune-ui
#              (and the in-tree one is excluded); dump into .../b. The manifest is restored on exit.
#   5 compare  the clone's dump_compare example (Luneterm's tool) on a vs b.
#
# Exit 0 only when parity holds, run A built the clone's crate, run B built this repo's crate, both
# dirs hold exactly 18 PNGs and the compare reports `18 files, 18 identical`.
# Exit 1: parity mismatch or a pixel difference. Exit 2: usage or setup error (missing dir, bad rev,
# manifest line not found, build failure, wrong count, wrong crate).
# Writes only under target/pixel-gate-luneterm/ (plus cargo's own cache); never prints the
# environment. Needs a logged-in macOS GUI session (hidden winit window). bash 3.2 safe.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
DEFAULT_REV="296cfdc9333fe6bcb982f7ab892bfe1612b17406"
EXPECT=18

LT=""
REV="$DEFAULT_REV"
LOCK="$ROOT/Cargo.lock"
PARITY_ONLY=0

die() {
  echo "pixel-gate-luneterm: $*" >&2
  exit 2
}
fail() {
  echo "pixel-gate-luneterm: FAILED: $*" >&2
  exit 1
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --luneterm) [ "$#" -ge 2 ] || die "--luneterm needs a value"; LT="$2"; shift 2 ;;
    --rev) [ "$#" -ge 2 ] || die "--rev needs a value"; REV="$2"; shift 2 ;;
    --lock) [ "$#" -ge 2 ] || die "--lock needs a value"; LOCK="$2"; shift 2 ;;
    --parity-only) PARITY_ONLY=1; shift ;;
    -h | --help) sed -n '2,27p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
done

if [ -z "$LT" ]; then
  common="$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir)" || die "not a git repo: $ROOT"
  LT="$common/../../Luneterm"
fi
[ -d "$LT" ] || die "Luneterm directory not found: $LT"
LT="$(cd "$LT" && pwd -P)"
git -C "$LT" rev-parse --git-dir > /dev/null 2>&1 || die "not a git checkout: $LT"
git -C "$LT" cat-file -e "$REV^{commit}" 2> /dev/null || die "rev not found in $LT: $REV"
REV="$(git -C "$LT" rev-parse "$REV^{commit}")"
[ -f "$LOCK" ] || die "lock file not found: $LOCK"

S="$ROOT/target/pixel-gate-luneterm"
CLONE="$S/luneterm"
TARGET="$S/target"
OUT_A="$S/a"
OUT_B="$S/b"

# --- Phase 1: lock parity -----------------------------------------------------------------------
# registry_pkgs <lock>: `name version` of every package that has a `source` (path packages have
# none), sorted and unique.
registry_pkgs() {
  awk '
    function flush() { if (name != "" && src) print name " " ver; name = ""; ver = ""; src = 0 }
    /^\[\[package\]\]/ { flush(); next }
    /^name = / { n = $0; sub(/^name = "/, "", n); sub(/"$/, "", n); name = n }
    /^version = / { v = $0; sub(/^version = "/, "", v); sub(/"$/, "", v); ver = v }
    /^source = / { src = 1 }
    END { flush() }
  ' "$1" | LC_ALL=C sort -u
}

mkdir -p "$S"
OURS="$S/lock-ours.txt"
THEIRS="$S/lock-theirs.txt"
git -C "$LT" show "$REV:Cargo.lock" > "$S/lock-luneterm.lock" 2> /dev/null || die "no Cargo.lock at $REV in $LT"
registry_pkgs "$LOCK" > "$OURS"
registry_pkgs "$S/lock-luneterm.lock" > "$THEIRS"
[ -s "$OURS" ] || die "no registry packages found in $LOCK (wrong file?)"
[ -s "$THEIRS" ] || die "no registry packages found in Luneterm's lock (wrong file?)"
offenders="$(LC_ALL=C comm -23 "$OURS" "$THEIRS")"
if [ -n "$offenders" ]; then
  echo "pixel-gate-luneterm: lock parity FAILED: not in Luneterm's lock at $REV:" >&2
  printf '%s\n' "$offenders" | sed 's/^/  /' >&2
  exit 1
fi
echo "pixel-gate-luneterm: lock parity ok ($(wc -l < "$OURS" | tr -d ' ') registry packages, all in Luneterm's lock)"
[ "$PARITY_ONLY" -eq 0 ] || exit 0

# --- Phase 2: scratch clone ---------------------------------------------------------------------
if [ ! -d "$CLONE/.git" ]; then
  git clone -q --no-checkout "$LT" "$CLONE" || die "clone failed"
else
  git -C "$CLONE" remote set-url origin "$LT"
  git -C "$CLONE" fetch -q origin || die "fetch failed"
fi
# The clone is this script's own: discard any leftover edit (for example an unrestored manifest).
git -C "$CLONE" checkout -q -f --detach "$REV" || die "checkout of $REV failed"
git -C "$CLONE" clean -fdq || die "clean failed"

MANIFEST="$CLONE/Cargo.toml"
MANIFEST_BAK="$S/Cargo.toml.orig"
restore_manifest() {
  if [ -f "$MANIFEST_BAK" ]; then
    cp "$MANIFEST_BAK" "$MANIFEST" && rm -f "$MANIFEST_BAK"
  fi
}
trap restore_manifest EXIT
trap 'exit 130' INT TERM

# cargo_in_clone <args...>: cargo in the clone with the gate's target dir.
cargo_in_clone() {
  (cd "$CLONE" && CARGO_TARGET_DIR="$TARGET" cargo "$@")
}

# dump_run <out dir> <log file>: build and run the spike in dump mode. The three variables are
# unset so a developer's shell cannot change which scenes are drawn.
dump_run() {
  rm -rf "$1"
  mkdir -p "$1"
  (cd "$CLONE" && env -u GLASS_UI_TERMVIEW -u GLASS_UI_OPEN_MENU -u GLASS_UI_DUMP \
    CARGO_TARGET_DIR="$TARGET" GLASS_UI_DUMP_FRAMES="$1" \
    cargo run --locked -p glass-ui) > "$2" 2>&1 || {
    tail -n 20 "$2" >&2
    die "spike build or dump failed (full log: $2)"
  }
}

# check_count <dir>: exactly $EXPECT PNGs and nothing else.
check_count() {
  local pngs all
  pngs="$(find "$1" -maxdepth 1 -type f -name '*.png' | wc -l | tr -d ' ')"
  all="$(find "$1" -mindepth 1 | wc -l | tr -d ' ')"
  [ "$pngs" = "$EXPECT" ] && [ "$all" = "$EXPECT" ] || die "$1 holds $pngs PNGs ($all entries), expected exactly $EXPECT"
}

# check_crate <expected crate dir> <label>: `cargo tree` must show which lune-ui this run builds.
check_crate() {
  local line
  line="$(cargo_in_clone tree --locked -i lune-ui --depth 0 2> /dev/null | head -n 1)" || die "cargo tree failed ($2)"
  case "$line" in
    *"($1)"*) echo "pixel-gate-luneterm: $2 builds $1" ;;
    *) die "$2 built the wrong lune-ui: '$line', expected ($1)" ;;
  esac
}

# --- Phase 3: run A (Luneterm's own crate) ------------------------------------------------------
check_crate "$CLONE/crates/lune-ui" "run A"
echo "pixel-gate-luneterm: run A: building and dumping (Luneterm crate)"
dump_run "$OUT_A" "$S/run-a.log"
check_count "$OUT_A"

# --- Phase 4: run B (this repo's crate) ---------------------------------------------------------
PATH_LINE='lune-ui = { path = "crates/lune-ui" }'
MEMBERS_LINE='members = ["crates/*", "spikes/*"]'
[ "$(grep -c -x -F -- "$PATH_LINE" "$MANIFEST")" = 1 ] || die "manifest line not found exactly once: $PATH_LINE"
[ "$(grep -c -x -F -- "$MEMBERS_LINE" "$MANIFEST")" = 1 ] || die "manifest line not found exactly once: $MEMBERS_LINE"
cp "$MANIFEST" "$MANIFEST_BAK"
NEW_PATH="lune-ui = { path = \"$ROOT/crates/lune-ui\" }" \
  PATH_LINE="$PATH_LINE" MEMBERS_LINE="$MEMBERS_LINE" \
  awk '
    $0 == ENVIRON["PATH_LINE"] { print ENVIRON["NEW_PATH"]; next }
    $0 == ENVIRON["MEMBERS_LINE"] { print; print "exclude = [\"crates/lune-ui\"]"; next }
    { print }
  ' "$MANIFEST_BAK" > "$MANIFEST"
check_crate "$ROOT/crates/lune-ui" "run B"
echo "pixel-gate-luneterm: run B: building and dumping (this repo's crate)"
dump_run "$OUT_B" "$S/run-b.log"
check_count "$OUT_B"
restore_manifest

# --- Phase 5: compare ---------------------------------------------------------------------------
echo "pixel-gate-luneterm: comparing $EXPECT scenes"
rc=0
cmp_out="$(cargo_in_clone run -q --locked -p glass-ui --example dump_compare -- "$OUT_A" "$OUT_B" 2>&1)" || rc=$?
printf '%s\n' "$cmp_out" | tail -n 25
case "$rc" in
  0) ;;
  1) fail "pixel difference between Luneterm's crate and this repo's" ;;
  *) die "dump_compare failed (exit $rc)" ;;
esac
summary="$(printf '%s\n' "$cmp_out" | tail -n 1)"
case "$summary" in
  "$EXPECT files, $EXPECT identical, 0 differ, max delta 0") echo "pixel-gate-luneterm: ok: $summary" ;;
  *) die "unexpected compare summary: $summary" ;;
esac

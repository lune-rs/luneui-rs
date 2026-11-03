#!/usr/bin/env bash
# Library boundary rules for the crates under crates/ (ported from Luneterm's
# scripts/check-lib-boundaries.sh). Grep only, so it runs the same on macOS (bash 3.2, BSD grep)
# and Linux (GNU grep). Prints each offending line with its rule number. Exit 1 on any offender,
# 0 when clean, 2 when grep itself fails (a bad path never passes).
#   1  no crates/*/Cargo.toml names an app (luneterm, lunegit)
#   2  no file under crates/ names luneterm
#   3  no file under crates/ names app or terminal words; exceptions are the fixed strings in
#      scripts/boundaries.allow
#   4  no crates/*/src file reads the clock or the environment (keeps frames deterministic)
#   5  every crates/*/Cargo.toml inherits the licence from the workspace
#   6  `unsafe` appears under crates/*/src only in paths containing /platform/ or /material/
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
ALLOW="scripts/boundaries.allow"
fail=0

# drop_allowed: reads grep hits on stdin, prints the lines that contain no fixed string from $ALLOW.
# Blank lines and lines starting with # in $ALLOW are ignored.
drop_allowed() {
  local line pat keep
  while IFS= read -r line; do
    keep=1
    while IFS= read -r pat || [ -n "$pat" ]; do
      case "$pat" in '' | '#'*) continue ;; esac
      case "$line" in *"$pat"*) keep=0; break ;; esac
    done < "$ALLOW"
    if [ "$keep" -eq 1 ]; then printf '%s\n' "$line"; fi
  done
}

# check <rule> <allow: yes|no> <grep args...>: every grep hit is an offender (after the allow list).
# grep exit 2 (an error, e.g. a missing path) stops the script, so a bad path can't pass silently.
check() {
  local rule="$1" allow="$2" out kept rc=0
  shift 2
  out="$(grep "$@" 2>&1)" || rc=$?
  if [ "$rc" -gt 1 ]; then
    echo "check-boundaries: grep failed (exit $rc) in rule $rule: $out" >&2
    exit 2
  fi
  if [ "$rc" -eq 0 ]; then
    kept="$out"
    if [ "$allow" = yes ]; then kept="$(printf '%s\n' "$out" | drop_allowed)"; fi
    if [ -n "$kept" ]; then
      printf '%s\n' "$kept" | sed "s/^/rule $rule: /"
      fail=1
    fi
  fi
}

check 1 no -n -i -E 'luneterm|lunegit' crates/*/Cargo.toml
check 2 no -rn -i luneterm crates
check 3 yes -rn -i -E 'lunegit|mock|terminal|xterm|scrollback|glass_ui|(^|[^[:alnum:]_])(pty|shell|ansi)($|[^[:alnum:]_])' crates
check 4 no -rn -F -e 'env::var' -e 'Instant::now' -e 'SystemTime::now' -e '.elapsed()' crates/*/src

for m in crates/*/Cargo.toml; do
  if ! grep -q -F 'license.workspace = true' "$m"; then
    echo "rule 5: $m: no 'license.workspace = true'"
    fail=1
  fi
done

# Rule 6: word `unsafe` in src, minus the lint names, outside platform/material modules.
# `grep -rn` output is `path:line:text`; filter paths with case.
rc=0
out="$(grep -rn -w 'unsafe' crates/*/src 2>&1)" || rc=$?
if [ "$rc" -gt 1 ]; then
  echo "check-boundaries: grep failed (exit $rc) in rule 6: $out" >&2
  exit 2
fi
if [ "$rc" -eq 0 ]; then
  while IFS= read -r line; do
    case "$line" in *'forbid(unsafe_code)'* | *'unsafe_op_in_unsafe_fn'*) continue ;; esac
    path="${line%%:*}"
    case "$path" in */platform/* | */material/*) continue ;; esac
    echo "rule 6: $line"
    fail=1
  done <<EOF2
$out
EOF2
fi

if [ "$fail" -ne 0 ]; then
  echo "check-boundaries: FAILED" >&2
  exit 1
fi
echo "check-boundaries: clean"

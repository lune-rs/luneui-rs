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
#   6  `unsafe` appears under crates/*/src only under crates/lune-ui-platform/src/platform/ or
#      crates/lune-ui-platform/src/material/; every other crate's src/lib.rs has
#      #![forbid(unsafe_code)]
#   7  crates/lune-ui-core/Cargo.toml declares no dependency of any kind
#   8  no file under crates/lune-ui-core names a third-party crate path (wgpu::, winit::, ...) or
#      `extern crate`
#   9  every crate dir under crates/ is registered below with its allowed dependency names; each
#      manifest's dependencies (any table form, workspace or dotted keys) are within its lists; an
#      unregistered crate dir or a `package =` rename (member manifests, root
#      [workspace.dependencies]) is an offender (fail closed)
#   10 no OS predicate (target_os, target_family, target_vendor, windows, unix) inside cfg(, cfg!(
#      or cfg_attr( in crates/*/src outside a /platform/ or /material/ directory, unless it is a
#      #[cfg(...)] attribute whose next code line is `mod <name>;` or a `use` declaration
#   11 crates/lune-ui/src holds only lib.rs, with no item definitions (fn, struct, enum, trait,
#      impl, const, static, type, macro_rules)
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

# Rule 6: word `unsafe` in src, minus the lint names, outside the platform crate's platform/ and
# material/ directories. `grep -rn` output is `path:line:text`; filter paths with case.
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
    case "$path" in
      crates/lune-ui-platform/src/platform/* | crates/lune-ui-platform/src/material/*) continue ;;
    esac
    echo "rule 6: $line"
    fail=1
  done <<EOF2
$out
EOF2
fi
# Every crate but the platform crate forbids unsafe code at the crate root.
for dir in crates/*/; do
  name="$(basename "$dir")"
  [ "$name" = lune-ui-platform ] && continue
  if [ ! -f "crates/$name/src/lib.rs" ]; then
    echo "rule 6: crates/$name/src/lib.rs: missing, cannot check #![forbid(unsafe_code)]"
    fail=1
  elif ! grep -q -F '#![forbid(unsafe_code)]' "crates/$name/src/lib.rs"; then
    echo "rule 6: crates/$name/src/lib.rs: no #![forbid(unsafe_code)]"
    fail=1
  fi
done

# Manifest parsing (rules 7 and 9): POSIX awk over section headers and keys. Prints one line per
# dependency: `dep <name>` (normal, build, target-specific) or `dev <name>`, and `package <line>`
# for any `package =` (a rename) inside a dependency section. A continuation line of a multi-line
# inline value that looks like a key is reported as a dependency too (fail closed).
read -r -d '' AWK_DEPS <<'AWKEOF' || true
BEGIN { mode = ""; kind = "dep" }
{
  line = $0
  sub(/#.*/, "", line)
  sub(/^[ \t]+/, "", line)
  sub(/[ \t]+$/, "", line)
  if (line == "") next
  if (substr(line, 1, 1) == "[") {
    h = line
    gsub(/^\[+/, "", h)
    gsub(/\]+$/, "", h)
    mode = ""
    kind = "dep"
    if (h ~ /dev-dependencies/) kind = "dev"
    if (h ~ /dependencies$/) {
      mode = "table"
    } else if (h ~ /dependencies\./) {
      mode = "single"
      name = h
      sub(/.*dependencies\./, "", name)
      gsub(/["\047]/, "", name)
      print kind, name
    }
    next
  }
  if (mode == "") next
  if (mode == "table" && match(line, /^[A-Za-z0-9_"\047-]+/)) {
    key = substr(line, RSTART, RLENGTH)
    rest = substr(line, RLENGTH + 1)
    if (rest ~ /^[ \t]*[.=]/) {
      gsub(/["\047]/, "", key)
      print kind, key
    }
  }
  if (line ~ /(^|[^A-Za-z0-9_-])package[ \t]*=/) print "package", line
}
AWKEOF

# scan_manifest <file>: sets SCAN to the awk output; a missing file or an awk failure exits 2.
scan_manifest() {
  if [ ! -f "$1" ]; then
    echo "check-boundaries: missing manifest $1" >&2
    exit 2
  fi
  SCAN="$(awk "$AWK_DEPS" "$1")" || {
    echo "check-boundaries: awk failed on $1" >&2
    exit 2
  }
}

# Rule 7: the core declares no dependency of any kind.
scan_manifest crates/lune-ui-core/Cargo.toml
if [ -n "$SCAN" ]; then
  while IFS= read -r l; do
    echo "rule 7: crates/lune-ui-core/Cargo.toml: declares ${l%% *} ${l#* }"
    fail=1
  done <<EOF2
$SCAN
EOF2
fi

# Rule 8: no third-party crate path in any file under the core.
THIRD='wgpu|winit|glyphon|cosmic_text|bytemuck|png|window_vibrancy|objc2[[:alnum:]_]*|raw_window_handle|pollster'
check 8 no -rn -E \
  -e "(^|[^[:alnum:]_])($THIRD)::" \
  -e '(^|[^[:alnum:]_])extern[[:space:]]+crate([^[:alnum:]_]|$)' \
  -e "(^|[^[:alnum:]_])use[[:space:]]+($THIRD)([^[:alnum:]_]|$)" \
  crates/lune-ui-core

# Rule 9: registered crates and their allowed dependency names (normal, then dev-only extras).
# Adapters name no lune-ui-* crate but lune-ui-core; only the facade names the adapters.
crate_deps() {
  case "$1" in
    lune-ui-core) echo "" ;;
    lune-ui-wgpu) echo "lune-ui-core wgpu glyphon bytemuck png" ;;
    lune-ui-winit) echo "lune-ui-core winit" ;;
    lune-ui-platform) echo "lune-ui-core winit window-vibrancy objc2-app-kit raw-window-handle" ;;
    lune-ui) echo "lune-ui-core lune-ui-wgpu lune-ui-winit lune-ui-platform" ;;
    *) return 1 ;;
  esac
}
crate_dev_deps() {
  case "$1" in
    lune-ui-wgpu) echo "pollster" ;;
    *) echo "" ;;
  esac
}
REGISTERED="lune-ui-core lune-ui-wgpu lune-ui-winit lune-ui-platform lune-ui"
for name in $REGISTERED; do
  if [ ! -d "crates/$name" ]; then
    echo "check-boundaries: registered crate dir crates/$name is missing" >&2
    exit 2
  fi
done
for dir in crates/*/; do
  name="$(basename "$dir")"
  if ! allowed="$(crate_deps "$name")"; then
    echo "rule 9: crates/$name: crate dir not registered in scripts/check-boundaries.sh"
    fail=1
    continue
  fi
  dev_allowed="$(crate_dev_deps "$name")"
  scan_manifest "crates/$name/Cargo.toml"
  [ -z "$SCAN" ] && continue
  while IFS= read -r l; do
    kind="${l%% *}"
    dep="${l#* }"
    case "$kind" in
      package)
        echo "rule 9: crates/$name/Cargo.toml: package rename: $dep"
        fail=1
        ;;
      dep)
        case " $allowed " in *" $dep "*) ;; *)
          echo "rule 9: crates/$name/Cargo.toml: dependency '$dep' not allowed"
          fail=1
          ;;
        esac
        ;;
      dev)
        case " $allowed $dev_allowed " in *" $dep "*) ;; *)
          echo "rule 9: crates/$name/Cargo.toml: dev-dependency '$dep' not allowed"
          fail=1
          ;;
        esac
        ;;
    esac
  done <<EOF2
$SCAN
EOF2
done
# A rename in the root manifest's [workspace.dependencies] would be inherited by name.
scan_manifest Cargo.toml
if [ -n "$SCAN" ]; then
  while IFS= read -r l; do
    if [ "${l%% *}" = package ]; then
      echo "rule 9: Cargo.toml: package rename in a dependency table: ${l#* }"
      fail=1
    fi
  done <<EOF2
$SCAN
EOF2
fi

# Rule 10: OS predicates in cfg outside platform/ and material/ directories. A line is an
# offender when it holds cfg(/cfg!(/cfg_attr( with an OS predicate; the one exception is a whole
# `#[cfg(...)]` attribute line whose next code line (blank lines, comments and other attributes
# skipped) is `mod <name>;` or a `use` declaration.
read -r -d '' AWK_CFG <<'AWKEOF' || true
function os_cfg(s) {
  if (s !~ /(cfg\(|cfg!\(|cfg_attr\()/) return 0
  if (s ~ /(target_os|target_family|target_vendor)/) return 1
  if (s ~ /(^|[^A-Za-z0-9_])(windows|unix)([^A-Za-z0-9_]|$)/) return 1
  return 0
}
function flush(   ) {
  if (pending != "") {
    print FILENAME ":" pline ": " pending
    bad = 1
    pending = ""
  }
}
{
  t = $0
  sub(/^[ \t]+/, "", t)
  if (t == "" || t ~ /^\/\//) next
  if (t ~ /^#\[/) {
    if (os_cfg(t)) {
      if (t ~ /^#\[cfg\(.*\)\]$/) {
        flush()
        pending = t
        pline = FNR
      } else {
        flush()
        print FILENAME ":" FNR ": " t
        bad = 1
      }
    }
    next
  }
  if (pending != "") {
    if (t ~ /^(pub(\([a-z]+\))?[ \t]+)?(mod[ \t]+[A-Za-z0-9_]+[ \t]*;|use[ \t])/) {
      pending = ""
    } else {
      flush()
    }
  }
  if (os_cfg(t)) {
    print FILENAME ":" FNR ": " t
    bad = 1
  }
}
END { flush(); exit(bad ? 1 : 0) }
AWKEOF

if ! cfg_files="$(find crates -type f -name '*.rs' -path '*/src/*' | sort)"; then
  echo "check-boundaries: find failed in rule 10" >&2
  exit 2
fi
if [ -z "$cfg_files" ]; then
  echo "check-boundaries: rule 10 found no source files under crates/*/src" >&2
  exit 2
fi
while IFS= read -r f; do
  case "$f" in */platform/* | */material/*) continue ;; esac
  rc=0
  hits="$(awk "$AWK_CFG" "$f")" || rc=$?
  if [ "$rc" -gt 1 ]; then
    echo "check-boundaries: awk failed (exit $rc) in rule 10: $f" >&2
    exit 2
  fi
  if [ "$rc" -eq 1 ]; then
    printf '%s\n' "$hits" | sed 's/^/rule 10: /'
    fail=1
  fi
done <<EOF2
$cfg_files
EOF2

# Rule 11: the facade crate holds only lib.rs, with no item definitions.
if [ ! -f crates/lune-ui/src/lib.rs ]; then
  echo "check-boundaries: missing crates/lune-ui/src/lib.rs" >&2
  exit 2
fi
for f in crates/lune-ui/src/*; do
  if [ "$f" != crates/lune-ui/src/lib.rs ]; then
    echo "rule 11: $f: the facade holds only lib.rs"
    fail=1
  fi
done
rc=0
out="$(grep -n -E '(^|[^[:alnum:]_])(fn|struct|enum|trait|impl|const|static|type|macro_rules)([^[:alnum:]_]|$)' crates/lune-ui/src/lib.rs 2>&1)" || rc=$?
if [ "$rc" -gt 1 ]; then
  echo "check-boundaries: grep failed (exit $rc) in rule 11: $out" >&2
  exit 2
fi
if [ "$rc" -eq 0 ]; then
  while IFS= read -r line; do
    text="${line#*:}"
    case "$(printf '%s' "$text" | sed 's/^[[:space:]]*//')" in '//'*) continue ;; esac
    echo "rule 11: crates/lune-ui/src/lib.rs:${line%%:*}: $text"
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

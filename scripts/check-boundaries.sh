#!/usr/bin/env bash
# Library boundary rules for the crates under crates/ (ported from Luneterm's
# scripts/check-lib-boundaries.sh). Runs the same on macOS (bash 3.2, BSD grep/awk) and Linux (GNU
# grep/awk). Prints each offending line with its rule number. Exit 1 on any offender, 0 when clean,
# 2 when a scanned path is missing or a tool (grep, awk, find, cargo) is missing or fails: a bad path
# never passes. A workspace cargo refuses to load is a rule 9 offender (its edges cannot be read).
#
# Threat model: the rules catch honest mistakes and fail closed on anything unusual (a form the
# script cannot read is an offender, not a pass). They are not a sandbox against deliberate evasion.
# Dependency edges and build targets come from cargo itself (`cargo metadata --no-deps`), so every
# TOML spelling cargo accepts is read the way cargo reads it.
#
#   1  no crates/*/Cargo.toml names an app (luneterm, lunegit)
#   2  no file under crates/ names luneterm
#   3  no file under crates/ names app or terminal words; exceptions are the fixed strings in
#      scripts/boundaries.allow
#   4  no crates/*/src file reads the clock or the environment (keeps frames deterministic)
#   5  every crates/*/Cargo.toml inherits the licence from the workspace
#   6  `unsafe` appears under crates/*/src only under crates/lune-ui-platform/src/platform/ or
#      crates/lune-ui-platform/src/material/; every other crate's src/lib.rs has
#      #![forbid(unsafe_code)] at column 0, preceded only by blank lines, `//` comments and
#      single-line inner attributes (no block comment, no item)
#   7  crates/lune-ui-core declares no dependency of any kind (per cargo metadata)
#   8  no file under crates/lune-ui-core names a third-party crate path (wgpu::, winit::, ...) or
#      `extern crate`
#   9  every crate dir under crates/ is registered below with its allowed dependency names and is a
#      workspace member whose package name is its dir name; each dependency cargo reports for it
#      (normal, build, dev, any target) is within its lists; a path dependency points at
#      crates/<its name>; a rename (`package =`) is an offender; the root Cargo.toml has no
#      `package =` key, [patch] or [replace] table, multi-line string or backslash escape
#   10 no OS predicate (target_os, target_family, target_vendor, windows, unix) inside cfg(, cfg!(
#      or cfg_attr( in crates/*/src outside a /platform/ or /material/ directory, unless it is a
#      #[cfg(...)] attribute whose next code line is `mod <name>;` or a `use` declaration; a
#      statement wrapped over several lines is joined before it is tested (strings and comments
#      skipped when counting brackets); one still open after 40 lines or at end of file is an
#      offender
#   11 crates/lune-ui/src holds only lib.rs, with no item definitions (fn, struct, enum, trait,
#      impl, const, static, type, macro_rules)
#   12 compiled code comes only from the scanned files: each member crate's build targets are its
#      src/lib.rs and the modules it reaches (no [lib] path, bin, example, bench or build script;
#      tests/ files are not library code and are not scanned), and no crates/*/src file has a
#      `path = "..."` attribute or an include!( (include_str! and include_bytes! are allowed); a
#      comment line or a `let path =` binding is not an attribute
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
# Every crate but the platform crate forbids unsafe code at the crate root. The forbid line must
# be in effect: everything above it is a blank line, a `//` comment or a single-line inner
# attribute, so it is not inside a block comment or a string. Exit 0 found, 1 missing (no forbid
# line anywhere), 3 something else comes first (printed as `<line>: <text>`, the first such line).
read -r -d '' AWK_FORBID <<'AWKEOF' || true
BEGIN { found = 0; bad = "" }
/^#!\[forbid\(unsafe_code\)\]$/ { found = 1; exit }
{
  if (bad != "") next
  t = $0
  sub(/^[ \t]+/, "", t)
  sub(/[ \t]+$/, "", t)
  if (t == "" || t ~ /^\/\//) next
  if (t ~ /^#!\[[A-Za-z_]+(\([A-Za-z0-9_, ]*\))?\]$/) next
  bad = FNR ": " $0
}
END {
  if (!found) exit 1
  if (bad != "") { print bad; exit 3 }
  exit 0
}
AWKEOF
for dir in crates/*/; do
  name="$(basename "$dir")"
  [ "$name" = lune-ui-platform ] && continue
  lib="crates/$name/src/lib.rs"
  if [ ! -f "$lib" ]; then
    echo "rule 6: $lib: missing, cannot check #![forbid(unsafe_code)]"
    fail=1
    continue
  fi
  rc=0
  first="$(awk "$AWK_FORBID" "$lib")" || rc=$?
  case "$rc" in
    0) ;;
    1)
      echo "rule 6: $lib: no #![forbid(unsafe_code)]"
      fail=1
      ;;
    3)
      echo "rule 6: $lib:$first: comes before #![forbid(unsafe_code)] (only blank lines, // comments and inner attributes may)"
      fail=1
      ;;
    *)
      echo "check-boundaries: awk failed (exit $rc) in rule 6: $lib" >&2
      exit 2
      ;;
  esac
done

# Dependencies and build targets (rules 7, 9, 12) as cargo reads them: `cargo metadata --no-deps`
# resolves nothing (no lock write, no network) and reports every declared dependency with its
# real package name, rename, kind and target, and every build target with its source path. A
# strict JSON reader flattens it into tab-separated records:
#   pkg <manifest> <package name>
#   dep <manifest> <normal|dev|build> <package> <rename|-> <target|-> <path|->
#   target <manifest> <kinds, comma-separated> <src_path>
# Paths are relative to the workspace root. JSON it cannot read makes awk exit 2.
read -r -d '' AWK_META <<'AWKEOF' || true
function die(m) {
  printf "check-boundaries: cargo metadata JSON: %s at byte %d\n", m, pos > "/dev/stderr"
  exit 2
}
function skip_ws(   c) {
  while (pos <= n) {
    c = substr(s, pos, 1)
    if (c != " " && c != "\t" && c != "\n" && c != "\r") break
    pos++
  }
}
# parse_string: at an opening quote; returns the contents with \" \\ \/ decoded and the other
# escapes kept as written (no raw control character reaches the output).
function parse_string(   out, c, e) {
  pos++
  out = ""
  while (pos <= n) {
    if (match(substr(s, pos, 256), /^[^"\\]+/)) {
      out = out substr(s, pos, RLENGTH)
      pos += RLENGTH
      continue
    }
    c = substr(s, pos, 1)
    if (c == "\"") { pos++; return out }
    e = substr(s, pos + 1, 1)
    if (e == "\"" || e == "\\" || e == "/") out = out e
    else if (e ~ /^[bfnrtu]$/) out = out "\\" e
    else die("bad escape")
    pos += 2
  }
  die("unterminated string")
}
function parse_value(path,   c, k, i) {
  skip_ws()
  c = substr(s, pos, 1)
  if (c == "{") {
    pos++
    skip_ws()
    if (substr(s, pos, 1) == "}") { pos++; return }
    while (1) {
      skip_ws()
      if (substr(s, pos, 1) != "\"") die("expected a key")
      k = parse_string()
      skip_ws()
      if (substr(s, pos, 1) != ":") die("expected ':'")
      pos++
      parse_value(path "/" k)
      skip_ws()
      c = substr(s, pos, 1)
      pos++
      if (c == "}") return
      if (c != ",") die("expected ',' or '}'")
    }
  }
  if (c == "[") {
    pos++
    skip_ws()
    if (substr(s, pos, 1) == "]") { pos++; return }
    for (i = 0; ; i++) {
      parse_value(path "/" i)
      skip_ws()
      c = substr(s, pos, 1)
      pos++
      if (c == "]") return
      if (c != ",") die("expected ',' or ']'")
    }
  }
  if (c == "\"") { keep(path, parse_string(), "s"); return }
  if (match(substr(s, pos, 64), /^(true|false|null|-?[0-9]+(\.[0-9]+)?([eE][-+]?[0-9]+)?)/)) {
    keep(path, substr(s, pos, RLENGTH), "l")
    pos += RLENGTH
    return
  }
  die("unexpected character")
}
# keep: stores the fields the rules read; everything else is parsed and dropped.
function keep(path, v, t,   a, c, p, j) {
  c = split(path, a, "/")
  if (c == 2 && a[2] == "workspace_root") { wsroot = v; return }
  if (c < 4 || a[2] != "packages" || a[3] !~ /^[0-9]+$/) return
  p = a[3] + 0
  if (p + 1 > np) np = p + 1
  if (c == 4 && (a[4] == "name" || a[4] == "manifest_path")) { pk[p, a[4]] = v; return }
  if (c == 6 && a[4] == "dependencies" && a[5] ~ /^[0-9]+$/) {
    j = a[5] + 0
    if (j + 1 > nd[p]) nd[p] = j + 1
    if (t == "l" && v == "null") v = "-"
    if (v == "") v = "\"\""
    dp[p, j, a[6]] = v
    return
  }
  if (a[4] == "targets" && a[5] ~ /^[0-9]+$/) {
    j = a[5] + 0
    if (j + 1 > nt[p]) nt[p] = j + 1
    if (c == 6 && a[6] == "src_path") tp[p, j] = v
    if (c == 7 && a[6] == "kind") tk[p, j] = (tk[p, j] == "" ? "" : tk[p, j] ",") v
  }
}
function rel(f) {
  if (index(f, wsroot "/") == 1) return substr(f, length(wsroot) + 2)
  return f
}
{ s = s $0 "\n" }
END {
  n = length(s)
  pos = 1
  parse_value("")
  skip_ws()
  if (pos <= n) die("trailing data")
  if (wsroot == "") die("no workspace_root")
  for (p = 0; p < np; p++) {
    if (pk[p, "manifest_path"] == "" || pk[p, "name"] == "") die("package " p " has no name or manifest_path")
    m = rel(pk[p, "manifest_path"])
    print "pkg\t" m "\t" pk[p, "name"]
    for (j = 0; j < nd[p]; j++) {
      if (!((p, j, "name") in dp) || !((p, j, "kind") in dp)) die("dependency " j " of " m " has no name or kind")
      k = dp[p, j, "kind"]
      if (k == "-") k = "normal"
      r = ((p, j, "rename") in dp) ? dp[p, j, "rename"] : "-"
      g = ((p, j, "target") in dp) ? dp[p, j, "target"] : "-"
      pa = ((p, j, "path") in dp) ? dp[p, j, "path"] : "-"
      if (pa != "-") pa = rel(pa)
      print "dep\t" m "\t" k "\t" dp[p, j, "name"] "\t" r "\t" g "\t" pa
    }
    for (j = 0; j < nt[p]; j++) {
      if (tk[p, j] == "" || tp[p, j] == "") die("target " j " of " m " has no kind or src_path")
      print "target\t" m "\t" tk[p, j] "\t" rel(tp[p, j])
    }
  }
}
AWKEOF

# A missing cargo or output the reader can't parse is a tool failure (exit 2). A workspace cargo
# refuses to load is a manifest offender: its dependencies can't be read, so rule 9 fails (exit 1).
if ! command -v cargo > /dev/null 2>&1; then
  echo "check-boundaries: cargo not found (rules 7, 9, 12 read cargo metadata)" >&2
  exit 2
fi
META=""
if META_JSON="$(cargo metadata --format-version 1 --no-deps --offline 2>/dev/null)"; then
  if ! META="$(printf '%s\n' "$META_JSON" | awk "$AWK_META")" || [ -z "$META" ]; then
    echo "check-boundaries: could not read cargo metadata output" >&2
    exit 2
  fi
else
  errtext="$(cargo metadata --format-version 1 --no-deps --offline 2>&1 > /dev/null || true)"
  err="$(printf '%s\n' "$errtext" | grep -m 1 '^error' || true)"
  case "$errtext" in
    *Cargo.toml*) ;;
    *)
      # No manifest named: cargo itself failed (toolchain, network), not a manifest.
      printf 'check-boundaries: cargo metadata failed:\n%s\n' "$errtext" >&2
      exit 2
      ;;
  esac
  bad_man="$(printf '%s\n' "$errtext" | grep -o -m 1 'crates/[A-Za-z0-9_.-]*/Cargo.toml' || true)"
  echo "rule 9: ${bad_man:-Cargo.toml}: cargo cannot load the workspace, dependencies unchecked: ${err:-cargo metadata failed}"
  if [ "$bad_man" = crates/lune-ui-core/Cargo.toml ]; then
    echo "rule 7: $bad_man: cargo cannot load it, so it is not shown to declare no dependency"
  fi
  fail=1
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
TAB="$(printf '\t')"
NL="
"
for dir in crates/*/; do
  name="$(basename "$dir")"
  if ! crate_deps "$name" > /dev/null; then
    echo "rule 9: crates/$name: crate dir not registered in scripts/check-boundaries.sh"
    fail=1
    continue
  fi
  [ -z "$META" ] && continue # cargo failed: reported above
  case "$META$NL" in
    *"pkg${TAB}crates/$name/Cargo.toml${TAB}$name$NL"*) ;;
    *)
      echo "rule 9: crates/$name/Cargo.toml: not a workspace member named '$name' in cargo metadata"
      fail=1
      ;;
  esac
done
while IFS="$TAB" read -r typ man f1 f2 f3 f4 f5; do
  case "$man" in crates/*/Cargo.toml) ;; *) continue ;; esac
  name="${man#crates/}"
  name="${name%/Cargo.toml}"
  allowed="$(crate_deps "$name")" || continue # unregistered: reported above
  dev_allowed="$(crate_dev_deps "$name")"
  case "$typ" in
    dep)
      kind="$f1" dep="$f2" ren="$f3" tgt="$f4" dpath="$f5"
      where=""
      [ "$tgt" != - ] && where=" (target $tgt)"
      if [ "$name" = lune-ui-core ]; then
        echo "rule 7: $man: declares $kind dependency '$dep'$where"
        fail=1
      fi
      if [ "$ren" != - ]; then
        echo "rule 9: $man: package rename: '$ren' is package '$dep'$where"
        fail=1
      fi
      if [ "$dpath" != - ] && [ "$dpath" != "crates/$dep" ]; then
        echo "rule 9: $man: path dependency '$dep' points at $dpath, not crates/$dep"
        fail=1
      fi
      case "$kind" in
        normal | build) list=" $allowed " ;;
        dev) list=" $allowed $dev_allowed " ;;
        *)
          echo "rule 9: $man: dependency '$dep' of unknown kind '$kind'"
          fail=1
          continue
          ;;
      esac
      case "$list" in *" $dep "*) ;; *)
        echo "rule 9: $man: $kind dependency '$dep' not allowed$where"
        fail=1
        ;;
      esac
      ;;
    target)
      tkind="$f1" src="$f2"
      ok=0
      case "$tkind:$src" in
        "lib:crates/$name/src/lib.rs") ok=1 ;;
        "test:crates/$name/tests/"*) case "$src" in *..*) ;; *) ok=1 ;; esac ;;
      esac
      if [ "$ok" -eq 0 ]; then
        echo "rule 12: $man: build target $tkind at $src (only src/lib.rs and tests/ are scanned)"
        fail=1
      fi
      ;;
  esac
done <<EOF2
$META
EOF2
# The root manifest: a rename in [workspace.dependencies] would be inherited (cargo metadata shows
# it on the members that use it, but an unused one is rejected too); [patch] and [replace] would
# redirect an edge; a multi-line string or an escape is a form this scan does not read.
check 9 no -H -n -E \
  -e "(^|[^A-Za-z0-9_-])[\"']?package[\"']?[[:space:]]*=" \
  -e "^[[:space:]]*\[+[[:space:]]*[\"']?(patch|replace)[]\"'.[:space:]]" \
  -e '"""' -e "'''" -e '\\' \
  Cargo.toml

# Rule 10: OS predicates in cfg outside platform/ and material/ directories. A statement is an
# offender when it holds cfg(/cfg!(/cfg_attr( (also `cfg!` with [] or {}) with an OS predicate; the
# one exception is a whole `#[cfg(...)]` attribute whose next code line (blank lines, comments and
# other attributes skipped) is `mod <name>;` or a `use` declaration. A statement in which a `#[` or
# any cfg trigger opens a bracket that does not close on its line is joined with the following
# lines (strings, raw strings and comments are skipped when counting); one still open after 40
# lines or at end of file is an offender. The first line's number is reported.
read -r -d '' AWK_CFG <<'AWKEOF' || true
BEGIN { CFGRE = "cfg(_attr)?[ \t]*\\(|cfg[ \t]*![ \t]*[[({]" }
function os_cfg(s) {
  if (s !~ CFGRE) return 0
  if (s ~ /(target_os|target_family|target_vendor)/) return 1
  if (s ~ /(^|[^A-Za-z0-9_])(windows|unix)([^A-Za-z0-9_]|$)/) return 1
  return 0
}
# open_at: 1 when the first bracket group that starts at or after `from` in s is still open at the
# end of s, else 0. Skips "..." (with \ escapes), r"..." and r#"..."# strings, /* */ comments and
# `//` comments up to the next newline. An unterminated string or comment counts as open.
function open_at(s, from,   i, c, d, seen, L, h, j, q, rest) {
  d = 0
  seen = 0
  L = length(s)
  for (i = from; i <= L; i++) {
    c = substr(s, i, 1)
    if (c == "\"") {
      h = 0
      j = i - 1
      while (j >= 1 && substr(s, j, 1) == "#") { h++; j-- }
      if (j >= 1 && substr(s, j, 1) == "r" && (j == 1 || substr(s, j - 1, 1) !~ /[A-Za-z0-9_]/ || substr(s, j - 1, 1) == "b")) {
        q = "\""
        while (h-- > 0) q = q "#"
        rest = substr(s, i + 1)
        if (!(j = index(rest, q))) return 1
        i += j + length(q) - 1
        continue
      }
      for (i++; i <= L; i++) {
        c = substr(s, i, 1)
        if (c == "\\") { i++; continue }
        if (c == "\"") break
      }
      if (i > L) return 1
      continue
    }
    if (c == "/" && substr(s, i + 1, 1) == "*") {
      rest = substr(s, i + 2)
      if (!(j = index(rest, "*/"))) return 1
      i += j + 2
      continue
    }
    if (c == "/" && substr(s, i + 1, 1) == "/") {
      rest = substr(s, i + 2)
      if (!(j = index(rest, "\n"))) return (seen && d > 0)
      i += j + 1
      continue
    }
    if (c ~ /[[({]/) { d++; seen = 1 }
    else if (c ~ /[])}]/) d--
    if (seen && d <= 0) return 0
  }
  return (seen && d > 0)
}
# stmt_open: 1 when a leading `#[` or any cfg trigger in s opens a group s does not close.
function stmt_open(s,   off, rest) {
  if (s ~ /^#\[/ && open_at(s, 1)) return 1
  off = 0
  rest = s
  while (match(rest, CFGRE)) {
    if (open_at(s, off + RSTART)) return 1
    off += RSTART + RLENGTH - 1
    rest = substr(s, off + 1)
  }
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
  ln = FNR
  n = 0
  stuck = ""
  while (stmt_open(t)) {
    if (n >= 40) { stuck = "still open after 40 lines"; break }
    if ((getline nxt) <= 0) { stuck = "still open at end of file"; break }
    n++
    sub(/^[ \t]+/, "", nxt)
    if (nxt == "" || nxt ~ /^\/\//) continue
    t = t "\n" nxt
  }
  gsub(/\n/, " ", t)
  if (stuck != "") {
    flush()
    print FILENAME ":" ln ": statement " stuck ", cannot be tested: " substr(t, 1, 120)
    bad = 1
    next
  }
  if (t ~ /^#\[/) {
    if (os_cfg(t)) {
      if (t ~ /^#\[cfg\(.*\)\]$/) {
        flush()
        pending = t
        pline = ln
      } else {
        flush()
        print FILENAME ":" ln ": " t
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
    print FILENAME ":" ln ": " t
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

# Rule 12 (sources): a `path = "..."` attribute (also inside cfg_attr) or include!( would compile a
# file the rules above never scan. The target half of rule 12 runs with rule 9 above.
# A comment line (`//`, doc comments included) and a `let path =` binding are not attributes, so
# their hits are dropped; any other hit, or a grep error (exit 2), is reported as before.
rc12=0
out12="$(grep -rn -E \
  -e '(^|[^[:alnum:]_])path[[:space:]]*=[[:space:]]*b?r?#*"' \
  -e '(^|[^[:alnum:]_])include[[:space:]]*!' \
  crates/*/src 2>&1)" || rc12=$?
if [ "$rc12" -gt 1 ]; then
  echo "check-boundaries: grep failed (exit $rc12) in rule 12: $out12" >&2
  exit 2
fi
if [ "$rc12" -eq 0 ]; then
  out12="$(printf '%s\n' "$out12" | grep -v -E \
    -e '^[^:]*:[0-9]+:[[:space:]]*//' \
    -e '(^|[^[:alnum:]_])let[[:space:]]+(mut[[:space:]]+)?path[[:space:]]*=' || true)"
  if [ -n "$out12" ]; then
    printf '%s\n' "$out12" | sed "s/^/rule 12: /"
    fail=1
  fi
fi

if [ "$fail" -ne 0 ]; then
  echo "check-boundaries: FAILED" >&2
  exit 1
fi
echo "check-boundaries: clean"

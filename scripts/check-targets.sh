#!/usr/bin/env bash
# Cross-target checks. Usage: scripts/check-targets.sh [--skip-linux]
#   windows  cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings
#            (needs that rustup target installed; prints the `rustup target add` hint when it is not)
#   linux    fmt --check, clippy --locked -D warnings, test --locked, in a rust:1.92 container with
#            the repo mounted read-only. Target, cargo and rustup state live in the named volumes
#            luneui-linux-{target,cargo,rustup}: created if missing, never removed. The container
#            is --rm. Skipped (not failed) when docker is missing, the daemon is not running, or
#            --skip-linux is given. This script never starts Docker.
# Prints `windows: passed|FAILED` and `linux: passed|FAILED|skipped (<reason>)`.
# Exit codes: 0 every half that ran passed; 1 a check failed; 2 usage error.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

WIN_TARGET=x86_64-pc-windows-msvc
skip_linux=0
for arg in "$@"; do
  case "$arg" in
    --skip-linux) skip_linux=1 ;;
    *) echo "usage: scripts/check-targets.sh [--skip-linux]" >&2; exit 2 ;;
  esac
done

failed=0

# Windows half.
installed="$(rustup target list --installed 2>/dev/null || true)"
if grep -qx "$WIN_TARGET" <<< "$installed"; then
  if cargo clippy --workspace --all-targets --locked --target "$WIN_TARGET" -- -D warnings; then
    echo "windows: passed"
  else
    echo "windows: FAILED"
    failed=1
  fi
else
  echo "windows: FAILED (run: rustup target add $WIN_TARGET)"
  failed=1
fi

# Linux half.
ensure_volumes() {
  local v
  for v in target cargo rustup; do
    docker volume inspect "luneui-linux-$v" >/dev/null 2>&1 \
      || docker volume create "luneui-linux-$v" >/dev/null || return 1
  done
}

if [ "$skip_linux" -eq 1 ]; then
  echo "linux: skipped (--skip-linux)"
elif ! command -v docker >/dev/null 2>&1; then
  echo "linux: skipped (docker not installed)"
elif ! docker info >/dev/null 2>&1; then
  echo "linux: skipped (docker daemon not running)"
elif ! ensure_volumes; then
  echo "linux: FAILED (could not create luneui-linux-* volumes)"
  failed=1
else
  if docker run --rm \
    -v "$ROOT:/work:ro" \
    -v luneui-linux-target:/target \
    -v luneui-linux-cargo:/usr/local/cargo \
    -v luneui-linux-rustup:/usr/local/rustup \
    -e CARGO_TARGET_DIR=/target \
    -w /work \
    rust:1.92 \
    bash -c '
      set -euo pipefail
      run() { echo "+ $*"; "$@" || { echo "FAILED: $*" >&2; exit 1; }; }
      run cargo fmt --all -- --check
      run cargo clippy --workspace --all-targets --locked -- -D warnings
      run cargo test --workspace --locked
      echo "check-targets linux: all passed"
    '; then
    echo "linux: passed"
  else
    echo "linux: FAILED"
    failed=1
  fi
fi

exit "$failed"

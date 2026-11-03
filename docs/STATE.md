<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-04 (M1 extraction, step 9 docs)

## Now

M1 (extraction) is implemented on branch `orch/toolkit-v1`, awaiting review and merge. `crates/lune-ui`
is Luneterm 296cfdc's `lune-ui`, moved as is (D10), proven by Luneterm's 18-scene spike gate at zero
delta (`scripts/pixel-gate-luneterm.sh`). Tooling every later milestone gates on: `tools/dump-compare`,
`apps/gallery` (15 reference scenes, `--dump`, `--list`), `scripts/pixel-gate.sh`,
`scripts/check-targets.sh`. Licences and `THIRD_PARTY_NOTICES.md` are in; DECISIONS D9-D23 record the
crate split, the dependencies and the rule-1 exception; the 26 `lune-ui` catalogue entries are
`in-progress`. `docs/architecture.md` holds the module map, the known deviations and the gates.

Wane is a third consumer (2026-11-04): 20 `planned` catalogue entries from its UI spec, `wane` added to
15 existing entries, and requirements R-platform-6, R-widget-4 and R-data-2.

## Next

1. M2 seams (D9): split `lune-ui` into `lune-ui-core`, `lune-ui-wgpu`, `lune-ui-winit`,
   `lune-ui-platform` and the facade, each seam a gated step with a contract test and a fake. Fixes the
   known deviations: inline `cfg(target_os)` in `gpu.rs` and `window.rs`, and `Gpu::new` needing a window.
2. A planner deep-analyses the `lunegit.*` stubs (and the stubs in library areas) by reading Lunegit's
   code, then replaces each "Rough stub" body with real needs.
3. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
4. `scripts/export-snapshot.sh` for Luneterm's vendored snapshot (D13), before Luneterm M9.

## Blockers / open questions

- None. The Luneterm consumption blocker is cleared by D13 (vendored snapshot until a remote exists).

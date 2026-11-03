<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-04 (M2 review round 2: dependency rules read cargo metadata; rule 12 pins compiled sources)

## Now

M2 (seams and crate split) is implemented on branch `orch/toolkit-v1`, awaiting review and merge. The
toolkit is five crates: pure `lune-ui-core` (no dependencies: painter, `Scene`, widgets, theme, seam
types and the `testing` fakes), adapters `lune-ui-wgpu`, `lune-ui-winit`, `lune-ui-platform`, and the
facade `lune-ui` that keeps every old module path. The four seams (display list, text, input and
output, OS glass) each have a core model, a fake and a contract test. One `Gpu` and one `TextCache`
serve any number of windows and headless targets; the gallery's `--dump` runs headless, with the 15
scenes at zero pixel delta against M1. `scripts/check-boundaries.sh` enforces rules 1 to 12 (core
purity, adapter dependency lists, `unsafe` and OS-cfg placement, facade only; dependency edges and build targets read from `cargo metadata`, multi-line `cfg` included). DECISIONS D24 to D39
record the M2 choices; `docs/architecture.md` holds the module map, seam table and multi-window model;
the catalogue has five new entries (`core.scene`, `core.input`, `core.testing`, `render.surface`,
`platform.input`).

Window-mode code (surfaces, present, output commands, OS glass) is compile- and review-checked only:
no gate opens a visible window.

Wane is a third consumer (2026-11-04): 20 `planned` catalogue entries from its UI spec, `wane` added to
15 existing entries, and requirements R-platform-6, R-widget-4 and R-data-2.

## Next

1. M3 (roadmap): catalogue analysis, docs only: deep-analyse the `lunegit.*` stubs and the stubs in
   library areas by reading Lunegit's code, confirm the planned entries against Wane's spec.
2. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
3. `scripts/export-snapshot.sh` for Luneterm's vendored snapshot (D13), before Luneterm M9. The
   migration map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None. Wane's OS behaviours (close-to-tray, single instance, autostart, notifications; R-platform-6)
  are assumed app-side and documented (answer 2026-11-03, reversible).

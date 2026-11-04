<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-04 (M4 review fix rounds 1-2: focus stack across overlays, Repaint catch-up, passive overlays, bench on a busy machine, a repeated focus request settles, bench self-test below 4 ms exits 2; D81-D85)

## Now

M4 (interaction and motion) is implemented on branch `orch/toolkit-v1`, awaiting the acceptance
bench, review and merge, on top of M3 (catalogue specs) and M2 (seams and crate split).
`lune-ui-core` now holds the per-window `Ui` frame (stable ids, retained state with keep-alive
scopes, layered hits with modal and pointer capture, `Response`, overlays with exit retention, focus
scopes, the semantic tree with actions, the string table, `FrameOutput` with `Repaint`) and the
closed-form motion system (easings, tween, spring, keyframes, velocity-preserving transitions, the
Full, Reduced and Off tiers, theme motion tokens, a 60 Hz conformance harness). Every moved widget
and overlay has a `Look`, a `paint` function, its old Painter-API wrapper (same pixels) and a `Ui`
method; rest identity is proven by exact `Scene` equality tests. `docs/architecture.md` holds the
module map, "The `Ui` frame" and "API changes since M1"; DECISIONS D59 to D85 record the M4 choices
(D81-D85 from review fix round 1).
The gallery has a page registry (two legacy pages, 15 Ui pages), `--dump`, `--list` (62 scene names)
and `--bench`. The cross-widget contract suite is `crates/lune-ui-core/tests/c4.rs`.
The pixel gate shows the 15 legacy scenes identical against the M1 base.

## Next

1. Run the acceptance bench (`cargo run --release -p gallery -- --bench`, 5 runs, runnable on a
   busy machine per D85, plus `--size 3456x2234` report-only); a failure is reported with per-page numbers, never loosened. Then review
   and merge M4.
2. M5 onward per the roadmap: text input (typing, search clear, IME composition) and the clipboard
   fake, then the remaining controls (button variants, the radio) and the layout and data
   components.
3. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
4. An export script for Luneterm's vendored snapshot, before Luneterm M9. The migration
   map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None. Deferred by decision (D74): button variants and the radio, text-field variants, typing and
  composition. Nested popups above a modal (a combo inside a dialog) are not supported: an overlay
  draws on the layer it names, and a later overlay's text on the same layer would draw under earlier
  fills; revisit with the dialog form variants.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed
  app-side and documented (answer 2026-11-03, reversible).

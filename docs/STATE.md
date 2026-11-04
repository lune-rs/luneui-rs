<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-04 (M5 steps 1 to 11 done; step 12 is next; M4 merged at d9fe71f)

## Now

M4 (interaction and motion) is merged to main at d9fe71f, on top of M3 (catalogue specs) and M2
(seams and crate split). M5 (foundations: layout, scroll, list, popover, text editing, drag and drop,
shortcuts, painter primitives) is in progress on branch `orch/toolkit-v1`; steps 1 to 11 (the frame
plumbing, layout helpers, shortcut registry, vector primitives, image and rotate, their gallery pages, the
scroll area and scrollbar, the virtualized list, popover and tooltip, and the gallery pages of those four, and the text queries on `TextSystem`: caret, hit, lines,
segments, D86) are done; step 12 (`TextEdit`) is next.
`lune-ui-core` holds the per-window `Ui` frame (stable ids, retained state with keep-alive scopes,
layered hits with modal and pointer capture, `Response`, overlays with exit retention, focus scopes,
the semantic tree with actions, the string table, `FrameOutput` with `Repaint`) and the closed-form
motion system. M5 step 1 opened the frame to ordered focus input (`Ui::focus_events`, keys with
modifiers, text, IME and clipboard answers), the `Keymap`, press details on `Response`, cursor and
IME-area output, `Xform::translate` and `Painter::push_clip`/`pop_clip`. `layout` holds the pure
stack, row, column and grid helpers; `shortcuts` the scoped chord registry (preempt at begin, bubble at
finish, `FrameOutput.fired`). `Ui::scroll_area` (f64 offsets, wheel and trackpad, reveals, follow-tail,
identity-keyed preservation, the overlay thumb) is in `widgets/scroll_area.rs` and `scrollbar.rs`;
`Painter::push_translate` moves content and keeps clips put on screen. `Ui::virtual_list`
(`widgets/virtual_list.rs`) builds only the rows in view on an untranslated scroll area: fixed rows or
measured ones in a Fenwick tree, keyed selection in a persistent `ListState`, no row tween while the
content moves. `widgets/popover.rs` holds the pure `place` (side, alignment, gap, margin, flip, shift;
the dropdown and menu call it) and `Ui::popover` (popup layer, light dismiss, the menu's curves);
`widgets/tooltip.rs` holds `Ui::tooltip`/`tooltip_with` and `Ui::hover_dwell` (800 ms
`MotionTokens::tooltip_delay`, `LAZY_HOVER_DWELL` 150 ms).
`docs/architecture.md` holds the module map, "The `Ui` frame" and "API changes since M1"; DECISIONS
D59 to D85 record the M4 choices.
The gallery has a page registry (two legacy pages, 24 Ui pages), `--dump`, `--list` (123 scene names)
and `--bench`. The cross-widget contract suite is `crates/lune-ui-core/tests/c4.rs`.
The pixel gate shows the 15 legacy scenes identical against the M1 base.

## Next

1. M5 steps 12 to 16 per `m05-foundations/plan.md`: `TextEdit`, the text field
   upgrade, drag and drop, gallery pages, docs and the acceptance bench.
2. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
3. An export script for Luneterm's vendored snapshot, before Luneterm M9. The migration
   map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None. Deferred by decision (D74): button variants and the radio, text-field variants, typing and
  composition. Nested popups above a modal (a combo inside a dialog) are not supported: an overlay
  draws on the layer it names, and a later overlay's text on the same layer would draw under earlier
  fills; revisit with the dialog form variants.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed
  app-side and documented (answer 2026-11-03, reversible).

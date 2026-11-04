<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-04 (M5 merged to main at 1aeaf1b after six review rounds; paused by the user before M6)

## Now

M5 (foundations) is merged to main at 1aeaf1b, on top of M4 (interaction and motion, d9fe71f), M3 (catalogue specs) and M2
(seams and crate split). M5 added layout helpers, the shortcut registry, the seven painter primitives and the `ImageStore` seam, the scroll
area and scrollbar, the virtualized list, popover and tooltip, text queries on `TextSystem`, the `TextEdit`
engine, the text field and text area upgrade with IME, drag and drop, and 12 new gallery pages with
their scenes (152 scene names in `--list`). DECISIONS D91 to D109 record the M5 plan decisions (D90 the
IME scenes' tofu boxes); the catalogue entries of those components are `in-progress` with their open
questions answered or marked, and `docs/architecture.md` has the module map, the focus queue, `Keymap`, the
shortcut phases, the per-layer draw order, the `ImageStore` seam and "API changes since M1".
`lune-ui-core` holds the per-window `Ui` frame, the closed-form motion system and, since M5, `layout`,
`shortcuts`, `text_edit`, `drag`, `image` and the widgets `scroll_area`, `scrollbar`, `virtual_list`,
`popover`, `tooltip` and `text_area`; `lune-ui-wgpu` takes `unicode-segmentation` (the core still has no
dependency). The gallery has a page registry (two legacy pages, 27 Ui pages), `--dump`, `--list` and
`--bench` (M5 acceptance run at 9878b69, load about 4: 0 pages over budget, worst page p99 11.9 ms at 2080 x 1320. A rerun at the merged tip at load 55 to 63 put `ui`, `a11y` and `drag-and-drop` over; the user waived it as machine load, and no controlled comparison finished; details in the task notes). The cross-widget contract suite is
`crates/lune-ui-core/tests/c4.rs`. The pixel gate shows the 15 legacy scenes identical against the M1 base.

## Carried limits (M5 did not fix them)

- An overlay above a modal is unsupported: a popover or tooltip inside a dialog draws under it, and so does
  a combo inside one (flagged to M7, with the menu's node name).
- Text cannot be cut by a rounded clip (it gets the clip's bounding rect): M7's page and panel keep text out of the
  corners. Text draws above every fill of its layer, so labels show through a sliding tab during `dnd-slide`
  and `dnd-settle`.
- The gallery bundles no CJK font: the IME scenes' "かな" draws as boxes (D90).
- Inside a cluster of several glyphs a click lands on either edge non-monotonically: `Buffer::hit` halves
  each glyph, not the cluster.
- A drag stopped right of a row cut inside a word (a URL, a path) leaves one grapheme out until the pointer
  reaches the next row (P20; the seam has no caret affinity, D112).

## Unverified here

- Real typing with an input method (roadmap R1) and bidi caret placement.
- Windows and Linux run time (the lint runs through `scripts/check-targets.sh`, Linux only with Docker).
- A text field's clipboard: the gallery answers from an in-process string until M10 (D107).

## Next

1. M6 per `docs/` roadmap (data.stat, kv-list and progress moved there by Q3).
2. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
3. An export script for Luneterm's vendored snapshot, before Luneterm M9. The migration
   map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None. Deferred by decision (D74): button variants and the radio (the text-field variants, typing and
  composition are built, M5). Nested popups above a modal are listed under the carried limits.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed
  app-side and documented (answer 2026-11-03, reversible).

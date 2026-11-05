<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-05 (M8 built on `orch/toolkit-v1`, step 7 closed, review round 1 fixed (F1 to F3, D175); awaiting re-review and merge; M7 is merged to main at 662d310)

## Now

M8 (data views: table, bar chart, pie chart, kanban board, lane graph) is built on `orch/toolkit-v1` (7 steps, DECISIONS
D160 to D174) and closed: the cross-widget suite, the disabled-keys rows, the catalogue audit, the architecture notes and the
bench are in. It waits for review and the merge. `docs/architecture.md` has the module map, the pages and "API changes since M1".

What M8 added:
- Carried limits paid first (D160): a real arrow on a focused scroll area no longer scrolls (F18); the area's Page, Home and
  End keys serve a focused child in a disabled scope; `virtual_list` and the combo consume then gate (and leave chords to the
  shortcuts), each with tests and a mutant.
- `data.table` (`Ui::table`, D161 to D163): the list frame with a crate-private `ListStyle` (flat rows, hover through
  `pointer_in`), a header with sort requests, tones, mono and numeric columns, wrapping rows, controls in cells; two Tab stops
  (the sortable header, the body); 100,000 rows build only the window.
- `data.bar-chart` and `data.pie-chart` (D164 to D166): painter rectangles and `Painter::sector`, value-change transitions
  (`Kind::Value`, `live` snaps), shares normalised each frame so the sweeps total 2π.
- `data.lane-graph` (D170, D172): a stateless row painter; the dot is a closed 32-point stroke over the lines.
- `data.kanban-board` (D167 to D169, D173): data-driven board on `FreeDrag`; a ghost one layer group above, drop line in
  the gap, landing with `SETTLE`, auto-scroll, one roving stop for all cards, Shift with an arrow moves a card.
  `drag::edge_step` and `Ui::drag_ghost` are shared plumbing.
- Five pages, 27 scenes, one `c4::m8` section (roles, names and states, injected actions on a table header and row and on a
  card, tab stops, the disabled walk, Global-binding silence, idle at rest). Two visual batches against Wane's mocks and
  Lunegit's values were accepted in round 1; the director's fixes were the table-tones and table-sorted scenes, the bar
  legend spacing (12) and the lane dot (D172).

M7 (overlays and containers) is merged to main at 662d310 (DECISIONS D137 to D159): a sixth layer `LayerId::Raised`, the
menu, flyout, toasts, command palette, dialog variants, page, panel, sidebar, side panel, icon rail, split panes, tree view
and settings page; `docs/architecture.md` has what each added. M6 (widgets) is merged at a75a708, M5 (foundations) at
1aeaf1b, M4 (interaction and motion) at d9fe71f, then M3 (catalogue) and M2 (seams and crate split). `lune-ui-core` holds the
per-window `Ui` frame, the closed-form motion system, `layout`, `shortcuts`, `text_edit`, `drag`, `image` and the widgets;
`lune-ui-wgpu` takes `unicode-segmentation` (the core still has no dependency). The gallery has a page registry (two legacy
pages, 64 Ui pages), `--dump`, `--list` and `--bench`.

Counts at the tip: gallery 66 pages, 332 scene names in `--list`; `cargo test --workspace` 1,940 passed, 0 failed.

Review round 1 (D175): Escape cancels a live pointer drag before the overlay around it dismisses, for `drag_gesture`, `free_drag_gesture`
and the splitter (a test in a dialog and a mutant each); a board column whose cards fit registers no scroll sense, so the wheel
reaches the board; 19 contract lines the review's mutants left standing now have tests that kill them. Gate after the fixes:
fmt, clippy, `cargo test --workspace` 1,940 passed and 0 failed, boundaries, component index; GPU tests 62 passed; `--list` 332;
pixel gate against 662d310 305 identical and 27 new. The bench below is from before the fixes (nothing they touch is on a
frame path the bench measures beyond one store flag).

Final gate at 7147e31: fmt, clippy `-D warnings`, `cargo test --workspace` 1,918 passed and 0 failed, boundaries clean, component
index up to date; `LUNEUI_REQUIRE_GPU=1 cargo test -p lune-ui-wgpu` 62 passed (the lane dot probe included); `--list` 332;
pixel gate against 662d310 (the M7 merge) 305 identical, 27 new, nothing allowed; Windows and Linux (Docker) pass. Bench
(`cargo run --release -p gallery -- --bench`, verdict per D85, 66 pages, 2080 x 1320, 5 runs, M1 Pro; load 7.53 at the start,
4.4 to 6.4 across the pages, 5.00 at the end; self-test in range, cpu p50 4.22, spread p99 +3.95): 0 pages over, every page 0 of
5 runs over; highest page p99 11.87 ms (`dialog`) and worst frame 15.68 ms (`flyout`); the five M8 pages `table`, `bar-chart`,
`pie-chart`, `kanban-board`, `lane-graph` read p99 7.97, 7.31, 6.07, 6.68, 6.11 ms and worst 13.10, 15.59, 10.83, 11.83, 8.57 ms.

## Carried limits

- Overlays above a modal go one level deep: those stacked further share `Raised`, so a third level that overlaps its
  parent interleaves (fills under text). Two edges of that rule (step 1): a child held on `Raised` after its parent
  closed sits above, and is eligible over, a dialog opened in that window; on `Raised` a child opened in a nested dialog's
  body registers before that dialog's footer buttons, so where they overlap the buttons win the hit.
- Disabled keys (M7 review F9, F14, F17; D159, D160): not every key reader holds D140's consume-then-gate.
  `docs/architecture.md`, "Disabled keys", is the one list of what holds, by which test or probe, what does not and what
  was not checked; it is not restated here. M8 added its rows (table header and body, board cards and column area, the
  board's buttons), each with a test and a mutant. Making the rest hold is open.
- Chords (M7 review F13): the tab strip and star rating (Enter, Space), search picker (Enter, Space), stepper (Up, Down)
  and slider (the arrows) take a Control, Alt or Command chord, so a shortcut on it dies on one of them while it works on
  a button (by probe). D157's rule holds for `activation_keys` (so the combo since D160), the splitter's keys but Escape
  in a drag, the nav list, `rove_keys`, the tree, the menu, the list and the scroll area.
- One blur per frame: a glass menu over a dialog with a blurred scrim has none of its own.
- Text cannot be cut by a rounded clip (it gets the clip's bounding rect). The toolkit keeps text out of the corners: the
  page and panel inset every text and padded body by the radius (8); a flush panel body is rounded-clipped for fills and
  images and its text inset of 8 (`panel::TEXT_INSET`) is the caller's duty (D148). Text draws above every fill of its
  layer, so labels show through a sliding tab during `dnd-slide` and `dnd-settle`.
- The gallery bundles no CJK font: the IME scenes' "かな" draws as boxes (D90).
- Inside a cluster of several glyphs a click lands on either edge non-monotonically: `Buffer::hit` halves
  each glyph, not the cluster. A drag stopped right of a row cut inside a word leaves one grapheme out until the
  pointer reaches the next row (P20; the seam has no caret affinity, D112).
- API gaps, marked in the entries, each an API change: the table has no column resize or reorder (D163; needs the user, reversible, assumed deferred); `StrokeStyle` has no cap mode, so the lane graph's lines and edges end butt where Lunegit's are round (D172); the board has no keyboard-move action in the access tree (D55, D169); the callout has no title line and no inline figure and the empty
  state's actions have no glyph (D136); the sidebar footer is one line in one family; the flyout's route state note has
  no place in `routes`; `Ui::combo` has no path to D143's glass list; toast history rows have no hover or expand;
  `Ui::filter_bar` has no option for the palette's 16 px glyph, 14 px text or radius 6; `PaletteRow` has no icon tone; the
  side panel's show moves only the content and fires on a swap, not on the column.
- Tree view: `cells` is not called on rows of a moving block; rows have no ShowMenu action; there is no folder glyph;
  `CHEVRON_GAP` and `GLYPH_GAP` have no source (the director accepted them). Split panes: the panes draw with the size the
  frame began with, so a size a frame's input changed shows in the panes on the catch-up frame.
- M8 limits (entries and D173, D174): the table has no horizontal scroll (`Fill` columns shrink, then the last columns clip),
  a sideways swipe over a board column that overflows moves nothing (the topmost scroll area takes the delta; a vertical wheel over
  the cards of a column that fits scrolls the board, D175), a `Measured` row height after a sort takes effect on the next frame, a control in a row scrolled out of view loses the
  focus (rows out of view are not built; by construction, not probed), a wrapping cell ignores `num`, and the sorted header's
  focus ring snaps. Bars are keyed by column index, so a rolling window morphs in place with no slide. The board moves one
  card a frame by Shift and arrow, takes no landing in a disabled scope, and an app that applies a move later than the next
  frame sees the landing retarget with a step; an empty card footer leaves 6 px under the last row.
- `role-gap: tab to panel association` of the tab strip stays open (D122). The real pointer feel of the tab drag, the
  slider, the splitter and the tree in window mode is not testable with scripted input.
- Review nits left open (M6 `review.md`): `toolbar_label` repeats `button_inner_colored` (F2); colour helpers exist twice
  (F3); a per-frame `to_owned` of a tab name (F4); a press held across a disabled frame activates a tab twice (F7).

## Unverified here

- Real typing with an input method (roadmap R1), screen readers (R1) and bidi caret placement.
- Windows and Linux run time (the lint runs through `scripts/check-targets.sh`, Linux only with Docker).
- A text field's clipboard: the gallery answers from an in-process string until M10 (D107).
- The flyout in its own window and its placement from the tray rect (M10).
- The real pointer feel of the board's drag, its ghost and its auto-scroll in window mode (scripted input only), and screen
  readers (R1).

## Next

1. Review and merge M8 (`orch/toolkit-v1`), then M9 (text views).
2. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
3. An export script for Luneterm's vendored snapshot, before Luneterm M9. The migration
   map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed
  app-side and documented (answer 2026-11-03, reversible).

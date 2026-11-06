<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-06 (M9 merged to main at 8f499d0, final gate at 9efda0b; M10 next)

## Now

M9 (text views: code block, code editor view, markdown preview, diff view) is merged to main at 8f499d0
(steps 1 to 9; DECISIONS D177 to D203), review rounds 1 to 4 fixed (D200 to D203), round 5 passed. `docs/architecture.md` has the module map,
the pages, "Disabled keys" and "API changes since M1" (the M9 block).

What M9 added:
- Carried limits paid first: text readers consume then gate (D177: `begin_edit` and `selectable_text`, so `text_field`, the filter
  bar's and the palette's box, `text_area` and the selectable text; `TextEdit::takes`), and a wheel delta routes per axis (D178), which
  closed the M8 limit of a sideways swipe over a board column.
- One engine, generic over the text (D179, D180): `TextEdit` edits through `EditText` and `EditBuffer`, implemented for `str`,
  `String` and `CodeDoc` (one `String` and a line index; at 11 MB and 1,000,000 lines an edit measures 0.9 ms).
- `text.code-editor-view` (D181 to D185, D195): `Ui::code_editor`, `Ui::code_line_rect`, lines on `RowHeights`, read-only or
  editable, wrapped or not, spans, bands, synced panes, plain Tab through `tab_taker`, IME; 15 scenes.
- `text.code-block` (D186, D196): the same frame read-only with a copy button and the check-mark feedback; the text's clip ends 4 px
  before the button and the bar lies in the bottom padding (step 8); 12 scenes.
- `text.markdown-preview` (D188, D197, D198): `Ui::markdown` over the `MdBlock` model, blocks as virtual rows, inline code pills with
  pads left out of Copy, links as Tab stops, task boxes, tables, images, fenced code through the block; 16 scenes.
- `text.diff-view` (D191, D192, D199): `Ui::diff_view`, unified and split on the list frame (a pixel scroll link between panes),
  `pair_rows`, hunk actions, intra-line marks, `Ui::diff_selected_text`; 14 scenes; each split pane names its table "(old)" or "(new)".
- Scaffold: `Style.italic` (D189), six theme tokens (D190), `widgets::text_lines`, rich selectable text by position (D187), the list
  frame's x axis, `fill`, `identity` and `link`.
- Step 8 (visual): the director accepted both batches (Wane's code block against its mock, Lunegit's values for the other three);
  the fixes are in D196 and D198 (hunk text `fg_quaternary`, the underline, strike and pill places, the link ring).
- Step 9: `c4::m9` (roles, names and states of the four in one frame, injected actions on a link, a task box, a hunk action, the copy
  button and a set-value, tab stops, the disabled walk, Global-binding silence for nine stops with a mutant each, idle at rest) and
  one `nested_wheel` case per composite; the "Disabled keys" rows; the catalogue audit (every scene name of the four entries in
  `--list`, both ways).

Counts at the tip: gallery 70 pages, 389 scene names in `--list`; `cargo test --workspace -- --list` names 2,279 tests.

Final gate at 9efda0b: fmt, clippy `-D warnings`, `cargo test --workspace` 2,279 passed and 0 failed, boundaries, component index;
`LUNEUI_REQUIRE_GPU=1 cargo test -p lune-ui-wgpu` passed; `--list` 389; pixel gate against 2474248 (the M8 merge) 332 identical, nothing allowed;
Windows and Linux (Docker) pass. Bench at 9efda0b (`cargo run --release -p gallery -- --bench`, verdict per D85, 2080 x 1320, 5 runs, 70 pages, M1 Pro;
load 4.8 to 6.3; self-test in range, cpu p50 4.18, spread p99 +3.62, max +4.97): 0 over budget, every page 0 of 5 runs over; highest page p99 9.05 ms and
worst frame 30.64 ms, both `code-editor-view` (under the 33 ms limit, the closest page to it); `diff-view` p99 2.34, `markdown-preview` 1.92, `code-block` 1.71.

M8 (data views: table, bar chart, pie chart, kanban board, lane graph) is merged to main at 2474248 (7 steps, DECISIONS
D160 to D176) and closed: the cross-widget suite, the disabled-keys rows, the catalogue audit, the architecture notes and the
bench are in. `docs/architecture.md` has the module map, the pages and "API changes since M1".

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

Counts at the tip: gallery 66 pages, 332 scene names in `--list`; `cargo test --workspace` 1,954 passed, 0 failed.

Review round 1 (D175): Escape cancels a live pointer drag before the overlay around it dismisses, for `drag_gesture`, `free_drag_gesture`
and the splitter (a test in a dialog and a mutant each); a board column whose cards fit registers no scroll sense, so the wheel
reaches the board; 19 contract lines the review's mutants left standing now have tests that kill them. Gate after the fixes:
fmt, clippy, `cargo test --workspace` 1,940 passed and 0 failed, boundaries, component index; GPU tests 62 passed; `--list` 332;
pixel gate against 662d310 305 identical and 27 new.

Review round 2 (D176): every composite that nests an area and knows its extent disables an axis with nothing to scroll, so a delta
reaches the area around it (the board and its columns, list, table, tree, tab strip, text area, toast history; `tests/c4.rs`
`nested_wheel`); an Escape in the frame a gesture ends, after the release or the capture's loss, is the overlay's; the armed-press
rule has its test on `free_drag_gesture`; the Escape routing entries say D175. Gate: `cargo test --workspace` 1,954 passed, 0 failed.

Review round 3 passed at 208608b with two nits left open: a wheel over a fit composite in a top popover reaches a list in a popover under it
(`pointer.rs:258`, two non-modal overlays on one layer; `&& h.tag >= tag` fixed both probes in a scratch copy, F11), and F5, F6 (a ghost's step on a
press mid-slide or a second Shift move during a landing).

Final gate at 208608b: fmt, clippy `-D warnings`, `cargo test --workspace` 1,954 passed and 0 failed, boundaries clean, component
index up to date; `LUNEUI_REQUIRE_GPU=1 cargo test -p lune-ui-wgpu` 62 passed; `--list` 332; pixel gate against 662d310 (the M7
merge) 305 identical, nothing allowed; Windows and Linux (Docker) pass. Bench (`cargo run --release -p gallery -- --bench`, verdict
per D85, 2080 x 1320, 5 runs, M1 Pro; load 6.46 at the start, 4.14 at the end; self-test in range, cpu p50 4.2, spread p99 about +7.6;
a second bench overlapped its first two minutes): 0 over, every page 0 of 5 runs over; highest page p99 12.08 ms (`dialog`), worst
frame 19.01 ms (`radio-group`).

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
  `page` and `settings_page` nested in another area keep a delta even when they fit (measured extent; the other composites pass it, D176), an Escape before the release in one frame is lost and the drop commits (the next Escape dismisses, D176), a `Measured` row height after a sort takes effect on the next frame, a control in a row scrolled out of view loses the
  focus (rows out of view are not built; by construction, not probed), a wrapping cell ignores `num`, and the sorted header's
  focus ring snaps. Bars are keyed by column index, so a rolling window morphs in place with no slide. The board moves one
  card a frame by Shift and arrow, takes no landing in a disabled scope, and an app that applies a move later than the next
  frame sees the landing retarget with a step; an empty card footer leaves 6 px under the last row.
- M9 limits (entries, D194, D183, Q16): a very long single line costs in proportion to its length, slow to first draw then cached (no
  windowing inside a line); a line inserted into a wrapped editor leaves stale heights below it until they are built, so the bar may
  shift; a word motion in the editor stops at its window's edge (a line, so on single-blank-line text one window past); the markdown's
  nested block (a table, a list, a quote) is built whole, and only the blocks in view are Tab stops; the editor's node carries the
  lines in view, so the `role-gap` full-text query stays open for M10's bridge; no folding and no in-editor search (D181, Lunegit has
  neither); a one-line code block scrolled fully right has its thumb under the copy button by about 4 px (the button is hit first);
  the markdown's inline underline, strike and pill are fills from row boxes, not a seam feature (D188); no italic face ships, so italic is the shaper's skew (D189);
  a page key or Home in the frame right after a far jump in a wrapped view lands off by what that frame measured (80 px in the probe: the shift the content queued lands after the jump); a pane pulled in the frame of its own wheel publishes nothing, so a pixel wheel on the other pane in the very frame one pane opens a document alone rests apart until the next scroll (D184's rule; a split diff opens both panes, so only linked code views can do it).
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

1. M9 review and merge to main; then M10 (platform bridges) per the roadmap.
2. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
3. An export script for Luneterm's vendored snapshot, before Luneterm M9. The migration
   map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed
  app-side and documented (answer 2026-11-03, reversible).

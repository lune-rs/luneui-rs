<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-06 (M10 built, steps 1 to 11 and review round 2 done on `orch/toolkit-v1`, awaiting the final gate and the merge; M9 is merged at 8f499d0)

## Now

M10 (infra and platform) is built on `orch/toolkit-v1` (steps 1 to 11; DECISIONS D204 to D227): the review, the final gate and the merge are
next. What it added:
- `lune_ui::json`: the bounded reader (D204; 1 MiB, depth 16, strings 4,096, keys 256, 8,192 members, 65,536 values), the loaders' policy on top.
- Theme files (D207 to D209): `Theme::from_json` over a base, the glass inside `Theme` (`Theme.glass`), derived tokens that follow their source,
  `AcrylicLevel`, and a theme swap is a hard cut (every colour transition snaps in the frame the theme differs).
- Locale files (D210): `Catalog::from_json` (strict: an unknown `luneui.` key fails the file), `StringTable::overlay`, `audit_template`.
- Input (D211, D212): `Event::FileHover`, `FileHoverCancelled`, `FileDrop` (a `PathBuf`; nothing opens it), `keymap()`, the pure `OutputState::plan_output`.
- The OS clipboard (D213 to D215): `arboard` in `lune-ui-winit`, a 32 MiB cap, the gallery's stub (D107) retired.
- AccessKit (D216, D217): `AccessBridge` behind the `accesskit` feature of `lune-ui-winit` and the facade; handlers touch only `Shared`.
- The tray (D218 to D227): the icon is a CPU raster (`tray::icon_bitmap`, a transparent ring, the dot's centre at 14 at 1x), placement from the item's
  rect (`bar_from_tray`, `place_surface`), `TrayToggle`, `Ui::flyout_window` and `Ui::menu_window`; `lune_ui_platform::{tray, popover}` over `tray-icon`
  and the macOS `NSPanel` class swap behind the `PopoverWindow` guard; the Windows popover activates (D224).
- Gallery: pages `theme`, `tray` and `file-drop` (24 new scenes), the live tray (`popover.rs`) and `gallery --smoke-platform` (D225).
- Step 10: the director accepted the tray icon and the two window surfaces in round 1 (D226, D227).
- Step 11: `c4::m10`, the theme reach audit (below), the catalogue and architecture notes, and the checks.
- Review round 2 fixed (D228 to D232): the tray's reopen guard is per surface and the focus watch follows the window (F1, F4); tray placement is in logical pixels (F2); a theme swap drops closing overlays (F3); a password field is `Role::PasswordInput` (S1); a text field and text area refuse an insert over 1 MiB (S2, S3, D232, D233).

Counts at the tip: gallery 73 pages, 413 scene names in `--list`; `cargo test --workspace -- --list` names 2,520 tests.
Release size (macOS, `cargo build --release -p gallery`, strip, fat LTO): 9,448,688 bytes at the M9 merge (8f499d0), 9,869,408 at this tip, +420,720 (+4.45%);
the AccessKit bridge is +237,728 of that (D216), the tray stack about +150,000 (a scratch estimate at step 8), `arboard` the rest.
Checked here at the tip (not the final gate, which is the orchestrator's): `gallery --smoke-platform` exit 0 (a real tray item, its rect, a synthetic click, the surface in its window, 25 frames in the
first open, the frontmost app unchanged, the panel key, two opens and closes, teardown); the guarded `real_round_trip` of the clipboard ran at step 5 and passed;
bench (`cargo run --release -p gallery -- --bench`, verdict per D85, 2080 x 1320, 5 runs, 73 pages, M1 Pro; load 4.46 to 12.5 and the machine was also
compiling for part of it; self-test in range, cpu p50 4.27, spread p99 +7.18, max +86.38): 0 over budget, no page over in 3 or more of 5 runs; highest page p99 13.57 ms
(`code-editor-view`, in 2 of 5 runs over its own limit), worst frame 80.47 ms (`markdown-preview`, 1 of 5 runs over, a load spike: its p99 is 7.08); the new pages
`theme` 7.34, `tray` 10.11, `file-drop` 7.04.

The theme reach audit (`tests/c4.rs`, `c4::m10`): the controls and a spread of M6 and M7 widgets (ghost, primary and icon buttons, a radio group, stars, a slider, a tab strip,
a filter bar, progress, badges and status dots in every tone, a spinner, a banner, the four toasts, the dropdown, the menu and the dialog; with hover, focus rings and a selection)
are drawn under a theme whose 71 colour fields are all distinct and none is a preset colour: no fill, stroke or text holds a preset colour (the scan finds them under the preset, so
it does not look at nothing, and a one-line mutant in `toggle.rs` is caught). It reaches 38 of the 71 fields; the other 33 belong to widgets the frame does not draw (scroll thumbs, the
markdown, code and diff colours, the page and panel glass tints, the caption buttons, the dirty mark, the count badge, a few greys), so for those the audit proves nothing and the
literal grep below is the only check. The grep over `widgets/` (`[0x..` and `[255, ..` forms, hex words, colour consts) finds two fixed colours, both on purpose: the pie chart's hover
wash `[255, 255, 255, 38]` (Lunegit's `StoragePieChartControl`) and the kanban board's `CLEAR`, a transparent transition target.

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
load 4.31 to 6.47; self-test in range, cpu p50 4.18, spread p99 +3.62, max +4.97): 0 over budget, every page 0 of 5 runs over; highest page p99 9.05 ms and
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
- M10 limits (entries and DECISIONS): the text field and the text area refuse a paste or set-value over 1 MiB whole (`EditFlags::max_insert`, D232, D233); the code editor sets no limit; assistive clients get the lines in view of an editor, not the document (D217); `Dismiss` is exposed only on a tooltip and `Dragging` and a mixed toggle
  are not sent (D217); a locale file written for a newer toolkit fails on an older one and the app falls back to English (D210); the Windows popover activates, so it takes focus when
  shown, where the entry said non-activating (D224, Q17); a surface is placed from the tray item's rect, so on Windows, where the rect may sit a few px inside the taskbar, "12 px above" may
  read a little more (D219); the macOS rect settles only after 300 ms, so the host waits for it (D225); the tray icon has no automatic inversion while the item is highlighted (D218); there is no
  Windows row scene (`MenuOverBar` is live only and was never viewed); a loaded theme's `WindowFallbackBackground` sets only `window_fallback`, and the inactive window's fill that Lunegit recomputes
  from the glass tint is the app's (`platform.window`); there is no drop-target widget; the toolkit holds Lunegit's theme and locale files as gallery fixtures with no open licence
  (D206): license or remove them before any public release, and M11's export must leave them out; `--parity-only` of `scripts/pixel-gate-luneterm.sh` fails by design (the lock has grown).
- `role-gap: tab to panel association` of the tab strip stays open (D122). The real pointer feel of the tab drag, the
  slider, the splitter and the tree in window mode is not testable with scripted input.
- Review nits left open (M6 `review.md`): `toolbar_label` repeats `button_inner_colored` (F2); colour helpers exist twice
  (F3); a per-frame `to_owned` of a tab name (F4); a press held across a disabled frame activates a tab twice (F7).

## Unverified here

- Real typing with an input method (roadmap R1), screen readers and an assistive client of the AccessKit bridge (R1: its tree is replayed into `accesskit_consumer`, no client ran), and bidi caret placement.
- Windows and Linux run time (the lint runs through `scripts/check-targets.sh`, Linux only with Docker): nothing of the tray, the popover, the clipboard or the bridge has run on Windows;
  Linux has no tray (`Unsupported`).
- The tray, past what `gallery --smoke-platform` measured on this Mac (an item, its rect, a synthetic click, the panel's key status, the frontmost app, two opens and closes, teardown): a real click on
  the item, real hover and keys in the panel, the OS shadow and corners of the surface, a menu bar on a second display, the item highlighted. The panel rests on AppKit behaviour Apple does not
  document and is proved only on this macOS version (D223). Run it again after a macOS update.
- The OS clipboard beyond the one guarded `real_round_trip` (macOS, plain text); a real file dragged over a window (the scenes send the events through the script).
- The real pointer feel of the board's drag, its ghost and its auto-scroll in window mode (scripted input only).

## Next

1. M10 review, the final gate (including the AccessKit-off line and `scripts/check-targets.sh`) and the merge.
2. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
3. M11: the done-standard pass on every entry, the export script for Luneterm's vendored snapshot (it must leave out `apps/gallery/assets/lunegit/`, D206), and the migration
   map, which is "API changes since M1" in `docs/architecture.md`. The release size for its report: the figures above.

## Blockers / open questions

- None.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed
  app-side and documented (answer 2026-11-03, reversible).

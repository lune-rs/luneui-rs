<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-05 (M7 done on `orch/toolkit-v1`, all 15 steps; a pause point, M8 waits for the user)

## Now

M7 (overlays and containers) is built and closed on `orch/toolkit-v1`. Review, the final gate and the merge to main are
the orchestrator's; this file's bench and pixel lines are for it to fill.

What M7 added (DECISIONS D137 to D156; `docs/architecture.md` has the module map and "API changes since M1"):
- Plumbing: a sixth layer `LayerId::Raised` between `Modal` and `Toast` (`LAYER_COUNT` 6), chosen once in `Ui::overlay`
  when a popup or modal opens above an active modal and held to its close, so a popover, tooltip, combo list, menu or
  nested dialog over a dialog draws and takes input on top with no widget change; overlay content is out of any
  enclosing `record`; `BackdropRequest` carries `layer`, `sigma` and `saturation` and the offscreen split follows it
  (`Scene::split`); `Painter::acrylic_radii`; `Node.level`; `Ui::node` marks every node in a disabled scope and keys are
  consumed before they are gated; `testing::disabled_violations`.
- Overlays: menu rows (`MenuRow::Entry`, `Header`), keys, one highlight and anchored menus (`Ui::menu_with`); the
  dropdown's glass surface and `max_rows`; the glass popover; the flyout (`Ui::flyout`); toasts and their history
  (`Toasts`, `Ui::toasts`, `Ui::toast_history`); dialog variants (`Ui::dialog_with`: form width, scrim blur, disabled
  confirm) on a shared `dialog::modal_host`; the command palette (`Ui::command_palette`).
- Containers: page, panel, sidebar, side panel, icon rail, split panes, tree view and settings page, each with its page,
  scenes, tests and semantic nodes; 12 `Theme` tokens (+ `status_error`), 5 glyphs, 9 strings.
- The cross-widget suite `crates/lune-ui-core/tests/c4.rs` has an M7 section (roles, names and states of every
  interactive M7 widget in one frame, default, expand, collapse, increment and set-value actions, one tab stop per
  composite, the disabled walk and Global-binding silence, idle at rest, and a dialog holding a combo with a toast and a
  tooltip over it routing press, Tab and Escape).
- Two visual batches (steps 13, 14) moved the Wane and Lunegit composites to their sources (D154, D155).
- The catalogue audit holds: all 79 scene names of the 14 M7 entries are in `gallery --list`, every Open question is
  answered or marked, every Build line and Depends id holds. The 14 entries stay `in-progress` (as M6's did).

Counts at the tip: gallery 61 pages, 305 scene names in `--list`; `cargo test --workspace` 1,720 passed, 0 failed.

Pixel gate (`scripts/pixel-gate.sh --base a75a708`, expected: 231 identical, 74 new, nothing allowed): the orchestrator's
final gate fills this in. Each step passed it at zero delta against its own base.

Bench (`cargo run --release -p gallery -- --bench`, verdict per D85 at 61 pages, self-test and load quoted): the
orchestrator's final gate fills this in. A page over only while the machine is loaded is reported, not refused.

M6 (widgets) is merged to main at a75a708: 21 catalogue entries, roving focus, `text::elide`, the button variants
(DECISIONS D113 to D136). Earlier: M5 (foundations, 1aeaf1b), M4 (interaction and motion, d9fe71f), M3 (catalogue specs),
M2 (seams and crate split). `lune-ui-core` holds the per-window `Ui` frame, the closed-form motion system, `layout`,
`shortcuts`, `text_edit`, `drag`, `image` and the widgets; `lune-ui-wgpu` takes `unicode-segmentation` (the core still has
no dependency). The gallery has a page registry (two legacy pages, 59 Ui pages), `--dump`, `--list` and `--bench`.

## Carried limits

- Overlays above a modal go one level deep: those stacked further share `Raised`, so a third level that overlaps its
  parent interleaves (fills under text). Three edges of that rule (step 1): a popup that closes when a modal opens and
  is reopened during its close keeps its lower layer and gets no input; a child held on `Raised` after its parent closed
  sits above, and is eligible over, a dialog opened in that window; on `Raised` a child opened in a nested dialog's body
  registers before that dialog's footer buttons, so where they overlap the buttons win the hit.
- One blur per frame: a glass menu over a dialog with a blurred scrim has none of its own.
- Text cannot be cut by a rounded clip (it gets the clip's bounding rect). The toolkit keeps text out of the corners: the
  page and panel inset every text and padded body by the radius (8); a flush panel body is rounded-clipped for fills and
  images and its text inset of 8 (`panel::TEXT_INSET`) is the caller's duty (D148). Text draws above every fill of its
  layer, so labels show through a sliding tab during `dnd-slide` and `dnd-settle`.
- The gallery bundles no CJK font: the IME scenes' "かな" draws as boxes (D90).
- Inside a cluster of several glyphs a click lands on either edge non-monotonically: `Buffer::hit` halves
  each glyph, not the cluster. A drag stopped right of a row cut inside a word leaves one grapheme out until the
  pointer reaches the next row (P20; the seam has no caret affinity, D112).
- API gaps, marked in the entries, each an API change: the callout has no title line and no inline figure and the empty
  state's actions have no glyph (D136); the sidebar footer is one line in one family; the flyout's route state note has
  no place in `routes`; `Ui::combo` has no path to D143's glass list; toast history rows have no hover or expand;
  `Ui::filter_bar` has no option for the palette's 16 px glyph, 14 px text or radius 6; `PaletteRow` has no icon tone; the
  side panel's show moves only the content and fires on a swap, not on the column.
- Tree view: `cells` is not called on rows of a moving block; rows have no ShowMenu action; there is no folder glyph;
  `CHEVRON_GAP` and `GLYPH_GAP` have no source (the director accepted them). Split panes: the panes draw with the size the
  frame began with, so a size a frame's input changed shows in the panes on the catch-up frame; on a row split an
  accessibility Increment or Decrement arrives as an arrow key and moves the strip up or down (the direction depends on
  `sized`).
- `role-gap: tab to panel association` of the tab strip stays open (D122). The real pointer feel of the tab drag, the
  slider, the splitter and the tree in window mode is not testable with scripted input.
- Review nits left open (M6 `review.md`): `toolbar_label` repeats `button_inner_colored` (F2); colour helpers exist twice
  (F3); a per-frame `to_owned` of a tab name (F4); a press held across a disabled frame activates a tab twice (F7).

## Unverified here

- Real typing with an input method (roadmap R1), screen readers (R1) and bidi caret placement.
- Windows and Linux run time (the lint runs through `scripts/check-targets.sh`, Linux only with Docker).
- A text field's clipboard: the gallery answers from an in-process string until M10 (D107).
- The flyout in its own window and its placement from the tray rect (M10).

## Next

1. M7 is a pause point: the orchestrator runs review and the final gate, then merges. M8 waits for the user.
2. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
3. An export script for Luneterm's vendored snapshot, before Luneterm M9. The migration
   map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed
  app-side and documented (answer 2026-11-03, reversible).

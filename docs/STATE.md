<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-05 (M7 in progress: steps 1 and 2 done)

## Now

M7 (overlays and containers) is in progress on `orch/toolkit-v1`. Step 1 (plumbing) is done: a sixth layer
`LayerId::Raised` (`Toast` is 5 now, `LAYER_COUNT` 6), chosen once in `Ui::overlay` when a popup or modal opens above
an active modal and held to its close, so no widget changed; overlay content is out of any enclosing `record`; a
`BackdropRequest` carries `layer`, `sigma` and `saturation`, the offscreen split follows it (`Scene::split`), of two
requests the lower layer keeps it, and `Painter::backdrop_with` sets the numbers; `Painter::acrylic_radii`; `Node.level`;
`Ui::node` marks every node in a disabled scope; `Ui::activation_keys` consumes before it gates (button, toolbar label,
checkbox, toggle, radio group); `icon_button_sensed`; `testing::disabled_violations`. No drawing changed.

Step 2 (scaffolding) is done, so the three tracks (A: steps 3 to 5, B: 6 to 8, C: 9 to 12) touch no shared file: 11 empty
widget modules, 11 page stubs (`flyout`, `toast-notifications`, `command-palette`, `page`, `panel`, `sidebar`,
`side-panel`, `icon-rail`, `split-panes`, `tree-view`, `settings-page`) registered in `PAGES` (61 pages, `--list` still
231 names until the scenes land), 12 `Theme` tokens (`menu_item_pressed`, `menu_text_disabled`, `toast_*`,
`dirty_indicator`, `count_badge`), 5 glyphs, 9 string defaults, the M7 motion rows, DECISIONS D137 to D153 and the 14
entries `in-progress`. Next: the tracks (steps 3 to 12), then visual batches 13 and 14 and the close.

M6 (widgets) is merged to main at a75a708 (14 steps, review passed in 3 rounds): roving focus (`Sense.skip_tab`,
`Sense::CLICK_ROVE`, `RoveAxis`, `Ui::rove_stop`, `rove_keys`, `rove_focus`), `text::elide` and `Painter::elide`,
`Ui::snap_f32`, `Painter::stroke_sweep` and `stroke_comet`, the button variants (ghost, small, icon button, bare
field chrome), and the 21 catalogue entries whose Build said M6: badge, status dot, verdict row, progress, spinner,
callout, stat, key-value list (with selectable text), setting and item rows, list row, form field, steps, nav list,
radio group, star rating, slider, filter bar, search picker, tab strip, toolbar, banner and empty state. Each has
its page, scenes, tests and semantic nodes. DECISIONS D113 to D136 record the plan decisions and the visual
rulings. The cross-widget suite `crates/lune-ui-core/tests/c4.rs` has an M6 section (roles, names and states of
every interactive widget in one frame, default and increment actions, one tab stop per composite, idle at rest).
The gallery has 50 pages and 231 scene names in `--list` (152 + 79); the workspace has 1,323 tests, 0 failed.
The pixel gate against M5's merge (1aeaf1b), measured at cd1e6ac: exit 0, 151 scenes identical (the 15 legacy ones
among them; `focus-ring-kinds` allowed, the radio joins it, D120; the new scenes are new).

Bench at cd1e6ac (`cargo run --release -p gallery -- --bench`, 5 runs, 50 pages, quiet machine: load average 2.7 to 4.5):
0 pages over by D85, every page 0 of 5 runs over; `dialog` p99 12.79 ms, the highest page, worst frame of any page 24.6 ms.
This settles the step 14 run, which had `dialog` over in 4 of 5 runs at load 18 to 45: that was the machine's load.

M6 review fix round 1 (F1): a disabled tab strip (`with_enabled(false, ..)`) activates nothing from Enter or Space
(both are still consumed), its tab and close nodes report `State::Disabled` with no `Focusable` and no actions, and an
open tab menu closes. No drawing changed. Round 2 (F5, F6): the `TabList` node reports `State::Disabled` too, the test
walks every node of the tree, and a Global binding proves Enter and Space on a focused tab do not reach the shortcuts.

Earlier milestones: M5 (foundations) is merged to main at 1aeaf1b on top of M4 (interaction and motion, d9fe71f), M3
(catalogue specs) and M2 (seams and crate split). `docs/architecture.md` has the module map, the focus queue and the
roving rule, `Keymap`, the shortcut phases, the per-layer draw order, the `ImageStore` seam and "API changes since
M1" (with "Since M6"). `lune-ui-core` holds the per-window `Ui` frame, the closed-form motion system, `layout`,
`shortcuts`, `text_edit`, `drag`, `image` and the widgets; `lune-ui-wgpu` takes `unicode-segmentation` (the core
still has no dependency). The gallery has a page registry (two legacy pages, 48 Ui pages), `--dump`, `--list` and
`--bench`. The pixel gate shows the 15 legacy scenes identical against the M1 base.

## Carried limits

- Overlays above a modal go one level deep: those stacked further share `Raised`, so a third level that overlaps its
  parent interleaves (fills under text). One blur per frame: a glass menu over a blurred scrim has none of its own.
- Text cannot be cut by a rounded clip (it gets the clip's bounding rect): M7's page and panel keep text out of the
  corners. Text draws above every fill of its layer, so labels show through a sliding tab during `dnd-slide`
  and `dnd-settle`.
- The gallery bundles no CJK font: the IME scenes' "かな" draws as boxes (D90).
- Inside a cluster of several glyphs a click lands on either edge non-monotonically: `Buffer::hit` halves
  each glyph, not the cluster.
- A drag stopped right of a row cut inside a word (a URL, a path) leaves one grapheme out until the pointer
  reaches the next row (P20; the seam has no caret affinity, D112).

- Marked gaps from the visual batches (entries list them): the callout has no title line and no inline figure
  (`CalloutOptions`), and the empty state's actions have no glyph (ruling C13, D136); both are API changes.
- `role-gap: tab to panel association` of the tab strip stays open (D122); the real pointer feel of the tab drag
  and the slider in window mode is not testable with scripted input.

- Review nits left open (M6 `review.md`): `toolbar_label` repeats `button_inner_colored` (F2); colour helpers exist twice
  (F3); a per-frame `to_owned` of a tab name (F4); a press held across a disabled frame activates a tab twice (F7).

## Unverified here

- Real typing with an input method (roadmap R1) and bidi caret placement.
- Windows and Linux run time (the lint runs through `scripts/check-targets.sh`, Linux only with Docker).
- A text field's clipboard: the gallery answers from an in-process string until M10 (D107).

## Next

1. M7 steps 3 to 15 (`m07-overlays/plan.md`): tracks A, B and C, two visual batches, close.
2. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
3. An export script for Luneterm's vendored snapshot, before Luneterm M9. The migration
   map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None. D74's deferral (button variants and the radio) was paid in M6 (D116, D120).
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed
  app-side and documented (answer 2026-11-03, reversible).

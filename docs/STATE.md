<!-- Short. Rewrite, do not append history; the log is git. Update in the same commit as the work. -->
# State

Last updated: 2026-11-05 (M6 widgets built and gated on `orch/toolkit-v1`, awaiting review and merge)

## Now

M6 (widgets) is complete on `orch/toolkit-v1` (14 steps, tip is this commit): roving focus (`Sense.skip_tab`,
`Sense::CLICK_ROVE`, `RoveAxis`, `Ui::rove_stop`, `rove_keys`, `rove_focus`), `text::elide` and `Painter::elide`,
`Ui::snap_f32`, `Painter::stroke_sweep` and `stroke_comet`, the button variants (ghost, small, icon button, bare
field chrome), and the 21 catalogue entries whose Build said M6: badge, status dot, verdict row, progress, spinner,
callout, stat, key-value list (with selectable text), setting and item rows, list row, form field, steps, nav list,
radio group, star rating, slider, filter bar, search picker, tab strip, toolbar, banner and empty state. Each has
its page, scenes, tests and semantic nodes. DECISIONS D113 to D136 record the plan decisions and the visual
rulings. The cross-widget suite `crates/lune-ui-core/tests/c4.rs` has an M6 section (roles, names and states of
every interactive widget in one frame, default and increment actions, one tab stop per composite, idle at rest).
The gallery has 50 pages and 231 scene names in `--list` (152 + 79); the workspace has 1,323 tests, 0 failed.
The pixel gate against M5's merge (1aeaf1b), measured at aeb6cbf: exit 0, 151 scenes identical (the 15 legacy ones
among them; `focus-ring-kinds` allowed, the radio joins it, D120; the new scenes are new).

Bench at aeb6cbf (`cargo run --release -p gallery -- --bench`, 5 runs, 50 pages, machine busy: load average 22 to 47):
0 pages over by D85; `dialog` p99 10.03 ms, over in 0 of 5 runs. The step 14 run had `dialog` over in 4 of 5 runs at
load 18 to 45, and the reviewer's A/B showed the M5 base going over on that page too, so read that as the machine's
load, not as settled on a quiet machine (D85, the user's standing direction: reported, not refused). The M6 pages
were at or below 8.9 ms p99 in the step 14 run (`tab-strip` 8.88, `banner` 8.33, `toolbar` 8.30).

M6 review fix round 1 (F1): a disabled tab strip (`with_enabled(false, ..)`) activates nothing from Enter or Space
(both are still consumed), its tab and close nodes report `State::Disabled` with no `Focusable` and no actions, and an
open tab menu closes. No drawing changed.

Earlier milestones: M5 (foundations) is merged to main at 1aeaf1b on top of M4 (interaction and motion, d9fe71f), M3
(catalogue specs) and M2 (seams and crate split). `docs/architecture.md` has the module map, the focus queue and the
roving rule, `Keymap`, the shortcut phases, the per-layer draw order, the `ImageStore` seam and "API changes since
M1" (with "Since M6"). `lune-ui-core` holds the per-window `Ui` frame, the closed-form motion system, `layout`,
`shortcuts`, `text_edit`, `drag`, `image` and the widgets; `lune-ui-wgpu` takes `unicode-segmentation` (the core
still has no dependency). The gallery has a page registry (two legacy pages, 48 Ui pages), `--dump`, `--list` and
`--bench`. The pixel gate shows the 15 legacy scenes identical against the M1 base.

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

- Marked gaps from the visual batches (entries list them): the callout has no title line and no inline figure
  (`CalloutOptions`), and the empty state's actions have no glyph (ruling C13, D136); both are API changes.
- `role-gap: tab to panel association` of the tab strip stays open (D122); the real pointer feel of the tab drag
  and the slider in window mode is not testable with scripted input.

## Unverified here

- Real typing with an input method (roadmap R1) and bidi caret placement.
- Windows and Linux run time (the lint runs through `scripts/check-targets.sh`, Linux only with Docker).
- A text field's clipboard: the gallery answers from an in-process string until M10 (D107).

## Next

1. Review of M6, then merge. The pixel gate (`scripts/pixel-gate.sh --base 1aeaf1b --allow focus-ring-kinds`), the
   Linux half of `scripts/check-targets.sh` (needs Docker) and a quiet-machine rerun of the bench's `dialog` page
   are the gate items not run by the implementer.
2. M7 (page, panel, flyout, toast, command palette), which uses `stat`, `kv_list`, `progress` and the roving rule.
3. CI (fmt, clippy `-D warnings`, tests on macOS, Windows and Linux) now that there is code to run.
4. An export script for Luneterm's vendored snapshot, before Luneterm M9. The migration
   map for M11 is "API changes since M1" in `docs/architecture.md`.

## Blockers / open questions

- None. D74's deferral (button variants and the radio) was paid in M6 (D116, D120). Nested popups above a modal are
  listed under the carried limits.
- Wane's OS behaviours (close-to-tray, single instance, autostart, notifications) are assumed
  app-side and documented (answer 2026-11-03, reversible).

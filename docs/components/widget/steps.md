---
id: widget.steps
title: Wizard steps
area: widget
status: done
consumers: wane
source: crates/lune-ui-core/src/widgets/steps.rs
owner: —
summary: Wizard steps: four 32 px rows in the sidebar slot with upcoming, current and done states; not clickable
---

## What it is

`steps`: wizard step list in the sidebar slot, on the same rows as the nav list. Upcoming, current and done states. Not clickable: Back and Continue move through the steps [W docs/ui/components.md:505-507].

## Needs

- Wane controls the toolkit lacks (R-widget-4).

## Existing code

`crates/lune-ui-core/src/widgets/steps.rs` (M6): `Ui::steps`, `Ui::steps_with_lead`. Gallery page `apps/gallery/src/pages/steps.rs`; tests `crates/lune-ui-core/tests/widget_steps.rs`. None before it in luneui-rs or Luneterm's `lune-ui` [W docs/ui/components.md:510]. Wane is the only source for this entry.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: `docs/ui/components.md:501-513,695`; `docs/ui/mocks/css/tokens.css:14,16,23,27,40,43`.
No conflict: one source. The row language is `widget.nav-item` [W docs/ui/components.md:509].

### Anatomy

- Rows 32 high, radius 6, 2 apart; padding 0 8; gap 8 [W docs/ui/components.md:506].
- Number circle 20 px with a 1 px `--border` ring, `fg_primary` on the current step (the mock's `.is-current .c-steps__num`; Wane's text names only `--border`, the mock is the reference; visual batch 3, C8); number 11 Medium [W docs/ui/components.md:506], [W docs/ui/mocks/css/tokens.css:27].
- Label 13 in a 148 px column that never wraps; a long label is shortened [W docs/ui/components.md:506,695].
- Lead line 13/20 tertiary, 12 px below the last row [W docs/ui/components.md:695] (`Ui::steps_with_lead`; the mock text says "first row", which cannot be a position under a list of rows), wrapping at the width.

### States

| state | look | cite |
|---|---|---|
| upcoming | tertiary label; ring number | [W docs/ui/components.md:505], [W docs/ui/mocks/css/tokens.css:14] |
| current | fill `surface_selected`; `fg_bright` label ([D58]), weight 500 | [W docs/ui/components.md:505], [W docs/ui/mocks/css/tokens.css:16,23] |
| done | accent circle with a 12 px check in `on_accent`; secondary label | [W docs/ui/components.md:505], [W docs/ui/mocks/css/tokens.css:40,43] |

Not interactive, so no hover, press, focus or disabled look [W docs/ui/components.md:507].

### Transitions

n/a — Wane specifies no motion for the step rows [W docs/ui/components.md:505-507]; state changes come from the owner's Back and Continue.

### Input and focus

n/a — not clickable [W docs/ui/components.md:507]. Back and Continue are the owner's buttons (`widget.button`).

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: the current step index and each step's label (passed every frame) [W docs/ui/components.md:507].
- Exit retention: no.

### Accessibility

- Role `navigation` (Wane's steps list is a labelled navigation landmark [W docs/ui/mocks/js/shell.js:117], [D55]) wrapping a `list` of `list-item`s; the current row is state `selected` and `current`; name = the step label; a done step is `checked` [D121] [infra.accessibility].

### Strings

None: step labels are app data.

### Gallery scenes

- `steps-states`: four rows, one each upcoming, current and done, settled.

### Painter primitives

- Existing: fill, ring, icon, text
- New: none

### Acceptance

- Pixel probes at scale 2: row 64 px high; number circle 40 px; label column 296 px and never wraps.
- Conformance: n/a — no transition.
- Semantic node: role navigation over a list; the current item is selected and current; a long label is shortened, not wrapped.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_steps.rs`: pixel probes `at_scale_2_a_row_is_64_px_the_circle_40_and_the_label_column_296`, `four_rows_of_32_two_apart_with_the_current_one_filled_surface_selected`, `a_label_never_wraps_and_never_leaves_its_148_px_column`; looks `an_upcoming_step_is_tertiary_over_a_ring_number`, `the_current_step_is_bright_and_medium`, `a_done_step_is_an_accent_circle_with_a_12_px_check_and_a_secondary_label`; no input `steps_take_no_input_and_no_tab_stop`; node `the_node_is_a_navigation_over_a_list_of_items_with_selected_current_and_checked`, `a_long_label_is_named_in_full_on_its_node`.

### Build

- Milestone: M6
- Depends on: widget.nav-item, core.painter, text.shaping
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_steps.rs::at_scale_2_a_row_is_64_px_the_circle_40_and_the_label_column_296, crates/lune-ui-core/tests/widget_steps.rs::a_label_never_wraps_and_never_leaves_its_148_px_column, crates/lune-ui-core/tests/widget_steps.rs::a_done_step_is_an_accent_circle_with_a_12_px_check_and_a_secondary_label, crates/lune-ui-core/tests/widget_steps.rs::the_node_is_a_navigation_over_a_list_of_items_with_selected_current_and_checked`; page `steps`; unverified here: none.

## Open questions

- None. `role-gap` is answered by [D121]: a done step carries the node state `checked`, from the existing vocabulary; no new field.

## Notes for the planner

Wane P2 in the mocks. Lives in the sidebar slot, so the sidebar owner places it.

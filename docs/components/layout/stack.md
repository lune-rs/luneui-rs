---
id: layout.stack
title: Stack, row, column and grid layout
area: layout
status: done
consumers: any
source: crates/lune-ui-core/src/layout.rs
owner: —
summary: Pure layout helpers: stack, row, column and grid with gaps, alignment, insets and fixed, fill and hug sizing
---

## What it is

Layout helpers that turn a parent rect and child sizes into child rects, so widgets stop taking
hand-computed rectangles. `stack` (one axis), `row` and `column` (its two axes) and `grid`
(Auto, fixed and star tracks) with a gap, alignment and insets. They draw nothing and hold no state:
the result is a function of the inputs, so frames stay deterministic. Lunegit lays out with
Avalonia's `StackPanel`, `Grid` and `DockPanel`; Wane specifies gaps on a 4 px grid.

## Needs

- Layout containers with gaps, alignment and insets (R-layout-1).
- No clock, environment or event-loop access (CLAUDE.md hard rule 2); totality: any input, including
  NaN and negative sizes, gives a usable result and never panics (R-layout-1, R-nfr-1).

## Existing code

Built in M5 (D108): `crates/lune-ui-core/src/layout.rs`, pure `stack`, `row`, `column` and `grid` over `Size`, `Align`, `Insets` and `Track`; widgets still take rects computed by the caller.
Design sources: Lunegit `StackPanel` (`Spacing` on 444 `<StackPanel` elements in `src/Lunegit.Desktop`), `Grid`
(165 `<Grid` elements) and `DockPanel` (`Controls/FilterBar.axaml:9`); Wane `docs/ui/mocks/css/tokens.css:131-143`.

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Lunegit src/Lunegit.Desktop/Views/TerminalPanelView.axaml:240-258,303 (StackPanel
Spacing 5, Grid `*,Auto`), MainWindow.axaml:298 (Grid `Auto,*,Auto,Auto`), Controls/FilterBar.axaml:9-44 (DockPanel), Controls/Terminal/TerminalTabDragController.cs:16-23
(totality contract); a grep of `Spacing="` over `src/Lunegit.Desktop/**/*.axaml` (most used values 8, 6, 4, 10, 20, 2, 5, 16; instrument: `grep -rhoE` piped to `sort | uniq -c`, counted
on 2026-11-04, `[derived: grep count]`); Wane docs/ui/README.md:88, docs/ui/mocks/css/tokens.css:131-143, docs/ui/components.md:198,226.
Conflict: Wane specifies "a 4 px spacing grid" with `--s1..--s6` = 4, 8, 12, 16, 20, 24 [W docs/ui/README.md:88], [W docs/ui/mocks/css/tokens.css:131-136];
Lunegit uses 5 and 6 and 10 as well [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:256], [L src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml:12]. Lunegit wins:
the layout helpers take any non-negative gap in logical px and define no token scale; each widget cites its own gap.

### Anatomy

- Axis helpers: `row` lays children left to right, `column` top to bottom; both are `stack` with an axis. A gap goes only between children, never before the first or after the last [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:252-256].
- Child main-axis size is one of fixed (px), hug (the child's own measured size) or fill with a weight (share of the leftover): the vocabulary of Avalonia's `Auto`, fixed and `*` tracks [L src/Lunegit.Desktop/MainWindow.axaml:298], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:303].
- Cross-axis alignment: start, center, end or stretch. Lunegit relies on stretch for tabs that must fill the 32 px strip [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:240-243].
- `grid`: columns and rows are tracks of fixed, auto or star size, plus a column gap and a row gap [L src/Lunegit.Desktop/MainWindow.axaml:298], [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:303].
- Insets: four per-side non-negative values shrink the parent rect before layout; the dock pattern (a left or right item and a filling item) is a row with one fill child [L src/Lunegit.Desktop/Controls/FilterBar.axaml:9-13].
- Totality: non-finite, zero or negative widths and gaps are forced to 0, and an empty list gives an empty result, as the tab drag maths does [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:16-23], [L src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs:433-440].
- Overflow: fixed and hug children that exceed the parent are laid out at their size and run past the rect; clipping and scrolling belong to the parent ([layout.scroll-area]).

### States

n/a — layout has no look and no interaction; it emits rects only.

### Transitions

n/a — layout never animates: layout of large surfaces is on the never-animate list [motion]; a moving item animates by transform inside its widget, not by re-laying out [L .claude/docs/animation-rules.md:149].

### Input and focus

n/a — no pointer sense, no key handling, no Tab stop. Layer: the layer of the widgets placed in it. Hit order follows call order within a layer [core.ui].

### Retained state

- Transient: none. Persistent: none. App-owned: gap, insets, track sizes and weights, passed every frame.
- Exit retention: no. The helpers keep nothing between frames [core.ui].

### Accessibility

n/a — no node of its own; child order in the semantic tree is call order [infra.accessibility]. An app that wants a named region wraps the children in a `group` node through its own widget.

### Strings

None: layout has no text.

### Gallery scenes

- `stack-gaps`: a row and a column of three fixed boxes with gaps 4, 8 and 12, settled.
- `stack-sizing`: a row of one hug, one fixed and two fill children of weights 1 and 2, settled.
- `stack-align`: cross-axis start, center, end and stretch side by side, settled.
- `stack-grid`: a grid with columns `Auto, *, Auto` and two rows with a row gap, settled.
- `stack-insets`: insets 8, 12, 8, 12 on a filled parent, with a scale 1 and scale 2 pair, settled.

### Painter primitives

- Existing: none (the helpers return rects; the gallery boxes use fill and ring).
- New: none.

### Acceptance

- Behaviour tests (pure functions, no input or time): a fill child of weight 2 gets twice the leftover of weight 1; the leftover is shared exactly (rects tile the parent to the last pixel, no drift over 1000 children).
- Totality: NaN, infinite, negative and zero sizes, an empty list and a single child return finite rects and never panic.
- Pixel probes at scale 2: a gap of 8 logical px is 16 device px between two filled boxes; an inset of 12 is 24 device px from the parent edge.
- Conformance lines: none, no transitions.
- Semantic-node test: n/a, no node; the child order matches call order.
- Tests, bullet by bullet (`crates/lune-ui-core/tests/layout.rs`). Behaviour: `a_weight_two_child_gets_twice_a_weight_one_childs_share`, `a_thousand_fill_children_tile_the_parent`, `a_gap_goes_only_between_children`, `cross_axis_alignment`, `a_column_runs_top_to_bottom`, `a_grid_of_auto_star_auto_columns_and_two_rows`, `fixed_and_hug_overflow_runs_past_the_parent_and_fills_get_nothing`. Totality: `bad_numbers_count_as_zero_and_every_rect_is_finite`, `bad_numbers_equal_zero_not_just_finite`, `an_empty_list_gives_an_empty_result`, and a single child of each size kind with a gap, a weight or an inset: `a_single_child_is_laid_out_whatever_its_size_kind_and_gap`. Pixel probes at scale 2, through the painter: `at_scale_2_a_gap_of_8_is_16_device_px_and_an_inset_of_12_is_24`. Child order: `rects_come_back_in_the_order_the_children_were_given`.

### Build

- Milestone: M5. Built in M5 (steps 2 and 6; the page is `stack`). M11 audited it against D47 (the `Done` line). Unverified here: the Windows and Linux run time (the lint for both is `scripts/check-targets.sh`).
- Depends on: core.ui
- Done (D47, M11): tests `crates/lune-ui-core/tests/layout.rs::a_weight_two_child_gets_twice_a_weight_one_childs_share, crates/lune-ui-core/tests/layout.rs::a_thousand_fill_children_tile_the_parent, crates/lune-ui-core/tests/layout.rs::bad_numbers_count_as_zero_and_every_rect_is_finite, crates/lune-ui-core/tests/layout.rs::a_single_child_is_laid_out_whatever_its_size_kind_and_gap, crates/lune-ui-core/tests/layout.rs::at_scale_2_a_gap_of_8_is_16_device_px_and_an_inset_of_12_is_24, crates/lune-ui-core/tests/layout.rs::rects_come_back_in_the_order_the_children_were_given`; page `stack` (scenes `stack-*`); unverified here: Windows and Linux run time.

## Open questions

- Answered (D108, no consumer yet): `wrap` (a flow layout) has no Lunegit or Wane use found in the files read; not specified until a consumer names it.
- Answered (D108, no consumer yet): whether min and max size constraints per child are needed (Lunegit's tab has `MinWidth` 108 and `MaxWidth` 270 [L src/Lunegit.Desktop/Views/TerminalPanelView.axaml:59-60]); M5 decides on the API.

## Notes for the planner

New in M3 (PD9). M5 builds it with `layout.scroll-area` and `layout.virtualized-list`; later widget specs assume the helpers exist but never require them to draw (the low-level `Painter<H>` stays public).

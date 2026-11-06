---
id: data.bar-chart
title: Stacked bar chart
area: data
status: done
consumers: wane
source: crates/lune-ui-core/src/widgets/bar_chart.rs
owner: —
summary: Hourly stacked columns, two series, three grid lines
---

## What it is

Hourly stacked columns, two series, three grid lines. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static. Hover tooltips with the exact value are a requirement (needs `overlay.tooltip`), not mocked.

## Needs

- Wane — Variants: none; the only chart type (no line, area or donut anywhere in the mocks) (R-data-2)
- Wane — States: data; empty (grid lines and x labels stay, no bars and no y labels, because the scale comes from data the UI does not have). (R-data-2)
- Wane — Sizes: Plot 160 high; y labels in a 40 px gutter (RznMono 11 tertiary); three 1 px `--border-subtle` grid lines (0, 50, 100%); x labels every 3 h; bars 12 wide, centred in equal slots, flat ends, 1 px between segments; series: cache read `--data`, uncached and cache write `--accent`; legend swatch 8 px, radius 3. (R-data-2)
- Wane — Behaviour: Static. Hover tooltips with the exact value are a requirement (needs `overlay.tooltip`), not mocked. (R-data-2)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): docs/mockups/Main.dc.html:82-100 (content); Lunegit Controls/StoragePieChartControl.cs:18 (custom drawing precedent).

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, Wane 69931d6, this repo fa2958b. Read: Wane `docs/ui/components.md:641-654`; Lunegit `src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:15-21` (custom control, `ChartDiameter` 160); `crates/lune-ui-core/src/theme.rs:57-64,63`
(`fg_secondary`, `fg_tertiary`, `border_subtle`, `accent`).
Conflicts:
- Series colour `--data`: no repo token (`grep -n 'data' crates/lune-ui-core/src/theme.rs` has no series token): token to add (M4), `data` = #5C8FE6 [W docs/ui/mocks/css/tokens.css:58], [D58].
- Accent: `accent` #767D85FF [R crates/lune-ui-core/src/theme.rs:64] is used for the uncached and write series, as Wane says.

### Anatomy

- Plot: 160 high [W docs/ui/components.md:641-654].
- Y labels: 40 px gutter; RznMono 11, `fg_tertiary` [W docs/ui/components.md:641-654], [R crates/lune-ui-core/src/theme.rs:58,82].
- Grid: three 1 px `border_subtle` lines at 0, 50 and 100% [W docs/ui/components.md:641-654], [R crates/lune-ui-core/src/theme.rs:63].
- X labels: every 3 h [W docs/ui/components.md:641-654].
- Bars: 12 wide, centred in equal slots; flat ends; 1 px gap between segments [W docs/ui/components.md:641-654].
- Series: cache read `--data` (token to add, [D58]); uncached and cache write `accent` [W docs/ui/components.md:641-654], [R crates/lune-ui-core/src/theme.rs:64].
- Legend: 8 px swatch, radius 3 [W docs/ui/components.md:641-654].

### States

| state | look | cite |
|---|---|---|
| data | bars, grid, labels, legend | [W docs/ui/components.md:641-654] |
| empty | grid and x labels stay; no bars; no y labels | [W docs/ui/components.md:641-654] |
| hover a column | exact value tooltip (overlay.tooltip); a requirement, not mocked | [W docs/ui/components.md:641-654], [overlay.tooltip] |
| focus-visible | not applicable: no focusable element in the mock | [W docs/ui/components.md:641-654] |

### Transitions

Value transitions (D165, D5 of M8): a segment's height tweens with `Kind::Value` (fast, quad-out; Reduced and Off snap) on its share of the plot (`value / max`), so a change of a value, of `max`, empty to data and back, and a rolling window all animate the same way; a column is keyed by its index and morphs in place, nothing slides. A column new to the chart appears at its height at once. `BarChartOptions.live` snaps (the never-animate rule for high-frequency content, as the stat). No hover motion. Corrects the earlier "n/a" under D40.

### Input and focus

- Pointer: hover a column shows its value through `overlay.tooltip` (requirement) [W docs/ui/components.md:641-654].
- Keyboard: none in the mock.
- Tab stop: no [derived: a static chart with no focusable element].
- Layer: whatever layer the owner draws it on; the tooltip uses the popup layer [derived].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: the hovered column; one tween per (column, series).
- Persistent: none.
- App-owned: the hourly series values (two series), the scale, the legend labels [W docs/ui/components.md:641-654].
- Exit retention: no.

### Accessibility

- Role `image` named by `label` (fallback `luneui.bar-chart.name`) with `description`; the legend entries are `static-text` children [infra.accessibility].
- Tooltip values: exposed through the tooltip's `tooltip` role [overlay.tooltip].
- Live region: none.

### Strings

One toolkit key, `luneui.bar-chart.name` ("Bar chart"), the fallback name. Axis labels and legend names are app text.

### Gallery scenes

- `bar-chart-data`: 24 hourly columns, two series, legend, settled.
- `bar-chart-empty`: no data, grid and x labels only, settled.
- `bar-chart-hover`: column 10 hovered with its tooltip, settled.
- `bar-chart-value-mid`: the data retargets; captured in the middle of the tween (`Moving`).

### Painter primitives

- Existing only: `fill` (bars, grid, swatches), `text` (labels). New: none.

### Acceptance

- Behaviour (scripted input, injected time): a hover on a column opens its tooltip with the exact value; an empty chart draws no bars.
- Pixel probes at scale 2: plot 320 px high; bar width 24 px; grid lines 2 px; y gutter 80 px.
- Conformance: the `Kind::Value` row on Full, Reduced and Off (`widget_bar_chart.rs`).
- Semantic node: `image` plot with a name; legend `static-text`.
- Tests, bullet by bullet. `crates/lune-ui-core/tests/widget_bar_chart.rs`: behaviour `a_column_shows_its_tip_after_the_delay_and_an_empty_tip_shows_none`, `hovered_reports_the_slot_under_the_pointer`, `an_empty_chart_keeps_the_grid_and_the_x_labels`, `values_that_are_not_numbers_never_panic_and_count_as_zero`, `a_stack_above_max_is_cut_at_the_top_of_the_plot`; pixel probes `the_entry_probes_at_scale_2_plot_320_bar_24_grid_2_gutter_80`, `series_0_is_at_the_bottom_and_segments_are_1_px_apart`, `the_legend_names_the_series_with_swatches_of_8_px_radius_3`; conformance `the_value_row_conforms_under_every_tier`, `a_change_of_value_animates_once_and_ends_exactly_at_its_height`, `off_and_reduced_snap`, `idle_at_rest`; node `the_chart_is_an_image_named_by_the_app_over_its_legend`, `an_empty_label_falls_to_the_toolkit_string_through_the_table`, `a_disabled_scope_marks_every_node`.

### Build

- Milestone: M8.
- Depends on: core.painter, core.theme, overlay.tooltip, infra.accessibility.
- Done (D47, M11): tests `crates/lune-ui-core/tests/widget_bar_chart.rs::a_column_shows_its_tip_after_the_delay_and_an_empty_tip_shows_none, crates/lune-ui-core/tests/widget_bar_chart.rs::an_empty_chart_keeps_the_grid_and_the_x_labels, crates/lune-ui-core/tests/widget_bar_chart.rs::the_entry_probes_at_scale_2_plot_320_bar_24_grid_2_gutter_80, crates/lune-ui-core/tests/widget_bar_chart.rs::the_value_row_conforms_under_every_tier, crates/lune-ui-core/tests/widget_bar_chart.rs::a_change_of_value_animates_once_and_ends_exactly_at_its_height, crates/lune-ui-core/tests/widget_bar_chart.rs::the_chart_is_an_image_named_by_the_app_over_its_legend`; page `bar-chart`; unverified here: none.

## Open questions

All answered in M8:
- Series colour `--data`: the app passes each series' colour (`BarSeries.color`); the gallery uses `data` and `accent`. Any number of series, series 0 at the bottom.
- The `image` name: `BarChartOptions.label` (empty falls to `luneui.bar-chart.name`) with `description` as the summary.
- Whether bars animate: yes, see Transitions.
- Custom-draw or painter rectangles: painter rectangles, no `CustomDraw`.

## Notes for the planner

Built in M8 (D164, D165). API: `Ui::bar_chart(key, rect, &BarChartOptions) -> BarChartResponse { hovered }`, `bar_chart::height()`. The top y label is centred on the top grid line, so it reaches 8 px above the rect. The legend sits under the x labels (Wane draws it in the panel header). Values that are negative, NaN or infinite count as 0; a stack above `max` is cut at the plot's top. The page's button retargets the data; the bench presses it.


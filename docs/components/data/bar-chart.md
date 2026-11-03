---
id: data.bar-chart
title: Stacked bar chart
area: data
status: planned
consumers: wane
source: —
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

n/a — static in the mock (Wane: "Static"); no bar-growth motion has a source. Data changes redraw without motion.

### Input and focus

- Pointer: hover a column shows its value through `overlay.tooltip` (requirement) [W docs/ui/components.md:641-654].
- Keyboard: none in the mock.
- Tab stop: no [derived: a static chart with no focusable element].
- Layer: whatever layer the owner draws it on; the tooltip uses the popup layer [derived].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: the hovered column.
- Persistent: none.
- App-owned: the hourly series values (two series), the scale, the legend labels [W docs/ui/components.md:641-654].
- Exit retention: no.

### Accessibility

- Role `image` for the plot, with the summary as its name (open question: the summary text); the legend entries are `static-text` [infra.accessibility].
- Tooltip values: exposed through the tooltip's `tooltip` role [overlay.tooltip].
- Live region: none.

### Strings

None: axis labels and legend names are app text (the mock's hour labels and series names are not in any source).

### Gallery scenes

- `bar-chart-data` (new): 24 hourly columns, two series, legend, settled.
- `bar-chart-empty` (new): no data, grid and x labels only, settled.
- `bar-chart-hover` (new): one column hovered with its tooltip, settled.

### Painter primitives

- Existing: fill (bars, grid, swatches), text (labels). [R crates/lune-ui-core/src/paint.rs:215,274]
- New: none.
- The chart is painter rectangles; a custom-draw hook is the alternative (Open questions).

### Acceptance

- Behaviour (scripted input, injected time): a hover on a column opens its tooltip with the exact value; an empty chart draws no bars. None exists.
- Pixel probes at scale 2: plot 320 px high; bar width 24 px; grid lines 2 px; y gutter 80 px.
- Conformance: none (no motion).
- Semantic node: `image` plot with a name; legend `static-text`.

### Build

- Milestone: M8.
- Depends on: core.painter, core.theme, overlay.tooltip, infra.accessibility.

## Open questions

- Series colour `--data`: token to add (M4), #5C8FE6 [W docs/ui/mocks/css/tokens.css:58], [D58]; further series colours (more than two) are not in the sources.
- The chart's summary text for the `image` name: not in the mock; open.
- Whether bars animate on a data change: no source; open.
- Drawn by a `render.custom-draw` hook or painter rectangles: the planner decides; the painter path is the default here.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.bar-chart`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, overview. Related stub `data.pie-chart` shows the custom-draw precedent in Lunegit.

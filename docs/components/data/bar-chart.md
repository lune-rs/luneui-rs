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

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: Drawn by a `render.custom-draw` hook or painter rectangles; nothing in CSS here depends on a browser effect.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.bar-chart`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, overview.

---
id: data.pie-chart
title: Pie chart
area: data
status: planned
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Controls/StoragePieChartControl.cs
owner: —
summary: 160 px pie of slices sized by value, drawn from the top clockwise, with a glass highlight over the top half
---

## What it is

A pie chart for storage usage: slices sized by their share of the total, drawn from the top clockwise, with a 15% white highlight over the top half [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:13-17,84-107]. No Wane mock: the Wane mocks have no pie (`data.bar-chart` says the only chart type is the bar, [W docs/ui/components.md:641-654]).

## Needs

- Custom drawing through the painter (R-data-1).
- Slices from app data with colour keys (R-data-2).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Controls/StoragePieChartControl.cs`

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:10-175` (constants 20-21, `Render` 80-108, `DrawSlice` 111-137, `DrawHalfCircleTop` 140-158, `GetBrush` 161-174);
`crates/lune-ui-core/src/paint.rs:215,243` (painter primitives, for the New list).
Conflicts: none (no Wane or moved pie). The highlight alpha is 38 of 255 (about 15%) [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:106].

### Anatomy

- Diameter 160; radius 80 [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:20-21,75].
- Start: top of the circle (−π/2), then clockwise [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:88,95-101,125-132].
- Slice: sweep = size ÷ total × 2π; a slice larger than half turns the large-arc flag [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:95,125].
- Slice colour: looked up from the app's colour key at each render, with a grey fallback [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:165-173].
- Highlight: one white half-circle over the top half, alpha 38 [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:106-107,140-158].

### States

| state | look | cite |
|---|---|---|
| slices | one sector per slice with a positive size | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:91-102] |
| empty (no slices or total ≤ 0) | nothing drawn | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:82,85] |
| zero-size slice | skipped | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:93] |
| single slice of the whole | the sweep is 2π; the arc from start to end at the same point is not checked | open question |
| hover a slice | none in the control | [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:80-108] |

### Transitions

n/a — the control has no motion; a value change redraws. Lunegit's never-animate list covers it: the chart is
continuously updating content when it changes often ([L .claude/docs/animation-rules.md:148]).

### Input and focus

- Pointer: none in the control.
- Keyboard: none.
- Tab stop: no.
- Layer: whatever the owner draws it on [derived: a custom control].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: none.
- Persistent: none.
- App-owned: the slices (size in bytes, colour key) [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:32-33,84].
- Exit retention: no.

### Accessibility

- Role `image`; name = the chart's title (app text); description = the slice list as text (open question: the slice text is not in the control).
- Live region: none.

### Strings

None: the control draws no text; the legend and values are app text.

### Gallery scenes

- `pie-chart-three` (new): three slices of different sizes, settled.
- `pie-chart-empty` (new): no slices, settled (nothing drawn).
- `pie-chart-half` (new): one slice larger than half the total (large-arc case), settled.

### Painter primitives

- Existing: fill, ring. [R crates/lune-ui-core/src/paint.rs:215,231]
- New: sector, path-fill, arc.
- Sector: each slice is `MoveTo(centre)`, `LineTo(arcStart)`, an arc to `arcEnd`, `LineTo(centre)` [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:113-137].
- Path-fill: the top half-circle highlight [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:140-158].
- Arc: the arc segment inside the sector [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:132].

### Acceptance

- Behaviour (scripted input, injected time): slice sweeps sum to 2π for any positive total; an empty input draws nothing; a zero slice is skipped. None exists.
- Pixel probes at scale 2: diameter 320 px; the first slice starts at the top centre; highlight alpha 38 over the top half.
- Conformance: none (no motion).
- Semantic node: `image` with a name; no per-slice nodes in the control.

### Build

- Milestone: M8.
- Depends on: core.painter, core.theme, infra.accessibility.

## Open questions

- Single-slice full circle: the arc from a point to the same point is not verified in the control; open.
- Slice labels and a legend: none in the control; open (the app or a new entry).
- Colour keys: the toolkit adds the chart series colour `data` (token to add, M4, [D58], [data.bar-chart]); slice colours beyond it are app-supplied per slice, as Lunegit looks them up at render [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:165-173].
- Hover on a slice: none in the control; open.

## Notes for the planner

The painter has no sector, arc or path-fill yet (paint.rs has fill, fill_radii, fill_over, ring, acrylic, backdrop, custom, text and stroke); M8 adds them under `core.painter`'s primitive table (step 15).

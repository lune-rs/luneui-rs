---
id: data.pie-chart
title: Pie chart
area: data
status: in-progress
consumers: lunegit
source: crates/lune-ui-core/src/widgets/pie_chart.rs
owner: toolkit-v1 M8
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

Value transitions (D166, D5 of M8): each slice's share tweens with `Kind::Value` (fast, quad-out; Reduced and Off snap) under the app's slice `key`; the shares drawn are divided by their sum every frame, so the sweeps total 2π in flight and after a retarget. `PieOptions.live` snaps. A slice removed leaves at once and the rest take the turn; from no slice to some, and a slice new to a chart that has others, appear at once. Corrects the earlier "n/a" under D40.

### Input and focus

- Pointer: none in the control.
- Keyboard: none.
- Tab stop: no.
- Layer: whatever the owner draws it on [derived: a custom control].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: one tween per slice key.
- Persistent: none.
- App-owned: the slices (size in bytes, colour key) [L src/Lunegit.Desktop/Controls/StoragePieChartControl.cs:32-33,84].
- Exit retention: no.

### Accessibility

- Role `image` named by `label` (fallback `luneui.pie-chart.name`), `description` the app's slice list as text; no children.
- Live region: none.

### Strings

One toolkit key, `luneui.pie-chart.name` ("Pie chart"), the fallback name. The control draws no text.

### Gallery scenes

- `pie-chart-three`: three slices, settled.
- `pie-chart-empty`: no slices, settled (nothing drawn).
- `pie-chart-half`: one slice above half the total.
- `pie-chart-full`: one slice of the whole.
- `pie-chart-value-mid`: the data retargets; captured in the middle of the tween (`Moving`).

### Painter primitives

- Existing: `sector` (each slice and the highlight, as one fan, no seam at the centre). The earlier "new: sector, path-fill, arc" is out of date: they exist. New: none.

### Acceptance

- Behaviour (scripted input, injected time): slice sweeps sum to 2π for any positive total; an empty input draws nothing; a zero slice is skipped. None exists.
- Pixel probes at scale 2: diameter 320 px; the first slice starts at the top centre; highlight alpha 38 over the top half.
- Conformance: the `Kind::Value` row on Full, Reduced and Off, and the sweeps totalling 2π in every frame (`widget_pie_chart.rs`).
- Semantic node: `image` with a name; no per-slice nodes in the control.

### Build

- Milestone: M8.
- Depends on: core.painter, core.theme, infra.accessibility.

## Open questions

All answered in M8:
- Single-slice full circle: `Painter::sector` at a sweep of 2π is the whole disc (one fan); tested at the four quarter points.
- Slice labels and a legend: none; the app draws its own.
- Colours: the app passes one colour per slice.
- Hover on a slice: none.

## Notes for the planner

Built in M8 (D164, D166). API: `Ui::pie_chart(key, rect, &PieOptions)` drawn at 160 px from the rect's top left, `pie_chart::sweeps(&[f32])`, `HIGHLIGHT`. The highlight is one `sector` from -π over π, drawn after the slices and only when a slice shows. A total that overflows f64 draws nothing. The page's button retargets the data; the bench presses it.


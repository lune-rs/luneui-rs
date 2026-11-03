---
id: data.lane-graph
title: Lane graph (row)
area: data
status: planned
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Controls/GraphRowControl.cs
owner: —
summary: One row of a lane graph: connection curves between lanes, vertical lane lines, and a node dot in its lane; generalised from commits to app data
---

## What it is

One row of a lane graph, drawn in three passes: connection curves between lanes (bottom), vertical lines for the active lanes (middle), and the node dot (top). The row is code-only: a `Control` with a custom `Render` [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:15,102-154]. Renamed from the Lunegit commit-graph row (D10): the entry is generalised from commits to lanes, nodes and edges, with the lane layout left to the app.

## Needs

- Custom drawing through the painter (R-core-1, R-data-1).
- Row count and scroll performance (R-nfr-5).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Controls/GraphRowControl.cs`

## Spec

### Evidence

Pinned: Lunegit 33a1dd41, this repo fa2958b. Read: `src/Lunegit.Desktop/Controls/GraphRowControl.cs:15-54` (class, constants 24-29, caches 32-39, `DataProperty` 43-54),
`:96-154` (measure, the three render passes), `:161-170` (lane pen: round caps), `:172-243` (connection geometry and `RoundCorner` with quadratic béziers);
`.claude/docs/animation-rules.md:146` (never animate the graph canvas).
Conflicts: none. The lane layout (which lane each commit takes) is outside the control: the app passes the data in (`DataProperty`, GraphRowControl.cs:43-54).

### Anatomy

- Row height 45 [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:24]; centre line at 22.5 [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:109].
- Lane width 25 [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:25]; left offset 20 [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:26]; lane x = 20 + lane × 25 [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:114,141,151].
- Measure: width = 20 + (max lane + 1) × 25 + 20; height 45 [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:96-99].
- Pass 1, connection curves: same-lane connections are straight vertical lines; cross-lane connections are a canonical geometry anchored at the child dot and translated to the row [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:112-135].
- Corner: a quadratic bézier at the bend, maximum radius 10 [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:29,215,241].
- Pass 2, lane lines: each active lane runs from the row's top (or the centre when it does not continue above) to the bottom (or the centre when it does not continue below) [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:139-148].
- Pass 3, node dot: radius 5, at the child lane x and the centre line [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:27,150-153].
- Edge pen: thickness 2.5, round caps [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:28,167].
- Colours: a brush per lane colour key and variant (`GraphStroke` for edges, `GraphDot` for the dot) [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:152,156-158,167].
- Geometry cache: bounded LRU of 2048 entries, so scroll distance does not grow it [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:35-39].

### States

| state | look | cite |
|---|---|---|
| node only (no lane lines) | dot in its lane | [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:150-153] |
| lane continues below | lane line to the row's bottom | [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:144-147] |
| lane continues above | lane line from the row's top | [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:144-147] |
| cross-lane connection | curve between lanes, with a corner of radius up to 10 | [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:112-135,215] |
| node selected or hovered | not in the control; the owner draws it | open question |

### Transitions

n/a — never animate the graph canvas: it renders via `DrawingContext` and must not be wrapped in transitions
([L .claude/docs/animation-rules.md:146]). A data change redraws.

### Input and focus

- Pointer: none in the control; the owning list handles selection.
- Keyboard: none.
- Tab stop: no (the row's owner is the stop).
- Layer: Base [derived: a custom control in the list, R crates/lune-ui-core/src/scene.rs:17-22].
- Focus scope: none.
- Pointer capture while pressed: none.

### Retained state

- Transient: none.
- Persistent: the geometry cache (bounded LRU, 2048 entries) and the pen cache keyed by colour key [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:32-39].
- App-owned: the row's data object (`DataProperty`): the row index, the dot lane and colour key, the active lanes with their continue flags and colour keys, and the outgoing connections [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:43-54,95-154].
- Exit retention: no.

### Accessibility

- Decorative graph: no node; the row's text (the owner's list-item) is the accessible content [infra.accessibility].
- Live region: none.

### Strings

None: the control draws no text; the row's text is the owner's.

### Gallery scenes

- `lane-graph-linear` (new): one lane, one node, lane lines above and below, settled.
- `lane-graph-merge` (new): two lanes merging into one node, with a connection curve, settled.
- `lane-graph-branch` (new): a branch leaving a node on a second lane, with a connection curve, settled.

### Painter primitives

- Existing: fill (node dot: a fill with the radius at half the diameter), stroke (lane lines and curves: `stroke` takes a point list, so a curve is flattened to points). [R crates/lune-ui-core/src/paint.rs:215,329]
- New: none.
- The bézier corner is drawn as a flattened point list through `stroke`; no new primitive is needed.

### Acceptance

- Behaviour (scripted input, injected time): a row with a dot in lane 2 draws the dot at x = 20 + 2 × 25 = 70 on the centre line 22.5; a lane that continues above starts at y = 0. None exists.
- Pixel probes at scale 2: row 90 px; node dot diameter 10 px at x = 40 + lane × 50; edge 5 px wide.
- Conformance: none (no motion).
- Semantic node: none (decorative); the owner's list-item carries the row.

### Build

- Milestone: M8.
- Depends on: core.painter, layout.virtualized-list, infra.accessibility.

## Open questions

- Flattening tolerance for the bézier corner through `stroke`: not in the source; open.
- Row count and scroll performance (R-nfr-5): the geometry cache bounds memory (2048); the frame budget is not measured here; open.
- Lane layout algorithm: outside the control; the app owns it; whether the toolkit ships a helper is open for the planner.
- Hover and selection looks for the row: the owner draws them; open.

## Notes for the planner

Renamed from the Lunegit commit-graph row (D10): the commit words are gone from the entry; lanes, nodes and edges are the app's data.

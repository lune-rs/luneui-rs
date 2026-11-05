---
id: data.lane-graph
title: Lane graph (row)
area: data
status: in-progress
consumers: lunegit
source: crates/lune-ui-core/src/widgets/lane_graph.rs
owner: toolkit-v1 M8
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
- Geometry cache: bounded LRU of 2048 entries [L src/Lunegit.Desktop/Controls/GraphRowControl.cs:35-39]; not ported (see Retained state).

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
- Persistent: none. Lunegit's 2048-entry geometry cache and pen cache are not ported: only the rows in view are built (a virtualized list), so there is nothing for an LRU to bound, and `edge_points` is a few dozen operations per edge (D170).
- App-owned: the row's data (`LaneRow`: the dot lane and colour, the lane lines with their continue flags, the edges with rows relative to the row). The lane layout is the app's: the toolkit ships no layout helper.
- Exit retention: no.

### Accessibility

- Decorative graph: no node; the row's text (the owner's list-item) is the accessible content [infra.accessibility].
- Live region: none.

### Strings

None: the control draws no text; the row's text is the owner's.

### Gallery scenes

- `lane-graph-linear`: one lane, a node per row, lane lines above and below, settled.
- `lane-graph-merge`: a node on lane 1 whose edge goes straight down and bends into the trunk, settled.
- `lane-graph-branch`: a branch leaving a node for lane 1, settled.
- `lane-graph-list`: the graph beside a text column in a 100,000-row `virtual_list`, row 2 selected and row 4 hovered (the owner's look), settled.

### Painter primitives

- Existing only: `stroke` (lane lines and edges, the corner flattened to points by `edge_points`) and a closed `stroke` for the dot (32 points on a circle of centre radius 2, stroke width 6: a disc of radius 5; `arc` flattens on the centre radius and left visible flats at scale 2, so it is not used). Everything is on the stroke list, which draws after every fill of a layer and in record order, so the dot is the top of the row's strokes; the GPU probe `a_lane_graph_dot_is_a_disc_of_diameter_10_over_a_line_of_another_colour` (lune-ui-wgpu, `primitives.rs`) proves the stroked circle reads as a solid disc. New: none.
- Butt ends, not Lunegit's round caps: `StrokeStyle` has no cap mode. An edge ends under the dot and lane lines meet at the row's edges, so no end shows.

### Acceptance

- Behaviour (scripted input, injected time): a row with a dot in lane 2 draws the dot at x = 20 + 2 × 25 = 70 on the centre line 22.5; a lane that continues above starts at y = 0. None exists.
- Pixel probes at scale 2: row 90 px; node dot diameter 20 device px (10 logical; the earlier "10 px" was logical) at x = 40 + lane × 50; edge 5 px wide.
- Conformance: none (no motion; never animated).
- Semantic node: none (decorative); the owner's list-item carries the row.

### Build

- Milestone: M8.
- Depends on: core.painter, layout.virtualized-list, infra.accessibility.

## Open questions

All answered in M8 (D170):
- Flattening tolerance: each quadratic corner is 8 segments (`CORNER_STEPS`); the chord stays under 0.25 device px at scale 2 for radius 10 (`the_flattened_corner_stays_within_a_quarter_device_pixel_at_scale_2`).
- Row count and scroll performance: the geometry is rebuilt per visible row each frame; the frame budget is the page's bench (100,000 rows in a virtualized list, scrolled), judged by D85.
- Lane layout: the app's; no helper.
- Hover and selection: the owner's (a `virtual_list` row); the `lane-graph-list` scene shows the list's own selected and hovered fills under the graph.

## Notes for the planner

Renamed from the Lunegit commit-graph row (D10): the commit words are gone from the entry; lanes, nodes and edges are the app's data. Built in M8 (D170). API: `lane_graph::paint(p, origin, &LaneRow)`, `Ui::lane_graph_row(rect, &LaneRow)`, `lane_graph::width(max_lane)`, `lane_graph::edge_points(d_lanes, d_rows)`. Geometry reaches past the row's rect and nothing clips it: the owner's list does. Rows in `LaneEdge` are relative to the row drawn.

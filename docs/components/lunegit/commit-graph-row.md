---
id: lunegit.commit-graph-row
title: Commit graph row
area: lunegit
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Controls/GraphRowControl.cs
owner: —
summary: Custom-drawn commit graph row (rough stub)
---

## What it is

Rough stub — not analysed. `GraphRowControl` is a code-only `Control` with a custom `Render`; notes call it a core rebuild item.

## Needs

- Custom drawing through the painter (R-core-1, R-data-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Controls/GraphRowControl.cs`

## Open questions

- Lane layout algorithm and where it lives (probably outside the control).
- Row count and scroll performance (R-nfr-5).

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

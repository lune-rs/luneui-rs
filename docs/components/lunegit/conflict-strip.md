---
id: lunegit.conflict-strip
title: Conflict resolver and hunk strip
area: lunegit
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/ConflictResolverView.axaml
owner: —
summary: Merge conflict resolver view and its hunk strip overlay (rough stub)
---

## What it is

Rough stub — not analysed. A resolver view plus `ConflictHunkStripOverlay`, an AvaloniaEdit-side overlay that was not read in depth.

## Needs

- Overlay on a text view (R-text-3, R-data-1).
- Keyboard bindings (R-infra-2).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/ConflictResolverView.axaml`
- `src/Lunegit.Desktop/Rendering/ConflictHunkStripOverlay.cs`

## Open questions

- How the strip is drawn and how it relates to the editor.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

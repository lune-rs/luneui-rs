---
id: layout.tree-view
title: Tree view and tree guides
area: layout
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Controls/TreeGuideControl.cs
owner: —
summary: Tree rows with guide lines for branches, tags, stash and files (rough stub)
---

## What it is

Rough stub — not analysed. `TreeGuideControl` draws tree guide lines; `SidePanelRow` models rows for the branch tree, tags and stash. A `TreeView` selector was not found in the style survey.

## Needs

- Tree rows with guide lines (R-layout-3).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Controls/TreeGuideControl.cs`
- `src/Lunegit.Desktop/ViewModels/SidePanelRow.cs`
- `src/Lunegit.Desktop/Views/SidePanels/BranchesPanelView.axaml`

## Open questions

- Expand and collapse behaviour and keyboard navigation are unverified.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

---
id: lunegit.side-panel
title: Side panel host and panels
area: lunegit
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/SidePanelView.axaml
owner: —
summary: Side panel host plus repository, branches, commit and file explorer panels (rough stub)
---

## What it is

Rough stub — not analysed. A host view and about nine panels (repository, branches, commit, file explorer, AI sessions, scratchpad, help, notification history, settings).

## Needs

- Host that switches panels (R-layout-1).
- Row lists inside panels (R-layout-3).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/SidePanelView.axaml`
- `src/Lunegit.Desktop/Views/SidePanels/RepositoryPanelView.axaml`
- `src/Lunegit.Desktop/Views/SidePanels/BranchesPanelView.axaml`
- `src/Lunegit.Desktop/Views/SidePanels/CommitPanelView.axaml`
- `src/Lunegit.Desktop/Views/SidePanels/FileExplorerPanelView.axaml`

## Open questions

- Which parts are generic panel chrome and which are Git UI.

## Notes for the planner

Mostly Git-specific content; only the host and panel chrome could be toolkit components.

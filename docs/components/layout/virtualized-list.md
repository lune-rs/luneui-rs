---
id: layout.virtualized-list
title: Virtualized list
area: layout
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/MainWindow.axaml
owner: —
summary: Virtualizing list used by several Lunegit views (rough stub)
---

## What it is

Rough stub — not analysed. `VirtualizingStackPanel` appears in these views. Whether the commit list is virtualized, and what `BoundedLru` caches, were not inspected.

## Needs

- Virtualized lists (R-layout-3).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/MainWindow.axaml`
- `src/Lunegit.Desktop/Views/TerminalPanelView.axaml`
- `src/Lunegit.Desktop/Views/Settings/LogsSettingsView.axaml`
- `src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml`
- `src/Lunegit.Desktop/Controls/BoundedLru.cs`

## Open questions

- Row count targets are TBD by planner (R-nfr-5).
- Variable row height?

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

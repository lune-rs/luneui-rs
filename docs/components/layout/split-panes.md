---
id: layout.split-panes
title: Resizable split panes
area: layout
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/MainWindow.axaml
owner: —
summary: Resizable split layout between panels and content (rough stub)
---

## What it is

Rough stub — not analysed. Lunegit's main window and content views host panels side by side. The survey did not verify which splitter control it uses.

## Needs

- Resizable split panes (R-layout-2).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/MainWindow.axaml`
- `src/Lunegit.Desktop/Views/RepositoryContentView.axaml`
- `src/Lunegit.Desktop/Views/ExplorerContentView.axaml`

## Open questions

- Which splitter does Lunegit use, and which sizes persist?
- Drag handling lives with `infra.drag-and-drop`.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

---
id: infra.drag-and-drop
title: In-window drag and drop
area: infra
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs
owner: —
summary: Drag and drop for tab reorder, kanban cards and the split diff (rough stub)
---

## What it is

Rough stub — not analysed. Terminal tab reorder, the scratchpad board and the split diff view all handle drag and drop.

## Needs

- Pointer capture and drag state (R-infra-5).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Controls/Terminal/TerminalTabDragController.cs`
- `src/Lunegit.Desktop/Views/Scratchpad/ScratchpadBoardView.axaml.cs`
- `src/Lunegit.Desktop/Views/SplitDiffView.axaml.cs`

## Open questions

- Is the OS drag-and-drop API involved or only pointer tracking? Unverified.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

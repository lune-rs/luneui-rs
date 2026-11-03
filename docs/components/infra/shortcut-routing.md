---
id: infra.shortcut-routing
title: Keyboard shortcut routing
area: infra
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/MainWindow.axaml
owner: —
summary: Per-view key bindings; no central registry located (rough stub)
---

## What it is

Rough stub — not analysed. `KeyBinding` and `KeyGesture` appear in several views; a central registry was not located.

## Needs

- Shortcut routing (R-infra-2).
- Widget focus and key handling (R-widget-3).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/MainWindow.axaml`
- `src/Lunegit.Desktop/Views/UnifiedDiffView.axaml`
- `src/Lunegit.Desktop/Views/ConflictResolverView.axaml`
- `src/Lunegit.Desktop/Controls/Terminal/TerminalKeyRouter.cs`

## Open questions

- Is there a central registry, and are shortcuts user-editable?

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

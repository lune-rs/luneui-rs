---
id: overlay.toast-notifications
title: Toasts and notification history
area: overlay
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/MainWindow.axaml
owner: —
summary: Toast overlay and the notification history panel (rough stub)
---

## What it is

Rough stub — not analysed. A toast overlay near `MainWindow.axaml:272`, a notification service and a history panel; type-to-accent and icon converters exist.

## Needs

- Toasts and history (R-overlay-2).
- Timed dismissal needs app-supplied time (R-nfr-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/MainWindow.axaml`
- `src/Lunegit.Desktop/Services/NotificationService.cs`
- `src/Lunegit.Desktop/Views/SidePanels/NotificationHistoryPanelView.axaml`

## Open questions

- Stacking, timing and actions on toasts.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

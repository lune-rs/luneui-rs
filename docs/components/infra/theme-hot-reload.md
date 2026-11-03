---
id: infra.theme-hot-reload
title: Theme JSON loading and hot reload
area: infra
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Theming/ThemeApplicator.cs
owner: —
summary: JSON themes applied at run time with a ThemeApplied event (rough stub)
---

## What it is

Rough stub — not analysed. `ThemeApplicator` applies JSON themes (keys like `GlassMaterialDarkOpacity`) and raises a static `ThemeApplied` event; sample themes: blue, dark-modern, green, orange, purple.

## Needs

- Themes from JSON re-applied at run time (R-infra-3, R-core-4).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Theming/ThemeApplicator.cs`
- `src/Lunegit.Desktop/SampleThemes/dark-modern.json`

## Open questions

- Which theme keys map to `Theme` tokens today.

## Notes for the planner

A planner must read the Lunegit code before anyone builds this; the lines above come from a file-name survey, not a read.

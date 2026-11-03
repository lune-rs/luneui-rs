---
id: lunegit.settings-pages
title: Settings pages
area: lunegit
status: stub
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/SettingsPageView.axaml
owner: —
summary: Settings page and its eight section views (rough stub)
---

## What it is

Rough stub — not analysed. A page plus eight section views (general, appearance, git, terminal, storage, logs, advanced, AI sessions).

## Needs

- Form widgets (R-widget-1) laid out in sections (R-layout-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/SettingsPageView.axaml`
- `src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml`
- `src/Lunegit.Desktop/Views/Settings/AppearanceSettingsView.axaml`

## Open questions

- Which controls the pages use besides the existing set.

## Notes for the planner

Compare `luneterm.settings-page`; a shared settings-page component may fall out of both.

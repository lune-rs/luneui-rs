---
id: lunegit.settings-pages
title: Settings pages
area: lunegit
status: external
consumers: lunegit
source: Lunegit: src/Lunegit.Desktop/Views/SettingsPageView.axaml
owner: —
summary: Lunegit's settings page and its eight section views (general, appearance, git, terminal, storage, logs, advanced, AI sessions); the reusable parts are toolkit entries
---

## What it is

Lunegit's settings page: a page shell with a restart banner, and eight section views. The page's reusable parts (the shell, the setting rows, the section nav) are toolkit entries; the eight sections are Lunegit's own content and stay in the app [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:9-60], [L src/Lunegit.Desktop/Views/Settings] (eight files at 33a1dd41).

## Needs

- Form widgets (R-widget-1) laid out in sections (R-layout-1).

## Existing code

Lunegit (`/Users/ijat/Projects/AvaloniaApps/Project Lunegit`, HEAD 33a1dd4), paths relative to the repo root:
- `src/Lunegit.Desktop/Views/SettingsPageView.axaml`
- `src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml`
- `src/Lunegit.Desktop/Views/Settings/AppearanceSettingsView.axaml`

## Spec

### Evidence

Pinned: Lunegit 33a1dd41. Read: `src/Lunegit.Desktop/Views/SettingsPageView.axaml:1-62` (page shell, restart banner, content host);
`src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:146-164` (a settings row: bordered, title and description, toggle);
the `Views/Settings/` listing at 33a1dd41: AdvancedSettingsView, AiSessionsSettingsView, AppearanceSettingsView, GeneralSettingsView,
GitSettingsView, LogsSettingsView, StorageSettingsView, TerminalSettingsView (each `.axaml` with `.axaml.cs`).
Conflicts: none. The eight sections are app content; the shell and rows are the reusable part (see Reusable parts).

### Reusable parts

- Page shell, restart banner and content host: `layout.settings-page` [L src/Lunegit.Desktop/Views/SettingsPageView.axaml:9-60].
- Setting row (bordered row, title and description, control at the right): `widget.row` [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:148-163].
- Toggle inside a row: `widget.toggle` [L src/Lunegit.Desktop/Views/Settings/GeneralSettingsView.axaml:160-163].
- Section nav: the nav of Lunegit's settings side panel was not read; `layout.sidebar` is the candidate (open question in `layout.settings-page`).

### Build

- Milestone: external — stays in lunegit.

## Open questions

- Which controls the pages use besides the existing set: the eight section views were not read; open.

## Notes for the planner

Compare `layout.settings-page`; the shell and rows there are the reusable part of this entry.

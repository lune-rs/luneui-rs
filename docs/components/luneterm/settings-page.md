---
id: luneterm.settings-page
title: Settings page layout
area: luneterm
status: planned
consumers: luneterm
source: —
owner: —
summary: Settings page built from toolkit widgets, live-applied and mirroring Lunegit's TerminalSettings (Luneterm M10)
---

## What it is

Luneterm M10 lists a live settings page with persistence. The widgets exist (`widget.toggle`, `widget.combo`, `widget.stepper`, `widget.text-field`); a page layout for them is not in `lune-ui` today per the inventory.

## Needs

- Existing form widgets (R-widget-1).
- Layout for sections and rows (R-layout-1).

## Existing code

The glass-ui spike has `spikes/glass-ui/src/settings_view.rs` (file exists; not read for this entry). Lunegit: `src/Lunegit.Desktop/Views/SettingsPageView.axaml` and `Views/Settings/*SettingsView.axaml`.

## Open questions

- Persistence (JSON, atomic, versioned) is app logic and out of scope for the toolkit.
- Whether a generic settings-page component exists or each app builds its own from widgets and layout containers.

## Notes for the planner

See `lunegit.settings-pages` for the Lunegit side. Roadmap scope: M10 row.

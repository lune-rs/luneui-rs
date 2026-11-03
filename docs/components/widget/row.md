---
id: widget.row
title: Row (setting row, list item row)
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Title with description and a right-hand control, or a compact item row with lead, name, meta and value
---

## What it is

Title with description and a right-hand control, or a compact item row with lead, name, meta and value. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Hover highlights the row; the control inside takes the input. Rows are not themselves clickable.

## Needs

- Wane — Variants: setting (title, description, control at the right; min height 56); item (compact, 32 high: lead dot, toggle or checkbox, name, meta, status, value); cols (`--cols`: toggle lead, title, badge, value in four fixed columns); meta mono (`__meta--mono`, RznMono 11 secondary); extra (`__extra`: a control under the title, no control at the right). (R-widget-4)
- Wane — States: rest; hover (`--surface-hover`); off (`is-off`: title secondary, value tertiary); disabled (`is-disabled`: opacity .4, inert). (R-widget-4)
- Wane — Sizes: Radius 6; setting padding 10 12, min height 56, title 13/20, description 11/16 secondary; item min height 32, padding 0 12, gap 8; cols grid 40 / 1fr / 88 / 176, padding 8 12; rows 2 apart; values RznMono 13. (R-widget-4)
- Wane — Behaviour: Hover highlights the row; the control inside takes the input. Rows are not themselves clickable. (R-widget-4)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: partial: the settings row exists as app code, not a widget (spikes/glass-ui/src/settings_view.rs:30,280)). Design source (not code to move): Luneterm spikes/glass-ui/src/settings_view.rs:290-345 (row: title 13 + description, control right, padding 10 12, hover radius 6).

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `widget.row`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run, requests, savings, settings.

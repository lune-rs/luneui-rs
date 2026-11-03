---
id: widget.nav-item
title: Navigation item
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Sidebar row: 16 px icon and a 13 px label
---

## What it is

Sidebar row: 16 px icon and a 13 px label. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Click navigates; Up and Down move focus; the active row marks the current screen (`aria-current`). Order: Overview, Sessions, Requests, Upstreams, Routes, Pricing, Local models, Token savings, Settings.

## Needs

- Wane — States: rest (secondary text); hover (`--surface-hover`, primary text); active (`--surface-selected`, bright, weight 500, no accent bar); focus. (R-widget-4)
- Wane — Sizes: Height 32 (`--nav-h`), radius 6, padding 0 8, gap 8, icon 16, label 13; rows 2 apart. (R-widget-4)
- Wane — Behaviour: Click navigates; Up and Down move focus; the active row marks the current screen (`aria-current`). Order: Overview, Sessions, Requests, Upstreams, Routes, Pricing, Local models, Token savings, Settings. (R-widget-4)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Luneterm spikes/glass-ui/src/settings_view.rs:229-242 (nav row, selection fill); Luneium prototypes/mockup/ui/styles.css:188 (`.ws-button`: 32 high, radius 6, hover fill).

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `widget.nav-item`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, local-models, overview, pricing, requests, routes, savings, sessions, settings, upstreams.

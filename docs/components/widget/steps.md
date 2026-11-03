---
id: widget.steps
title: Wizard steps
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Numbered step list with done, current and upcoming states (first run)
---

## What it is

Numbered step list with done, current and upcoming states (first run). Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Not clickable: Back and Continue move through the steps. Lives in the sidebar slot.

## Needs

- Wane — Variants: none; four steps in the mocks (R-widget-4)
- Wane — States: upcoming (tertiary, ring number); current (`--surface-selected`, bright, 500); done (accent circle with a 12 px check, secondary text). (R-widget-4)
- Wane — Sizes: Rows 32, radius 6, 2 apart, padding 0 8, gap 8; number circle 20 (1 px `--border`, 11 Medium); label 13 in a 148 px column that never wraps (a long label is shortened). (R-widget-4)
- Wane — Behaviour: Not clickable: Back and Continue move through the steps. Lives in the sidebar slot. (R-widget-4)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Wane `docs/ui/mocks/css/components.css` (widget.steps), the same row language as `widget.nav-item`.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).

## Notes for the planner

Wane priority P2 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `widget.steps`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, first-run.

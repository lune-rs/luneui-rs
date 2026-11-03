---
id: widget.radio-group
title: Radio group
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Vertical group of mutually exclusive options
---

## What it is

Vertical group of mutually exclusive options. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Click selects; Arrow keys move the selection inside the group; Tab enters the group once. Used by Settings, "Run the proxy" (three options).

## Needs

- Wane — Variants: none (vertical only) (R-widget-4)
- Wane — States: unselected; selected (accent fill, 6 px dot in `--on-accent`); hover; focus; disabled (opacity .4). (R-widget-4)
- Wane — Sizes: Circle 16 x 16, 1 px `--text-secondary` ring; row min height 28; label 13 secondary, gap 8. (R-widget-4)
- Wane — Behaviour: Click selects; Arrow keys move the selection inside the group; Tab enters the group once. Used by Settings, "Run the proxy" (three options). (R-widget-4)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Lunegit Desktop App/src/Lunegit.Desktop/Themes/DarkTheme.axaml:190-197 (accent ring, white dot).

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `widget.radio-group`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, settings.

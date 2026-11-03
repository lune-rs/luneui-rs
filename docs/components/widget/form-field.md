---
id: widget.form-field
title: Form field (label, control, hint)
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Label over a control with a hint line and an error state
---

## What it is

Label over a control with a hint line and an error state. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: The label is associated with the control (click focuses it). Validation timing is the owner's.

## Needs

- Wane — Variants: stacked (default); inline (`--inline`, label at the left, for header selects) (R-widget-4)
- Wane — States: normal; error (`is-error`: 1 px `--err` ring on the control, hint in `--err`, no glow). (R-widget-4)
- Wane — Sizes: Column gap 4; label 13/20 secondary; hint 11/16 tertiary; the control keeps its own size. (R-widget-4)
- Wane — Behaviour: The label is associated with the control (click focuses it). Validation timing is the owner's. (R-widget-4)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Wane `docs/ui/mocks/css/components.css` (widget.form-field); no lune-ui or Lunegit equivalent.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `widget.form-field`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, local-models, overview, pricing, requests, routes, savings, sessions, upstreams.

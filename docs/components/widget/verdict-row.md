---
id: widget.verdict-row
title: Verdict check row
area: widget
status: planned
consumers: wane
source: —
owner: —
summary: Check result row: tone mark, name, text (quant runnability checks)
---

## What it is

Check result row: tone mark, name, text (quant runnability checks). Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static.

## Needs

- Wane — Variants: three-column (default); stacked (`--stacked`: mark and name on line 1, text below in 11 px secondary, hairlines between rows, for a ~214 px column) (R-widget-4)
- Wane — States: mark OK (ok), WARN (warn), NO (err); the tone is on the mark only. (R-widget-4)
- Wane — Sizes: Rows min 24; grid 40 / 92 / 1fr; mark RznMono 11 Medium; name 13 tertiary; text 13/20 primary. Stacked: grid 40 / 1fr, padding 4 0. (R-widget-4)
- Wane — Behaviour: Static. (R-widget-4)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Wane `docs/ui/mocks/css/components.css` (widget.verdict-row); no lune-ui source.

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).

## Notes for the planner

Wane priority P1 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `widget.verdict-row`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, local-models.

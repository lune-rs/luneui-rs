---
id: data.stat
title: Stat cell
area: data
status: planned
consumers: wane
source: —
owner: —
summary: A labelled figure in a strip: label, mono value, sub-line, optional meter
---

## What it is

A labelled figure in a strip: label, mono value, sub-line, optional meter. Named by Wane's UI mocks (2026-11-04); the look is Lunegit's dark glass. Behaviour: Static. Not a card: a strip (`c-panel--strip`) of cells.

## Needs

- Wane — Variants: default; gold (`--gold`, the net-saved figure only, D6); text (value in sans 14, for a word); compact (flyout: padding 6 8, value 14/20); meter (`__meter`: a `data.progress` closes the cell). (R-data-2)
- Wane — States: no data: value "—", sub-line "No data" (no gold on a placeholder); sub-line tone ok or warn for a good or bad delta. (R-data-2)
- Wane — Sizes: Padding 12 16; label 11/16 tertiary; value RznMono Medium 20/28 tabular, bright, ellipsis; sub-line 11/16; strip cells share the width with a 1 px hairline between them. (R-data-2)
- Wane — Behaviour: Static. Not a card: a strip (`c-panel--strip`) of cells. (R-data-2)

## Existing code

None in luneui-rs or Luneterm's `lune-ui` (Wane: missing). Design source (not code to move): Wane `docs/ui/mocks/css/components.css` (data.stat); no lune-ui source. Value at most 22 px (A4).

## Open questions

- Whether this is a toolkit component or app code composed from the painter: the planner decides (Wane needs it either way).
- Native rendering: Needs tabular figures from RznMono; none otherwise.

## Notes for the planner

Wane priority P0 (P0 = needed by Overview, Requests or Settings). Full spec: Wane `docs/ui/components.md`, section `data.stat`; mock: Wane `docs/ui/mocks/components.html`. Screens using it: components, local-models, overview, pricing, savings, sessions, tray.
